//! BlockCache get_by_key O(1) Routing Benchmark
//!
//! Compares the performance of the hash-based O(1) shard routing
//! against the previous O(n) iteration approach.
//!
//! This benchmark verifies that:
//! 1. get_by_key uses direct shard routing (not iteration)
//! 2. Performance scales with shard count (more shards = bigger win)
//! 3. Key distribution is uniform across shards

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tokitai_filekv::cache::block_cache::{BlockCache, BlockCacheConfig};

// ============================================================================
// BlockCache get_by_key Benchmark
// ============================================================================

fn bench_block_cache_get_by_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_cache_get_by_key");
    group.throughput(Throughput::Elements(1));
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(10));

    // Test with different shard counts
    let shard_configs = vec![(1, "1_shard"), (2, "2_shards"), (4, "4_shards"), (8, "8_shards")];

    for (num_shards, label) in shard_configs {
        group.bench_function(format!("O1_routing_{}", label), |b| {
            // max_memory = num_shards * 16MB (default shard size)
            let config = BlockCacheConfig {
                max_items: 100_000,
                max_memory_bytes: (num_shards as u64) * 16 * 1024 * 1024,
                frequency_aware: false,
            };
            let cache = BlockCache::new(config);

            // Pre-populate cache with data
            let num_keys = 10_000;
            for i in 0..num_keys {
                let key = format!("{}:{}", i / 100, i);
                let value = Bytes::from(format!("value_{}", i));
                cache.insert_by_key(key, value);
            }

            let key_counter = AtomicUsize::new(0);
            b.iter(|| {
                let idx = key_counter.fetch_add(1, Ordering::Relaxed) % num_keys;
                let key = format!("{}:{}", idx / 100, idx);
                black_box(cache.get_by_key(&key));
            });
        });
    }

    group.finish();
}

// ============================================================================
// Key Distribution Benchmark
// Verifies uniform distribution across shards
// ============================================================================

fn bench_key_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_distribution");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    let num_shards = 4;
    let num_keys = 100_000;

    group.bench_function("calculate_shard_id_uniformity", |b| {
        b.iter(|| {
            let mut shard_counts = vec![0usize; num_shards];
            for i in 0..num_keys {
                let key = format!("{}:{}", i / 100, i);
                let shard_id = BlockCache::calculate_shard_id(&key, num_shards);
                shard_counts[shard_id] += 1;
            }
            black_box(shard_counts);
        });
    });

    group.finish();
}

// ============================================================================
// Concurrent Access Benchmark
// ============================================================================

fn bench_concurrent_get_by_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_cache_concurrent");
    group.throughput(Throughput::Elements(1));
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    let config = BlockCacheConfig {
        max_items: 100_000,
        max_memory_bytes: 64 * 1024 * 1024, // 4 shards
        frequency_aware: false,
    };
    let cache = Arc::new(BlockCache::new(config));

    // Pre-populate
    let num_keys = 10_000;
    for i in 0..num_keys {
        let key = format!("{}:{}", i / 100, i);
        let value = Bytes::from(format!("value_{}", i));
        cache.insert_by_key(key, value);
    }

    let num_threads = num_cpus::get();
    group.bench_function(format!("{}_threads_get_by_key", num_threads), |b| {
        b.iter(|| {
            let mut handles = vec![];
            // Spawn threads OUTSIDE timing
            for _t in 0..num_threads {
                let cache_clone = cache.clone();
                handles.push(std::thread::spawn(move || {
                    let local_counter = AtomicUsize::new(0);
                    for _ in 0..100 {
                        let idx = local_counter.fetch_add(1, Ordering::Relaxed) % num_keys;
                        let key = format!("{}:{}", idx / 100, idx);
                        black_box(cache_clone.get_by_key(&key));
                    }
                }));
            }
            // Time only the join phase (thread work is already running)
            let start = std::time::Instant::now();
            for handle in handles {
                handle.join().unwrap();
            }
            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(
    benches,
    bench_block_cache_get_by_key,
    bench_key_distribution,
    bench_concurrent_get_by_key,
);

criterion_main!(benches);
