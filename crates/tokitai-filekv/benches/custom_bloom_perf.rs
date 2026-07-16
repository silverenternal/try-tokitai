//! CustomBloom Performance Benchmarks
//!
//! Measures:
//! - V3 format load time (direct bitset load)
//! - V1/V2 format load time (rebuild from keys)
//! - Negative query latency (key not in bloom)
//! - Positive query latency (key in bloom)
//! - 1000 segments cache hit rate simulation
//!
//! Setup is OUTSIDE b.iter().

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use rand::Rng;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

use bloom::BloomFilter;
use tokitai_filekv::bloom::{load_custom_bloom_with_migration, CustomBloom};
use tokitai_filekv::ASMS;

const DEFAULT_FPR: f32 = 0.01;

/// Create a temp directory for tests
fn temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

/// Create and save a V3 format CustomBloom
fn create_and_save_v3(dir: &TempDir, segment_id: u64, num_items: usize, fpr: f64) -> PathBuf {
    let path = dir.path().join(format!("bloom_{:06}.bin", segment_id));
    let mut bloom = CustomBloom::with_capacity(num_items, fpr);
    for i in 0..num_items {
        bloom.insert(format!("key_{}", i).as_bytes());
    }
    bloom.save_to_file(&path).unwrap();
    path
}

/// Create and save a V2 format BloomFilter (old format)
fn create_and_save_v2(dir: &TempDir, segment_id: u64, num_items: usize) -> PathBuf {
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use tokitai_filekv::BLOOM_MAGIC;

    let path = dir.path().join(format!("bloom_{:06}.bin", segment_id));
    let mut bloom = BloomFilter::with_rate(DEFAULT_FPR, num_items as u32);
    let keys: Vec<String> = (0..num_items).map(|i| format!("key_{}", i)).collect();
    for key in &keys {
        bloom.insert(key);
    }

    // Save in V2 format manually
    let temp_path = dir.path().join(format!("bloom_{:06}.tmp", segment_id));
    let mut file = BufWriter::new(File::create(&temp_path).unwrap());

    const CURRENT_BLOOM_VERSION: u32 = 2;
    file.write_all(&BLOOM_MAGIC.to_le_bytes()).unwrap();
    file.write_all(&CURRENT_BLOOM_VERSION.to_le_bytes()).unwrap();
    file.write_all(&(bloom.num_bits() as u32).to_le_bytes()).unwrap();
    file.write_all(&bloom.num_hashes().to_le_bytes()).unwrap();
    file.write_all(&(keys.len() as u64).to_le_bytes()).unwrap();
    for key in &keys {
        let key_bytes = key.as_bytes();
        file.write_all(&(key_bytes.len() as u32).to_le_bytes()).unwrap();
        file.write_all(key_bytes).unwrap();
    }
    file.flush().unwrap();
    file.get_ref().sync_all().unwrap();
    drop(file);

    fs::rename(&temp_path, &path).unwrap();
    path
}

// ============================================================================
// V3 Load Performance
// ============================================================================

fn bench_v3_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("v3_load");

    let sizes = vec![100, 1000, 10000, 100000];

    for size in sizes {
        group.bench_function(format!("load_{}_items", size), |b| {
            let dir = temp_dir();
            let _path = create_and_save_v3(&dir, 1, size, 0.01);
            let path = dir.path().join("bloom_000001.bin");

            b.iter(|| {
                let bloom = CustomBloom::load_from_file(&path).unwrap().unwrap();
                black_box(bloom.num_bits());
            });
        });
    }

    group.finish();
}

// ============================================================================
// V2 Load Performance (with migration)
// ============================================================================

fn bench_v2_load_with_migration(c: &mut Criterion) {
    let mut group = c.benchmark_group("v2_load_with_migration");

    let sizes = vec![100, 1000, 10000];

    for size in sizes {
        group.bench_function(format!("load_and_migrate_{}_items", size), |b| {
            let dir = temp_dir();
            let _path = create_and_save_v2(&dir, 1, size);

            b.iter(|| {
                let bloom = load_custom_bloom_with_migration(dir.path(), 1).unwrap().unwrap();
                black_box(bloom.num_bits());
            });
        });
    }

    group.finish();
}

// ============================================================================
// Negative Query Performance
// ============================================================================

fn bench_negative_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("negative_query");
    group.throughput(Throughput::Elements(1));

    let sizes = vec![1000, 10000, 100000];

    for size in sizes {
        let _dir = temp_dir();
        let bloom = CustomBloom::with_capacity(size, 0.01);

        group.bench_function(format!("{}items", size), |b| {
            let key_counter = AtomicUsize::new(0);
            b.iter(|| {
                let key = format!("nonexistent_{}", key_counter.fetch_add(1, Ordering::Relaxed));
                black_box(bloom.contains(key.as_bytes()));
            });
        });
    }

    group.finish();
}

// ============================================================================
// Positive Query Performance
// ============================================================================

fn bench_positive_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("positive_query");
    group.throughput(Throughput::Elements(1));

    let sizes = vec![1000, 10000, 100000];

    for size in sizes {
        let mut bloom = CustomBloom::with_capacity(size, 0.01);
        for i in 0..size {
            bloom.insert(format!("key_{}", i).as_bytes());
        }

        group.bench_function(format!("{}items", size), |b| {
            let key_counter = AtomicUsize::new(0);
            b.iter(|| {
                let key = format!("key_{}", key_counter.fetch_add(1, Ordering::Relaxed) % size);
                black_box(bloom.contains(key.as_bytes()));
            });
        });
    }

    group.finish();
}

// ============================================================================
// 1000 Segments Cache Hit Rate Simulation
// ============================================================================

fn bench_1000_segments_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("1000_segments_cache");

    let num_segments = 1000;
    let items_per_segment = 1000;
    let cache_capacity = 800; // Cache can hold 80% of segments

    // Create V3 bloom filters for 1000 segments
    let dir = temp_dir();
    for seg_id in 0..num_segments {
        create_and_save_v3(&dir, seg_id as u64, items_per_segment, 0.01);
    }

    // Simulate LRU cache with zipfian access pattern (80/20 rule)
    group.bench_function("zipfian_access_lru_cache", |b| {
        use std::collections::HashMap;

        let mut cache: HashMap<u64, CustomBloom> = HashMap::with_capacity(cache_capacity);
        let mut hits = 0u64;
        let mut misses = 0u64;
        let mut lru_order: Vec<u64> = Vec::with_capacity(cache_capacity);

        b.iter_custom(|iters| {
            let start = std::time::Instant::now();

            for _ in 0..iters {
                // Zipfian-like access: prefer hot segments (first 20%)
                let mut rng = rand::thread_rng();
                let segment_id = if rng.gen_range(0..100) < 80 {
                    // Hot segment: first 20%
                    rng.gen_range(0..(num_segments / 5)) as u64
                } else {
                    // Cold segment: remaining 80%
                    (num_segments / 5 + rng.gen_range(0..(num_segments * 4 / 5))) as u64
                };

                // Check cache
                if cache.contains_key(&segment_id) {
                    hits += 1;
                    // Move to end of LRU (most recent)
                    if let Some(pos) = lru_order.iter().position(|&id| id == segment_id) {
                        lru_order.remove(pos);
                        lru_order.push(segment_id);
                    }
                } else {
                    misses += 1;
                    // Load from disk
                    if let Ok(Some(bloom)) =
                        CustomBloom::load_from_file(&dir.path().join(format!("bloom_{:06}.bin", segment_id)))
                    {
                        // Evict if full
                        if cache.len() >= cache_capacity {
                            if let Some(evict_id) = lru_order.first().copied() {
                                cache.remove(&evict_id);
                                lru_order.remove(0);
                            }
                        }
                        cache.insert(segment_id, bloom);
                        lru_order.push(segment_id);
                    }
                }
            }

            let hit_rate = hits as f64 / (hits + misses) as f64;
            println!(
                "  Cache hit rate: {:.2}% ({} hits, {} misses)",
                hit_rate * 100.0,
                hits,
                misses
            );

            start.elapsed()
        });
    });

    group.finish();
}

// ============================================================================
// V3 vs V2 File Size Comparison
// ============================================================================

fn bench_file_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_size_comparison");

    let sizes = vec![100, 1000, 10000];

    for size in sizes {
        let dir = temp_dir();

        // Create V3
        let v3_path = create_and_save_v3(&dir, 1, size, 0.01);
        let v3_size = fs::metadata(&v3_path).unwrap().len();

        // Create V2
        let v2_path = create_and_save_v2(&dir, 2, size);
        let v2_size = fs::metadata(&v2_path).unwrap().len();

        println!(
            "Size comparison ({} items): V3={} bytes, V2={} bytes, savings={:.1}%",
            size,
            v3_size,
            v2_size,
            (1.0 - v3_size as f64 / v2_size as f64) * 100.0
        );
    }

    group.bench_function("dummy", |b| b.iter(|| black_box(())));

    group.finish();
}

// ============================================================================
// Criterion main
// ============================================================================

criterion_group!(
    benches,
    bench_v3_load,
    bench_v2_load_with_migration,
    bench_negative_query,
    bench_positive_query,
    bench_1000_segments_cache,
    bench_file_size_comparison
);

criterion_main!(benches);
