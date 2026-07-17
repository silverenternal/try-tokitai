//! FileKV 性能基准测试
//!
//! 测试 FileKV 核心性能指标：
//! - 写入延迟（目标：5-7µs）
//! - 读取延迟（目标：2-3µs）
//! - 热点读取延迟（目标：0.5µs，Block Cache 命中）
//! - 批量写入性能
//! - 混合读写性能
//! - Compaction 影响

use std::time::{Duration, Instant};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use tokitai_context::file_kv::{FileKV, FileKVConfig, MemTableConfig, DictionaryCompressionConfig};
use tokitai_context::block_cache::BlockCacheConfig;
use tokitai_context::compaction::{CompactionConfig, CompactionStrategy};
use tokitai_context::audit_log::AuditLogConfig;

/// 创建测试用的 FileKV 实例
fn setup_file_kv() -> (tempfile::TempDir, FileKV) {
    let temp_dir = tempfile::tempdir().unwrap();
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");
    
    std::fs::create_dir_all(&segment_dir).unwrap();
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&wal_dir).unwrap();
    
    let config = FileKVConfig {
        memtable: MemTableConfig {
            flush_threshold_bytes: 4 * 1024 * 1024, // 4MB
            max_entries: 100_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
        },
        segment_dir,
        enable_wal: false, // Disable WAL for benchmarks
        wal_dir,
        index_dir,
        cache: BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            min_block_size: 64,
            max_block_size: 1024 * 1024,
        },
        enable_bloom: true,
        enable_background_flush: false, // Disable background flush for benchmarks
        background_flush_interval_ms: 100,
        compaction: CompactionConfig {
            strategy: CompactionStrategy::SizeTiered,
            min_segments: 4,
            max_segment_size_bytes: 16 * 1024 * 1024,
            target_segment_size_bytes: 8 * 1024 * 1024,
            max_compact_segments: 8,
            auto_compact: false, // Disable auto-compaction for benchmarks
            check_interval: 100,
            num_levels: 7,
            level_size_ratio: 10.0,
            overlap_threshold: 0.5,
        },
        segment_preallocate_size: 16 * 1024 * 1024, // 16MB
        // P1-013: WAL rotation configuration
        wal_max_size_bytes: 100 * 1024 * 1024,
        wal_max_files: 5,
        // P2-012: Write coalescing configuration
        write_coalescing_enabled: false, // Disable for accurate single-write measurement
        // P2-004: Cache warming configuration
        cache_warming_enabled: false,
        // P2-014: Dictionary compression configuration
        compression: DictionaryCompressionConfig::default(),
        // P3-001: Async I/O configuration
        async_io_enabled: false,
        async_io_max_concurrent_writes: 4,
        async_io_max_queue_depth: 1024,
        async_io_write_timeout_ms: 5000,
        async_io_enable_coalescing: false,
        async_io_coalesce_window_ms: 10,
        // P2-009: Checkpoint directory
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        // P2-013: Audit log configuration
        audit_log: AuditLogConfig {
            log_dir: temp_dir.path().join("audit_logs"),
            enabled: false,
            max_file_size_bytes: 100 * 1024 * 1024,
            max_files: 10,
            record_latency: false,
            include_value_hash: false,
            flush_on_write: false,
        },
    };

    let kv = FileKV::open(config).unwrap();
    (temp_dir, kv)
}

/// 基准测试：单次写入性能
fn bench_single_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("Single Write");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));

    // P1-001: Benchmark with FileKV reuse to measure actual write overhead
    group.bench_function("Write 64B key-value (reuse instance)", |b| {
        let (_temp_dir, kv) = setup_file_kv();
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(key, value)).unwrap();
        });
    });

    // Original benchmark (includes initialization overhead)
    group.bench_function("Write 64B key-value (with init)", |b| {
        b.iter(|| {
            let (_temp_dir, kv) = setup_file_kv();
            let key = "test_key_000000000000000000000000000000";
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(key, value)).unwrap();
        });
    });

    group.bench_function("Write 1KB key-value (reuse instance)", |b| {
        let (_temp_dir, kv) = setup_file_kv();
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            let value = vec![b'x'; 1024];
            black_box(kv.put(key, &value)).unwrap();
        });
    });

    group.bench_function("Write 4KB key-value (reuse instance)", |b| {
        let (_temp_dir, kv) = setup_file_kv();
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            let value = vec![b'x'; 4096];
            black_box(kv.put(key, &value)).unwrap();
        });
    });

    group.finish();
}

/// 基准测试：批量写入性能
fn bench_batch_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("Batch Write");
    group.measurement_time(Duration::from_secs(15));
    group.warm_up_time(Duration::from_secs(3));

    for &count in &[10, 50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &n| {
                b.iter(|| {
                    let (_temp_dir, kv) = setup_file_kv();
                    
                    let start = Instant::now();
                    for i in 0..n {
                        let key = format!("key_{:08}", i);
                        let value = format!("value_{:08}_{}", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        kv.put(key.as_str(), value.as_bytes()).unwrap();
                    }
                    let elapsed = start.elapsed();
                    
                    black_box(elapsed);
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：单次读取性能（热数据，Block Cache 命中）
fn bench_single_read_hot(c: &mut Criterion) {
    let mut group = c.benchmark_group("Single Read (Hot)");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));

    group.bench_function("Read 64B value (hot, cache hit)", |b| {
        b.iter(|| {
            let (_temp_dir, kv) = setup_file_kv();
            let key = "test_key_000000000000000000000000000000";
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            kv.put(key, value).unwrap();
            
            // 先读取一次，确保数据在 cache 中
            let _ = kv.get(key);
            
            // 测试 cache hit 的读取性能
            let start = Instant::now();
            for _ in 0..100 {
                black_box(kv.get(key)).unwrap();
            }
            start.elapsed() / 100
        });
    });

    group.bench_function("Read 1KB value (hot, cache hit)", |b| {
        b.iter(|| {
            let (_temp_dir, kv) = setup_file_kv();
            let key = "test_key_000000000000000000000000000000";
            let value = vec![b'x'; 1024];
            kv.put(key, &value).unwrap();
            
            let _ = kv.get(key);
            
            let start = Instant::now();
            for _ in 0..100 {
                black_box(kv.get(key)).unwrap();
            }
            start.elapsed() / 100
        });
    });

    group.finish();
}

/// 基准测试：Bloom Filter 负向查询（key 不存在）
fn bench_bloom_filter_negative(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bloom Filter (Negative)");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));

    group.bench_function("Get non-existent key", |b| {
        b.iter(|| {
            let (_temp_dir, kv) = setup_file_kv();
            
            // 先写入一些数据
            for i in 0..100 {
                let key = format!("existing_key_{:08}", i);
                let value = format!("value_{}", i);
                kv.put(key.as_str(), value.as_bytes()).unwrap();
            }
            
            // 查询不存在的 key
            let start = Instant::now();
            for _ in 0..100 {
                let non_existent_key = "non_existent_key_xxxxxxxxxxxxxxxxxxxxxxxx";
                black_box(kv.get(non_existent_key)).unwrap();
            }
            start.elapsed() / 100
        });
    });

    group.finish();
}

/// 基准测试：混合读写性能
fn bench_mixed_read_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mixed Read/Write");
    group.measurement_time(Duration::from_secs(15));
    group.warm_up_time(Duration::from_secs(3));

    for &write_ratio in &[10, 30, 50, 70, 90] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}% write", write_ratio)),
            &write_ratio,
            |b, &ratio| {
                b.iter(|| {
                    let (_temp_dir, kv) = setup_file_kv();
                    
                    // 预写入一些数据
                    for i in 0..100 {
                        let key = format!("key_{:08}", i);
                        let value = format!("value_{}", i);
                        kv.put(key.as_str(), value.as_bytes()).unwrap();
                    }
                    
                    let start = Instant::now();
                    for i in 0..1000 {
                        if i % 100 < ratio {
                            // Write
                            let key = format!("key_{:08}", i);
                            let value = format!("new_value_{}", i);
                            kv.put(key.as_str(), value.as_bytes()).unwrap();
                        } else {
                            // Read
                            let key = format!("key_{:08}", i % 100);
                            black_box(kv.get(key.as_str())).unwrap();
                        }
                    }
                    let elapsed = start.elapsed();
                    
                    black_box(elapsed);
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：删除操作性能
fn bench_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("Delete Operation");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));

    group.bench_function("Delete key", |b| {
        b.iter(|| {
            let (_temp_dir, kv) = setup_file_kv();
            
            // 先写入
            let key = "test_key_to_delete";
            let value = "test_value";
            kv.put(key, value.as_bytes()).unwrap();
            
            // 删除
            let start = Instant::now();
            black_box(kv.delete(key)).unwrap();
            start.elapsed()
        });
    });

    group.finish();
}

/// 基准测试：MemTable 读取（最快路径）
fn bench_memtable_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("MemTable Read");
    group.measurement_time(Duration::from_secs(10));
    group.warm_up_time(Duration::from_secs(2));

    group.bench_function("Read from MemTable (unflushed)", |b| {
        b.iter(|| {
            let (_temp_dir, kv) = setup_file_kv();
            let key = "test_key_memtable";
            let value = "test_value_memtable_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
            kv.put(key, value.as_bytes()).unwrap();
            
            // 直接读取（不刷盘，数据在 MemTable 中）
            let start = Instant::now();
            for _ in 0..100 {
                black_box(kv.get(key)).unwrap();
            }
            start.elapsed() / 100
        });
    });

    group.finish();
}

/// 基准测试：批量加载后查询
fn bench_query_after_bulk_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("Query After Bulk Load");
    group.measurement_time(Duration::from_secs(20));
    group.warm_up_time(Duration::from_secs(5));

    for &entry_count in &[100, 500, 1000, 5000, 10000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |b, &n| {
                b.iter(|| {
                    let (_temp_dir, kv) = setup_file_kv();
                    
                    // 批量写入
                    let load_start = Instant::now();
                    for i in 0..n {
                        let key = format!("key_{:010}", i);
                        let value = format!("value_{:010}_{}", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        kv.put(key.as_str(), value.as_bytes()).unwrap();
                    }
                    let load_time = load_start.elapsed();
                    
                    // 随机读取测试
                    let mut total_read_time = Duration::ZERO;
                    for _iter in 0..10 {
                        let iter_start = Instant::now();
                        for i in (0..n).step_by(n / 100 + 1) {
                            let key = format!("key_{:010}", i);
                            black_box(kv.get(key.as_str())).unwrap();
                        }
                        total_read_time += iter_start.elapsed();
                    }
                    let avg_read_time = total_read_time / 10;
                    
                    black_box((load_time, avg_read_time));
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：写入后刷盘再读取（完整路径）
fn bench_write_flush_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("Write-Flush-Read");
    group.measurement_time(Duration::from_secs(15));
    group.warm_up_time(Duration::from_secs(3));

    group.bench_function("Write + Flush + Read (64B)", |b| {
        b.iter(|| {
            let (temp_dir, kv) = setup_file_kv();
            let key = "test_key_000000000000000000000000000000";
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            
            kv.put(key, value).unwrap();
            
            // 强制刷盘
            drop(kv);
            
            // 重新打开，确保从 segment 读取
            let segment_dir = temp_dir.path().join("segments");
            let index_dir = temp_dir.path().join("index");
            let wal_dir = temp_dir.path().join("wal");
            
            let config = FileKVConfig {
                memtable: MemTableConfig {
                    flush_threshold_bytes: 4 * 1024 * 1024,
                    max_entries: 100_000,
                    max_memory_bytes: 64 * 1024 * 1024,
                },
                segment_dir,
                enable_wal: false,
                wal_dir: wal_dir.clone(),
                index_dir,
                cache: BlockCacheConfig {
                    max_items: 10_000,
                    max_memory_bytes: 64 * 1024 * 1024,
                    min_block_size: 64,
                    max_block_size: 1024 * 1024,
                },
                enable_bloom: true,
                enable_background_flush: false,
                background_flush_interval_ms: 100,
                compaction: CompactionConfig {
                    strategy: CompactionStrategy::SizeTiered,
                    min_segments: 4,
                    max_segment_size_bytes: 16 * 1024 * 1024,
                    target_segment_size_bytes: 8 * 1024 * 1024,
                    max_compact_segments: 8,
                    auto_compact: false,
                    check_interval: 100,
                    num_levels: 7,
                    level_size_ratio: 10.0,
                    overlap_threshold: 0.5,
                },
                segment_preallocate_size: 16 * 1024 * 1024,
                wal_max_size_bytes: 100 * 1024 * 1024,
                wal_max_files: 5,
                write_coalescing_enabled: false,
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
                    log_dir: wal_dir.join("audit_logs"),
                    enabled: false,
                    max_file_size_bytes: 100 * 1024 * 1024,
                    max_files: 10,
                    record_latency: false,
                    include_value_hash: false,
                    flush_on_write: false,
                },
            };

            let kv = FileKV::open(config).unwrap();
            black_box(kv.get(key)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_write,
    bench_batch_write,
    bench_single_read_hot,
    bench_bloom_filter_negative,
    bench_mixed_read_write,
    bench_delete,
    bench_memtable_read,
    bench_query_after_bulk_load,
    bench_write_flush_read,
);

criterion_main!(benches);
