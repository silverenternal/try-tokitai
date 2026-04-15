//! Adaptive Bloom Filter Cache Benchmarks
//!
//! INNO-001: Performance benchmarks for multi-layer bloom filter cache

use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;
use tokitai_filekv::{BloomFilter, ASMS};

use tokitai_filekv::{
    AdaptiveBloomCache, AdaptiveBloomCacheConfig,
    BloomFilterCache, BloomFilterCacheConfig,
    FPRController,
};
use tokitai_filekv::bloom::migration::{
    MigrationController, MigrationThresholds,
};
use tokitai_filekv::bloom::compressed::CompressedBloom;
use tokitai_filekv::core::error::FileKVResult;

/// Create test bloom filter
fn create_test_bloom(num_elements: usize, fpr: f64) -> BloomFilter {
    let mut bloom = BloomFilter::with_rate(fpr as f32, num_elements as u32);
    for i in 0..num_elements {
        ASMS::insert(&mut bloom, &format!("key_{}", i));
    }
    bloom
}

/// Mock loader for AdaptiveBloomCache (returns BloomFilter + zone_map entries)
fn mock_adaptive_loader(_segment_id: u64) -> FileKVResult<Option<(BloomFilter, Vec<String>)>> {
    Ok(Some((create_test_bloom(1000, 0.01), vec![])))
}

/// Mock loader for BloomFilterCache (returns just BloomFilter)
fn mock_bfc_loader(_segment_id: u64) -> FileKVResult<Option<BloomFilter>> {
    Ok(Some(create_test_bloom(1000, 0.01)))
}

/// Benchmark adaptive bloom cache insert
fn bench_adaptive_bloom_insert(c: &mut Criterion) {
    let config = AdaptiveBloomCacheConfig::default();
    let cache = AdaptiveBloomCache::try_new(config).unwrap();

    let mut group = c.benchmark_group("adaptive_bloom_insert");
    group.throughput(Throughput::Elements(1));

    group.bench_function("insert_l1", |b| {
        b.iter(|| {
            let bloom = create_test_bloom(1000, 0.01);
            cache.insert(black_box(1), bloom);
        })
    });

    group.bench_function("insert_l2_compressed", |b| {
        b.iter(|| {
            let bloom = create_test_bloom(1000, 0.01);
            cache.insert(black_box(2), bloom);
            for i in 3..8 {
                let b = create_test_bloom(1000, 0.01);
                cache.insert(i, b);
            }
        })
    });

    group.finish();
}

/// Benchmark adaptive bloom cache get
fn bench_adaptive_bloom_get(c: &mut Criterion) {
    let config = AdaptiveBloomCacheConfig::default();
    let cache = AdaptiveBloomCache::try_new(config).unwrap();

    for i in 1..=100 {
        let bloom = create_test_bloom(1000, 0.01);
        cache.insert(i, bloom);
    }

    let mut group = c.benchmark_group("adaptive_bloom_get");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_l1_hit", |b| {
        b.iter(|| {
            let _ = cache.get(black_box(1), &mock_adaptive_loader);
        })
    });

    group.bench_function("get_l2_hit", |b| {
        for i in 101..150 {
            let _ = cache.get(i, &mock_adaptive_loader);
        }
        b.iter(|| {
            let _ = cache.get(black_box(200), &mock_adaptive_loader);
        })
    });

    group.bench_function("get_l3_miss", |b| {
        b.iter(|| {
            let _ = cache.get(black_box(9999), &mock_adaptive_loader);
        })
    });

    group.finish();
}

/// Benchmark bloom filter contains check
fn bench_adaptive_bloom_contains(c: &mut Criterion) {
    let config = AdaptiveBloomCacheConfig::default();
    let cache = AdaptiveBloomCache::try_new(config).unwrap();

    let mut bloom = create_test_bloom(1000, 0.01);
    ASMS::insert(&mut bloom, &"target_key".to_string());
    cache.insert(1, bloom);

    let mut group = c.benchmark_group("adaptive_bloom_contains");
    group.throughput(Throughput::Elements(1));

    group.bench_function("contains_positive", |b| {
        b.iter(|| {
            let result = cache.contains(black_box(1), "target_key", &mock_adaptive_loader);
            debug_assert!(result.unwrap().unwrap());
        })
    });

    group.bench_function("contains_negative", |b| {
        b.iter(|| {
            let result = cache.contains(black_box(1), "nonexistent_key", &mock_adaptive_loader);
            debug_assert!(!result.unwrap().unwrap());
        })
    });

    group.finish();
}

/// Benchmark traditional bloom filter cache
fn bench_traditional_bloom_cache(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let config = BloomFilterCacheConfig::default();
    let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

    for i in 1..=100 {
        let bloom = create_test_bloom(1000, 0.01);
        cache.insert(i, bloom);
    }

    let mut group = c.benchmark_group("traditional_bloom_cache");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_hit", |b| {
        b.iter(|| {
            let _ = cache.get(black_box(1), &mock_bfc_loader);
        })
    });

    group.bench_function("get_miss", |b| {
        b.iter(|| {
            let _ = cache.get(black_box(9999), &mock_bfc_loader);
        })
    });

    group.finish();
}

/// Compare adaptive vs traditional cache performance
fn bench_adaptive_vs_traditional(c: &mut Criterion) {
    let adaptive_config = AdaptiveBloomCacheConfig::default();
    let adaptive_cache = AdaptiveBloomCache::try_new(adaptive_config).unwrap();

    let temp_dir = TempDir::new().unwrap();
    let traditional_config = BloomFilterCacheConfig::default();
    let traditional_cache = BloomFilterCache::new(traditional_config, temp_dir.path().to_path_buf());

    for i in 1..=100 {
        let bloom1 = create_test_bloom(1000, 0.01);
        let bloom2 = create_test_bloom(1000, 0.01);
        adaptive_cache.insert(i, bloom1);
        traditional_cache.insert(i, bloom2);
    }

    let mut group = c.benchmark_group("adaptive_vs_traditional");
    group.throughput(Throughput::Elements(1));

    group.bench_function("adaptive_cache_get", |b| {
        b.iter(|| {
            let _ = adaptive_cache.get(black_box(50), &mock_adaptive_loader);
        })
    });

    group.bench_function("traditional_cache_get", |b| {
        b.iter(|| {
            let _ = traditional_cache.get(black_box(50), &mock_bfc_loader);
        })
    });

    group.finish();
}

/// Benchmark compression ratio
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    group.bench_function("compress_sparse_bloom", |b| {
        b.iter(|| {
            let bits = vec![0u8; 1024];
            let compressed = CompressedBloom::compress(&bits, false);
            debug_assert!(compressed.is_ok());
        })
    });

    group.bench_function("compress_dense_bloom", |b| {
        b.iter(|| {
            let mut bits = vec![0u8; 1024];
            for i in 0..512 {
                bits[i] = 0xFF;
            }
            let compressed = CompressedBloom::compress(&bits, false);
            debug_assert!(compressed.is_ok());
        })
    });

    group.bench_function("compress_with_huffman", |b| {
        b.iter(|| {
            let bits = vec![0u8; 1024];
            let compressed = CompressedBloom::compress(&bits, true);
            debug_assert!(compressed.is_ok());
        })
    });

    group.finish();
}

/// Benchmark migration performance
fn bench_migration_performance(c: &mut Criterion) {
    let thresholds = MigrationThresholds::default();
    let controller = MigrationController::new(thresholds);

    let mut group = c.benchmark_group("migration");
    group.throughput(Throughput::Elements(1));

    group.bench_function("record_access_low_qps", |b| {
        b.iter(|| {
            let _ = controller.record_access(black_box(1));
        })
    });

    group.bench_function("record_access_high_qps", |b| {
        b.iter(|| {
            let _ = controller.record_access(black_box(1));
        })
    });

    group.finish();
}

/// Benchmark FPR controller
fn bench_fpr_controller(c: &mut Criterion) {
    let controller = FPRController::with_defaults();

    let mut group = c.benchmark_group("fpr_controller");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_level", |b| {
        b.iter(|| {
            let _ = controller.get_level(black_box(1));
        })
    });

    group.bench_function("get_fpr", |b| {
        b.iter(|| {
            let _ = controller.get_current_fpr(black_box(1));
        })
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(50)
        .noise_threshold(0.02)
        .significance_level(0.05);
    targets =
        bench_adaptive_bloom_insert,
        bench_adaptive_bloom_get,
        bench_adaptive_bloom_contains,
        bench_traditional_bloom_cache,
        bench_adaptive_vs_traditional,
        bench_compression_ratio,
        bench_migration_performance,
        bench_fpr_controller,
);

criterion_main!(benches);
