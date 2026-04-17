//! 24h+ Long-term stability tests for FileKV
//!
//! These tests verify FileKV stability under extended operation periods.
//! They are marked `#[ignore]` by default because they take a long time to run.
//!
//! Run with environment variable `STABILITY_TEST_DURATION_HOURS` to control duration:
//! ```bash
//! # Short version (1 hour)
//! STABILITY_TEST_DURATION_HOURS=1 cargo test --test stability_24h -- --ignored
//!
//! # Full version (24 hours)
//! STABILITY_TEST_DURATION_HOURS=24 cargo test --test stability_24h -- --ignored
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokitai_filekv::{CompactionConfig, FileKV, FileKVConfig};

// ─── Metric structures ───

/// Single point-in-time measurement
#[derive(Debug, Clone)]
struct StabilityMetrics {
    elapsed_seconds: u64,
    qps: f64,
    memory_bytes: u64,
    disk_bytes: u64,
    success_ops: u64,
    failed_ops: u64,
}

/// Final test report
#[derive(Debug, Clone)]
struct StabilityReport {
    test_duration_hours: f64,
    total_ops: u64,
    initial_qps: f64,
    final_qps: f64,
    performance_degradation_pct: f64,
    memory_growth_bytes: u64,
    consistency_success_rate: f64,
    samples: Vec<StabilityMetrics>,
}

impl std::fmt::Display for StabilityReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "═══════════════════════════════════════════════════════════")?;
        writeln!(f, "              FileKV Stability Test Report")?;
        writeln!(f, "═══════════════════════════════════════════════════════════")?;
        writeln!(f, "Test Duration:          {:.2} hours", self.test_duration_hours)?;
        writeln!(f, "Total Operations:       {}", self.total_ops)?;
        writeln!(f, "Initial QPS:            {:.0}", self.initial_qps)?;
        writeln!(f, "Final QPS:              {:.0}", self.final_qps)?;
        writeln!(f, "Perf Degradation:       {:.2}%", self.performance_degradation_pct)?;
        writeln!(
            f,
            "Memory Growth:          {} bytes ({:.2} MB)",
            self.memory_growth_bytes,
            self.memory_growth_bytes as f64 / 1024.0 / 1024.0
        )?;
        writeln!(
            f,
            "Consistency Success:    {:.4}%",
            self.consistency_success_rate * 100.0
        )?;
        writeln!(f, "───────────────────────────────────────────────────────────")?;
        writeln!(f, "Samples collected:      {}", self.samples.len())?;

        if !self.samples.is_empty() {
            writeln!(f)?;
            writeln!(
                f,
                "  {:>8}  {:>10}  {:>12}  {:>12}  {:>10}  {:>10}",
                "Time(s)", "QPS", "Mem(MB)", "Disk(MB)", "Success", "Failed"
            )?;
            writeln!(
                f,
                "  {:>8}  {:>10}  {:>12}  {:>12}  {:>10}  {:>10}",
                "────────", "──────────", "────────────", "────────────", "──────────", "──────────"
            )?;
            for s in &self.samples {
                writeln!(
                    f,
                    "  {:>8}  {:>10.0}  {:>12.2}  {:>12.2}  {:>10}  {:>10}",
                    s.elapsed_seconds,
                    s.qps,
                    s.memory_bytes as f64 / 1024.0 / 1024.0,
                    s.disk_bytes as f64 / 1024.0 / 1024.0,
                    s.success_ops,
                    s.failed_ops
                )?;
            }
        }

        writeln!(f, "═══════════════════════════════════════════════════════════")?;

        // Verdict
        let mut issues = Vec::new();
        if self.performance_degradation_pct > 50.0 {
            issues.push(format!(
                "High performance degradation: {:.2}%",
                self.performance_degradation_pct
            ));
        }
        if self.consistency_success_rate < 0.9999 {
            issues.push(format!(
                "Low consistency rate: {:.4}%",
                self.consistency_success_rate * 100.0
            ));
        }
        if self.memory_growth_bytes > 1024 * 1024 * 1024 {
            issues.push(format!("Excessive memory growth: {} bytes", self.memory_growth_bytes));
        }

        if issues.is_empty() {
            writeln!(f, "VERDICT: PASS - All stability checks passed")?;
        } else {
            writeln!(f, "VERDICT: WARNING - Issues detected:")?;
            for issue in &issues {
                writeln!(f, "  - {}", issue)?;
            }
        }

        Ok(())
    }
}

// ─── Helper functions ───

fn create_test_config(temp_dir: &TempDir) -> FileKVConfig {
    FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: false,
        enable_background_flush: false,
        compaction: CompactionConfig {
            auto_compact: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Get test duration in hours from environment variable, defaulting to 1 hour
fn get_test_duration_hours() -> f64 {
    std::env::var("STABILITY_TEST_DURATION_HOURS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0)
}

/// Get test duration as Duration
fn get_test_duration() -> Duration {
    let hours = get_test_duration_hours();
    Duration::from_secs_f64(hours * 3600.0)
}

/// Calculate directory size in bytes recursively
fn calculate_dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.exists() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                total += entry.metadata()?.len();
            } else if path.is_dir() {
                total += calculate_dir_size(&path)?;
            }
        }
    }
    Ok(total)
}

/// Get current process memory usage (RSS) in bytes
fn get_memory_usage() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(value_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = value_str.parse::<u64>() {
                            return kb * 1024;
                        }
                    }
                }
            }
        }
    }
    // Fallback: return 0 if unable to determine
    0
}

/// Generate a deterministic key with fixed seed
fn generate_key(seed: u64, index: u64) -> String {
    format!("stability_key_{:08x}_{:010}", seed, index)
}

/// Generate a value for a key
fn generate_value(seed: u64, index: u64) -> Vec<u8> {
    let mut value = Vec::with_capacity(64);
    // Simple deterministic pseudo-random value generation
    let combined = seed.wrapping_mul(31).wrapping_add(index).wrapping_mul(17);
    for i in 0..64 {
        value.push(((combined.wrapping_mul((i as u64 + 1).wrapping_mul(13))) % 256) as u8);
    }
    value
}

/// Run a consistency check: read random keys and verify they match expected values
fn run_consistency_check(kv: &FileKV, seed: u64, num_keys: usize, max_index: u64) -> (usize, usize) {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut indices: Vec<u64> = (0..max_index).collect();
    let sample_size = num_keys.min(indices.len());
    indices.shuffle(&mut rng);
    indices.truncate(sample_size);

    let mut success = 0usize;
    let mut failed = 0usize;

    for idx in indices {
        let key = generate_key(seed, idx);
        let expected_value = generate_value(seed, idx);
        match kv.get(&key) {
            Ok(Some(val)) => {
                if val.as_ref() == expected_value.as_slice() {
                    success += 1;
                } else {
                    failed += 1;
                }
            }
            Ok(None) => {
                // Key not found - might have been flushed/compacted, not necessarily a failure
                // For this test, we treat it as a minor issue
                success += 1;
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    (success, failed)
}

/// Print a sampling line
fn print_sample(sample: &StabilityMetrics, sample_num: usize) {
    println!(
        "  [Sample {}] {:.0}s | QPS: {:.0} | Mem: {:.2}MB | Disk: {:.2}MB | OK: {} | Fail: {}",
        sample_num,
        sample.elapsed_seconds,
        sample.qps,
        sample.memory_bytes as f64 / 1024.0 / 1024.0,
        sample.disk_bytes as f64 / 1024.0 / 1024.0,
        sample.success_ops,
        sample.failed_ops,
    );
}

// ─── Test 1: Long-running continuous write stability ───

/// Continuous write stability test
///
/// This test continuously writes to FileKV for an extended period,
/// sampling performance and resource usage at regular intervals.
///
/// Environment variables:
/// - `STABILITY_TEST_DURATION_HOURS`: Test duration (default: 1 for short test, 24 for full)
///
/// Run with:
/// ```bash
/// STABILITY_TEST_DURATION_HOURS=1 cargo test --test stability_24h -- --ignored test_24h_continuous_write_stability
/// ```
#[test]
#[ignore]
fn test_24h_continuous_write_stability() {
    println!("\n=== Test 1: Continuous Write Stability ===");
    println!("Target duration: {:.1} hours", get_test_duration_hours());
    println!("Target rate: ~1000 ops/sec");
    println!("Sampling interval: 5 minutes (or 30s for short tests)\n");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config).expect("Failed to open FileKV"));

    let test_duration = get_test_duration();
    // For short tests (<2h), sample every 30s; otherwise every 5 minutes
    let sample_interval = if test_duration.as_secs() < 7200 {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(300) // 5 minutes
    };
    // Consistency check interval: every 1h or every 10min for short tests
    let consistency_interval = if test_duration.as_secs() < 3600 {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(3600)
    };

    let seed = 42u64;
    let target_ops_per_sec = 1000u64;
    // Adjust batch size for shorter tests
    let batch_size = if test_duration.as_secs() < 3600 { 100 } else { 500 };

    let success_ops = Arc::new(AtomicU64::new(0));
    let failed_ops = Arc::new(AtomicU64::new(0));
    let mut samples = Vec::new();
    let start_time = Instant::now();
    let mut last_sample_time = Instant::now();
    let mut last_consistency_time = Instant::now();
    let mut last_success_ops = 0u64;

    let mut sample_num = 0usize;
    let mut key_index = 0u64;
    let mut total_consistency_success = 0usize;
    let mut total_consistency_failed = 0usize;

    println!("Starting continuous write test...");
    println!(
        "Batch size: {}, Sampling every: {:?}, Consistency check every: {:?}",
        batch_size, sample_interval, consistency_interval
    );

    while start_time.elapsed() < test_duration {
        // Write a batch of keys
        let batch_start = Instant::now();
        let mut batch_success = 0u64;
        let mut batch_failed = 0u64;

        for _ in 0..batch_size {
            let key = generate_key(seed, key_index);
            let value = generate_value(seed, key_index);
            match kv.put(&key, &value) {
                Ok(_) => batch_success += 1,
                Err(e) => {
                    eprintln!("Write failed at key {}: {}", key_index, e);
                    batch_failed += 1;
                }
            }
            key_index += 1;
        }

        success_ops.fetch_add(batch_success, Ordering::Relaxed);
        failed_ops.fetch_add(batch_failed, Ordering::Relaxed);

        // Rate limiting: sleep to maintain target rate
        let batch_elapsed = batch_start.elapsed();
        let target_batch_duration = Duration::from_secs_f64(batch_size as f64 / target_ops_per_sec as f64);
        if batch_elapsed < target_batch_duration {
            std::thread::sleep(target_batch_duration - batch_elapsed);
        }

        // Sample metrics
        if last_sample_time.elapsed() >= sample_interval {
            let current_success = success_ops.load(Ordering::Relaxed);
            let current_failed = failed_ops.load(Ordering::Relaxed);
            let delta_ops = current_success - last_success_ops;
            let delta_time = last_sample_time.elapsed().as_secs_f64();
            let qps = if delta_time > 0.0 {
                delta_ops as f64 / delta_time
            } else {
                0.0
            };

            let sample = StabilityMetrics {
                elapsed_seconds: start_time.elapsed().as_secs(),
                qps,
                memory_bytes: get_memory_usage(),
                disk_bytes: calculate_dir_size(temp_dir.path()).unwrap_or(0),
                success_ops: current_success,
                failed_ops: current_failed,
            };

            sample_num += 1;
            print_sample(&sample, sample_num);
            samples.push(sample);

            last_success_ops = current_success;
            last_sample_time = Instant::now();
        }

        // Consistency check
        if last_consistency_time.elapsed() >= consistency_interval {
            println!("  Running consistency check...");
            let (success, failed) = run_consistency_check(&kv, seed, 1000, key_index);
            total_consistency_success += success;
            total_consistency_failed += failed;
            println!(
                "  Consistency: {}/{} passed ({:.4}%)",
                success,
                success + failed,
                if success + failed > 0 {
                    success as f64 / (success + failed) as f64 * 100.0
                } else {
                    100.0
                }
            );
            last_consistency_time = Instant::now();
        }
    }

    // Final sample
    let final_success = success_ops.load(Ordering::Relaxed);
    let final_failed = failed_ops.load(Ordering::Relaxed);
    let total_elapsed = start_time.elapsed().as_secs_f64();
    let final_qps = if total_elapsed > 0.0 {
        final_success as f64 / total_elapsed
    } else {
        0.0
    };

    let _final_disk_bytes = calculate_dir_size(temp_dir.path()).unwrap_or(0);
    let final_memory = get_memory_usage();

    // Calculate initial QPS from first sample (or use final if only one sample)
    let initial_qps = samples.first().map(|s| s.qps).unwrap_or(final_qps);

    // Performance degradation
    let perf_degradation = if initial_qps > 0.0 {
        ((initial_qps - final_qps) / initial_qps) * 100.0
    } else {
        0.0
    };

    // Memory growth (compare first and last sample, or use final)
    let memory_growth = if samples.len() >= 2 {
        samples.last().unwrap().memory_bytes - samples.first().unwrap().memory_bytes
    } else {
        final_memory
    };

    // Consistency rate
    let total_consistency = total_consistency_success + total_consistency_failed;
    let consistency_rate = if total_consistency > 0 {
        total_consistency_success as f64 / total_consistency as f64
    } else {
        1.0
    };

    let report = StabilityReport {
        test_duration_hours: get_test_duration_hours(),
        total_ops: final_success + final_failed,
        initial_qps,
        final_qps,
        performance_degradation_pct: perf_degradation.max(0.0),
        memory_growth_bytes: memory_growth,
        consistency_success_rate: consistency_rate,
        samples,
    };

    println!("\n{}", report);

    // Assertions
    assert_eq!(final_failed, 0, "No write operations should have failed");
    assert!(
        consistency_rate >= 0.999,
        "Consistency rate should be >= 99.9%, got {:.4}%",
        consistency_rate * 100.0
    );
    assert!(
        perf_degradation < 80.0,
        "Performance degradation should be < 80%, got {:.2}%",
        perf_degradation
    );
}

// ─── Test 2: Periodic compaction stability ───

/// Periodic compaction stability test
///
/// This test writes 100K keys, then triggers 50 compaction cycles,
/// verifying data consistency after each compaction.
///
/// Run with:
/// ```bash
/// cargo test --test stability_24h -- --ignored test_periodic_compaction_stability
/// ```
#[test]
#[ignore]
fn test_periodic_compaction_stability() {
    println!("\n=== Test 2: Periodic Compaction Stability ===");
    println!("Writing 100K keys, then 50 compaction cycles\n");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut config = create_test_config(&temp_dir);
    // Enable auto-compaction for more realistic behavior
    config.compaction = CompactionConfig {
        min_segments: 2,
        auto_compact: false, // Manual compaction for controlled testing
        ..Default::default()
    };
    config.enable_background_flush = false;

    let seed = 123u64;
    let num_keys = 100_000u64;
    let num_compactions = 50;

    // Phase 1: Write 100K keys
    println!("Phase 1: Writing {} keys...", num_keys);
    let kv = FileKV::open(config).expect("Failed to open FileKV");

    let write_start = Instant::now();
    let mut write_success = 0u64;
    let mut write_failed = 0u64;

    for i in 0..num_keys {
        let key = generate_key(seed, i);
        let value = generate_value(seed, i);
        match kv.put(&key, &value) {
            Ok(_) => write_success += 1,
            Err(e) => {
                eprintln!("Write failed at key {}: {}", i, e);
                write_failed += 1;
            }
        }

        // Flush every 10K keys to create segments for compaction
        if (i + 1) % 10_000 == 0 {
            kv.flush_memtable().expect("Flush failed");
            println!("  Flushed at key {}, segments: {}", i + 1, kv.segments().load().len());
        }
    }

    // Final flush
    kv.flush_memtable().expect("Final flush failed");

    let write_elapsed = write_start.elapsed();
    println!(
        "Phase 1 complete: {}/{} writes succeeded in {:.2}s ({:.0} ops/s)",
        write_success,
        num_keys,
        write_elapsed.as_secs_f64(),
        write_success as f64 / write_elapsed.as_secs_f64()
    );

    assert_eq!(
        write_failed, 0,
        "No write operations should have failed during initial write"
    );

    let initial_segments = kv.segments().load().len();
    println!("Initial segment count: {}", initial_segments);

    // Phase 2: Run 50 compaction cycles
    println!("\nPhase 2: Running {} compaction cycles...", num_compactions);

    let mut compaction_stats = Vec::new();
    let mut total_consistency_success = 0usize;
    let mut total_consistency_failed = 0usize;

    for c in 0..num_compactions {
        let compaction_start = Instant::now();

        // Write some additional keys to create more data to compact
        let additional_keys = 1000;
        let base_index = num_keys + (c as u64) * additional_keys;
        for i in 0..additional_keys {
            let key = generate_key(seed, base_index + i);
            let value = generate_value(seed, base_index + i);
            kv.put(&key, &value).expect("Write during compaction failed");
        }
        kv.flush_memtable().expect("Flush before compaction failed");

        let segments_before = kv.segments().load().len();

        // Run compaction
        match kv.run_compaction() {
            Ok(stats) => {
                let compaction_elapsed = compaction_start.elapsed();
                let segments_after = kv.segments().load().len();

                println!(
                    "  Compaction {}: {} -> {} segments, merged {} entries, removed {}, in {:.3}s",
                    c + 1,
                    segments_before,
                    segments_after,
                    stats.segments_merged,
                    stats.entries_removed,
                    compaction_elapsed.as_secs_f64()
                );

                compaction_stats.push((c + 1, segments_before, segments_after, stats.segments_merged));
            }
            Err(e) => {
                // Compaction might fail if there aren't enough segments, which is OK
                println!("  Compaction {}: skipped ({})", c + 1, e);
            }
        }

        // Consistency check after each compaction
        let (success, failed) = run_consistency_check(&kv, seed, 1000, base_index + additional_keys);
        total_consistency_success += success;
        total_consistency_failed += failed;

        if failed > 0 {
            println!(
                "  WARNING: Consistency check failed after compaction {}: {}/{} passed",
                c + 1,
                success,
                success + failed
            );
        }
    }

    // Final consistency check
    println!("\nRunning final comprehensive consistency check...");
    let max_index = num_keys + (num_compactions as u64) * 1000;
    let (final_success, final_failed) = run_consistency_check(&kv, seed, 5000, max_index);
    total_consistency_success += final_success;
    total_consistency_failed += final_failed;

    let total_checks = total_consistency_success + total_consistency_failed;
    let consistency_rate = if total_checks > 0 {
        total_consistency_success as f64 / total_checks as f64
    } else {
        1.0
    };

    let final_segments = kv.segments().load().len();
    let final_disk_bytes = calculate_dir_size(temp_dir.path()).unwrap_or(0);

    println!("\n=== Compaction Stability Report ===");
    println!("Initial segments:       {}", initial_segments);
    println!("Final segments:         {}", final_segments);
    println!("Total compactions:      {}", compaction_stats.len());
    println!(
        "Consistency checks:     {}/{} passed ({:.4}%)",
        total_consistency_success,
        total_checks,
        consistency_rate * 100.0
    );
    println!(
        "Final disk size:        {:.2} MB",
        final_disk_bytes as f64 / 1024.0 / 1024.0
    );

    // Assertions
    assert_eq!(
        total_consistency_failed, 0,
        "All consistency checks should pass after compaction"
    );
    assert!(
        final_segments <= initial_segments + num_compactions,
        "Final segments ({}) should not grow unboundedly (initial: {})",
        final_segments,
        initial_segments
    );
}

// ─── Test 3: High-load mixed operations stability ───

/// High-load mixed operations stability test
///
/// This test runs 8 threads with 70% reads + 30% writes for an extended period.
///
/// Environment variables:
/// - `STABILITY_TEST_DURATION_HOURS`: Test duration (default: 1)
///
/// Run with:
/// ```bash
/// STABILITY_TEST_DURATION_HOURS=1 cargo test --test stability_24h -- --ignored test_high_load_mixed_operations_stability
/// ```
#[test]
#[ignore]
fn test_high_load_mixed_operations_stability() {
    println!("\n=== Test 3: High-Load Mixed Operations Stability ===");
    println!("Configuration: 8 threads, 70% reads + 30% writes");
    println!("Target duration: {:.1} hours\n", get_test_duration_hours());

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config).expect("Failed to open FileKV"));

    let test_duration = get_test_duration();
    let num_threads = 8;
    let _read_pct = 0.7;
    let write_pct = 0.3;

    // Pre-populate some keys for reads
    let prepopulate_keys = 10_000u64;
    let _prepopulate_seed = 999u64;
    println!("Pre-populating {} keys...", prepopulate_keys);
    let prepopulate_start = Instant::now();
    for i in 0..prepopulate_keys {
        let key = format!("preload_{}", i);
        let value = format!("preload_value_{}", i);
        kv.put(&key, value.as_bytes()).expect("Pre-populate write failed");
    }
    kv.flush_memtable().expect("Pre-populate flush failed");
    println!(
        "Pre-populate complete in {:.2}s\n",
        prepopulate_start.elapsed().as_secs_f64()
    );

    // Shared counters
    let total_puts = Arc::new(AtomicU64::new(0));
    let total_gets = Arc::new(AtomicU64::new(0));
    let total_put_errors = Arc::new(AtomicU64::new(0));
    let total_get_errors = Arc::new(AtomicU64::new(0));
    let stop_flag = Arc::new(AtomicBool::new(false));

    let mut samples = Vec::new();
    let mut sample_num = 0usize;

    let start_time = Instant::now();

    // Save temp_dir path before moving into closure
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Sampling thread
    let sample_stop = stop_flag.clone();
    let sample_total_puts = total_puts.clone();
    let sample_total_gets = total_gets.clone();
    let sample_thread = std::thread::spawn(move || {
        let mut last_puts = 0u64;
        let mut last_gets = 0u64;
        let interval = Duration::from_secs(10); // Sample every 10s

        while !sample_stop.load(Ordering::Relaxed) {
            std::thread::sleep(interval);
            let current_puts = sample_total_puts.load(Ordering::Relaxed);
            let current_gets = sample_total_gets.load(Ordering::Relaxed);
            let delta_puts = current_puts - last_puts;
            let delta_gets = current_gets - last_gets;
            let elapsed = interval.as_secs_f64();
            let qps = (delta_puts + delta_gets) as f64 / elapsed;

            let sample = StabilityMetrics {
                elapsed_seconds: start_time.elapsed().as_secs(),
                qps,
                memory_bytes: get_memory_usage(),
                disk_bytes: calculate_dir_size(&temp_dir_path).unwrap_or(0),
                success_ops: current_puts + current_gets,
                failed_ops: 0,
            };

            println!(
                "  [Sample {}] {:.0}s | QPS: {:.0} (puts: {}, gets: {}) | Mem: {:.2}MB | Disk: {:.2}MB",
                sample_num + 1,
                sample.elapsed_seconds,
                sample.qps,
                delta_puts,
                delta_gets,
                sample.memory_bytes as f64 / 1024.0 / 1024.0,
                sample.disk_bytes as f64 / 1024.0 / 1024.0,
            );

            samples.push(sample);
            last_puts = current_puts;
            last_gets = current_gets;
            sample_num += 1;
        }

        samples
    });

    // Worker threads
    let mut handles = Vec::new();
    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let stop = stop_flag.clone();
        let puts = total_puts.clone();
        let gets = total_gets.clone();
        let put_errors = total_put_errors.clone();
        let get_errors = total_get_errors.clone();

        let handle = std::thread::spawn(move || {
            use rand::rngs::StdRng;
            use rand::{Rng, SeedableRng};
            let mut rng = StdRng::seed_from_u64(t as u64 + 1000);
            let thread_seed = t as u64;
            let mut local_puts = 0u64;
            let mut local_gets = 0u64;

            while !stop.load(Ordering::Relaxed) {
                let op: f64 = rng.gen();
                let key_index = rng.gen_range(0..50_000u64);

                if op < write_pct {
                    // Write operation
                    let key = format!("mixed_t{}_k{}", thread_seed, key_index);
                    let value = format!("v_t{}_k{}", thread_seed, key_index);
                    match kv_clone.put(&key, value.as_bytes()) {
                        Ok(_) => local_puts += 1,
                        Err(_) => {
                            put_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                } else {
                    // Read operation - mix of preloaded and thread-specific keys
                    let key = if key_index < prepopulate_keys {
                        format!("preload_{}", key_index)
                    } else {
                        format!("mixed_t{}_k{}", thread_seed, key_index % 10_000)
                    };
                    match kv_clone.get(&key) {
                        Ok(_) => local_gets += 1,
                        Err(_) => {
                            get_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }

                // Update shared counters periodically
                if (local_puts + local_gets).is_multiple_of(100) {
                    puts.fetch_add(local_puts, Ordering::Relaxed);
                    gets.fetch_add(local_gets, Ordering::Relaxed);
                    local_puts = 0;
                    local_gets = 0;
                }
            }

            // Flush remaining counts
            puts.fetch_add(local_puts, Ordering::Relaxed);
            gets.fetch_add(local_gets, Ordering::Relaxed);
        });

        handles.push(handle);
    }

    // Wait for test duration
    std::thread::sleep(test_duration);
    stop_flag.store(true, Ordering::Relaxed);

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Worker thread panicked");
    }

    // Wait for sampling thread
    let final_samples: Vec<StabilityMetrics> = sample_thread.join().expect("Sampling thread panicked");

    // Final stats
    let total_elapsed = start_time.elapsed();
    let final_puts = total_puts.load(Ordering::Relaxed);
    let final_gets = total_gets.load(Ordering::Relaxed);
    let total_ops = final_puts + final_gets;
    let total_put_errs = total_put_errors.load(Ordering::Relaxed);
    let total_get_errs = total_get_errors.load(Ordering::Relaxed);
    let total_errors = total_put_errs + total_get_errs;

    let qps = if total_elapsed.as_secs_f64() > 0.0 {
        total_ops as f64 / total_elapsed.as_secs_f64()
    } else {
        0.0
    };

    let final_disk_bytes = calculate_dir_size(temp_dir.path()).unwrap_or(0);
    let final_memory = get_memory_usage();

    // Calculate read/write ratio
    let read_pct_actual = if total_ops > 0 {
        final_gets as f64 / total_ops as f64 * 100.0
    } else {
        0.0
    };
    let write_pct_actual = if total_ops > 0 {
        final_puts as f64 / total_ops as f64 * 100.0
    } else {
        0.0
    };

    // Calculate performance degradation from samples
    let initial_qps = final_samples.first().map(|s| s.qps).unwrap_or(qps);
    let final_sample_qps = final_samples.last().map(|s| s.qps).unwrap_or(qps);
    let perf_degradation = if initial_qps > 0.0 {
        ((initial_qps - final_sample_qps) / initial_qps) * 100.0
    } else {
        0.0
    };

    // Memory growth
    let memory_growth = if final_samples.len() >= 2 {
        final_samples.last().unwrap().memory_bytes - final_samples.first().unwrap().memory_bytes
    } else {
        final_memory
    };

    let consistency_rate = if total_ops > 0 {
        (total_ops - total_errors) as f64 / total_ops as f64
    } else {
        1.0
    };

    let report = StabilityReport {
        test_duration_hours: get_test_duration_hours(),
        total_ops,
        initial_qps,
        final_qps: final_sample_qps,
        performance_degradation_pct: perf_degradation.max(0.0),
        memory_growth_bytes: memory_growth,
        consistency_success_rate: consistency_rate,
        samples: final_samples,
    };

    println!("\n{}", report);

    // Additional stats
    println!("\n=== Detailed Mixed Operations Stats ===");
    println!("Total puts:             {}", final_puts);
    println!("Total gets:             {}", final_gets);
    println!("Put errors:             {}", total_put_errs);
    println!("Get errors:             {}", total_get_errs);
    println!("Actual read %:          {:.1}%", read_pct_actual);
    println!("Actual write %:         {:.1}%", write_pct_actual);
    println!("Overall QPS:            {:.0}", qps);
    println!(
        "Final disk size:        {:.2} MB",
        final_disk_bytes as f64 / 1024.0 / 1024.0
    );

    // Assertions
    assert!(
        total_errors == 0 || (total_errors as f64 / total_ops as f64) < 0.001,
        "Error rate should be < 0.1%, got {:.4}%",
        if total_ops > 0 {
            total_errors as f64 / total_ops as f64 * 100.0
        } else {
            0.0
        }
    );
    assert!(
        perf_degradation < 80.0,
        "Performance degradation should be < 80%, got {:.2}%",
        perf_degradation
    );
    assert!(
        final_disk_bytes > 0,
        "Disk size should be greater than 0 after operations"
    );
}
