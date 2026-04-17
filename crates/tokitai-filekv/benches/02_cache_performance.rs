//! Cache Performance Benchmarks - FIXED
//!
//! Measures cache-related performance:
//! - Hot cache hit (BlockCache)
//! - Cold cache miss (disk read)
//! - Cache eviction behavior
//!
//! FIX: Setup is OUTSIDE b.iter().

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config, setup_kv, warm_cache};

// ============================================================================
// Cache Hit Benchmarks
// ============================================================================

fn bench_cache_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    group.bench_function("hot_cache_get_64B", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Write and flush
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);
        warm_cache(&kv, num_keys);

        // Benchmark hot cache reads
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed) % num_keys);
            black_box(kv.get(&key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Cache Miss Benchmarks
// ============================================================================

fn bench_cache_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_miss");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    group.bench_function("cold_cache_get_64B", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Write and flush but DON'T warm cache
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);

        // Benchmark cold cache reads (each requires disk I/O)
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed) % num_keys);
            black_box(kv.get(&key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Mixed Workload (simulate real access patterns)
// ============================================================================

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    group.bench_function("80_percent_reads_20_percent_writes", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Pre-populate
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);
        warm_cache(&kv, num_keys);

        let op_counter = AtomicUsize::new(0);
        b.iter(|| {
            let op_idx = op_counter.fetch_add(1, Ordering::Relaxed);
            let key = bench_key(op_idx % num_keys);

            if op_idx.is_multiple_of(5) {
                // 20% writes
                let new_value = bench_value(64);
                black_box(kv.put(&key, &new_value)).unwrap();
            } else {
                // 80% reads
                black_box(kv.get(&key)).unwrap();
            }
        });
    });

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(benches, bench_cache_hit, bench_cache_miss, bench_mixed_workload);

criterion_main!(benches);
