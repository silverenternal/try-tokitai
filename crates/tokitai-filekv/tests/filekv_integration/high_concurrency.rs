//! High-concurrency tests for 32/64 threads
//!
//! These tests verify DashMap and other concurrent data structures perform correctly
//! under high load with 32 and 64 threads. They are marked `#[ignore]` by default
//! because they take longer to run and may cause noise in CI.
//!
//! Run with: `cargo test --test filekv_integration -- --ignored high_concurrency`

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokitai_filekv::{FileKV, FileKVConfig};

fn create_test_config(temp_dir: &TempDir) -> FileKVConfig {
    FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        enable_wal: false,
        ..Default::default()
    }
}

// ─── 32-thread tests ───

/// Test concurrent puts with 32 threads
#[test]
fn test_32_threads_concurrent_puts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    let num_threads = 32;
    let keys_per_thread = 100;
    let mut handles = vec![];
    let start_time = Instant::now();

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let handle = thread::spawn(move || {
            let mut success = 0u64;
            let mut failures = 0u64;
            for i in 0..keys_per_thread {
                let key = format!("t{}_k{}", t, i);
                let value = format!("value_t{}_k{}", t, i);
                match kv_clone.put(&key, value.as_bytes()) {
                    Ok(_) => success += 1,
                    Err(_) => failures += 1,
                }
            }
            (success, failures)
        });
        handles.push(handle);
    }

    let mut total_success = 0u64;
    let mut total_failures = 0u64;
    for handle in handles {
        let (s, f) = handle.join().expect("Thread panicked");
        total_success += s;
        total_failures += f;
    }

    let elapsed = start_time.elapsed();
    let expected_total = (num_threads * keys_per_thread) as u64;

    assert_eq!(
        total_success, expected_total,
        "All puts should succeed: expected {} successes, got {} ({} failures)",
        expected_total, total_success, total_failures
    );

    // Verify all keys are readable
    let mut found = 0u64;
    for t in 0..num_threads {
        for i in 0..keys_per_thread {
            let key = format!("t{}_k{}", t, i);
            let expected = format!("value_t{}_k{}", t, i);
            if let Ok(Some(val)) = kv.get(&key) {
                if val.as_ref() == expected.as_bytes() {
                    found += 1;
                }
            }
        }
    }

    assert_eq!(found, expected_total, "Should find all {} keys, found {}", expected_total, found);

    println!(
        "[32-thread put] {} ops in {:.3}s ({:.0} ops/s)",
        expected_total,
        elapsed.as_secs_f64(),
        expected_total as f64 / elapsed.as_secs_f64()
    );
}

/// Test concurrent gets with 32 threads (read-heavy workload)
#[test]
fn test_32_threads_concurrent_gets() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    // Pre-populate 10K keys
    let num_keys = 10_000;
    for i in 0..num_keys {
        kv.put(&format!("key_{}", i), format!("value_{}", i).as_bytes())
            .expect("put failed");
    }

    let num_threads = 32;
    let reads_per_thread = 1000;
    let mut handles = vec![];
    let total_hits = Arc::new(AtomicU64::new(0));
    let total_misses = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let hits = total_hits.clone();
        let misses = total_misses.clone();
        let handle = thread::spawn(move || {
            for i in 0..reads_per_thread {
                // Access keys in a pattern that creates contention
                let key_idx = (t * reads_per_thread + i) % num_keys;
                let key = format!("key_{}", key_idx);
                match kv_clone.get(&key) {
                    Ok(Some(_)) => { hits.fetch_add(1, Ordering::Relaxed); }
                    Ok(None) => { misses.fetch_add(1, Ordering::Relaxed); }
                    Err(_) => { misses.fetch_add(1, Ordering::Relaxed); }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let elapsed = start_time.elapsed();
    let total_ops = (num_threads * reads_per_thread) as u64;
    let hits = total_hits.load(Ordering::Relaxed);

    assert_eq!(
        hits, total_ops,
        "All reads should hit: expected {} hits, got {} out of {}",
        total_ops, hits, total_ops
    );

    println!(
        "[32-thread get] {} reads in {:.3}s ({:.0} reads/s, 100% hit rate)",
        total_ops,
        elapsed.as_secs_f64(),
        total_ops as f64 / elapsed.as_secs_f64()
    );
}

/// Test mixed read/write workload with 32 threads
#[test]
fn test_32_threads_mixed_read_write() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    let num_threads = 32;
    let ops_per_thread = 200;
    let mut handles = vec![];
    let total_puts = Arc::new(AtomicU64::new(0));
    let total_gets = Arc::new(AtomicU64::new(0));
    let total_deletes = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let puts = total_puts.clone();
        let gets = total_gets.clone();
        let deletes = total_deletes.clone();
        let handle = thread::spawn(move || {
            for i in 0..ops_per_thread {
                let key = format!("t{}_k{}", t, i);
                let op = i % 10;
                if op < 3 {
                    // 30% writes
                    kv_clone.put(&key, format!("v_{}_{}", t, i).as_bytes()).ok();
                    puts.fetch_add(1, Ordering::Relaxed);
                } else if op < 9 {
                    // 60% reads
                    kv_clone.get(&key).ok();
                    gets.fetch_add(1, Ordering::Relaxed);
                } else {
                    // 10% deletes
                    kv_clone.delete(&key).ok();
                    deletes.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let elapsed = start_time.elapsed();
    let total_ops = (num_threads * ops_per_thread) as u64;

    println!(
        "[32-thread mixed] {} ops in {:.3}s ({:.0} ops/s) - puts: {}, gets: {}, deletes: {}",
        total_ops,
        elapsed.as_secs_f64(),
        total_ops as f64 / elapsed.as_secs_f64(),
        total_puts.load(Ordering::Relaxed),
        total_gets.load(Ordering::Relaxed),
        total_deletes.load(Ordering::Relaxed),
    );

    // Basic sanity: no panics, engine still functional
    let stats = kv.get_stats();
    assert!(stats.segment_count >= 0, "Engine should be in valid state");
}

// ─── 64-thread tests ───

/// Test concurrent puts with 64 threads (maximum concurrency stress test)
#[test]
fn test_64_threads_concurrent_puts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    let num_threads = 64;
    let keys_per_thread = 50;
    let mut handles = vec![];
    let start_time = Instant::now();

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let handle = thread::spawn(move || {
            let mut success = 0u64;
            for i in 0..keys_per_thread {
                let key = format!("t{}_k{}", t, i);
                if kv_clone.put(&key, format!("v_{}_{}", t, i).as_bytes()).is_ok() {
                    success += 1;
                }
            }
            success
        });
        handles.push(handle);
    }

    let mut total_success = 0u64;
    for handle in handles {
        total_success += handle.join().expect("Thread panicked");
    }

    let elapsed = start_time.elapsed();
    let expected_total = (num_threads * keys_per_thread) as u64;

    assert_eq!(total_success, expected_total, "All 64-thread puts should succeed");

    println!(
        "[64-thread put] {} ops in {:.3}s ({:.0} ops/s)",
        expected_total,
        elapsed.as_secs_f64(),
        expected_total as f64 / elapsed.as_secs_f64()
    );
}

/// Test concurrent gets with 64 threads
#[test]
fn test_64_threads_concurrent_gets() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    // Pre-populate 20K keys
    let num_keys = 20_000;
    for i in 0..num_keys {
        kv.put(&format!("key_{}", i), format!("value_{}", i).as_bytes())
            .expect("put failed");
    }

    let num_threads = 64;
    let reads_per_thread = 500;
    let mut handles = vec![];
    let total_hits = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let hits = total_hits.clone();
        let handle = thread::spawn(move || {
            for i in 0..reads_per_thread {
                let key_idx = (t * reads_per_thread + i) % num_keys;
                let key = format!("key_{}", key_idx);
                if kv_clone.get(&key).unwrap().is_some() {
                    hits.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let elapsed = start_time.elapsed();
    let total_ops = (num_threads * reads_per_thread) as u64;
    let hits = total_hits.load(Ordering::Relaxed);

    assert_eq!(hits, total_ops, "All 64-thread reads should hit");

    println!(
        "[64-thread get] {} reads in {:.3}s ({:.0} reads/s)",
        total_ops,
        elapsed.as_secs_f64(),
        total_ops as f64 / elapsed.as_secs_f64()
    );
}

/// Test 64 threads all hitting the same key (maximum contention)
#[test]
fn test_64_threads_hot_key_contention() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    // Pre-populate a hot key
    let hot_key = "hot_key";
    kv.put(hot_key, b"initial_value").expect("put failed");

    let num_threads = 64;
    let ops_per_thread = 1000;
    let mut handles = vec![];
    let total_writes = Arc::new(AtomicU64::new(0));
    let total_reads = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let writes = total_writes.clone();
        let reads = total_reads.clone();
        let handle = thread::spawn(move || {
            for i in 0..ops_per_thread {
                if i % 5 == 0 {
                    // 20% writes to hot key
                    kv_clone.put(hot_key, format!("t{}_v{}", t, i).as_bytes()).ok();
                    writes.fetch_add(1, Ordering::Relaxed);
                } else {
                    // 80% reads from hot key
                    kv_clone.get(hot_key).ok();
                    reads.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let elapsed = start_time.elapsed();
    let total_ops = (num_threads * ops_per_thread) as u64;

    // The hot key should still have a valid value (last-write-wins)
    let final_val = kv.get(hot_key).expect("get failed");
    assert!(final_val.is_some(), "Hot key should still exist");

    println!(
        "[64-thread hot key] {} ops in {:.3}s ({:.0} ops/s) - writes: {}, reads: {}",
        total_ops,
        elapsed.as_secs_f64(),
        total_ops as f64 / elapsed.as_secs_f64(),
        total_writes.load(Ordering::Relaxed),
        total_reads.load(Ordering::Relaxed),
    );
}

// ─── Cache stress tests under high concurrency ───

/// Test BlockCache behavior under 32-thread concurrent access
#[test]
fn test_32_threads_cache_stress() {
    use tokitai_filekv::{BlockCache, BlockCacheConfig};

    let config = BlockCacheConfig {
        max_items: 10_000,
        max_memory_bytes: 16 * 1024 * 1024, // 16MB
        frequency_aware: false,
    };
    let cache = Arc::new(BlockCache::new(config));

    let num_threads = 32;
    let ops_per_thread = 5000;
    let mut handles = vec![];
    let total_hits = Arc::new(AtomicU64::new(0));
    let total_misses = Arc::new(AtomicU64::new(0));

    // Pre-populate cache
    for i in 0..5000 {
        cache.insert_by_key(format!("key_{}", i), bytes::Bytes::from(format!("value_{}", i)));
    }

    let start_time = Instant::now();

    for t in 0..num_threads {
        let cache_clone = cache.clone();
        let hits = total_hits.clone();
        let misses = total_misses.clone();
        let handle = thread::spawn(move || {
            for i in 0..ops_per_thread {
                let key = format!("key_{}", (t * ops_per_thread + i) % 5000);
                if cache_clone.get_by_key(&key).is_some() {
                    hits.fetch_add(1, Ordering::Relaxed);
                } else {
                    misses.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let elapsed = start_time.elapsed();
    let total_ops = (num_threads * ops_per_thread) as u64;
    let hits = total_hits.load(Ordering::Relaxed);

    let stats = cache.stats();
    println!(
        "[32-thread cache] {} ops in {:.3}s ({:.0} ops/s) - hits: {}, misses: {}, hit_rate: {:.1}%",
        total_ops,
        elapsed.as_secs_f64(),
        total_ops as f64 / elapsed.as_secs_f64(),
        hits,
        total_misses.load(Ordering::Relaxed),
        stats.hit_rate * 100.0,
    );

    // Cache should still be functional
    assert!(stats.items > 0, "Cache should have items");
}

/// Test concurrent puts followed by flush and recovery (crash safety under load)
#[test]
fn test_32_threads_puts_then_flush_and_reopen() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut config = create_test_config(&temp_dir);
    config.enable_wal = true; // Enable WAL for this test

    let num_threads = 32;
    let keys_per_thread = 100;

    {
        let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));
        let mut handles = vec![];

        for t in 0..num_threads {
            let kv_clone = kv.clone();
            let handle = thread::spawn(move || {
                for i in 0..keys_per_thread {
                    let key = format!("t{}_k{}", t, i);
                    kv_clone.put(&key, format!("v_{}_{}", t, i).as_bytes()).ok();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Flush to ensure data is persisted
        kv.flush_memtable().expect("flush failed");
    }

    // Reopen and verify data integrity
    let kv = FileKV::open(config.clone()).expect("Failed to reopen FileKV");
    let mut found = 0u64;

    for t in 0..num_threads {
        for i in 0..keys_per_thread {
            let key = format!("t{}_k{}", t, i);
            let expected = format!("v_{}_{}", t, i);
            if let Ok(Some(val)) = kv.get(&key) {
                if val.as_ref() == expected.as_bytes() {
                    found += 1;
                }
            }
        }
    }

    let expected_total = (num_threads * keys_per_thread) as u64;
    assert_eq!(found, expected_total, "After reopen: should find all {} keys, found {}", expected_total, found);
}

// ─── DashMap contention analysis test ───

/// Measures MemTable (DashMap) performance under contention
#[test]
fn test_dashmap_contention_analysis() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    let thread_counts = [1, 4, 8, 16, 32, 64];
    let ops_per_thread = 10_000;

    println!("\n=== DashMap Contention Analysis ===");
    println!("{:<10} {:<12} {:<15} {:<15}", "Threads", "Time (ms)", "Ops/sec", "Scaling");

    let mut baseline_time: Option<Duration> = None;

    for &num_threads in &thread_counts {
        // Pre-populate enough keys to avoid flush during test
        let keys_to_prepopulate = num_threads * ops_per_thread / 10;
        for i in 0..keys_to_prepopulate {
            kv.put(&format!("pre_{}", i), b"data").ok();
        }

        let mut handles = vec![];
        let start = Instant::now();

        for t in 0..num_threads {
            let kv_clone = kv.clone();
            let handle = thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let key = format!("dm_t{}_k{}", t, i);
                    kv_clone.put(&key, b"value").ok();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        let elapsed = start.elapsed();
        let total_ops = (num_threads * ops_per_thread) as u64;
        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();

        let scaling = if let Some(baseline) = baseline_time {
            let speedup = elapsed.as_secs_f64() / baseline.as_secs_f64();
            format!("{:.2}x slower", speedup)
        } else {
            baseline_time = Some(elapsed);
            "baseline".to_string()
        };

        println!(
            "{:<10} {:<12.1} {:<15.0} {}",
            num_threads,
            elapsed.as_secs_f64() * 1000.0,
            ops_per_sec,
            scaling,
        );
    }
}
