//! 10M Keys Professional Benchmark Suite - PERF-BENCH-002
//!
//! Supplements `07_professional_benchmark.rs` with:
//! 1. Value size comparison (64B / 256B / 1KB / 4KB)
//! 2. Scaling dataset benchmarks (100K -> 1M -> 5M) with amplification trends
//! 3. Compaction before/after read performance comparison
//! 4. Mixed workload under compaction pressure
//!
//! ## Usage
//! ```bash
//! cargo bench --features benchmarks --bench 09_10m
//! ```

mod common;

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config};

// ============================================================================
// Constants
// ============================================================================

/// Value sizes to compare
const VALUE_SIZES: [(usize, &str); 4] = [(64, "64B"), (256, "256B"), (1024, "1KB"), (4096, "4KB")];

/// Scaling dataset sizes
const SCALING_SIZES: [(usize, &str); 3] = [(100_000, "100K"), (1_000_000, "1M"), (5_000_000, "5M")];

/// Number of keys for compaction impact test
const COMPACTION_TEST_KEYS: usize = 500_000;

/// Default value size for scaling/compaction benchmarks
const DEFAULT_VALUE_SIZE: usize = 256;

/// Read samples for latency measurement
const READ_SAMPLES: usize = 10_000;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for large-scale benchmarks with production-like settings
fn large_scale_config(temp_dir: &TempDir) -> tokitai_filekv::FileKVConfig {
    let mut config = quick_bench_config(temp_dir);
    config.memtable.flush_threshold_bytes = 8 * 1024 * 1024; // 8MB
    config.cache.max_memory_bytes = 512 * 1024 * 1024; // 512MB
    config.cache.max_items = 200_000;
    config.cache.frequency_aware = true;
    config.compaction.auto_compact = false; // Manual compaction control
    config.compaction.leveled_compaction_enabled = true;
    config.compaction.level_size_multiplier = 10;
    config.compaction.max_level = 4;
    config.compaction.min_segments = 3;
    config.block_size = 4096;
    config.enable_bloom = true;
    config
}

/// Configuration with auto-compaction enabled for mixed workload tests
fn auto_compact_config(temp_dir: &TempDir) -> tokitai_filekv::FileKVConfig {
    let mut config = large_scale_config(temp_dir);
    config.compaction.auto_compact = true;
    config.compaction.l0_file_count_threshold = 3;
    config
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

/// Write `num_keys` entries with given value size and return elapsed time + stats
fn write_keys_timed(
    kv: &tokitai_filekv::FileKV,
    num_keys: usize,
    value_size: usize,
) -> (Duration, tokitai_filekv::FileKVStatsSnapshot) {
    let start = Instant::now();
    for i in 0..num_keys {
        let key = bench_key(i);
        let value = bench_value(value_size);
        kv.put(&key, &value).unwrap();
    }
    let elapsed = start.elapsed();
    flush_kv(kv);
    let stats = kv.get_stats();
    (elapsed, stats)
}

/// Measure read latency for random keys
fn measure_read_latencies(kv: &tokitai_filekv::FileKV, num_keys: usize, samples: usize) -> Vec<Duration> {
    let mut latencies = Vec::with_capacity(samples);
    for i in 0..samples {
        let idx = (i * 10007) % num_keys; // Prime stride for distribution
        let key = bench_key(idx);
        let op_start = Instant::now();
        let _ = kv.get(&key).unwrap();
        latencies.push(op_start.elapsed());
    }
    latencies
}

// ============================================================================
// Test 1: Value Size Comparison (64B / 256B / 1KB / 4KB)
// ============================================================================

fn bench_write_value_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_value_sizes");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    let num_keys = 100_000;
    group.throughput(Throughput::Elements(num_keys as u64));

    group.bench_function("compare_value_sizes_100k", |b| {
        b.iter(|| {
            let mut results = Vec::new();

            for (value_size, label) in VALUE_SIZES {
                let temp_dir = TempDir::new().unwrap();
                let config = large_scale_config(&temp_dir);
                let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

                let (elapsed, stats) = write_keys_timed(&kv, num_keys, value_size);
                let disk_size = dirs_size(&[config.segment_dir.as_path(), config.index_dir.as_path()]);
                let logical_bytes = num_keys as u64 * (bench_key(0).len() as u64 + value_size as u64);
                let qps = num_keys as f64 / elapsed.as_secs_f64();
                let space_amp = disk_size as f64 / logical_bytes as f64;

                results.push((
                    label,
                    value_size,
                    elapsed,
                    qps,
                    disk_size,
                    logical_bytes,
                    space_amp,
                    stats.write_amplification_factor,
                ));

                println!(
                    "\n  Value size {}: {} keys in {} ({:.0} ops/sec, SA={:.2}x, WA={:.2}x)",
                    label,
                    num_keys,
                    format_duration(elapsed),
                    qps,
                    space_amp,
                    stats.write_amplification_factor
                );
            }

            // Summary table
            println!("\n{}", "=".repeat(90));
            println!("VALUE SIZE COMPARISON: {} keys", num_keys);
            println!("{}", "-".repeat(90));
            println!(
                "{:>6} | {:>10} | {:>12} | {:>10} | {:>10} | {:>8} | {:>8}",
                "Size", "Elapsed", "QPS", "Disk Size", "Logical", "SA", "WA"
            );
            println!("{}", "-".repeat(90));
            for (label, _size, elapsed, qps, disk, logical, sa, wa) in &results {
                println!(
                    "{:>6} | {:>12} | {:>10.0} | {:>8}MB | {:>8}MB | {:>6.2}x | {:>6.2}x",
                    label,
                    format_duration(*elapsed),
                    qps,
                    disk / (1024 * 1024),
                    logical / (1024 * 1024),
                    sa,
                    wa
                );
            }
            println!("{}", "=".repeat(90));

            // JSON result
            let mut metrics = serde_json::Map::new();
            let mut size_results = serde_json::Map::new();
            for (label, _size, elapsed, qps, disk, logical, sa, wa) in &results {
                let mut entry = serde_json::Map::new();
                entry.insert(
                    "elapsed_ms".to_string(),
                    serde_json::Value::Number((elapsed.as_millis() as u64).into()),
                );
                entry.insert("qps".to_string(), serde_json::json!(qps));
                entry.insert("disk_size_bytes".to_string(), serde_json::Value::Number((*disk).into()));
                entry.insert(
                    "logical_size_bytes".to_string(),
                    serde_json::Value::Number((*logical).into()),
                );
                entry.insert("space_amplification".to_string(), serde_json::json!(sa));
                entry.insert("write_amplification".to_string(), serde_json::json!(wa));
                size_results.insert(label.to_string(), serde_json::Value::Object(entry));
            }
            metrics.insert(
                "value_size_results".to_string(),
                serde_json::Value::Object(size_results),
            );
            metrics.insert("num_keys".to_string(), serde_json::Value::Number(num_keys.into()));

            let json_result = to_json_result("write_value_sizes_100k", &metrics);
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box(results);
        });
    });

    group.finish();
}

// ============================================================================
// Test 2: Scaling Dataset Benchmark (100K -> 1M -> 5M)
// ============================================================================

fn bench_scaling_dataset(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_dataset");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));

    group.bench_function("scaling_100k_to_5m", |b| {
        b.iter(|| {
            let mut results = Vec::new();

            for (num_keys, label) in SCALING_SIZES {
                let temp_dir = TempDir::new().unwrap();
                let config = large_scale_config(&temp_dir);
                let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

                let (elapsed, stats) = write_keys_timed(&kv, num_keys, DEFAULT_VALUE_SIZE);
                let disk_size = dirs_size(&[config.segment_dir.as_path(), config.index_dir.as_path()]);
                let logical_bytes = num_keys as u64 * (bench_key(0).len() as u64 + DEFAULT_VALUE_SIZE as u64);
                let qps = num_keys as f64 / elapsed.as_secs_f64();

                results.push((
                    label,
                    num_keys,
                    elapsed,
                    qps,
                    disk_size,
                    logical_bytes,
                    stats.write_amplification_factor,
                    stats.read_amplification_factor,
                    stats.space_amplification_factor,
                ));

                println!(
                    "\n  Scale {}: {} keys in {} ({:.0} ops/sec, WA={:.2}x, RA={:.2}x, SA={:.2}x)",
                    label,
                    num_keys,
                    format_duration(elapsed),
                    qps,
                    stats.write_amplification_factor,
                    stats.read_amplification_factor,
                    stats.space_amplification_factor
                );
            }

            // Summary table
            println!("\n{}", "=".repeat(110));
            println!("SCALING DATASET BENCHMARK ({}B values)", DEFAULT_VALUE_SIZE);
            println!("{}", "-".repeat(110));
            println!(
                "{:>6} | {:>8} | {:>12} | {:>10} | {:>10} | {:>8} | {:>8} | {:>8}",
                "Scale", "Keys", "Elapsed", "QPS", "Disk Size", "WA", "RA", "SA"
            );
            println!("{}", "-".repeat(110));
            for (label, keys, elapsed, qps, disk, _logical, wa, ra, sa) in &results {
                println!(
                    "{:>6} | {:>8} | {:>12} | {:>10.0} | {:>8}MB | {:>6.2}x | {:>6.2}x | {:>6.2}x",
                    label,
                    keys,
                    format_duration(*elapsed),
                    qps,
                    disk / (1024 * 1024),
                    wa,
                    ra,
                    sa
                );
            }
            println!("{}", "=".repeat(110));

            // JSON result
            let mut metrics = serde_json::Map::new();
            let mut scale_results = serde_json::Map::new();
            for (label, keys, elapsed, qps, disk, _logical, wa, ra, sa) in &results {
                let mut entry = serde_json::Map::new();
                entry.insert("num_keys".to_string(), serde_json::Value::Number((*keys).into()));
                entry.insert(
                    "elapsed_ms".to_string(),
                    serde_json::Value::Number((elapsed.as_millis() as u64).into()),
                );
                entry.insert("qps".to_string(), serde_json::json!(qps));
                entry.insert("disk_size_bytes".to_string(), serde_json::Value::Number((*disk).into()));
                entry.insert("write_amplification".to_string(), serde_json::json!(wa));
                entry.insert("read_amplification".to_string(), serde_json::json!(ra));
                entry.insert("space_amplification".to_string(), serde_json::json!(sa));
                scale_results.insert(label.to_string(), serde_json::Value::Object(entry));
            }
            metrics.insert("scaling_results".to_string(), serde_json::Value::Object(scale_results));
            metrics.insert(
                "value_size".to_string(),
                serde_json::Value::Number(DEFAULT_VALUE_SIZE.into()),
            );

            let json_result = to_json_result("scaling_dataset", &metrics);
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box(results);
        });
    });

    group.finish();
}

// ============================================================================
// Test 3: Compaction Impact on Read Performance
// ============================================================================

fn bench_compaction_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("compaction_impact");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));

    group.bench_function("read_before_after_compaction_500k", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = large_scale_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            // Phase 1: Write data
            println!("\n  Phase 1: Writing {} keys...", COMPACTION_TEST_KEYS);
            let (write_elapsed, _stats) = write_keys_timed(&kv, COMPACTION_TEST_KEYS, DEFAULT_VALUE_SIZE);
            println!("  Write complete in {}", format_duration(write_elapsed));

            // Measure segment count before compaction
            let segments_before = kv.get_stats().segment_count;
            let disk_before = dirs_size(&[config.segment_dir.as_path(), config.index_dir.as_path()]);

            // Phase 2: Measure read performance BEFORE compaction
            println!(
                "  Phase 2: Measuring read latency (before compaction, {} segments)...",
                segments_before
            );
            let latencies_before = measure_read_latencies(&kv, COMPACTION_TEST_KEYS, READ_SAMPLES);
            let sorted_before = {
                let mut s = latencies_before.clone();
                s.sort();
                s
            };
            let avg_before_us =
                latencies_before.iter().map(|d| d.as_micros() as f64).sum::<f64>() / READ_SAMPLES as f64;
            let p50_before_us = percentile(&sorted_before, 50.0).as_micros() as f64;
            let p99_before_us = percentile(&sorted_before, 99.0).as_micros() as f64;
            let p999_before_us = percentile(&sorted_before, 99.9).as_micros() as f64;

            println!(
                "  Before compaction: avg={:.2}us, p50={:.2}us, p99={:.2}us, p999={:.2}us",
                avg_before_us, p50_before_us, p99_before_us, p999_before_us
            );

            // Phase 3: Run compaction
            println!("  Phase 3: Running compaction...");
            let compaction_start = Instant::now();
            let compaction_stats = kv.run_compaction().unwrap();
            let compaction_elapsed = compaction_start.elapsed();
            println!(
                "  Compaction complete in {} (merged {} segments)",
                format_duration(compaction_elapsed),
                compaction_stats.segments_merged
            );

            // Measure segment count after compaction
            let segments_after = kv.get_stats().segment_count;
            let disk_after = dirs_size(&[config.segment_dir.as_path(), config.index_dir.as_path()]);

            // Phase 4: Measure read performance AFTER compaction
            println!(
                "  Phase 4: Measuring read latency (after compaction, {} segments)...",
                segments_after
            );
            let latencies_after = measure_read_latencies(&kv, COMPACTION_TEST_KEYS, READ_SAMPLES);
            let sorted_after = {
                let mut s = latencies_after.clone();
                s.sort();
                s
            };
            let avg_after_us = latencies_after.iter().map(|d| d.as_micros() as f64).sum::<f64>() / READ_SAMPLES as f64;
            let p50_after_us = percentile(&sorted_after, 50.0).as_micros() as f64;
            let p99_after_us = percentile(&sorted_after, 99.0).as_micros() as f64;
            let p999_after_us = percentile(&sorted_after, 99.9).as_micros() as f64;

            println!(
                "  After compaction:  avg={:.2}us, p50={:.2}us, p99={:.2}us, p999={:.2}us",
                avg_after_us, p50_after_us, p99_after_us, p999_after_us
            );

            // Improvement ratios
            let speedup_avg = avg_before_us / avg_after_us;
            let speedup_p99 = p99_before_us / p99_after_us;
            let disk_reduction = (disk_before as f64 - disk_after as f64) / disk_before as f64 * 100.0;

            println!("\n{}", "=".repeat(90));
            println!("COMPACTION IMPACT: {} keys", COMPACTION_TEST_KEYS);
            println!("{}", "-".repeat(90));
            println!(
                "  Segments: {} -> {} (reduced by {})",
                segments_before,
                segments_after,
                segments_before - segments_after
            );
            println!(
                "  Disk size: {:.2} MB -> {:.2} MB ({:.1}% reduction)",
                disk_before as f64 / (1024.0 * 1024.0),
                disk_after as f64 / (1024.0 * 1024.0),
                disk_reduction
            );
            println!();
            println!("  Read latency:");
            println!(
                "    Avg:  {:.2}us -> {:.2}us ({:.2}x speedup)",
                avg_before_us, avg_after_us, speedup_avg
            );
            println!("    p50:  {:.2}us -> {:.2}us", p50_before_us, p50_after_us);
            println!(
                "    p99:  {:.2}us -> {:.2}us ({:.2}x speedup)",
                p99_before_us, p99_after_us, speedup_p99
            );
            println!("    p999: {:.2}us -> {:.2}us", p999_before_us, p999_after_us);
            println!();
            println!(
                "  Compaction time: {} (merged {} segments, removed {} entries)",
                format_duration(compaction_elapsed),
                compaction_stats.segments_merged,
                compaction_stats.entries_removed
            );
            println!("{}", "=".repeat(90));

            // JSON result
            let mut metrics = serde_json::Map::new();
            metrics.insert(
                "num_keys".to_string(),
                serde_json::Value::Number(COMPACTION_TEST_KEYS.into()),
            );
            metrics.insert(
                "segments_before".to_string(),
                serde_json::Value::Number(segments_before.into()),
            );
            metrics.insert(
                "segments_after".to_string(),
                serde_json::Value::Number(segments_after.into()),
            );
            metrics.insert(
                "disk_before_bytes".to_string(),
                serde_json::Value::Number(disk_before.into()),
            );
            metrics.insert(
                "disk_after_bytes".to_string(),
                serde_json::Value::Number(disk_after.into()),
            );
            metrics.insert(
                "compaction_time_ms".to_string(),
                serde_json::Value::Number((compaction_elapsed.as_millis() as u64).into()),
            );
            metrics.insert(
                "segments_merged".to_string(),
                serde_json::Value::Number(compaction_stats.segments_merged.into()),
            );

            let mut before = serde_json::Map::new();
            before.insert("avg_us".to_string(), serde_json::json!(avg_before_us));
            before.insert("p50_us".to_string(), serde_json::json!(p50_before_us));
            before.insert("p99_us".to_string(), serde_json::json!(p99_before_us));
            before.insert("p999_us".to_string(), serde_json::json!(p999_before_us));
            metrics.insert("before_compaction".to_string(), serde_json::Value::Object(before));

            let mut after = serde_json::Map::new();
            after.insert("avg_us".to_string(), serde_json::json!(avg_after_us));
            after.insert("p50_us".to_string(), serde_json::json!(p50_after_us));
            after.insert("p99_us".to_string(), serde_json::json!(p99_after_us));
            after.insert("p999_us".to_string(), serde_json::json!(p999_after_us));
            metrics.insert("after_compaction".to_string(), serde_json::Value::Object(after));

            metrics.insert("speedup_avg".to_string(), serde_json::json!(speedup_avg));
            metrics.insert("speedup_p99".to_string(), serde_json::json!(speedup_p99));

            let json_result = to_json_result("compaction_impact_500k", &metrics);
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box((latencies_before, latencies_after, compaction_stats));
        });
    });

    group.finish();
}

// ============================================================================
// Test 4: Mixed Workload with Compaction
// ============================================================================

fn bench_mixed_with_compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_with_compaction");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));
    group.throughput(Throughput::Elements(500_000));

    group.bench_function("mixed_70r30w_500k_with_compaction", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = auto_compact_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config.clone()).unwrap();

            let prepopulate_keys = 250_000;
            let total_ops = 500_000;
            let read_ratio = 0.7;

            // Phase 1: Pre-populate
            println!("\n  Pre-populating {} keys...", prepopulate_keys);
            let (write_elapsed, _) = write_keys_timed(&kv, prepopulate_keys, DEFAULT_VALUE_SIZE);
            println!("  Pre-population complete in {}", format_duration(write_elapsed));

            // Phase 2: Mixed workload with compaction monitoring
            println!(
                "  Running mixed workload ({} ops, {:.0}% read / {:.0}% write)...",
                total_ops,
                read_ratio * 100.0,
                (1.0 - read_ratio) * 100.0
            );

            let op_idx = AtomicUsize::new(0);
            let read_count = AtomicUsize::new(0);
            let write_count = AtomicUsize::new(0);
            let mut latencies_read = Vec::with_capacity((total_ops as f64 * read_ratio) as usize);
            let mut latencies_write = Vec::with_capacity((total_ops as f64 * (1.0 - read_ratio)) as usize);

            // Track latency changes over time (buckets of 10% each)
            let bucket_size = total_ops / 10;
            let mut bucket_read_latencies = vec![Vec::new(); 10];
            let mut bucket_write_latencies = vec![Vec::new(); 10];

            let start = Instant::now();

            for _ in 0..total_ops {
                let idx = op_idx.fetch_add(1, Ordering::Relaxed);
                let bucket = idx / bucket_size;
                let is_read = (idx % 10) < (read_ratio * 10.0) as usize;

                if is_read {
                    let key = bench_key(idx % prepopulate_keys);
                    let op_start = Instant::now();
                    let result = kv.get(&key).unwrap();
                    let op_elapsed = op_start.elapsed();
                    latencies_read.push(op_elapsed);
                    if bucket < 10 {
                        bucket_read_latencies[bucket].push(op_elapsed);
                    }
                    read_count.fetch_add(1, Ordering::Relaxed);
                    black_box(result);
                } else {
                    let key = bench_key(idx % (prepopulate_keys * 2));
                    let value = bench_value(DEFAULT_VALUE_SIZE);
                    let op_start = Instant::now();
                    kv.put(&key, &value).unwrap();
                    let op_elapsed = op_start.elapsed();
                    latencies_write.push(op_elapsed);
                    if bucket < 10 {
                        bucket_write_latencies[bucket].push(op_elapsed);
                    }
                    write_count.fetch_add(1, Ordering::Relaxed);
                }
            }
            let elapsed = start.elapsed();

            let reads = read_count.load(Ordering::Relaxed);
            let writes = write_count.load(Ordering::Relaxed);
            let stats = kv.get_stats();

            // Compute per-bucket latency trends
            let mut bucket_avg_read = Vec::new();
            let mut bucket_avg_write = Vec::new();
            for b_idx in 0..10 {
                if !bucket_read_latencies[b_idx].is_empty() {
                    let avg = bucket_read_latencies[b_idx]
                        .iter()
                        .map(|d| d.as_micros() as f64)
                        .sum::<f64>()
                        / bucket_read_latencies[b_idx].len() as f64;
                    bucket_avg_read.push(avg);
                }
                if !bucket_write_latencies[b_idx].is_empty() {
                    let avg = bucket_write_latencies[b_idx]
                        .iter()
                        .map(|d| d.as_micros() as f64)
                        .sum::<f64>()
                        / bucket_write_latencies[b_idx].len() as f64;
                    bucket_avg_write.push(avg);
                }
            }

            // Latency statistics
            let mut sorted_read = latencies_read.clone();
            sorted_read.sort();
            let read_avg = latencies_read.iter().map(|d| d.as_micros() as f64).sum::<f64>() / reads as f64;
            let read_p50 = percentile(&sorted_read, 50.0).as_micros() as f64;
            let read_p99 = percentile(&sorted_read, 99.0).as_micros() as f64;
            let read_p999 = percentile(&sorted_read, 99.9).as_micros() as f64;

            let mut sorted_write = latencies_write.clone();
            sorted_write.sort();
            let write_avg = latencies_write.iter().map(|d| d.as_micros() as f64).sum::<f64>() / writes as f64;
            let write_p50 = percentile(&sorted_write, 50.0).as_micros() as f64;
            let write_p99 = percentile(&sorted_write, 99.0).as_micros() as f64;
            let write_p999 = percentile(&sorted_write, 99.9).as_micros() as f64;

            // Segment count
            let final_segments = kv.get_stats().segment_count;

            // Print results
            println!("\n{}", "=".repeat(90));
            println!(
                "MIXED WORKLOAD WITH COMPACTION: {} ops ({}% R / {}% W)",
                total_ops,
                (read_ratio * 100.0) as usize,
                ((1.0 - read_ratio) * 100.0) as usize
            );
            println!("{}", "-".repeat(90));
            println!(
                "  Total ops: {} ({} reads, {} writes) in {}",
                total_ops,
                reads,
                writes,
                format_duration(elapsed)
            );
            println!("  Overall QPS: {:.0} ops/sec", total_ops as f64 / elapsed.as_secs_f64());
            println!("  Final segments: {}", final_segments);
            println!();
            println!(
                "  Read latency:  avg={:.2}us, p50={:.2}us, p99={:.2}us, p999={:.2}us",
                read_avg, read_p50, read_p99, read_p999
            );
            println!(
                "  Write latency: avg={:.2}us, p50={:.2}us, p99={:.2}us, p999={:.2}us",
                write_avg, write_p50, write_p99, write_p999
            );
            println!();
            println!("  Read latency trend (per 10% bucket):");
            for (i, avg) in bucket_avg_read.iter().enumerate() {
                println!("    Bucket {}: {:.2}us", i, avg);
            }
            println!("  Write latency trend (per 10% bucket):");
            for (i, avg) in bucket_avg_write.iter().enumerate() {
                println!("    Bucket {}: {:.2}us", i, avg);
            }
            println!();
            println!(
                "  WA={:.2}x, RA={:.2}x, SA={:.2}x",
                stats.write_amplification_factor, stats.read_amplification_factor, stats.space_amplification_factor
            );
            println!("{}", "=".repeat(90));

            // JSON result
            let mut metrics = serde_json::Map::new();
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
                "final_segments".to_string(),
                serde_json::Value::Number(final_segments.into()),
            );

            let mut read_lat = serde_json::Map::new();
            read_lat.insert("avg_us".to_string(), serde_json::json!(read_avg));
            read_lat.insert("p50_us".to_string(), serde_json::json!(read_p50));
            read_lat.insert("p99_us".to_string(), serde_json::json!(read_p99));
            read_lat.insert("p999_us".to_string(), serde_json::json!(read_p999));
            metrics.insert("read_latency".to_string(), serde_json::Value::Object(read_lat));

            let mut write_lat = serde_json::Map::new();
            write_lat.insert("avg_us".to_string(), serde_json::json!(write_avg));
            write_lat.insert("p50_us".to_string(), serde_json::json!(write_p50));
            write_lat.insert("p99_us".to_string(), serde_json::json!(write_p99));
            write_lat.insert("p999_us".to_string(), serde_json::json!(write_p999));
            metrics.insert("write_latency".to_string(), serde_json::Value::Object(write_lat));

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

            // Latency trend data
            let mut trend = serde_json::Map::new();
            let mut read_trend = serde_json::Map::new();
            let mut write_trend = serde_json::Map::new();
            for (i, avg) in bucket_avg_read.iter().enumerate() {
                read_trend.insert(format!("bucket_{}", i), serde_json::json!(avg));
            }
            for (i, avg) in bucket_avg_write.iter().enumerate() {
                write_trend.insert(format!("bucket_{}", i), serde_json::json!(avg));
            }
            trend.insert("read_avg_us".to_string(), serde_json::Value::Object(read_trend));
            trend.insert("write_avg_us".to_string(), serde_json::Value::Object(write_trend));
            metrics.insert("latency_trend".to_string(), serde_json::Value::Object(trend));

            let json_result = to_json_result("mixed_with_compaction_500k", &metrics);
            println!("\nJSON_RESULT:{}", serde_json::to_string(&json_result).unwrap());

            black_box((elapsed, reads, writes, stats));
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    name = bench_value_sizes;
    config = common::fast_criterion_config();
    targets = bench_write_value_sizes
);

criterion_group!(
    name = bench_scaling;
    config = common::fast_criterion_config();
    targets = bench_scaling_dataset
);

criterion_group!(
    name = bench_compaction;
    config = common::fast_criterion_config();
    targets = bench_compaction_impact
);

criterion_group!(
    name = bench_mixed_compaction;
    config = common::fast_criterion_config();
    targets = bench_mixed_with_compaction
);

criterion_main!(
    bench_value_sizes,
    bench_scaling,
    bench_compaction,
    bench_mixed_compaction,
);
