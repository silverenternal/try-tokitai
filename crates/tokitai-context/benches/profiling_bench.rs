//! Profiling Benchmark Suite
//!
//! This benchmark suite is designed to identify performance hotspots
//! by running targeted tests with detailed timing breakdowns.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tokitai_context::file_kv::{FileKV, FileKVConfig, MemTableConfig, DictionaryCompressionConfig};
use tokitai_context::block_cache::BlockCacheConfig;
use tokitai_context::compaction::{CompactionConfig, CompactionStrategy};
use tokitai_context::audit_log::AuditLogConfig;

/// Create a minimal FileKV instance for profiling
fn setup_minimal_kv() -> (tempfile::TempDir, FileKV) {
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
        enable_wal: false,
        wal_dir,
        index_dir,
        cache: BlockCacheConfig {
            max_items: 100_000,
            max_memory_bytes: 128 * 1024 * 1024,
            min_block_size: 64,
            max_block_size: 1024 * 1024,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        compaction: CompactionConfig {
            strategy: CompactionStrategy::SizeTiered,
            min_segments: 20,
            max_segment_size_bytes: 64 * 1024 * 1024,
            target_segment_size_bytes: 32 * 1024 * 1024,
            max_compact_segments: 16,
            auto_compact: false,
            check_interval: 10000,
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

/// Benchmark: MemTable insert only (no WAL, no flush)
fn bench_memtable_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("MemTable Insert");
    
    group.bench_function("Insert 64B (no WAL)", |b| {
        let (_temp_dir, kv) = setup_minimal_kv();
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(key, value)).unwrap();
        });
    });

    group.bench_function("Insert 1KB (no WAL)", |b| {
        let (_temp_dir, kv) = setup_minimal_kv();
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            let value = vec![b'x'; 1024];
            black_box(kv.put(key, &value)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: String allocation overhead
fn bench_string_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("String Allocation");

    group.bench_function("String format!()", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(format!("key_{:08}", i));
            }
        });
    });

    group.bench_function("String to_string()", |b| {
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            for _ in 0..100 {
                black_box(key.to_string());
            }
        });
    });

    group.finish();
}

/// Benchmark: Hash computation
fn bench_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Hash Computation");

    group.bench_function("xxh3 64B", |b| {
        use xxhash_rust::xxh3::Xxh3;
        use std::hash::Hasher;

        b.iter(|| {
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            let mut hasher = Xxh3::default();
            hasher.write(value);
            black_box(hasher.finish());
        });
    });

    group.bench_function("xxh3 1KB", |b| {
        use xxhash_rust::xxh3::Xxh3;
        use std::hash::Hasher;

        let value = vec![b'x'; 1024];
        b.iter(|| {
            let mut hasher = Xxh3::default();
            hasher.write(&value);
            black_box(hasher.finish());
        });
    });

    group.finish();
}

/// Benchmark: Mutex contention simulation
fn bench_mutex_contention(c: &mut Criterion) {
    use std::sync::{Arc, Mutex};

    let mut group = c.benchmark_group("Mutex Contention");

    group.bench_function("Mutex lock/unlock (single thread)", |b| {
        let data = Arc::new(Mutex::new(vec![0u8; 64]));
        b.iter(|| {
            let mut guard = data.lock().unwrap();
            guard[0] += 1;
        });
    });

    group.bench_function("Mutex lock/unlock (1KB data)", |b| {
        let data = Arc::new(Mutex::new(vec![0u8; 1024]));
        b.iter(|| {
            let mut guard = data.lock().unwrap();
            guard[0] += 1;
        });
    });

    group.finish();
}

/// Benchmark: Arc clone overhead
fn bench_arc_clone(c: &mut Criterion) {
    use std::sync::Arc;

    let mut group = c.benchmark_group("Arc Clone");

    group.bench_function("Arc clone 64B", |b| {
        let data: Arc<[u8]> = Arc::new([0u8; 64]);
        b.iter(|| {
            black_box(data.clone());
        });
    });

    group.bench_function("Arc clone 1KB", |b| {
        let data: Arc<[u8]> = Arc::new([0u8; 1024]);
        b.iter(|| {
            black_box(data.clone());
        });
    });

    group.finish();
}

/// Benchmark: Bytes allocation
fn bench_bytes_allocation(c: &mut Criterion) {
    use bytes::Bytes;

    let mut group = c.benchmark_group("Bytes Allocation");

    group.bench_function("Bytes::copy_from_slice 64B", |b| {
        let data = b"test_value_000000000000000000000000000000000000000000000000000000000000";
        b.iter(|| {
            black_box(Bytes::copy_from_slice(data));
        });
    });

    group.bench_function("Bytes::copy_from_slice 1KB", |b| {
        let data = vec![b'x'; 1024];
        b.iter(|| {
            black_box(Bytes::copy_from_slice(&data));
        });
    });

    group.finish();
}

/// Benchmark: DashMap operations
fn bench_dashmap_operations(c: &mut Criterion) {
    use dashmap::DashMap;

    let mut group = c.benchmark_group("DashMap Operations");

    group.bench_function("DashMap insert", |b| {
        let map: DashMap<String, Vec<u8>> = DashMap::new();
        b.iter(|| {
            let key = format!("key_{}", 0);
            let value = vec![b'x'; 64];
            black_box(map.insert(key, value));
        });
    });

    group.bench_function("DashMap get", |b| {
        let map: DashMap<String, Vec<u8>> = DashMap::new();
        map.insert("test_key".to_string(), vec![b'x'; 64]);
        b.iter(|| {
            black_box(map.get("test_key"));
        });
    });

    group.finish();
}

/// Benchmark: Full write path breakdown
fn bench_write_path_breakdown(c: &mut Criterion) {
    let mut group = c.benchmark_group("Write Path Breakdown");

    group.bench_function("Full put() 64B", |b| {
        let (_temp_dir, kv) = setup_minimal_kv();
        b.iter(|| {
            let key = "test_key_000000000000000000000000000000";
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(key, value)).unwrap();
        });
    });

    // Benchmark with pre-allocated strings
    group.bench_function("Full put() with pre-alloc key", |b| {
        let (_temp_dir, kv) = setup_minimal_kv();
        let key = "test_key_000000000000000000000000000000".to_string();
        b.iter(|| {
            let value = b"test_value_000000000000000000000000000000000000000000000000000000000000";
            black_box(kv.put(&key, value)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    name = profiling_benches;
    config = Criterion::default().sample_size(50);
    targets = 
        bench_memtable_insert,
        bench_string_allocation,
        bench_hash_computation,
        bench_mutex_contention,
        bench_arc_clone,
        bench_bytes_allocation,
        bench_dashmap_operations,
        bench_write_path_breakdown,
);

criterion_main!(profiling_benches);
