//! Basic Operations Benchmarks - FIXED
//!
//! Measures fundamental FileKV operations:
//! - Single put (various value sizes, with/without WAL)
//! - Single get (hot cache, cold cache)
//! - Delete operations
//! - Batch operations
//!
//! FIX: Setup is OUTSIDE b.iter() to avoid measuring initialization overhead.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config, setup_kv, wal_bench_config, warm_cache};

// ============================================================================
// Single Write Benchmarks
// ============================================================================

fn bench_single_write_no_wal(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_no_wal");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    for (name, value_size) in &[("64B", 64), ("1KB", 1024), ("4KB", 4096)] {
        group.bench_with_input(BenchmarkId::new("put", name), value_size, |b, &value_size| {
            let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));
            let value = bench_value(value_size);
            let key_counter = AtomicUsize::new(0);
            b.iter(|| {
                let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed));
                black_box(kv.put(&key, &value)).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_single_write_wal(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_wal");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    for (name, value_size) in &[("64B", 64), ("1KB", 1024), ("4KB", 4096)] {
        group.bench_with_input(BenchmarkId::new("put", name), value_size, |b, &value_size| {
            let (_temp_dir, kv) = setup_kv(wal_bench_config(&TempDir::new().unwrap()));
            let value = bench_value(value_size);
            let key_counter = AtomicUsize::new(0);
            b.iter(|| {
                let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed));
                black_box(kv.put(&key, &value)).unwrap();
            });
        });
    }

    group.finish();
}

// ============================================================================
// Single Read Benchmarks (setup OUTSIDE iter)
// ============================================================================

fn bench_single_read_hot_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_hot_cache");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    for (name, value_size) in &[("64B", 64), ("1KB", 1024), ("4KB", 4096)] {
        group.bench_with_input(BenchmarkId::new("get", name), value_size, |b, &value_size| {
            // Setup OUTSIDE iter
            let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

            // Write data
            for i in 0..num_keys {
                let key = bench_key(i);
                let value = bench_value(value_size);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);
            warm_cache(&kv, num_keys);

            // Benchmark hot cache read
            let target_key = bench_key(num_keys / 2);
            b.iter(|| {
                black_box(kv.get(&target_key)).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_single_read_cold_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_cold_cache");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    group.bench_function("get_64B_cold", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Write data and flush
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);

        // Don't warm cache - measure cold read from disk
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed) % num_keys);
            black_box(kv.get(&key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Delete Benchmarks
// ============================================================================

fn bench_delete_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    // Each iteration: write a unique key, then delete it.
    // Measures the full write-then-delete cycle without re-deleting stale keys.
    group.bench_function("delete", |b| {
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed));
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
            black_box(kv.delete(&key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Batch Write Benchmarks
// ============================================================================

fn bench_batch_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_write");
    group.throughput(Throughput::Elements(100));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let batch_size = 100usize;

    group.bench_function("batch_100", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Use the actual put_batch() API for atomic batch writes
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let start = key_counter.fetch_add(batch_size, Ordering::Relaxed);
            let entries: Vec<(String, Vec<u8>)> = (start..start + batch_size)
                .map(|i| (bench_key(i), bench_value(64)))
                .collect();
            let refs: Vec<(&str, &[u8])> = entries.iter().map(|(k, v)| (k.as_str(), v.as_slice())).collect();
            kv.put_batch(&refs).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(
    benches,
    bench_single_write_no_wal,
    bench_single_write_wal,
    bench_single_read_hot_cache,
    bench_single_read_cold_cache,
    bench_delete_operation,
    bench_batch_write
);

criterion_main!(benches);
