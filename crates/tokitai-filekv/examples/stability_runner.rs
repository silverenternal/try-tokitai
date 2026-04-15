//! Stability runner example for long-running stability tests
//!
//! This is a standalone binary that runs continuous put/get/delete operations
//! for a configurable duration to stress-test FileKV under sustained load.
//!
//! Usage:
//!   cargo run --release --example stability_runner [DURATION_SECONDS]
//!
//! Example:
//!   cargo run --release --example stability_runner 3600  # 1 hour

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::env;

use tokitai_filekv::{FileKV, FileKVConfig};

fn main() -> anyhow::Result<()> {
    let duration_secs: u64 = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(300); // Default: 5 minutes

    println!("=== FileKV Stability Runner ===");
    println!("Duration: {}s ({:.1} minutes)", duration_secs, duration_secs as f64 / 60.0);
    println!();

    let temp_dir = tempfile::tempdir()?;
    let mut config = FileKVConfig::default();
    config.segment_dir = temp_dir.path().join("segments");
    config.wal_dir = temp_dir.path().join("wal");
    config.index_dir = temp_dir.path().join("index");
    config.enable_wal = true;
    config.aggressive.wal_sync_mode = tokitai_filekv::WalSyncMode::Batch;

    let kv = FileKV::open(config)?;
    let kv = Arc::new(kv);

    let running = Arc::new(AtomicBool::new(true));
    let total_puts = Arc::new(AtomicU64::new(0));
    let total_gets = Arc::new(AtomicU64::new(0));
    let total_deletes = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));
    let key_count = Arc::new(AtomicU64::new(0));

    // Writer thread
    let writer_running = running.clone();
    let writer_puts = total_puts.clone();
    let writer_errors = total_errors.clone();
    let writer_keys = key_count.clone();
    let writer_kv = kv.clone();
    let writer_handle = std::thread::spawn(move || {
        let mut batch = Vec::new();
        while writer_running.load(Ordering::Relaxed) {
            // Write in batches of 100
            batch.clear();
            let base = writer_keys.fetch_add(100, Ordering::Relaxed);
            for i in 0..100 {
                batch.push((format!("key_{}", base + i), format!("value_{}", base + i)));
            }

            for (key, value) in &batch {
                if let Err(e) = writer_kv.put(key, value.as_bytes()) {
                    writer_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Put error: {}", e);
                }
            }
            writer_puts.fetch_add(100, Ordering::Relaxed);

            // Flush every 1000 puts
            if writer_puts.load(Ordering::Relaxed) % 1000 == 0 {
                if let Err(e) = writer_kv.flush_memtable() {
                    eprintln!("Flush error: {}", e);
                }
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    });

    // Reader thread
    let reader_running = running.clone();
    let reader_gets = total_gets.clone();
    let reader_errors = total_errors.clone();
    let reader_keys = key_count.clone();
    let reader_kv = kv.clone();
    let reader_handle = std::thread::spawn(move || {
        while reader_running.load(Ordering::Relaxed) {
            let current_keys = reader_keys.load(Ordering::Relaxed);
            if current_keys == 0 {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            // Read random existing keys
            for i in 0..50 {
                let key_idx = i % current_keys;
                let key = format!("key_{}", key_idx);
                match reader_kv.get(&key) {
                    Ok(Some(_)) => { reader_gets.fetch_add(1, Ordering::Relaxed); }
                    Ok(None) => { reader_gets.fetch_add(1, Ordering::Relaxed); }
                    Err(e) => {
                        reader_errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("Get error: {}", e);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    });

    // Delete thread
    let deleter_running = running.clone();
    let deleter_deletes = total_deletes.clone();
    let _deleter_errors = total_errors.clone();
    let deleter_keys = key_count.clone();
    let deleter_kv = kv.clone();
    let deleter_handle = std::thread::spawn(move || {
        while deleter_running.load(Ordering::Relaxed) {
            let current_keys = deleter_keys.load(Ordering::Relaxed);
            if current_keys < 100 {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }

            // Delete old keys (keep recent ones)
            for i in 0..10 {
                let key = format!("key_{}", i);
                if deleter_kv.delete(&key).is_ok() {
                    deleter_deletes.fetch_add(1, Ordering::Relaxed);
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    });

    // Monitor loop
    let start = Instant::now();
    let deadline = start + Duration::from_secs(duration_secs);
    let mut last_report = Instant::now();

    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_secs(10));

        if last_report.elapsed() >= Duration::from_secs(30) {
            let elapsed = start.elapsed().as_secs();
            let puts = total_puts.load(Ordering::Relaxed);
            let gets = total_gets.load(Ordering::Relaxed);
            let deletes = total_deletes.load(Ordering::Relaxed);
            let errors = total_errors.load(Ordering::Relaxed);
            let keys = key_count.load(Ordering::Relaxed);
            let stats = kv.get_stats();

            println!(
                "[{:>5}s] puts={:>8} gets={:>8} dels={:>5} errors={:>3} keys={:>8} segments={:>3} size={:.1}MB",
                elapsed,
                puts,
                gets,
                deletes,
                errors,
                keys,
                stats.segment_count,
                stats.total_size_bytes as f64 / 1_048_576.0,
            );
            last_report = Instant::now();
        }
    }

    // Signal threads to stop
    running.store(false, Ordering::SeqCst);
    writer_handle.join().ok();
    reader_handle.join().ok();
    deleter_handle.join().ok();

    // Final stats
    let elapsed = start.elapsed();
    let puts = total_puts.load(Ordering::Relaxed);
    let gets = total_gets.load(Ordering::Relaxed);
    let deletes = total_deletes.load(Ordering::Relaxed);
    let errors = total_errors.load(Ordering::Relaxed);

    println!();
    println!("=== Stability Test Complete ===");
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("Total puts:    {}", puts);
    println!("Total gets:    {}", gets);
    println!("Total deletes: {}", deletes);
    println!("Total errors:  {}", errors);
    println!("Ops/sec:       {:.0}", (puts + gets + deletes) as f64 / elapsed.as_secs_f64());

    if errors > 0 {
        println!("STATUS: FAILED ({} errors)", errors);
        std::process::exit(1);
    } else {
        println!("STATUS: PASSED");
    }

    Ok(())
}
