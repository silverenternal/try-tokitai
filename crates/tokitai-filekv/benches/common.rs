//! Common utilities and setup for FileKV benchmarks
//!
//! This module provides shared helper functions, configurations, and fixtures
//! to reduce code duplication across all benchmark files.

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use criterion::Criterion;
use tokitai_filekv::{
    AggressiveConfig, AuditLogConfig, FileKV, FileKVConfig, MemTableConfig,
};
use tokitai_filekv::cache::block_cache::BlockCacheConfig;
use tokitai_filekv::compaction::CompactionConfig;
use tokitai_filekv::core::types::BlockCompressionConfig;
use tokitai_filekv::io::StdFs;
use tokitai_filekv::DictionaryCompressionConfig;

/// Default benchmark timeout to prevent hangs
pub const DEFAULT_BENCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Quick benchmark configuration (fast setup, minimal overhead)
pub fn quick_bench_config(temp_dir: &TempDir) -> FileKVConfig {
    FileKVConfig {
        memtable: MemTableConfig {
            flush_threshold_bytes: 64 * 1024, // 64KB for quick flushes
            max_entries: 100_000,
            max_memory_bytes: 64 * 1024 * 1024,
            shards: num_cpus::get() * 2, // POL-007: DashMap 分片数量
        },
        segment_dir: temp_dir.path().join("segments"),
        enable_wal: false,
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        cache: BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024,
            frequency_aware: false,
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
        fs: Arc::new(StdFs),
    }
}

/// WAL-enabled benchmark configuration
pub fn wal_bench_config(temp_dir: &TempDir) -> FileKVConfig {
    let mut config = quick_bench_config(temp_dir);
    config.enable_wal = true;
    config
}

/// Create a fresh FileKV instance for benchmarks
/// Setup should be called OUTSIDE of b.iter() for accurate measurements
pub fn setup_kv(config: FileKVConfig) -> (TempDir, FileKV) {
    let temp_dir = TempDir::new().unwrap();

    // Create directories
    std::fs::create_dir_all(&config.segment_dir).unwrap();
    std::fs::create_dir_all(&config.wal_dir).unwrap();
    std::fs::create_dir_all(&config.index_dir).unwrap();

    // Re-create config with actual temp_dir paths
    let config = if !config.enable_wal {
        quick_bench_config(&temp_dir)
    } else {
        wal_bench_config(&temp_dir)
    };

    let kv = FileKV::open(config).unwrap();
    (temp_dir, kv)
}

/// Pre-populate FileKV with test data
pub fn populate_kv(kv: &FileKV, count: usize) {
    for i in 0..count {
        let key = format!("key_{:08}", i);
        let value = format!("value_{:08}_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", i);
        kv.put(&key, value.as_bytes()).unwrap();
    }
}

/// Flush memtable and ensure data is on disk
pub fn flush_kv(kv: &FileKV) {
    kv.flush_memtable().unwrap();
}

/// Warm up the block cache by reading all keys
pub fn warm_cache(kv: &FileKV, count: usize) {
    for i in 0..count {
        let key = format!("key_{:08}", i);
        let _ = kv.get(&key);
    }
}

/// Criterion default fast configuration for benchmarks
/// Uses small sample sizes and short warm-up to keep total runtime under 5 minutes
pub fn fast_criterion_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .sample_size(10)
        .noise_threshold(0.02)
        .significance_level(0.05)
}

/// Generate a key for benchmarking
#[inline]
pub fn bench_key(index: usize) -> String {
    format!("key_{:08}", index)
}

/// Generate a value for benchmarking
#[inline]
pub fn bench_value(size: usize) -> Vec<u8> {
    vec![b'x'; size]
}

/// Small value (64 bytes)
pub fn small_value() -> Vec<u8> {
    bench_value(64)
}

/// Medium value (1KB)
pub fn medium_value() -> Vec<u8> {
    bench_value(1024)
}

/// Large value (4KB)
pub fn large_value() -> Vec<u8> {
    bench_value(4096)
}
