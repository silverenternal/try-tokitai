//! Bloom Filter Benchmarks - FIXED
//!
//! Measures Bloom Filter performance:
//! - Negative lookup (key not present, should be rejected quickly)
//! - Positive lookup (key present, should proceed to disk)
//! - False positive rate impact
//!
//! FIX: Avoid memory allocation bug by using reasonable FPR values.
//! Setup is OUTSIDE b.iter().

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{black_box, Criterion, Throughput, criterion_group, criterion_main};
use tempfile::TempDir;

use common::{
    setup_kv, warm_cache, flush_kv,
    quick_bench_config,
    bench_key, bench_value,
};

// ============================================================================
// Bloom Negative Lookup (key doesn't exist, bloom should reject quickly)
// ============================================================================

fn bench_bloom_negative(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_negative");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    group.bench_function("negative_lookup_nonexistent_key", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Write some keys and flush
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);
        warm_cache(&kv, num_keys);

        // Benchmark lookup with keys that DON'T exist
        // Bloom filter should quickly reject these
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let nonexistent_key = format!("nonexistent_{:012}", key_counter.fetch_add(1, Ordering::Relaxed));
            black_box(kv.get(&nonexistent_key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Bloom Positive Lookup (key exists, bloom should allow proceeding)
// ============================================================================

fn bench_bloom_positive(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_positive");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_keys = 1000;

    group.bench_function("positive_lookup_existing_key_cold_cache", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Write keys and flush
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);
        // Don't warm cache - we want to measure bloom filter + disk read

        // Benchmark lookup with keys that DO exist
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let key = bench_key(key_counter.fetch_add(1, Ordering::Relaxed) % num_keys);
            black_box(kv.get(&key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Multi-Segment Bloom (test bloom with multiple segments)
// ============================================================================

fn bench_bloom_multi_segment(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_multi_segment");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let keys_per_segment = 500;
    let num_segments = 3;

    group.bench_function("negative_lookup_multi_segment", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        // Create multiple segments by writing and flushing in batches
        for seg in 0..num_segments {
            let start_idx = seg * keys_per_segment;
            for i in start_idx..start_idx + keys_per_segment {
                let key = bench_key(i);
                let value = bench_value(64);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);
        }

        // Benchmark negative lookup across multiple segments
        // Each segment's bloom filter must be checked
        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let nonexistent_key = format!("nonexistent_{:012}", key_counter.fetch_add(1, Ordering::Relaxed));
            black_box(kv.get(&nonexistent_key)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(benches, 
    bench_bloom_negative, 
    bench_bloom_positive, 
    bench_bloom_multi_segment
);

criterion_main!(benches);
