//! INNO-002: Range Query End-to-End Benchmarks
//!
//! This benchmark suite measures the performance of INNO-002 features:
//! - Zone Map-based block pruning
//! - Sequential prefetching

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

use tokitai_filekv::cache::block_cache::BlockCacheConfig;
use tokitai_filekv::compaction::CompactionConfig;
use tokitai_filekv::core::types::BlockCompressionConfig;
use tokitai_filekv::io::MemFs;
use tokitai_filekv::AuditLogConfig;
use tokitai_filekv::{AggressiveConfig, FileKV, FileKVConfig, MemTableConfig, RangeEntry};

/// Create a test FileKV instance with INNO-002 enabled
/// 可配置数据量和 segment 数量
fn create_test_kv_with_inno002(num_keys: usize, num_segments: usize) -> (FileKV, TempDir) {
    let temp_dir = TempDir::new().unwrap();

    // 计算每个 segment 的 key 数量
    let keys_per_segment = num_keys / num_segments;
    let flush_threshold = if num_segments == 1 {
        10 * 1024 * 1024 // 10MB for single segment
    } else {
        // 根据数据量动态调整 flush threshold
        (keys_per_segment * 100) as u64 // ~100 bytes per key-value pair
    };

    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        enable_wal: false,
        enable_bloom: true,
        enable_background_flush: false,
        block_size: 8192,
        block_compression: BlockCompressionConfig::default(),
        memtable: MemTableConfig {
            flush_threshold_bytes: flush_threshold.max(64 * 1024) as usize, // 最小 64KB
            max_entries: 100000,
            max_memory_bytes: 64 * 1024 * 1024,
            ..Default::default()
        },
        cache: BlockCacheConfig {
            max_memory_bytes: 128 * 1024 * 1024,
            max_items: 10000,
            frequency_aware: false,
        },
        compaction: CompactionConfig::default(),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        audit_log: AuditLogConfig::default(),
        aggressive: AggressiveConfig::balanced(),
        fs: Arc::new(MemFs::new()),
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    // 分批插入数据
    let batch_size = 1000.min(num_keys);
    let mut inserted = 0;

    while inserted < num_keys {
        let end = (inserted + batch_size).min(num_keys);
        let entries: Vec<(String, Vec<u8>)> = (inserted..end)
            .map(|i| {
                let key = format!("key_{:08}", i);
                let value = format!("value_{:08}_{}", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").into_bytes();
                (key, value)
            })
            .collect();

        let entries_ref: Vec<(&str, &[u8])> = entries.iter().map(|(k, v)| (k.as_str(), v.as_slice())).collect();

        kv.put_batch(&entries_ref).unwrap();
        inserted = end;

        // 定期 flush 来创建多个 segment
        if num_segments > 1 && inserted % keys_per_segment == 0 && inserted > 0 {
            kv.flush_memtable().unwrap();
        }
    }

    // 最终 flush
    kv.flush_memtable().unwrap();

    (kv, temp_dir)
}

/// Benchmark range query with different range sizes and data scales
fn bench_range_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_query");

    // 测试不同数据规模：10K, 100K
    for &num_keys in &[10_000, 100_000] {
        let (kv, _temp_dir) = create_test_kv_with_inno002(num_keys, 1);

        // 测试不同 range 大小
        let range_sizes = if num_keys >= 10_000 {
            vec![10, 100, 1000, 10_000]
        } else {
            vec![10, 100, 1000]
        };

        for range_size in range_sizes {
            if range_size > num_keys {
                continue;
            }

            let start_key = format!("key_{:08}", 0);
            let end_key = format!("key_{:08}", range_size - 1);

            group.bench_with_input(
                BenchmarkId::new(format!("{}keys", num_keys), format!("range={}", range_size)),
                &(start_key, end_key),
                |b, (start, end)| {
                    b.iter(|| {
                        let mut count = 0;
                        let iterator = kv.range(black_box(start.as_str()), black_box(end.as_str())).unwrap();
                        for result in iterator {
                            let _entry: RangeEntry = result.unwrap();
                            count += 1;
                        }
                        assert_eq!(count, range_size, "Should return exactly {} entries", range_size);
                        count
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark range query across multiple segments
fn bench_range_query_cross_segment(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_query_cross_segment");

    // 创建多 segment 场景：100K keys 分布在 5 个 segment
    let (kv, _temp_dir) = create_test_kv_with_inno002(100_000, 5);

    // 测试跨 segment 的 range query
    group.bench_function("cross_segment_small_range", |b| {
        // 小范围查询（可能只涉及 1-2 个 segment）
        let start_key = "key_00040000";
        let end_key = "key_00040099";
        b.iter(|| {
            let mut count = 0;
            let iterator = kv.range(black_box(start_key), black_box(end_key)).unwrap();
            for result in iterator {
                let _entry: RangeEntry = result.unwrap();
                count += 1;
            }
            count
        });
    });

    group.bench_function("cross_segment_large_range", |b| {
        // 大范围查询（涉及多个 segment）
        let start_key = "key_00020000";
        let end_key = "key_00079999";
        b.iter(|| {
            let mut count = 0;
            let iterator = kv.range(black_box(start_key), black_box(end_key)).unwrap();
            for result in iterator {
                let _entry: RangeEntry = result.unwrap();
                count += 1;
            }
            count
        });
    });

    group.finish();
}

/// Benchmark sequential prefetching
fn bench_sequential_prefetch(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_prefetch");
    group.throughput(Throughput::Elements(1));

    // 测试不同数据规模
    for &num_keys in &[10_000, 100_000] {
        let (kv, _temp_dir) = create_test_kv_with_inno002(num_keys, 1);

        group.bench_with_input(
            BenchmarkId::new(format!("{}keys", num_keys), "100_sequential"),
            &num_keys,
            |b, &_n| {
                b.iter(|| {
                    let mut count = 0;
                    for i in 0..100 {
                        let key = format!("key_{:08}", i);
                        if let Ok(Some(_value)) = kv.get(black_box(&key)) {
                            count += 1;
                        }
                    }
                    count
                });
            },
        );
    }

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
        bench_range_query,
        bench_range_query_cross_segment,
        bench_sequential_prefetch,
);

criterion_main!(benches);
