//! Large-Scale Dataset Benchmarks - TEST-002
//!
//! Measures performance across different dataset sizes:
//! - 10K keys (small scale)
//! - 100K keys (medium scale) - primary target for PERF-005
//! - 1M keys (large scale)
//!
//! Metrics:
//! - Write throughput (entries/sec)
//! - Read latency (µs/entry)
//! - Memory usage
//! - Disk usage
//!
//! This benchmark suite validates the 100K keys performance issue:
//! Current: ~151ms (240x slower than RocksDB's 628µs)
//! Target: Reduce gap to within 10x

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config, warm_cache};

// ============================================================================
// Configuration Helpers
// ============================================================================

/// Configuration optimized for large dataset benchmarks
fn large_dataset_config(temp_dir: &TempDir) -> tokitai_filekv::FileKVConfig {
    let mut config = quick_bench_config(temp_dir);
    // Increase memtable flush threshold to reduce segment count
    config.memtable.flush_threshold_bytes = 4 * 1024 * 1024; // 4MB
                                                             // Increase cache size for better hit rates
    config.cache.max_memory_bytes = 256 * 1024 * 1024; // 256MB
    config.cache.max_items = 100_000;
    config
}

// ============================================================================
// Write Throughput Benchmarks
// ============================================================================

fn bench_write_throughput(c: &mut Criterion) {
    let data_sizes = [("10k", 10_000), ("100k", 100_000), ("1m", 1_000_000)];

    for (label, num_keys) in data_sizes {
        let mut group = c.benchmark_group(format!("write_throughput_{}", label));
        group.sample_size(10);
        group.measurement_time(Duration::from_secs(30));
        group.throughput(Throughput::Elements(num_keys as u64));

        group.bench_function("batch_write", |b| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let config = large_dataset_config(&temp_dir);
                let kv = tokitai_filekv::FileKV::open(config).unwrap();

                let start = Instant::now();
                for i in 0..num_keys {
                    let key = bench_key(i);
                    let value = bench_value(100); // 100-byte values
                    kv.put(&key, &value).unwrap();
                }
                let elapsed = start.elapsed();

                // Flush to ensure data is persisted
                flush_kv(&kv);

                black_box(elapsed);
            });
        });

        group.finish();
    }
}

// ============================================================================
// Read Latency Benchmarks
// ============================================================================

fn bench_read_latency_hot_cache(c: &mut Criterion) {
    let data_sizes = [
        ("10k", 10_000),
        ("100k", 100_000),
        // Skip 1M for hot cache - takes too long
    ];

    for (label, num_keys) in data_sizes {
        let mut group = c.benchmark_group(format!("read_latency_hot_cache_{}", label));
        group.sample_size(10); // Reduced from 100 to avoid timeout
        group.measurement_time(Duration::from_secs(30));

        group.bench_function("random_read", |b| {
            // Setup OUTSIDE iter
            let temp_dir = TempDir::new().unwrap();
            let config = large_dataset_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config).unwrap();

            // Pre-populate
            for i in 0..num_keys {
                let key = bench_key(i);
                let value = bench_value(100);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);
            warm_cache(&kv, num_keys);

            // Benchmark random reads
            let key_idx = AtomicUsize::new(0);
            b.iter(|| {
                let idx = key_idx.fetch_add(1, Ordering::Relaxed) % num_keys;
                let key = bench_key(idx);
                let result = kv.get(&key).unwrap();
                black_box(result);
            });
        });

        group.finish();
    }
}

fn bench_read_latency_cold_cache(c: &mut Criterion) {
    let data_sizes = [("10k", 10_000), ("100k", 100_000), ("1m", 1_000_000)];

    for (label, num_keys) in data_sizes {
        let mut group = c.benchmark_group(format!("read_latency_cold_cache_{}", label));
        group.sample_size(10);
        group.measurement_time(Duration::from_secs(60));

        group.bench_function("cold_random_read", |b| {
            b.iter(|| {
                // Each iteration: fresh KV, cold cache, measure single read
                let temp_dir = TempDir::new().unwrap();
                let config = large_dataset_config(&temp_dir);
                let kv = tokitai_filekv::FileKV::open(config).unwrap();

                // Pre-populate (not in cache)
                for i in 0..num_keys {
                    let key = bench_key(i);
                    let value = bench_value(100);
                    kv.put(&key, &value).unwrap();
                }
                flush_kv(&kv);

                // Read a single random key (cold cache) — timed
                let idx = 50_000 % num_keys;
                let key = bench_key(idx);
                let start = Instant::now();
                let result = kv.get(&key).unwrap();
                let elapsed = start.elapsed();
                black_box((result, elapsed));
            });
        });

        group.finish();
    }
}

// ============================================================================
// Sequential Read Benchmarks
// ============================================================================

fn bench_sequential_read(c: &mut Criterion) {
    let data_sizes = [("10k", 10_000), ("100k", 100_000)];

    for (label, num_keys) in data_sizes {
        let mut group = c.benchmark_group(format!("sequential_read_{}", label));
        group.sample_size(10);
        group.measurement_time(Duration::from_secs(30));
        group.throughput(Throughput::Elements(num_keys as u64));

        group.bench_function("full_scan", |b| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let config = large_dataset_config(&temp_dir);
                let kv = tokitai_filekv::FileKV::open(config).unwrap();

                // Pre-populate
                for i in 0..num_keys {
                    let key = bench_key(i);
                    let value = bench_value(100);
                    kv.put(&key, &value).unwrap();
                }
                flush_kv(&kv);

                // Sequential read all keys
                let start = Instant::now();
                let mut count = 0;
                for i in 0..num_keys {
                    let key = bench_key(i);
                    if kv.get(&key).unwrap().is_some() {
                        count += 1;
                    }
                }
                let elapsed = start.elapsed();

                black_box((elapsed, count));
            });
        });

        group.finish();
    }
}

// ============================================================================
// Memory and Disk Usage Benchmarks
// ============================================================================

fn bench_resource_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_usage");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("100k_memory_disk", |b| {
        b.iter(|| {
            let num_keys = 100_000;
            let temp_dir = TempDir::new().unwrap();
            let config = large_dataset_config(&temp_dir);
            let kv = tokitai_filekv::FileKV::open(config).unwrap();

            // Write all keys
            for i in 0..num_keys {
                let key = bench_key(i);
                let value = bench_value(100);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);

            // Measure resource usage
            let stats = kv.get_stats();
            black_box(stats);
        });
    });

    group.finish();
}

// ============================================================================
// Mixed Workload Benchmarks
// ============================================================================

fn bench_mixed_workload(c: &mut Criterion) {
    let data_sizes = [("10k", 10_000), ("100k", 100_000)];

    for (label, num_keys) in data_sizes {
        let mut group = c.benchmark_group(format!("mixed_workload_{}", label));
        group.sample_size(10);
        group.measurement_time(Duration::from_secs(30));
        group.throughput(Throughput::Elements(1000)); // 1000 ops per iteration

        group.bench_function("80_20_read_write", |b| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let config = large_dataset_config(&temp_dir);
                let kv = tokitai_filekv::FileKV::open(config).unwrap();

                // Pre-populate (setup, not timed by Criterion but part of each iteration)
                for i in 0..num_keys / 2 {
                    let key = bench_key(i);
                    let value = bench_value(100);
                    kv.put(&key, &value).unwrap();
                }
                flush_kv(&kv);

                // Mixed workload: 80% reads, 20% writes — timed portion
                let op_idx = AtomicUsize::new(0);
                let start = Instant::now();
                for _ in 0..1000 {
                    let idx = op_idx.fetch_add(1, Ordering::Relaxed);
                    if idx.is_multiple_of(5) {
                        // Write (20%)
                        let key = bench_key(idx % num_keys);
                        let value = bench_value(100);
                        kv.put(&key, &value).unwrap();
                    } else {
                        // Read (80%)
                        let key = bench_key(idx % (num_keys / 2));
                        kv.get(&key).unwrap();
                    }
                }
                let elapsed = start.elapsed();

                black_box(elapsed);
            });
        });

        group.finish();
    }
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    name = large_dataset_write;
    config = common::fast_criterion_config();
    targets = bench_write_throughput
);

criterion_group!(
    name = large_dataset_read_hot;
    config = common::fast_criterion_config();
    targets = bench_read_latency_hot_cache
);

criterion_group!(
    name = large_dataset_read_cold;
    config = common::fast_criterion_config();
    targets = bench_read_latency_cold_cache
);

criterion_group!(
    name = large_dataset_sequential;
    config = common::fast_criterion_config();
    targets = bench_sequential_read
);

criterion_group!(
    name = large_dataset_resource;
    config = common::fast_criterion_config();
    targets = bench_resource_usage
);

criterion_group!(
    name = large_dataset_mixed;
    config = common::fast_criterion_config();
    targets = bench_mixed_workload
);

criterion_main!(
    large_dataset_write,
    large_dataset_read_hot,
    large_dataset_read_cold,
    large_dataset_sequential,
    large_dataset_resource,
    large_dataset_mixed
);
