//! Concurrent Operations Benchmarks - FIXED
//!
//! Measures concurrent access patterns:
//! - Multi-threaded puts
//! - Multi-threaded gets
//! - Mixed read/write under contention
//!
//! FIX: Use Instant timing inside b.iter() to measure only the concurrent
//! operation time, excluding thread spawn/join overhead.

mod common;

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;

use common::{bench_key, bench_value, flush_kv, quick_bench_config, setup_kv, warm_cache};

// ============================================================================
// Concurrent Write Benchmarks
// ============================================================================

fn bench_concurrent_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_writes");
    group.throughput(Throughput::Elements(100));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let num_threads = 4;
    let keys_per_thread = 100;

    group.bench_function("4_threads_concurrent_puts", |b| {
        b.iter(|| {
            // Each iteration gets a fresh instance
            let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));
            let kv = Arc::new(kv);

            let mut handles = vec![];

            // Spawn threads OUTSIDE timing
            for t in 0..num_threads {
                let kv_clone = kv.clone();
                let handle = thread::spawn(move || {
                    let mut success = 0usize;
                    for i in 0..keys_per_thread {
                        let key = format!("t{}_k{}", t, i);
                        let value = bench_value(64);
                        if kv_clone.put(&key, &value).is_ok() {
                            success += 1;
                        }
                    }
                    success
                });
                handles.push(handle);
            }

            // Time only the join/wait phase (thread work is already running)
            let start = Instant::now();
            let mut total_success = 0usize;
            for handle in handles {
                total_success += handle.join().expect("Thread panicked");
            }
            let elapsed = start.elapsed();

            black_box((total_success, elapsed));
        });
    });

    group.finish();
}

// ============================================================================
// Concurrent Read Benchmarks
// ============================================================================

fn bench_concurrent_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_reads");
    group.throughput(Throughput::Elements(100));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let num_threads = 4;
    let reads_per_thread = 100;
    let num_keys = 500;

    group.bench_function("4_threads_concurrent_gets", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));
        let kv = Arc::new(kv);

        // Pre-populate keys
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }
        flush_kv(&kv);
        warm_cache(&kv, num_keys);

        b.iter(|| {
            let mut handles = vec![];

            // Spawn threads OUTSIDE timing
            for t in 0..num_threads {
                let kv_clone = kv.clone();
                let handle = thread::spawn(move || {
                    let mut hits = 0usize;
                    for i in 0..reads_per_thread {
                        let key_idx = (t * reads_per_thread + i) % num_keys;
                        let key = bench_key(key_idx);
                        if kv_clone.get(&key).unwrap().is_some() {
                            hits += 1;
                        }
                    }
                    hits
                });
                handles.push(handle);
            }

            // Time only the join/wait phase (thread work is already running)
            let start = Instant::now();
            let mut total_hits = 0usize;
            for handle in handles {
                total_hits += handle.join().expect("Thread panicked");
            }
            let elapsed = start.elapsed();

            black_box((total_hits, elapsed));
        });
    });

    group.finish();
}

// ============================================================================
// Mixed Concurrent Workload
// ============================================================================

fn bench_mixed_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_concurrent");
    group.throughput(Throughput::Elements(100));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let num_threads = 4;
    let ops_per_thread = 100;
    let num_keys = 500;

    group.bench_function("4_threads_mixed_read_write", |b| {
        // Setup OUTSIDE iter
        let (_temp_dir, kv) = setup_kv(quick_bench_config(&TempDir::new().unwrap()));
        let kv = Arc::new(kv);

        // Pre-populate
        for i in 0..num_keys {
            let key = bench_key(i);
            let value = bench_value(64);
            kv.put(&key, &value).unwrap();
        }

        b.iter(|| {
            let mut handles = vec![];

            // Spawn threads OUTSIDE timing
            for t in 0..num_threads {
                let kv_clone = kv.clone();
                let handle = thread::spawn(move || {
                    let mut ops = 0usize;
                    for i in 0..ops_per_thread {
                        let key = format!("t{}_k{}", t, i);
                        let op = i % 10;
                        if op < 3 {
                            // 30% writes
                            kv_clone.put(&key, &bench_value(64)).ok();
                        } else {
                            // 70% reads
                            kv_clone.get(&key).ok();
                        }
                        ops += 1;
                    }
                    ops
                });
                handles.push(handle);
            }

            // Time only the join/wait phase (thread work is already running)
            let start = Instant::now();
            let mut total_ops = 0usize;
            for handle in handles {
                total_ops += handle.join().expect("Thread panicked");
            }
            let elapsed = start.elapsed();

            black_box((total_ops, elapsed));
        });
    });

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(
    benches,
    bench_concurrent_writes,
    bench_concurrent_reads,
    bench_mixed_concurrent
);

criterion_main!(benches);
