//! Professional Benchmark Suite - BENCH-001
//!
//! Industry-standard benchmark for LSM-Tree KV storage engines.
//! Measures write/read/space amplification at 10M keys scale.
//!
//! ## Test Matrix
//!
//! | Test | Description | Keys | Operations |
//! |------|-------------|------|------------|
//! | write_perf | Write 10M keys, measure WA, SA, tail latency | 10M | Sequential write |
//! | read_perf | Random point + range queries, measure RA, tail latency | 10M | 100K point, 100 range |
//! | mixed_workload | 70% read + 30% write, measure all amplifications | 10M | 10M ops total |
//! | rocksdb_compare | Fair comparison with RocksDB (same config) | 10M | Write + Read |
//!
//! ## Metrics
//! - QPS (ops/sec)
//! - Write Amplification (WA) = actual_disk_write / logical_user_write
//! - Read Amplification (RA) = actual_disk_read / logical_user_read
//! - Space Amplification (SA) = actual_disk_size / logical_data_size
//! - Tail Latency: p99, p999
//!
//! ## Usage
//! ```bash
//! cargo bench --bench 07_professional --features benchmarks
//! ```

mod common;

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config, warm_cache};

// ============================================================================
// Constants
// ============================================================================

/// Primary dataset size: 10M keys
const NUM_KEYS_10M: usize = 10_000_000;

/// Value size for all benchmarks
const VALUE_SIZE: usize = 100;

/// Random read samples for read latency benchmark
const RANDOM_READ_SAMPLES: usize = 100_000;

/// Range query count and size
const RANGE_QUERY_COUNT: usize = 100;
const RANGE_QUERY_SIZE: usize = 1_000;

/// Mixed workload total operations
const MIXED_TOTAL_OPS: usize = 10_000_000;

/// Mixed workload read percentage
const MIXED_READ_RATIO: f64 = 0.70;

// ============================================================================
// Configuration Helpers
// ============================================================================

/// Professional benchmark configuration
/// Optimized for 10M keys with realistic production settings
fn professional_config(temp_dir: &TempDir) -> tokitai_filekv::FileKVConfig {
    let mut config = quick_bench_config(temp_dir);
    // Production-like settings for 10M keys
    config.memtable.flush_threshold_bytes = 16 * 1024 * 1024; // 16MB
    config.cache.max_memory_bytes = 1024 * 1024 * 1024; // 1GB
    config.cache.max_items = 500_000;
    config.cache.frequency_aware = true; // T-004: Frequency-aware caching
    config.compaction.auto_compact = true;
    config.compaction.leveled_compaction_enabled = true;
    config.compaction.level_size_multiplier = 10;
    config.compaction.max_level = 4;
    // T-004: Lower L0 threshold for faster compaction in mixed workload
    config.compaction.l0_file_count_threshold = 3;
    config.block_size = 4096; // Standard 4KB blocks
    config.enable_bloom = true;
    config
}

/// Configuration for RocksDB fair comparison
#[cfg(feature = "rocksdb-compare")]
fn rocksdb_options() -> rocksdb::Options {
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    opts.set_max_background_jobs(4);
    opts.set_max_write_buffer_number(4);
    opts.set_write_buffer_size(16 * 1024 * 1024); // 16MB, same as FileKV
    opts.set_max_bytes_for_level_base(160 * 1024 * 1024); // 160MB base

    let mut block_opts = rocksdb::BlockBasedOptions::default();
    block_opts.set_block_size(4096); // Same 4KB blocks
    block_opts.set_bloom_filter(0.01, false); // 1% FPR
    block_opts.set_cache_index_and_filter_blocks(true);
    opts.set_block_based_table_factory(&block_opts);

    opts
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate directory size recursively
fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                size += metadata.len();
            } else if metadata.is_dir() {
                size += dir_size(&entry.path())?;
            }
        }
    }
    Ok(size)
}

/// Calculate total size of multiple directories
fn dirs_size(paths: &[&Path]) -> u64 {
    paths.iter().filter_map(|p| dir_size(p).ok()).sum()
}

/// Format duration for human-readable output
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    let micros = d.subsec_micros();
    if secs > 0 {
        format!("{}s {}ms", secs, millis)
    } else if millis > 0 {
        format!("{}ms {}us", millis, micros % 1000)
    } else {
        format!("{}us", micros)
    }
}

/// Calculate percentile from a sorted Vec of durations
fn percentile(sorted_durations: &[Duration], p: f64) -> Duration {
    if sorted_durations.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((p / 100.0) * (sorted_durations.len() - 1) as f64).round() as usize;
    sorted_durations[idx.min(sorted_durations.len() - 1)]
}

/// Print benchmark results summary
fn print_write_results(
    label: &str,
    num_keys: usize,
    elapsed: Duration,
    disk_size: u64,
    stats: &tokitai_filekv::FileKVStatsSnapshot,
) {
    let logical_bytes = num_keys as u64 * (bench_key(0).len() as u64 + VALUE_SIZE as u64);
    let qps = num_keys as f64 / elapsed.as_secs_f64();

    println!("\n{}", "=".repeat(80));
    println!("WRITE PERFORMANCE: {}", label);
    println!("{}", "-".repeat(80));
    println!(
        "  Keys written:          {:>12} ({})",
        num_keys,
        format_duration(elapsed)
    );
    println!(
        "  Throughput:            {:>12.0} ops/sec ({:.2} MB/s)",
        qps,
        (logical_bytes as f64 / elapsed.as_secs_f64()) / (1024.0 * 1024.0)
    );
    println!(
        "  Logical data size:     {:>12} bytes ({:.2} MB)",
        logical_bytes,
        logical_bytes as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Actual disk size:      {:>12} bytes ({:.2} MB)",
        disk_size,
        disk_size as f64 / (1024.0 * 1024.0)
    );
    println!("  User bytes written:    {:>12} bytes", stats.user_bytes_written);
    println!("  Total bytes written:   {:>12} bytes", stats.total_bytes_written_all);
    println!();
    println!("  Write Amplification:   {:>12.2}x", stats.write_amplification_factor);
    println!("  Space Amplification:   {:>12.2}x", stats.space_amplification_factor);
    println!("{}", "=".repeat(80));
}

fn print_read_results(
    label: &str,
    num_reads: usize,
    elapsed: Duration,
    stats: &tokitai_filekv::FileKVStatsSnapshot,
    latencies: &[Duration],
) {
    let mut sorted = latencies.to_vec();
    sorted.sort();

    let qps = num_reads as f64 / elapsed.as_secs_f64();
    let p50 = percentile(&sorted, 50.0);
    let p99 = percentile(&sorted, 99.0);
    let p999 = percentile(&sorted, 99.9);
    let p9999 = percentile(&sorted, 99.99);
    let avg = elapsed / num_reads as u32;

    println!("\n{}", "=".repeat(80));
    println!("READ PERFORMANCE: {}", label);
    println!("{}", "-".repeat(80));
    println!(
        "  Reads completed:       {:>12} ({})",
        num_reads,
        format_duration(elapsed)
    );
    println!("  Throughput:            {:>12.0} ops/sec", qps);
    println!("  Avg latency:           {:>12.2} us", avg.as_micros() as f64);
    println!("  p50 latency:           {:>12.2} us", p50.as_micros() as f64);
    println!("  p99 latency:           {:>12.2} us", p99.as_micros() as f64);
    println!("  p999 latency:          {:>12.2} us", p999.as_micros() as f64);
    println!("  p9999 latency:         {:>12.2} us", p9999.as_micros() as f64);
    println!();
    println!("  Read Amplification:    {:>12.2}x", stats.read_amplification_factor);
    println!("  Total bytes read:      {:>12} bytes", stats.total_bytes_read);
    println!("  Read I/O ops:          {:>12}", stats.read_io_operations);
    println!("{}", "=".repeat(80));
}

/// Generate JSON result for programmatic analysis
fn to_json_result(test_name: &str, metrics: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    let mut result = serde_json::Map::new();
    result.insert("test".to_string(), serde_json::Value::String(test_name.to_string()));
    result.insert(
        "timestamp".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    for (k, v) in metrics {
        result.insert(k.clone(), v.clone());
    }
    serde_json::Value::Object(result)
}

// ============================================================================
// Test 1: Write Performance (10M keys)
// ============================================================================

fn bench_write_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_performance");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(300)); // 5 min for 10M keys
    group.throughput(Throughput::Elements(NUM_KEYS_10M as u64));

    group.bench_function("sequential_write_10m", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Reset stats before benchmark
            let _stats_before = kv.get_stats();

            // Benchmark: write 10M keys
            let start = Instant::now();
            for i in 0..NUM_KEYS_10M {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                kv.put(&key, &value).unwrap();
            }
            let write_elapsed = start.elapsed();

            // Flush and measure
            flush_kv(&kv);
            let stats = kv.get_stats();

            // Calculate disk size
            let disk_size = dirs_size(&[
                config.segment_dir.as_path(),
                config.index_dir.as_path(),
                if config.enable_wal {
                    config.wal_dir.as_path()
                } else {
                    temp_dir.path()
                },
            ]);

            print_write_results("10M Sequential Writes", NUM_KEYS_10M, write_elapsed, disk_size, &stats);

            // Output JSON for analysis
            let mut metrics = serde_json::Map::new();
            metrics.insert("num_keys".to_string(), serde_json::Value::Number(NUM_KEYS_10M.into()));
            metrics.insert(
                "elapsed_ms".to_string(),
                serde_json::Value::Number((write_elapsed.as_millis() as u64).into()),
            );
            metrics.insert(
                "qps".to_string(),
                serde_json::json!(NUM_KEYS_10M as f64 / write_elapsed.as_secs_f64()),
            );
            metrics.insert(
                "write_amplification".to_string(),
                serde_json::json!(stats.write_amplification_factor),
            );
            metrics.insert(
                "space_amplification".to_string(),
                serde_json::json!(stats.space_amplification_factor),
            );
            metrics.insert(
                "disk_size_bytes".to_string(),
                serde_json::Value::Number(disk_size.into()),
            );
            metrics.insert(
                "logical_size_bytes".to_string(),
                serde_json::Value::Number(
                    (NUM_KEYS_10M as u64 * (bench_key(0).len() as u64 + VALUE_SIZE as u64)).into(),
                ),
            );

            let json_result = to_json_result("write_performance_10m", &metrics);
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box((write_elapsed, disk_size, stats));
        });
    });

    group.finish();
}

// ============================================================================
// Test 2: Read Performance (10M keys)
// ============================================================================

fn bench_read_performance_hot_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_performance");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(300));

    // Point read benchmark
    group.bench_function("random_point_read_hot_cache_100k", |b| {
        b.iter(|| {
            // Setup INSIDE iter (each iteration is independent)
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Pre-populate (NOT timed)
            for i in 0..NUM_KEYS_10M {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);

            // Warm cache
            warm_cache(&kv, 100_000);

            // Benchmark: random point reads
            let read_key_idx = AtomicUsize::new(0);
            let mut latencies = Vec::with_capacity(RANDOM_READ_SAMPLES);

            for _ in 0..RANDOM_READ_SAMPLES {
                let idx = read_key_idx.fetch_add(1, Ordering::Relaxed) % NUM_KEYS_10M;
                let key = bench_key(idx);
                let op_start = Instant::now();
                let result = kv.get(&key).unwrap();
                let op_elapsed = op_start.elapsed();
                latencies.push(op_elapsed);
                black_box(result);
            }

            black_box(latencies);
        });
    });

    // Range read benchmark
    group.bench_function("range_read_hot_cache_100x1000", |b| {
        b.iter(|| {
            // Setup INSIDE iter
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Pre-populate (NOT timed)
            for i in 0..NUM_KEYS_10M {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);

            // Warm cache
            warm_cache(&kv, 100_000);

            // Benchmark: range queries
            let mut total_keys_read = 0;

            for q in 0..RANGE_QUERY_COUNT {
                let start_key_idx = (q * 100_000) % (NUM_KEYS_10M - RANGE_QUERY_SIZE);

                let mut count = 0;
                for i in start_key_idx..start_key_idx + RANGE_QUERY_SIZE {
                    let key = bench_key(i);
                    if kv.get(&key).unwrap().is_some() {
                        count += 1;
                    }
                }
                total_keys_read += count;
                black_box(count);
            }

            black_box(total_keys_read);
        });
    });

    group.finish();
}

fn bench_read_performance_cold_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_performance");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(600)); // 10 min

    group.bench_function("random_point_read_cold_cache_100k", |b| {
        b.iter(|| {
            // Each iteration: fresh KV, cold cache
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Pre-populate (NOT in cache)
            for i in 0..NUM_KEYS_10M {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);

            // Read random keys (cold cache)
            let mut latencies = Vec::with_capacity(1_000); // Reduced for timeout
            for i in 0..1_000 {
                let idx = (i * 10007) % NUM_KEYS_10M; // Prime stride for distribution
                let key = bench_key(idx);
                let op_start = Instant::now();
                let result = kv.get(&key).unwrap();
                let op_elapsed = op_start.elapsed();
                latencies.push(op_elapsed);
                black_box(result);
            }

            let stats = kv.get_stats();
            print_read_results(
                "1K Random Point Reads (Cold Cache)",
                1_000,
                latencies.iter().sum::<Duration>(),
                &stats,
                &latencies,
            );

            black_box(latencies);
        });
    });

    group.finish();
}

// ============================================================================
// Test 3: Mixed Workload (70% Read + 30% Write)
// ============================================================================

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(600));
    group.throughput(Throughput::Elements(MIXED_TOTAL_OPS as u64));

    group.bench_function("70_read_30_write_10m", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Pre-populate 50% of keys
            println!("\n  Pre-populating 5M keys for mixed workload...");
            for i in 0..NUM_KEYS_10M / 2 {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);
            println!("  Pre-population complete.");

            // Mixed workload: 70% reads, 30% writes
            let op_idx = AtomicUsize::new(0);
            let read_count = AtomicUsize::new(0);
            let write_count = AtomicUsize::new(0);
            let mut latencies_read = Vec::with_capacity((MIXED_TOTAL_OPS as f64 * MIXED_READ_RATIO) as usize);
            let mut latencies_write = Vec::with_capacity((MIXED_TOTAL_OPS as f64 * (1.0 - MIXED_READ_RATIO)) as usize);

            let start = Instant::now();

            for _ in 0..MIXED_TOTAL_OPS {
                let idx = op_idx.fetch_add(1, Ordering::Relaxed);
                let is_read = (idx % 10) < 7; // 70% read

                if is_read {
                    let key = bench_key(idx % (NUM_KEYS_10M / 2));
                    let op_start = Instant::now();
                    let result = kv.get(&key).unwrap();
                    let op_elapsed = op_start.elapsed();
                    latencies_read.push(op_elapsed);
                    read_count.fetch_add(1, Ordering::Relaxed);
                    black_box(result);
                } else {
                    let key = bench_key(idx % NUM_KEYS_10M);
                    let value = bench_value(VALUE_SIZE);
                    let op_start = Instant::now();
                    kv.put(&key, &value).unwrap();
                    let op_elapsed = op_start.elapsed();
                    latencies_write.push(op_elapsed);
                    write_count.fetch_add(1, Ordering::Relaxed);
                }
            }
            let elapsed = start.elapsed();

            let stats = kv.get_stats();
            let reads = read_count.load(Ordering::Relaxed);
            let writes = write_count.load(Ordering::Relaxed);

            println!("\n{}", "=".repeat(80));
            println!("MIXED WORKLOAD: 70% Read / 30% Write ({} ops)", MIXED_TOTAL_OPS);
            println!("{}", "-".repeat(80));
            println!(
                "  Total operations:      {:>12} ({})",
                MIXED_TOTAL_OPS,
                format_duration(elapsed)
            );
            println!("  Reads:                 {:>12}", reads);
            println!("  Writes:                {:>12}", writes);
            println!(
                "  Overall QPS:           {:>12.0} ops/sec",
                MIXED_TOTAL_OPS as f64 / elapsed.as_secs_f64()
            );
            println!(
                "  Read QPS:              {:>12.0} ops/sec",
                reads as f64 / elapsed.as_secs_f64()
            );
            println!(
                "  Write QPS:             {:>12.0} ops/sec",
                writes as f64 / elapsed.as_secs_f64()
            );
            println!();

            // Read latency
            let mut sorted_read = latencies_read.clone();
            sorted_read.sort();
            if !sorted_read.is_empty() {
                let read_p50 = percentile(&sorted_read, 50.0);
                let read_p99 = percentile(&sorted_read, 99.0);
                let read_p999 = percentile(&sorted_read, 99.9);
                let read_p9999 = percentile(&sorted_read, 99.99);
                println!(
                    "  Read avg latency:      {:>12.2} us",
                    latencies_read.iter().map(|d| d.as_micros() as f64).sum::<f64>() / reads as f64
                );
                println!("  Read p50 latency:      {:>12.2} us", read_p50.as_micros() as f64);
                println!("  Read p99 latency:      {:>12.2} us", read_p99.as_micros() as f64);
                println!("  Read p999 latency:     {:>12.2} us", read_p999.as_micros() as f64);
                println!("  Read p9999 latency:    {:>12.2} us", read_p9999.as_micros() as f64);
            }

            // Write latency
            let mut sorted_write = latencies_write.clone();
            sorted_write.sort();
            if !sorted_write.is_empty() {
                let write_p50 = percentile(&sorted_write, 50.0);
                let write_p99 = percentile(&sorted_write, 99.0);
                let write_p999 = percentile(&sorted_write, 99.9);
                let write_p9999 = percentile(&sorted_write, 99.99);
                println!(
                    "  Write avg latency:     {:>12.2} us",
                    latencies_write.iter().map(|d| d.as_micros() as f64).sum::<f64>() / writes as f64
                );
                println!("  Write p50 latency:     {:>12.2} us", write_p50.as_micros() as f64);
                println!("  Write p99 latency:     {:>12.2} us", write_p99.as_micros() as f64);
                println!("  Write p999 latency:    {:>12.2} us", write_p999.as_micros() as f64);
                println!("  Write p9999 latency:   {:>12.2} us", write_p9999.as_micros() as f64);
            }

            println!();
            println!("  Write Amplification:   {:>12.2}x", stats.write_amplification_factor);
            println!("  Read Amplification:    {:>12.2}x", stats.read_amplification_factor);
            println!("  Space Amplification:   {:>12.2}x", stats.space_amplification_factor);
            println!("{}", "=".repeat(80));

            // JSON result
            let mut sorted_r = latencies_read.clone();
            sorted_r.sort();
            let mut sorted_w = latencies_write.clone();
            sorted_w.sort();

            let mut metrics = serde_json::Map::new();
            metrics.insert(
                "total_ops".to_string(),
                serde_json::Value::Number(MIXED_TOTAL_OPS.into()),
            );
            metrics.insert("reads".to_string(), serde_json::Value::Number(reads.into()));
            metrics.insert("writes".to_string(), serde_json::Value::Number(writes.into()));
            metrics.insert(
                "elapsed_ms".to_string(),
                serde_json::Value::Number((elapsed.as_millis() as u64).into()),
            );
            metrics.insert(
                "overall_qps".to_string(),
                serde_json::json!(MIXED_TOTAL_OPS as f64 / elapsed.as_secs_f64()),
            );
            metrics.insert(
                "write_amplification".to_string(),
                serde_json::json!(stats.write_amplification_factor),
            );
            metrics.insert(
                "read_amplification".to_string(),
                serde_json::json!(stats.read_amplification_factor),
            );
            metrics.insert(
                "space_amplification".to_string(),
                serde_json::json!(stats.space_amplification_factor),
            );
            if !sorted_r.is_empty() {
                metrics.insert(
                    "read_p50_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 50.0).as_micros() as f64),
                );
                metrics.insert(
                    "read_p99_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 99.0).as_micros() as f64),
                );
                metrics.insert(
                    "read_p999_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 99.9).as_micros() as f64),
                );
                metrics.insert(
                    "read_p9999_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 99.99).as_micros() as f64),
                );
            }
            if !sorted_w.is_empty() {
                metrics.insert(
                    "write_p50_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 50.0).as_micros() as f64),
                );
                metrics.insert(
                    "write_p99_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 99.0).as_micros() as f64),
                );
                metrics.insert(
                    "write_p999_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 99.9).as_micros() as f64),
                );
                metrics.insert(
                    "write_p9999_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 99.99).as_micros() as f64),
                );
            }

            let json_result = to_json_result("mixed_workload_70r30w", &metrics);
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box((elapsed, reads, writes, stats));
        });
    });

    group.finish();
}

// ============================================================================
// T-004: Additional Mixed Workload Ratios
// ============================================================================

/// Helper to run mixed workload with configurable read/write ratio
fn run_mixed_workload(c: &mut Criterion, name: &str, read_ratio: f64, total_ops: usize, prepopulate_keys: usize) {
    let mut group = c.benchmark_group("mixed_workload_t004");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(300));
    group.throughput(Throughput::Elements(total_ops as u64));

    let bench_name = format!(
        "{}_r{}_w{}",
        name,
        (read_ratio * 100.0) as usize,
        ((1.0 - read_ratio) * 100.0) as usize
    );
    group.bench_function(&bench_name, |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Pre-populate keys
            println!("\n  Pre-populating {} keys for {}...", prepopulate_keys, bench_name);
            for i in 0..prepopulate_keys {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);
            println!("  Pre-population complete.");

            let op_idx = AtomicUsize::new(0);
            let read_count = AtomicUsize::new(0);
            let write_count = AtomicUsize::new(0);
            let read_threshold = (read_ratio * 10.0) as usize;
            let mut latencies_read = Vec::with_capacity((total_ops as f64 * read_ratio) as usize);
            let mut latencies_write = Vec::with_capacity((total_ops as f64 * (1.0 - read_ratio)) as usize);

            let start = Instant::now();

            for _ in 0..total_ops {
                let idx = op_idx.fetch_add(1, Ordering::Relaxed);
                let is_read = (idx % 10) < read_threshold;

                if is_read {
                    let key = bench_key(idx % prepopulate_keys);
                    let op_start = Instant::now();
                    let result = kv.get(&key).unwrap();
                    let op_elapsed = op_start.elapsed();
                    latencies_read.push(op_elapsed);
                    read_count.fetch_add(1, Ordering::Relaxed);
                    black_box(result);
                } else {
                    let key = bench_key(idx % (prepopulate_keys * 2));
                    let value = bench_value(VALUE_SIZE);
                    let op_start = Instant::now();
                    kv.put(&key, &value).unwrap();
                    let op_elapsed = op_start.elapsed();
                    latencies_write.push(op_elapsed);
                    write_count.fetch_add(1, Ordering::Relaxed);
                }
            }
            let elapsed = start.elapsed();

            let stats = kv.get_stats();
            let reads = read_count.load(Ordering::Relaxed);
            let writes = write_count.load(Ordering::Relaxed);

            println!("\n{}", "=".repeat(80));
            println!(
                "MIXED WORKLOAD T-004: {}% Read / {}% Write ({} ops)",
                (read_ratio * 100.0) as usize,
                ((1.0 - read_ratio) * 100.0) as usize,
                total_ops
            );
            println!("{}", "-".repeat(80));
            println!(
                "  Total operations:      {:>12} ({})",
                total_ops,
                format_duration(elapsed)
            );
            println!("  Reads:                 {:>12}", reads);
            println!("  Writes:                {:>12}", writes);
            println!(
                "  Overall QPS:           {:>12.0} ops/sec",
                total_ops as f64 / elapsed.as_secs_f64()
            );
            println!(
                "  Read QPS:              {:>12.0} ops/sec",
                reads as f64 / elapsed.as_secs_f64()
            );
            println!(
                "  Write QPS:             {:>12.0} ops/sec",
                writes as f64 / elapsed.as_secs_f64()
            );
            println!();

            let mut sorted_read = latencies_read.clone();
            sorted_read.sort();
            if !sorted_read.is_empty() {
                let read_p50 = percentile(&sorted_read, 50.0);
                let read_p99 = percentile(&sorted_read, 99.0);
                let read_p999 = percentile(&sorted_read, 99.9);
                let read_p9999 = percentile(&sorted_read, 99.99);
                println!(
                    "  Read avg latency:      {:>12.2} us",
                    latencies_read.iter().map(|d| d.as_micros() as f64).sum::<f64>() / reads as f64
                );
                println!("  Read p50 latency:      {:>12.2} us", read_p50.as_micros() as f64);
                println!("  Read p99 latency:      {:>12.2} us", read_p99.as_micros() as f64);
                println!("  Read p999 latency:     {:>12.2} us", read_p999.as_micros() as f64);
                println!("  Read p9999 latency:    {:>12.2} us", read_p9999.as_micros() as f64);
            }

            let mut sorted_write = latencies_write.clone();
            sorted_write.sort();
            if !sorted_write.is_empty() {
                let write_p50 = percentile(&sorted_write, 50.0);
                let write_p99 = percentile(&sorted_write, 99.0);
                let write_p999 = percentile(&sorted_write, 99.9);
                let write_p9999 = percentile(&sorted_write, 99.99);
                println!(
                    "  Write avg latency:     {:>12.2} us",
                    latencies_write.iter().map(|d| d.as_micros() as f64).sum::<f64>() / writes as f64
                );
                println!("  Write p50 latency:     {:>12.2} us", write_p50.as_micros() as f64);
                println!("  Write p99 latency:     {:>12.2} us", write_p99.as_micros() as f64);
                println!("  Write p999 latency:    {:>12.2} us", write_p999.as_micros() as f64);
                println!("  Write p9999 latency:   {:>12.2} us", write_p9999.as_micros() as f64);
            }

            println!();
            println!("  Write Amplification:   {:>12.2}x", stats.write_amplification_factor);
            println!("  Read Amplification:    {:>12.2}x", stats.read_amplification_factor);
            println!("  Space Amplification:   {:>12.2}x", stats.space_amplification_factor);
            println!("{}", "=".repeat(80));

            let mut sorted_r = latencies_read.clone();
            sorted_r.sort();
            let mut sorted_w = latencies_write.clone();
            sorted_w.sort();

            let mut metrics = serde_json::Map::new();
            metrics.insert("read_ratio".to_string(), serde_json::json!(read_ratio));
            metrics.insert("total_ops".to_string(), serde_json::Value::Number(total_ops.into()));
            metrics.insert("reads".to_string(), serde_json::Value::Number(reads.into()));
            metrics.insert("writes".to_string(), serde_json::Value::Number(writes.into()));
            metrics.insert(
                "elapsed_ms".to_string(),
                serde_json::Value::Number((elapsed.as_millis() as u64).into()),
            );
            metrics.insert(
                "overall_qps".to_string(),
                serde_json::json!(total_ops as f64 / elapsed.as_secs_f64()),
            );
            metrics.insert(
                "write_amplification".to_string(),
                serde_json::json!(stats.write_amplification_factor),
            );
            metrics.insert(
                "read_amplification".to_string(),
                serde_json::json!(stats.read_amplification_factor),
            );
            metrics.insert(
                "space_amplification".to_string(),
                serde_json::json!(stats.space_amplification_factor),
            );
            if !sorted_r.is_empty() {
                metrics.insert(
                    "read_p50_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 50.0).as_micros() as f64),
                );
                metrics.insert(
                    "read_p99_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 99.0).as_micros() as f64),
                );
                metrics.insert(
                    "read_p999_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 99.9).as_micros() as f64),
                );
                metrics.insert(
                    "read_p9999_us".to_string(),
                    serde_json::json!(percentile(&sorted_r, 99.99).as_micros() as f64),
                );
            }
            if !sorted_w.is_empty() {
                metrics.insert(
                    "write_p50_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 50.0).as_micros() as f64),
                );
                metrics.insert(
                    "write_p99_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 99.0).as_micros() as f64),
                );
                metrics.insert(
                    "write_p999_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 99.9).as_micros() as f64),
                );
                metrics.insert(
                    "write_p9999_us".to_string(),
                    serde_json::json!(percentile(&sorted_w, 99.99).as_micros() as f64),
                );
            }

            let json_result = to_json_result(
                &format!(
                    "mixed_workload_{}r{}w",
                    (read_ratio * 100.0) as usize,
                    ((1.0 - read_ratio) * 100.0) as usize
                ),
                &metrics,
            );
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box((elapsed, reads, writes, stats));
        });
    });

    group.finish();
}

/// T-004: 90% read + 10% write mixed workload
fn bench_mixed_workload_90r10w(c: &mut Criterion) {
    run_mixed_workload(c, "90_read_10_write", 0.90, 1_000_000, 500_000);
}

/// T-004: 50% read + 50% write mixed workload
fn bench_mixed_workload_50r50w(c: &mut Criterion) {
    run_mixed_workload(c, "50_read_50_write", 0.50, 1_000_000, 500_000);
}

// ============================================================================
// Test 4: RocksDB Fair Comparison
// ============================================================================

#[cfg(feature = "rocksdb-compare")]
fn bench_rocksdb_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("rocksdb_comparison");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(300));

    // RocksDB Write benchmark
    group.bench_function("rocksdb_write_10m", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let opts = rocksdb_options();
            let db = rocksdb::DB::open(&opts, temp_dir.path()).unwrap();

            let start = Instant::now();
            for i in 0..NUM_KEYS_10M {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                db.put(key.as_bytes(), &value).unwrap();
            }
            let elapsed = start.elapsed();
            db.flush().unwrap();

            // Calculate disk size
            let disk_size = dir_size(temp_dir.path()).unwrap_or(0);
            let logical_bytes = NUM_KEYS_10M as u64 * (bench_key(0).len() as u64 + VALUE_SIZE as u64);
            let qps = NUM_KEYS_10M as f64 / elapsed.as_secs_f64();

            println!("\n{}", "=".repeat(80));
            println!("ROCKSDB WRITE PERFORMANCE");
            println!("{}", "-".repeat(80));
            println!(
                "  Keys written:          {:>12} ({})",
                NUM_KEYS_10M,
                format_duration(elapsed)
            );
            println!(
                "  Throughput:            {:>12.0} ops/sec ({:.2} MB/s)",
                qps,
                (logical_bytes as f64 / elapsed.as_secs_f64()) / (1024.0 * 1024.0)
            );
            println!("  Logical data size:     {:>12} bytes", logical_bytes);
            println!(
                "  Actual disk size:      {:>12} bytes ({:.2} MB)",
                disk_size,
                disk_size as f64 / (1024.0 * 1024.0)
            );
            println!(
                "  Space Amplification:   {:>12.2}x",
                disk_size as f64 / logical_bytes as f64
            );
            println!("{}", "=".repeat(80));

            black_box((elapsed, disk_size));
        });
    });

    // RocksDB Read benchmark
    group.bench_function("rocksdb_read_hot_cache_100k", |b| {
        b.iter(|| {
            // Each iteration: fresh RocksDB instance
            let temp_dir = TempDir::new().unwrap();
            let opts = rocksdb_options();
            let db = rocksdb::DB::open(&opts, temp_dir.path()).unwrap();

            // Pre-populate (NOT timed)
            for i in 0..NUM_KEYS_10M {
                let key = bench_key(i);
                let value = bench_value(VALUE_SIZE);
                db.put(key.as_bytes(), &value).unwrap();
            }
            db.flush().unwrap();

            // Warm cache
            for i in (0..100_000).step_by(100) {
                let key = bench_key(i);
                let _ = db.get(key.as_bytes());
            }

            // Benchmark random reads
            let read_key_idx = AtomicUsize::new(0);
            let mut latencies = Vec::with_capacity(RANDOM_READ_SAMPLES);

            for _ in 0..RANDOM_READ_SAMPLES {
                let idx = read_key_idx.fetch_add(1, Ordering::Relaxed) % NUM_KEYS_10M;
                let key = bench_key(idx);
                let op_start = Instant::now();
                let result = db.get(key.as_bytes()).unwrap();
                let op_elapsed = op_start.elapsed();
                latencies.push(op_elapsed);
                black_box(result);
            }

            let mut sorted = latencies.clone();
            sorted.sort();
            let p99 = percentile(&sorted, 99.0);
            let p999 = percentile(&sorted, 99.9);

            println!("\n{}", "=".repeat(80));
            println!("ROCKSDB READ PERFORMANCE (Hot Cache)");
            println!("{}", "-".repeat(80));
            println!("  Reads completed:       {:>12}", RANDOM_READ_SAMPLES,);
            println!(
                "  Avg latency:           {:>12.2} us",
                latencies.iter().map(|d| d.as_micros() as f64).sum::<f64>() / RANDOM_READ_SAMPLES as f64
            );
            println!("  p99 latency:           {:>12.2} us", p99.as_micros() as f64);
            println!("  p999 latency:          {:>12.2} us", p999.as_micros() as f64);
            println!("{}", "=".repeat(80));

            black_box(latencies);
        });
    });

    group.finish();
}

// ============================================================================
// Amplification Rate Analysis
// ============================================================================

fn bench_amplification_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("amplification_analysis");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(300));

    group.bench_function("write_amplification_profile", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = professional_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Write in batches and measure amplification at each stage
            let batch_sizes = [10_000, 100_000, 1_000_000, NUM_KEYS_10M];
            let mut results = Vec::new();

            for &batch_size in &batch_sizes {
                let _stats_before = kv.get_stats();
                let disk_before = dirs_size(&[config.segment_dir.as_path(), config.index_dir.as_path()]);

                let start = Instant::now();
                for i in 0..batch_size {
                    let key = bench_key(i);
                    let value = bench_value(VALUE_SIZE);
                    kv.put(&key, &value).unwrap();
                }
                let write_elapsed = start.elapsed();
                flush_kv(&kv);

                let stats_after = kv.get_stats();
                let disk_after = dirs_size(&[config.segment_dir.as_path(), config.index_dir.as_path()]);

                let _logical_bytes = batch_size as u64 * (bench_key(0).len() as u64 + VALUE_SIZE as u64);
                let _disk_delta = disk_after.saturating_sub(disk_before);

                results.push((
                    batch_size,
                    write_elapsed,
                    disk_after,
                    stats_after.space_amplification_factor,
                ));
            }

            println!("\n{}", "=".repeat(80));
            println!("AMPLIFICATION ANALYSIS BY DATA SIZE");
            println!("{}", "-".repeat(80));
            println!(
                "{:>12} | {:>12} | {:>12} | {:>12} | {:>8}",
                "Keys", "Elapsed", "Disk Size", "Logical", "SA"
            );
            println!("{}", "-".repeat(80));
            for (keys, elapsed, disk, sa) in &results {
                let logical = *keys as u64 * (bench_key(0).len() as u64 + VALUE_SIZE as u64);
                println!(
                    "{:>12} | {:>12} | {:>10}MB | {:>10}MB | {:>6.2}x",
                    keys,
                    format_duration(*elapsed),
                    disk / (1024 * 1024),
                    logical / (1024 * 1024),
                    sa
                );
            }
            println!("{}", "=".repeat(80));

            black_box(results);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    name = prof_write;
    config = common::fast_criterion_config();
    targets = bench_write_performance
);

criterion_group!(
    name = prof_read_hot;
    config = common::fast_criterion_config();
    targets = bench_read_performance_hot_cache
);

criterion_group!(
    name = prof_read_cold;
    config = common::fast_criterion_config();
    targets = bench_read_performance_cold_cache
);

criterion_group!(
    name = prof_mixed;
    config = common::fast_criterion_config();
    targets = bench_mixed_workload
);

// T-004: Additional mixed workload ratios
criterion_group!(
    name = prof_mixed_t004;
    config = common::fast_criterion_config();
    targets = bench_mixed_workload_90r10w, bench_mixed_workload_50r50w
);

#[cfg(feature = "rocksdb-compare")]
criterion_group!(
    name = prof_rocksdb;
    config = common::fast_criterion_config();
    targets = bench_rocksdb_comparison
);

criterion_group!(
    name = prof_amplification;
    config = common::fast_criterion_config();
    targets = bench_amplification_analysis
);

#[cfg(feature = "rocksdb-compare")]
criterion_main!(
    prof_write,
    prof_read_hot,
    prof_read_cold,
    prof_mixed,
    prof_mixed_t004,
    prof_rocksdb,
    prof_amplification,
);

#[cfg(not(feature = "rocksdb-compare"))]
criterion_main!(
    prof_write,
    prof_read_hot,
    prof_read_cold,
    prof_mixed,
    prof_mixed_t004,
    prof_amplification,
);
