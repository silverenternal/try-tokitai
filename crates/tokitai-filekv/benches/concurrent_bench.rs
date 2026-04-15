//! Concurrent Multi-threaded Benchmarks
//!
//! This benchmark suite measures FileKV performance under concurrent load:
//! - Multi-threaded mixed read/write workloads
//! - Contended access patterns
//!
//! **修复记录 (2026-04-10)**:
//! - BENCH-004: 修复吞吐量计算错误 (thread_count vs total_ops)
//! - BENCH-005: 修复 pre-population 在 iter_custom 内的问题

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::Rng;

use tokitai_filekv::{FileKV, FileKVConfig, MemTableConfig, DictionaryCompressionConfig, AggressiveConfig};
use tokitai_filekv::cache::block_cache::BlockCacheConfig;
use tokitai_filekv::compaction::CompactionConfig;
use tokitai_filekv::AuditLogConfig;
use tokitai_filekv::core::types::BlockCompressionConfig;
use tokitai_filekv::io::MemFs;

/// Create test FileKV instance for concurrent benchmarks
fn setup_concurrent_kv() -> (tempfile::TempDir, FileKV) {
    let temp_dir = tempfile::tempdir().unwrap();
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");

    std::fs::create_dir_all(&segment_dir).unwrap();
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&wal_dir).unwrap();

    let config = FileKVConfig {
        memtable: MemTableConfig {
            flush_threshold_bytes: 64 * 1024 * 1024,
            max_entries: 1_000_000,
            max_memory_bytes: 256 * 1024 * 1024,
        },
        segment_dir,
        enable_wal: true,
        wal_dir,
        index_dir,
        cache: BlockCacheConfig {
            max_items: 100_000,
            max_memory_bytes: 256 * 1024 * 1024,
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
            max_segment_size_bytes: 256 * 1024 * 1024,
            target_segment_size_bytes: 128 * 1024 * 1024,
            async_compaction_enabled: false,
            leveled_compaction_enabled: true,
            level_size_multiplier: 10,
            max_level: 3,
            l0_file_count_threshold: 4,
            parallel_compaction_enabled: true,
            streaming_compaction_enabled: true,
        },
        segment_preallocate_size: 64 * 1024 * 1024,
        wal_max_size_bytes: 1024 * 1024 * 1024,
        wal_max_files: 10,
        cache_warming_enabled: false,
        compression: DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 8,
        async_io_max_queue_depth: 4096,
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
        fs: Arc::new(MemFs::new()),
    };

    let kv = FileKV::open(config).unwrap();
    (temp_dir, kv)
}

/// Pre-populate FileKV with test data
/// 修复：pre-population 移到 iter_custom 外部
fn populate_kv(kv: &FileKV, count: u64) {
    for i in 0..count {
        let key = format!("key_{:06}", i);
        let value = format!("value_{:06}", i);
        kv.put(&key, value.as_bytes()).unwrap();
    }
}

/// Benchmark: Concurrent mixed read/write workload
/// 修复 BENCH-005: Pre-population 移到 iter_custom 外部
/// 修复 BENCH-004: 吞吐量基于 total_ops 而不是 thread_count
/// 修复：使用固定操作次数而不是固定时间，避免 Criterion 采样问题
fn bench_concurrent_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_mixed_workload");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    for thread_count in [1, 4, 8, 16, 32, 64] {
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter_custom(|iters| {
                    let mut total_duration = Duration::new(0, 0);
                    let mut total_ops = 0u64;

                    for _ in 0..iters {
                        let (_temp_dir, kv) = setup_concurrent_kv();
                        let kv = Arc::new(kv);

                        // Pre-populate with 10K entries (OUTSIDE measurement)
                        populate_kv(&kv, 10_000);

                        let thread_ops = Arc::new(AtomicU64::new(0));
                        let mut handles = vec![];

                        let ops_per_thread = 1000u64;

                        for _ in 0..thread_count {
                            let kv = Arc::clone(&kv);
                            let thread_ops = Arc::clone(&thread_ops);

                            let handle = thread::spawn(move || {
                                let mut rng = rand::thread_rng();
                                let mut local_ops = 0u64;

                                for _ in 0..ops_per_thread {
                                    if rng.gen::<f64>() < 0.8 {
                                        // Read
                                        let key_idx = rng.gen_range(0..10_000);
                                        let key = format!("key_{:06}", key_idx);
                                        if let Ok(Some(_)) = kv.get(&key) {
                                            local_ops += 1;
                                        }
                                    } else {
                                        // Write
                                        let key_idx = rng.gen_range(0..10_000);
                                        let key = format!("key_{:06}", key_idx);
                                        let value = format!("value_{}_{}", key_idx, local_ops);
                                        if kv.put(&key, value.as_bytes()).is_ok() {
                                            local_ops += 1;
                                        }
                                    }
                                }

                                thread_ops.fetch_add(local_ops, Ordering::Relaxed);
                            });

                            handles.push(handle);
                        }

                        let start = Instant::now();
                        for handle in handles {
                            handle.join().unwrap();
                        }
                        let elapsed = start.elapsed();

                        total_duration += elapsed;
                        total_ops += thread_ops.load(Ordering::Relaxed);
                    }

                    // Report ops/sec to stdout for visibility
                    let ops_per_sec = if total_duration.as_secs_f64() > 0.0 {
                        total_ops as f64 / total_duration.as_secs_f64()
                    } else {
                        0.0
                    };
                    println!("  [mixed_workload] Ops/sec: {:.0} (threads={}, total_ops={}, duration={:.3}s)",
                             ops_per_sec, thread_count, total_ops, total_duration.as_secs_f64());

                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Concurrent read scalability
/// 修复 BENCH-005: Pre-population 移到 iter_custom 外部
/// 修复：使用固定操作次数而不是固定时间
fn bench_concurrent_read_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_read_scalability");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    for thread_count in [1, 4, 8, 16, 32, 64] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter_custom(|iters| {
                    let mut total_duration = Duration::new(0, 0);
                    let mut total_reads = 0u64;

                    for _ in 0..iters {
                        let (_temp_dir, kv) = setup_concurrent_kv();
                        let kv = Arc::new(kv);

                        // Pre-populate with 100K entries (OUTSIDE measurement)
                        populate_kv(&kv, 100_000);

                        let thread_reads = Arc::new(AtomicU64::new(0));
                        let mut handles = vec![];

                        let reads_per_thread = 10000u64;

                        for _ in 0..thread_count {
                            let kv = Arc::clone(&kv);
                            let thread_reads = Arc::clone(&thread_reads);

                            let handle = thread::spawn(move || {
                                let mut rng = rand::thread_rng();
                                let mut local_reads = 0u64;

                                for _ in 0..reads_per_thread {
                                    let key_idx = rng.gen_range(0..100_000);
                                    let key = format!("key_{:06}", key_idx);
                                    if let Ok(Some(_)) = kv.get(&key) {
                                        local_reads += 1;
                                    }
                                }

                                thread_reads.fetch_add(local_reads, Ordering::Relaxed);
                            });

                            handles.push(handle);
                        }

                        let start = Instant::now();
                        for handle in handles {
                            handle.join().unwrap();
                        }
                        let elapsed = start.elapsed();

                        total_duration += elapsed;
                        total_reads += thread_reads.load(Ordering::Relaxed);
                    }

                    // Report reads/sec to stdout for visibility
                    let reads_per_sec = if total_duration.as_secs_f64() > 0.0 {
                        total_reads as f64 / total_duration.as_secs_f64()
                    } else {
                        0.0
                    };
                    println!("  [read_scalability] Reads/sec: {:.0} (threads={}, total_reads={}, duration={:.3}s)",
                             reads_per_sec, thread_count, total_reads, total_duration.as_secs_f64());

                    total_duration
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
        .noise_threshold(0.05)
        .significance_level(0.05);
    targets =
        bench_concurrent_mixed_workload,
        bench_concurrent_read_scalability,
);

criterion_main!(benches);
