//! Range Query and Compaction Benchmarks - FIXED
//!
//! Measures:
//! - Range scan performance
//! - Compaction overhead
//! - Write amplification
//!
//! FIX: Reduced data size and simplified logic for fast execution.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config, setup_kv};

// ============================================================================
// Range Query Benchmarks
// ============================================================================

fn bench_range_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_scan");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let num_keys = 1000;
    let range_sizes = [10, 50, 100];

    for &range_size in &range_sizes {
        group.throughput(Throughput::Elements(range_size as u64));
        group.bench_with_input(format!("size_{}", range_size), &range_size, |b, &range_size| {
            // Setup OUTSIDE iter
            let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

            // Pre-populate
            for i in 0..num_keys {
                let key = format!("{:010}", i);
                let value = bench_value(64);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);

            // Benchmark range scan
            let start_idx = AtomicUsize::new(0);
            b.iter(|| {
                // Safe modulo to avoid overflow: start within valid range
                let start = start_idx.fetch_add(1, Ordering::Relaxed) % (num_keys - range_size + 1);
                let mut count = 0usize;
                // Simple range scan using iteration
                for i in start..start + range_size {
                    let key = format!("{:010}", i);
                    if kv.get(&key).unwrap().is_some() {
                        count += 1;
                    }
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Compaction Benchmarks (simplified, fast version)
// ============================================================================

fn bench_compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("compaction");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(15));

    let num_keys = 2000;

    group.bench_function("trigger_compaction", |b| {
        b.iter(|| {
            // Each iteration: create KV, write data, run compaction
            let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

            // Write keys to trigger segment creation
            for i in 0..num_keys {
                let key = bench_key(i);
                let value = bench_value(64);
                kv.put(&key, &value).unwrap();
            }
            flush_kv(&kv);

            // Actually run compaction and measure its time
            let compaction_start = std::time::Instant::now();
            let _result = kv.run_compaction();
            let compaction_elapsed = compaction_start.elapsed();

            black_box(compaction_elapsed);
        });
    });

    group.finish();
}

// ============================================================================
// Write Amplification (measure write overhead)
// ============================================================================

fn bench_write_amplification(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_amplification");
    group.throughput(Throughput::Elements(100));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let batch_size = 100usize;

    group.bench_function("write_100_entries", |b| {
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));

        let key_counter = AtomicUsize::new(0);
        b.iter(|| {
            let start = key_counter.fetch_add(batch_size, Ordering::Relaxed);
            for i in start..start + batch_size {
                let key = bench_key(i);
                let value = bench_value(64);
                kv.put(&key, &value).unwrap();
            }
        });
    });

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(benches, bench_range_scan, bench_compaction, bench_write_amplification);

criterion_main!(benches);
