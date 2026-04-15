//! FileKV 性能基准测试
//!
//! 测试 FileKV 核心性能指标：
//! - 写入延迟（有 WAL vs 无 WAL）
//! - 批量写入性能 (put_batch vs 循环 put)
//! - 读取延迟（热点缓存命中）
//!
//! **修复记录 (2026-04-10)**:
//! - BENCH-001/002: 修复 setup 在 iter 内的问题，现在测量的是真实操作

use std::time::Duration;
use std::sync::atomic::{AtomicU64, Ordering};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

use tokitai_filekv::{FileKV, FileKVConfig, MemTableConfig, DictionaryCompressionConfig, AggressiveConfig};
use tokitai_filekv::cache::block_cache::BlockCacheConfig;
use tokitai_filekv::io::StdFs;
use tokitai_filekv::compaction::CompactionConfig;
use tokitai_filekv::core::types::BlockCompressionConfig;
use tokitai_filekv::AuditLogConfig;

/// 创建测试用的 FileKV 实例
fn setup_file_kv_internal(enable_wal: bool) -> (tempfile::TempDir, FileKV) {
    let temp_dir = tempfile::tempdir().unwrap();
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");

    std::fs::create_dir_all(&segment_dir).unwrap();
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&wal_dir).unwrap();

    let config = FileKVConfig {
        memtable: MemTableConfig {
            flush_threshold_bytes: 4 * 1024 * 1024,
            max_entries: 100_000,
            max_memory_bytes: 64 * 1024 * 1024,
        },
        segment_dir,
        enable_wal,
        wal_dir,
        index_dir,
        cache: BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        block_size: 8192,
        block_compression: BlockCompressionConfig::default(),
        compaction: CompactionConfig {
            min_segments: 4,
            auto_compact: false,
            check_interval: 100,
            max_segment_size_bytes: 16 * 1024 * 1024,
            target_segment_size_bytes: 8 * 1024 * 1024,
            async_compaction_enabled: false,
            leveled_compaction_enabled: true,
            level_size_multiplier: 10,
            max_level: 3,
            l0_file_count_threshold: 4,
            parallel_compaction_enabled: true,
            streaming_compaction_enabled: true,
        },
        segment_preallocate_size: 16 * 1024 * 1024,
        wal_max_size_bytes: 100 * 1024 * 1024,
        wal_max_files: 5,
        
        cache_warming_enabled: false,
        compression: DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 4,
        async_io_max_queue_depth: 1024,
        async_io_write_timeout_ms: 5000,
        async_io_enable_coalescing: false,
        async_io_coalesce_window_ms: 10,
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        audit_log: AuditLogConfig {
            log_dir: temp_dir.path().join("audit_logs"),
            enabled: false,
            rotation_interval_hours: 24,
            retention_days: 30,
        },
        aggressive: AggressiveConfig::performance(),
        enable_adaptive_bloom_cache: true,
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
        enable_background_cache_rebalance: false,
        fs: std::sync::Arc::new(StdFs),
    };

    let kv = FileKV::open(config).unwrap();
    (temp_dir, kv)
}

fn setup_file_kv_no_wal() -> (tempfile::TempDir, FileKV) {
    setup_file_kv_internal(false)
}

fn setup_file_kv_with_wal() -> (tempfile::TempDir, FileKV) {
    setup_file_kv_internal(true)
}

/// 创建测试用的 FileKV 实例（小 flush threshold，用于测试 segment 读写）
fn setup_file_kv_with_small_flush() -> (tempfile::TempDir, FileKV) {
    let temp_dir = tempfile::tempdir().unwrap();
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");

    std::fs::create_dir_all(&segment_dir).unwrap();
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&wal_dir).unwrap();

    let config = FileKVConfig {
        memtable: MemTableConfig {
            flush_threshold_bytes: 64 * 1024,  // 64KB，最小允许值
            max_entries: 100_000,
            max_memory_bytes: 64 * 1024 * 1024,
        },
        segment_dir,
        enable_wal: false,
        wal_dir,
        index_dir,
        cache: BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        block_size: 8192,
        block_compression: BlockCompressionConfig::default(),
        compaction: CompactionConfig {
            min_segments: 4,
            auto_compact: false,
            check_interval: 100,
            max_segment_size_bytes: 16 * 1024 * 1024,
            target_segment_size_bytes: 8 * 1024 * 1024,
            async_compaction_enabled: false,
            leveled_compaction_enabled: true,
            level_size_multiplier: 10,
            max_level: 3,
            l0_file_count_threshold: 4,
            parallel_compaction_enabled: true,
            streaming_compaction_enabled: true,
        },
        segment_preallocate_size: 16 * 1024 * 1024,
        wal_max_size_bytes: 100 * 1024 * 1024,
        wal_max_files: 5,
        
        cache_warming_enabled: false,
        compression: DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 4,
        async_io_max_queue_depth: 1024,
        async_io_write_timeout_ms: 5000,
        async_io_enable_coalescing: false,
        async_io_coalesce_window_ms: 10,
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        audit_log: AuditLogConfig {
            log_dir: temp_dir.path().join("audit_logs"),
            enabled: false,
            rotation_interval_hours: 24,
            retention_days: 30,
        },
        aggressive: AggressiveConfig::performance(),
        enable_adaptive_bloom_cache: true,
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
        enable_background_cache_rebalance: false,
        fs: std::sync::Arc::new(StdFs),
    };

    let kv = FileKV::open(config).unwrap();
    (temp_dir, kv)
}

/// 基准测试：单次写入性能 (对比有 WAL vs 无 WAL)
/// 修复：使用 iter_with_large_setup 将 FileKV 创建移到 setup 阶段
fn bench_single_write(c: &mut Criterion) {
    // === 无 WAL 测试 ===
    let mut group_no_wal = c.benchmark_group("Single Write (No WAL)");
    group_no_wal.measurement_time(Duration::from_secs(5));
    group_no_wal.warm_up_time(Duration::from_secs(1));

    group_no_wal.bench_function("64B", |b| {
        let (_temp_dir, kv) = setup_file_kv_no_wal();
        b.iter(|| {
            let key = format!("bench_key_{:012}", rand::random::<u64>());
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(&key, value)).unwrap();
        });
    });

    group_no_wal.bench_function("1KB", |b| {
        let (_temp_dir, kv) = setup_file_kv_no_wal();
        b.iter(|| {
            let key = format!("bench_key_{:012}", rand::random::<u64>());
            let value = vec![b'x'; 1024];
            black_box(kv.put(&key, &value)).unwrap();
        });
    });

    group_no_wal.bench_function("4KB", |b| {
        let (_temp_dir, kv) = setup_file_kv_no_wal();
        b.iter(|| {
            let key = format!("bench_key_{:012}", rand::random::<u64>());
            let value = vec![b'x'; 4096];
            black_box(kv.put(&key, &value)).unwrap();
        });
    });

    group_no_wal.finish();

    // === 有 WAL 测试 ===
    let mut group_with_wal = c.benchmark_group("Single Write (With WAL)");
    group_with_wal.measurement_time(Duration::from_secs(5));
    group_with_wal.warm_up_time(Duration::from_secs(1));

    group_with_wal.bench_function("64B", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_wal();
        b.iter(|| {
            let key = format!("bench_key_{:012}", rand::random::<u64>());
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(&key, value)).unwrap();
        });
    });

    group_with_wal.bench_function("1KB", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_wal();
        b.iter(|| {
            let key = format!("bench_key_{:012}", rand::random::<u64>());
            let value = vec![b'x'; 1024];
            black_box(kv.put(&key, &value)).unwrap();
        });
    });

    group_with_wal.bench_function("4KB", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_wal();
        b.iter(|| {
            let key = format!("bench_key_{:012}", rand::random::<u64>());
            let value = vec![b'x'; 4096];
            black_box(kv.put(&key, &value)).unwrap();
        });
    });

    group_with_wal.finish();
}

/// 基准测试：批量写入性能
/// 修复：使用 iter_with_large_setup 将 FileKV 创建移到 setup 阶段
fn bench_batch_write(c: &mut Criterion) {
    // === 循环 put ===
    let mut group_loop = c.benchmark_group("Batch Write (Loop put)");
    group_loop.measurement_time(Duration::from_secs(10));
    group_loop.warm_up_time(Duration::from_secs(2));

    for &count in &[10, 100, 1000] {
        group_loop.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                let (_temp_dir, kv) = setup_file_kv_no_wal();
                b.iter(|| {
                    for i in 0..n {
                        let key = format!("key_{:08}", i);
                        let value = format!("value_{:08}_{}", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        kv.put(key.as_str(), value.as_bytes()).unwrap();
                    }
                });
            },
        );
    }
    group_loop.finish();

    // === put_batch ===
    let mut group_batch = c.benchmark_group("Batch Write (put_batch)");
    group_batch.measurement_time(Duration::from_secs(10));
    group_batch.warm_up_time(Duration::from_secs(2));

    for &count in &[10, 100, 1000] {
        group_batch.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                let data: Vec<(String, Vec<u8>)> = (0..n)
                    .map(|i| {
                        let key = format!("key_{:08}", i);
                        let value = format!("value_{:08}_{}", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").into_bytes();
                        (key, value)
                    })
                    .collect();

                let (_temp_dir, kv) = setup_file_kv_no_wal();
                b.iter(|| {
                    let entries: Vec<(&str, &[u8])> = data.iter()
                        .map(|(k, v)| (k.as_str(), v.as_slice()))
                        .collect();
                    black_box(kv.put_batch(&entries)).unwrap();
                });
            },
        );
    }
    group_batch.finish();

    // === put_batch with WAL ===
    let mut group_batch_wal = c.benchmark_group("Batch Write (put_batch + WAL)");
    group_batch_wal.measurement_time(Duration::from_secs(10));
    group_batch_wal.warm_up_time(Duration::from_secs(2));

    for &count in &[10, 100, 1000] {
        group_batch_wal.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                let data: Vec<(String, Vec<u8>)> = (0..n)
                    .map(|i| {
                        let key = format!("key_{:08}", i);
                        let value = format!("value_{:08}_{}", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").into_bytes();
                        (key, value)
                    })
                    .collect();

                let (_temp_dir, kv) = setup_file_kv_with_wal();
                b.iter(|| {
                    let entries: Vec<(&str, &[u8])> = data.iter()
                        .map(|(k, v)| (k.as_str(), v.as_slice()))
                        .collect();
                    black_box(kv.put_batch(&entries)).unwrap();
                });
            },
        );
    }
    group_batch_wal.finish();
}

/// 基准测试：单次读取性能（热数据，Block Cache 命中）
/// 修复：先写入足够多的数据并 flush 到 segment，确保测试真实的文件 I/O 路径
fn bench_single_read_hot(c: &mut Criterion) {
    let mut group = c.benchmark_group("Single Read (Hot from Segments)");
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));

    group.bench_function("64B value (segment hit)", |b| {
        // 使用极小的 flush_threshold 确保数据快速 flush 到 segment
        let (_temp_dir, kv) = setup_file_kv_with_small_flush();

        // 写入 100 个 key 并 flush 到 segment
        for i in 0..100 {
            let key = format!("hot_key_{:08}", i);
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            kv.put(&key, value).unwrap();
        }
        kv.flush_memtable().unwrap();

        // 预热 block cache
        for i in 0..100 {
            let key = format!("hot_key_{:08}", i);
            let _ = kv.get(&key);
        }

        // 测试从 segment + block cache 读取
        let target_key = "hot_key_00000050";
        b.iter(|| {
            black_box(kv.get(target_key)).unwrap();
        });
    });

    group.bench_function("1KB value (segment hit)", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_small_flush();

        for i in 0..100 {
            let key = format!("hot_key_{:08}", i);
            let value = vec![b'x'; 1024];
            kv.put(&key, &value).unwrap();
        }
        kv.flush_memtable().unwrap();

        // 预热 block cache
        for i in 0..100 {
            let key = format!("hot_key_{:08}", i);
            let _ = kv.get(&key);
        }

        let target_key = "hot_key_00000050";
        b.iter(|| {
            black_box(kv.get(target_key)).unwrap();
        });
    });

    group.finish();
}

/// 基准测试：Bloom Filter 负向查找
/// 修复：先写入数据并 flush 到 segment，确保 bloom filter 在 segment 上生效
fn bench_bloom_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bloom Filter (Segment-based)");
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));

    group.bench_function("Get non-existent key (bloom negative on segment)", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_small_flush();

        // 写入 1000 个 key 并 flush 到 segment
        for i in 0..1000 {
            let key = format!("existing_key_{:08}", i);
            let value = format!("value_{}", i);
            kv.put(&key, value.as_bytes()).unwrap();
        }
        kv.flush_memtable().unwrap();

        // 测试查找不存在的 key（应该被 bloom filter 快速过滤）
        b.iter(|| {
            black_box(kv.get("non_existent_key_12345678")).ok();
        });
    });

    group.finish();
}

/// 基准测试：从 segment 文件读取（memtable miss + segment hit）
/// 这个测试模拟真实场景：数据已经在 segment 文件中，需要从文件读取
fn bench_get_from_segments(c: &mut Criterion) {
    let mut group = c.benchmark_group("Single Read (Segment File I/O)");
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));

    group.bench_function("64B value (cold read, first access)", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_small_flush();

        // 写入数据并 flush 到 segment
        for i in 0..1000 {
            let key = format!("seg_key_{:08}", i);
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            kv.put(&key, value).unwrap();
        }
        kv.flush_memtable().unwrap();

        // 使用未访问过的 key 来模拟冷读取（key 存在于 segment 中）
        let mut key_idx = 0u64;
        b.iter(|| {
            let key = format!("seg_key_{:08}", key_idx);
            key_idx += 1;
            black_box(kv.get(&key)).unwrap();
        });
    });

    group.bench_function("64B value (warm cache, segment hit)", |b| {
        let (_temp_dir, kv) = setup_file_kv_with_small_flush();

        // 写入数据并 flush 到 segment
        for i in 0..1000 {
            let key = format!("seg_key_{:08}", i);
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            kv.put(&key, value).unwrap();
        }
        kv.flush_memtable().unwrap();

        // 预热 block cache
        for i in 0..1000 {
            let key = format!("seg_key_{:08}", i);
            let _ = kv.get(&key);
        }

        // 测试从 cache 读取
        let target_key = "seg_key_00000500";
        b.iter(|| {
            black_box(kv.get(target_key)).unwrap();
        });
    });

    group.finish();
}

/// 基准测试：写入吞吐量（修复 BENCH-006 + BENCH-007）
/// 修复：
/// - FileKV 创建移到 iter_custom 循环外，只创建一次
/// - 使用 AtomicU64 计数器生成唯一 key，避免重复写入相同 key
/// - 添加大数据量测试 (100K keys)
fn bench_write_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Write Throughput");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));
    group.throughput(Throughput::Elements(1));

    for &count in &[100, 1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter_custom(|iters| {
                    // BENCH-006 FIX: FileKV 创建在循环外，只创建一次
                    let (_temp_dir, kv) = setup_file_kv_no_wal();

                    // BENCH-007 FIX: 使用原子计数器生成唯一 key
                    let key_counter = AtomicU64::new(0);

                    let start = std::time::Instant::now();
                    // 在同一实例上运行多次写入迭代
                    for _ in 0..iters {
                        for i in 0..n {
                            // 使用唯一 key 而不是重复写入相同 key
                            let unique_id = key_counter.fetch_add(1, Ordering::Relaxed);
                            let key = format!("key_{:010}_{:08}", unique_id / 1000000, i);
                            let value = format!("value_{:08}", i);
                            kv.put(&key, value.as_bytes()).unwrap();
                        }
                    }
                    start.elapsed()
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
        bench_single_write,
        bench_batch_write,
        bench_single_read_hot,
        bench_bloom_filter,
        bench_get_from_segments,
        bench_write_throughput,
        bench_compaction,
        bench_random_vs_sequential_read,
        bench_write_amplification,
);
criterion_main!(benches);

/// 基准测试：Compaction 性能
/// 测量 compaction 触发和执行时间
fn bench_compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Compaction");
    group.measurement_time(Duration::from_secs(30));
    group.warm_up_time(Duration::from_secs(5));

    group.bench_function("trigger_auto_compaction", |b| {
        b.iter_custom(|iters| {
            let mut total_compaction_time = Duration::ZERO;

            for _ in 0..iters {
                // 创建新的 FileKV 实例，启用 auto compaction
                let temp_dir = tempfile::tempdir().unwrap();
                let segment_dir = temp_dir.path().join("segments");
                let index_dir = temp_dir.path().join("index");
                let wal_dir = temp_dir.path().join("wal");

                std::fs::create_dir_all(&segment_dir).unwrap();
                std::fs::create_dir_all(&index_dir).unwrap();
                std::fs::create_dir_all(&wal_dir).unwrap();

                let config = FileKVConfig {
                    memtable: MemTableConfig {
                        flush_threshold_bytes: 64 * 1024, // 64KB，快速触发 flush
                        max_entries: 100_000,
                        max_memory_bytes: 64 * 1024 * 1024,
                    },
                    segment_dir,
                    enable_wal: false,
                    wal_dir,
                    index_dir,
                    cache: BlockCacheConfig {
                        max_items: 10_000,
                        max_memory_bytes: 64 * 1024 * 1024,
                    },
                    enable_bloom: true,
                    enable_background_flush: false,
                    background_flush_interval_ms: 100,
                    block_size: 8192,
                    block_compression: BlockCompressionConfig::default(),
                    compaction: CompactionConfig {
                        min_segments: 4,
                        auto_compact: true, // 启用自动 compaction
                        check_interval: 100,
                        max_segment_size_bytes: 16 * 1024 * 1024,
                        target_segment_size_bytes: 8 * 1024 * 1024,
                        async_compaction_enabled: false,
                        leveled_compaction_enabled: true,
                        level_size_multiplier: 10,
                        max_level: 3,
                        l0_file_count_threshold: 4,
                        parallel_compaction_enabled: true,
                        streaming_compaction_enabled: true,
                    },
                    segment_preallocate_size: 16 * 1024 * 1024,
                    wal_max_size_bytes: 100 * 1024 * 1024,
                    wal_max_files: 5,
                    
                    cache_warming_enabled: false,
                    compression: DictionaryCompressionConfig::default(),
                    async_io_enabled: false,
                    async_io_max_concurrent_writes: 4,
                    async_io_max_queue_depth: 1024,
                    async_io_write_timeout_ms: 5000,
                    async_io_enable_coalescing: false,
                    async_io_coalesce_window_ms: 10,
                    checkpoint_dir: temp_dir.path().join("checkpoints"),
                    audit_log: AuditLogConfig {
                        log_dir: temp_dir.path().join("audit_logs"),
                        enabled: false,
                        rotation_interval_hours: 24,
                        retention_days: 30,
                    },
                    aggressive: AggressiveConfig::performance(),
                    enable_adaptive_bloom_cache: true,
                    enable_zone_map_pruning: true,
                    enable_sequential_prefetch: true,
                    enable_background_cache_rebalance: false,
        fs: std::sync::Arc::new(StdFs),
                };

                let kv = FileKV::open(config).unwrap();

                // 写入足够多的数据来触发 compaction (多个 segment)
                let start = std::time::Instant::now();
                for i in 0..10_000 {
                    let key = format!("compaction_key_{:010}", i);
                    let value = vec![b'x'; 1024]; // 1KB values
                    kv.put(&key, &value).unwrap();

                    // 定期 flush 来创建多个 segment
                    if i % 1000 == 0 && i > 0 {
                        kv.flush_memtable().unwrap();
                    }
                }
                kv.flush_memtable().unwrap();

                // 等待 compaction 完成（如果有）
                std::thread::sleep(Duration::from_millis(500));
                let elapsed = start.elapsed();
                total_compaction_time += elapsed;
            }

            total_compaction_time
        });
    });

    group.finish();
}

/// 基准测试：随机读 vs 顺序读对比
/// 测量不同访问模式下的读取性能
fn bench_random_vs_sequential_read(c: &mut Criterion) {
    // 准备数据
    let (_temp_dir, kv) = setup_file_kv_with_small_flush();

    // 写入 10,000 个 key 并 flush
    let num_keys = 10_000;
    for i in 0..num_keys {
        let key = format!("access_key_{:08}", i);
        let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
        kv.put(&key, value).unwrap();
    }
    kv.flush_memtable().unwrap();

    // 预热 cache
    for i in 0..num_keys {
        let key = format!("access_key_{:08}", i);
        let _ = kv.get(&key);
    }

    let mut group = c.benchmark_group("Random vs Sequential Read");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));
    group.throughput(Throughput::Elements(1));

    // 顺序读
    group.bench_function("sequential_read", |b| {
        let mut idx = 0u64;
        b.iter(|| {
            let key = format!("access_key_{:08}", idx % num_keys);
            idx += 1;
            black_box(kv.get(&key)).unwrap();
        });
    });

    // 随机读
    group.bench_function("random_read", |b| {
        b.iter(|| {
            let idx = rand::random::<u64>() % num_keys;
            let key = format!("access_key_{:08}", idx);
            black_box(kv.get(&key)).unwrap();
        });
    });

    group.finish();
}

/// 基准测试：写入放大测量
/// 测量实际写入磁盘的数据量与用户写入数据量的比值
fn bench_write_amplification(c: &mut Criterion) {
    let mut group = c.benchmark_group("Write Amplification");
    group.measurement_time(Duration::from_secs(20));
    group.warm_up_time(Duration::from_secs(3));

    for &num_keys in &[10_000, 50_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_keys),
            &num_keys,
            |b, &n| {
                b.iter_custom(|iters| {
                    let mut total_user_bytes: u64 = 0;
                    let mut total_disk_bytes: u64 = 0;

                    for _ in 0..iters {
                        let temp_dir = tempfile::tempdir().unwrap();
                        let segment_dir = temp_dir.path().join("segments");
                        let index_dir = temp_dir.path().join("index");
                        let wal_dir = temp_dir.path().join("wal");

                        std::fs::create_dir_all(&segment_dir).unwrap();
                        std::fs::create_dir_all(&index_dir).unwrap();
                        std::fs::create_dir_all(&wal_dir).unwrap();

                        let config = FileKVConfig {
                            memtable: MemTableConfig {
                                flush_threshold_bytes: 256 * 1024, // 256KB
                                max_entries: 100_000,
                                max_memory_bytes: 64 * 1024 * 1024,
                            },
                            segment_dir: segment_dir.clone(), // Clone to keep ownership
                            enable_wal: false,
                            wal_dir,
                            index_dir,
                            cache: BlockCacheConfig {
                                max_items: 10_000,
                                max_memory_bytes: 64 * 1024 * 1024,
                            },
                            enable_bloom: true,
                            enable_background_flush: false,
                            background_flush_interval_ms: 100,
                            block_size: 8192,
                            block_compression: BlockCompressionConfig::default(),
                            compaction: CompactionConfig {
                                min_segments: 4,
                                auto_compact: true,
                                check_interval: 100,
                                max_segment_size_bytes: 16 * 1024 * 1024,
                                target_segment_size_bytes: 8 * 1024 * 1024,
                                async_compaction_enabled: false,
                                leveled_compaction_enabled: true,
                                level_size_multiplier: 10,
                                max_level: 3,
                                l0_file_count_threshold: 4,
                                parallel_compaction_enabled: true,
                                streaming_compaction_enabled: true,
                            },
                            segment_preallocate_size: 16 * 1024 * 1024,
                            wal_max_size_bytes: 100 * 1024 * 1024,
                            wal_max_files: 5,
                            
                            cache_warming_enabled: false,
                            compression: DictionaryCompressionConfig::default(),
                            async_io_enabled: false,
                            async_io_max_concurrent_writes: 4,
                            async_io_max_queue_depth: 1024,
                            async_io_write_timeout_ms: 5000,
                            async_io_enable_coalescing: false,
                            async_io_coalesce_window_ms: 10,
                            checkpoint_dir: temp_dir.path().join("checkpoints"),
                            audit_log: AuditLogConfig {
                                log_dir: temp_dir.path().join("audit_logs"),
                                enabled: false,
                                rotation_interval_hours: 24,
                                retention_days: 30,
                            },
                            aggressive: AggressiveConfig::performance(),
                            enable_adaptive_bloom_cache: true,
                            enable_zone_map_pruning: true,
                            enable_sequential_prefetch: true,
                            enable_background_cache_rebalance: false,
        fs: std::sync::Arc::new(StdFs),
                        };

                        let kv = FileKV::open(config).unwrap();

                        let start = std::time::Instant::now();

                        // 用户写入
                        for i in 0..n {
                            let key = format!("amp_key_{:010}", i);
                            let value = vec![b'x'; 1024]; // 1KB
                            total_user_bytes += key.len() as u64 + value.len() as u64;
                            kv.put(&key, &value).unwrap();
                        }

                        // 等待 flush 和 compaction 完成
                        kv.flush_memtable().unwrap();
                        std::thread::sleep(Duration::from_secs(1));

                        // 测量磁盘写入量（通过 segment 文件大小）
                        if let Ok(entries) = std::fs::read_dir(&segment_dir) {
                            for entry in entries.flatten() {
                                if let Ok(metadata) = entry.metadata() {
                                    total_disk_bytes += metadata.len();
                                }
                            }
                        }

                        // 测量耗时
                        let _elapsed = start.elapsed();
                    }

                    // 输出写入放大比率
                    if total_user_bytes > 0 {
                        let amplification = total_disk_bytes as f64 / total_user_bytes as f64;
                        eprintln!(
                            "Write Amplification ({} keys): {:.2}x (user: {} MB, disk: {} MB)",
                            n,
                            amplification,
                            total_user_bytes / (1024 * 1024),
                            total_disk_bytes / (1024 * 1024)
                        );
                    }

                    // 返回总磁盘写入量作为 benchmark 指标
                    std::time::Duration::from_millis(total_disk_bytes / (1024 * 1024))
                });
            },
        );
    }

    group.finish();
}
