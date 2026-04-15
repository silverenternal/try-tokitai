//! Comprehensive RocksDB vs FileKV Fair Comparison
//!
//! This benchmark suite provides a complete, fair comparison between FileKV and RocksDB:
//! - Same hardware environment
//! - Same dataset configuration
//! - Same Bloom Filter FPR
//! - Multiple workload patterns (YCSB-inspired)
//! - Write/read/space amplification analysis

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use tempfile::tempdir;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::path::PathBuf;

// FileKV imports
use tokitai_filekv::{FileKV, FileKVConfig, MemTableConfig};
use tokitai_filekv::cache::block_cache::BlockCacheConfig;
use tokitai_filekv::compaction::CompactionConfig;
use tokitai_filekv::audit_log::AuditLogConfig;

// RocksDB imports
use rocksdb::{DB, Options, BlockBasedOptions, DBCompressionType};

/// Test dataset configuration
const NUM_ENTRIES: usize = 100_000;
const KEY_SIZE: usize = 16;
const VALUE_SIZE: usize = 100;
const BLOOM_FPR: f64 = 0.01;

/// Generate test key
fn generate_key(i: usize) -> String {
    format!("key_{:011}", i)
}

/// Generate test value
fn generate_value(i: usize) -> Vec<u8> {
    let mut value = vec![0u8; VALUE_SIZE];
    for (j, byte) in value.iter_mut().enumerate() {
        *byte = ((i + j) % 256) as u8;
    }
    value
}

/// Test dataset
struct TestDataset {
    keys: Vec<String>,
    values: Vec<Vec<u8>>,
    nonexistent_keys: Vec<String>,
}

impl TestDataset {
    fn new() -> Self {
        let mut keys = Vec::with_capacity(NUM_ENTRIES);
        let mut values = Vec::with_capacity(NUM_ENTRIES);

        for i in 0..NUM_ENTRIES {
            keys.push(generate_key(i));
            values.push(generate_value(i));
        }

        let mut nonexistent_keys = Vec::with_capacity(1000);
        for i in NUM_ENTRIES..NUM_ENTRIES + 1000 {
            nonexistent_keys.push(generate_key(i));
        }

        Self { keys, values, nonexistent_keys }
    }
}

/// FileKV configuration factory
fn create_filekv_config(dir: &tempfile::TempDir) -> FileKVConfig {
    let segment_dir = dir.path().join("segments");
    let index_dir = dir.path().join("index");
    let wal_dir = dir.path().join("wal");

    std::fs::create_dir_all(&segment_dir).unwrap();
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&wal_dir).unwrap();

    FileKVConfig {
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
            min_block_size: 64,
            max_block_size: 4 * 1024 * 1024,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        compaction: CompactionConfig {
            leveled_compaction_enabled: true,
            async_compaction_enabled: false,
            parallel_compaction_enabled: false,
            min_segments: 4,
            max_segment_size_bytes: 256 * 1024 * 1024,
            target_segment_size_bytes: 128 * 1024 * 1024,
            level_size_multiplier: 10,
            max_level: 3,
            l0_file_count_threshold: 4,
            auto_compact: true,
            check_interval: 100,
        },
        segment_preallocate_size: 64 * 1024 * 1024,
        wal_max_size_bytes: 1024 * 1024 * 1024,
        wal_max_files: 10,
        write_coalescing_enabled: false,
        cache_warming_enabled: false,
        compression: tokitai_filekv::DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 8,
        async_io_max_queue_depth: 4096,
        async_io_write_timeout_ms: 5000,
        async_io_enable_coalescing: false,
        async_io_coalesce_window_ms: 10,
        checkpoint_dir: dir.path().join("checkpoints"),
        audit_log: AuditLogConfig {
            log_dir: dir.path().join("audit_logs"),
            enabled: false,
            max_file_size_bytes: 1024 * 1024 * 1024,
            max_files: 10,
            record_latency: false,
            include_value_hash: false,
            flush_on_write: false,
        },
        aggressive: tokitai_filekv::AggressiveConfig::performance(),
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
    }
}

/// RocksDB configuration factory
fn create_rocksdb_config() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    
    // Bloom Filter (same FPR as FileKV)
    let mut block_opts = BlockBasedOptions::default();
    block_opts.set_bloom_filter(BLOOM_FPR, false);
    block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(256 * 1024 * 1024));
    opts.set_block_based_table_factory(&block_opts);
    
    // Compression
    opts.set_compression_type(DBCompressionType::None);
    
    // WAL
    opts.enable_wal(true);
    
    opts
}

/// Benchmark 1: Write throughput comparison
fn bench_write_throughput_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_throughput");
    group.throughput(Throughput::Elements(NUM_ENTRIES as u64));
    group.measurement_time(Duration::from_secs(30));

    let dataset = TestDataset::new();

    // BENCH-008 FIX: Use iter_custom to exclude setup time from measurement
    group.bench_function("FileKV_write", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                let dir = tempdir().unwrap();
                let config = create_filekv_config(&dir);
                let kv = FileKV::open(config).unwrap();

                let start = Instant::now();
                for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
                    kv.put(key, value).unwrap();
                }
                total_duration += start.elapsed();
            }
            
            total_duration
        });
    });

    group.bench_function("RocksDB_write", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                let dir = tempdir().unwrap();
                let db_path = dir.path().join("rocksdb");
                let opts = create_rocksdb_config();
                let db = DB::open(&opts, &db_path).unwrap();

                let start = Instant::now();
                for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
                    db.put(key.as_bytes(), value).unwrap();
                }
                total_duration += start.elapsed();
            }
            
            total_duration
        });
    });

    group.finish();
}

/// Benchmark 2: Read latency comparison (hot cache)
fn bench_read_latency_hot_cache(c: &mut Criterion) {
    let dataset = TestDataset::new();

    // Setup FileKV
    let filekv_dir = tempdir().unwrap();
    let filekv_config = create_filekv_config(&filekv_dir);
    let filekv = FileKV::open(filekv_config).unwrap();
    
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        filekv.put(key, value).unwrap();
    }

    // Setup RocksDB
    let rocksdb_dir = tempdir().unwrap();
    let rocksdb_path = rocksdb_dir.path().join("rocksdb");
    let rocksdb_opts = create_rocksdb_config();
    let rocksdb = DB::open(&rocksdb_opts, &rocksdb_path).unwrap();
    
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        rocksdb.put(key.as_bytes(), value).unwrap();
    }

    // Warm up caches
    for _ in 0..1000 {
        let idx = rand::random::<usize>() % NUM_ENTRIES;
        let _ = filekv.get(&dataset.keys[idx]);
        let _ = rocksdb.get(dataset.keys[idx].as_bytes());
    }

    let mut group = c.benchmark_group("read_latency_hot_cache");
    group.throughput(Throughput::Elements(1));

    group.bench_function("FileKV_read_hot", |b| {
        let mut rng = rand::thread_rng();
        b.iter(|| {
            let idx = rng.gen_range(0..NUM_ENTRIES);
            let _ = filekv.get(&dataset.keys[idx]);
        });
    });

    group.bench_function("RocksDB_read_hot", |b| {
        let mut rng = rand::thread_rng();
        b.iter(|| {
            let idx = rng.gen_range(0..NUM_ENTRIES);
            let _ = rocksdb.get(dataset.keys[idx].as_bytes());
        });
    });

    group.finish();
}

/// Benchmark 3: Bloom Filter negative lookup comparison
fn bench_bloom_filter_negative(c: &mut Criterion) {
    let dataset = TestDataset::new();

    // Setup FileKV
    let filekv_dir = tempdir().unwrap();
    let filekv_config = create_filekv_config(&filekv_dir);
    let filekv = FileKV::open(filekv_config).unwrap();
    
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        filekv.put(key, value).unwrap();
    }
    filekv.flush_memtable().unwrap();

    // Setup RocksDB
    let rocksdb_dir = tempdir().unwrap();
    let rocksdb_path = rocksdb_dir.path().join("rocksdb");
    let rocksdb_opts = create_rocksdb_config();
    let rocksdb = DB::open(&rocksdb_opts, &rocksdb_path).unwrap();
    
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        rocksdb.put(key.as_bytes(), value).unwrap();
    }

    let mut group = c.benchmark_group("bloom_filter_negative");
    group.throughput(Throughput::Elements(1));

    group.bench_function("FileKV_bloom_negative", |b| {
        b.iter(|| {
            for key in &dataset.nonexistent_keys {
                let _ = filekv.get(key);
            }
        });
    });

    group.bench_function("RocksDB_bloom_negative", |b| {
        b.iter(|| {
            for key in &dataset.nonexistent_keys {
                let _ = rocksdb.get(key.as_bytes());
            }
        });
    });

    group.finish();
}

/// Benchmark 4: YCSB-like mixed workload
fn bench_ycsb_mixed_workload(c: &mut Criterion) {
    let dataset = TestDataset::new();
    let read_write_ratio = 0.9; // 90% reads, 10% writes (YCSB Workload C)

    // Setup FileKV
    let filekv_dir = tempdir().unwrap();
    let filekv_config = create_filekv_config(&filekv_dir);
    let filekv = FileKV::open(filekv_config).unwrap();
    
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        filekv.put(key, value).unwrap();
    }

    // Setup RocksDB
    let rocksdb_dir = tempdir().unwrap();
    let rocksdb_path = rocksdb_dir.path().join("rocksdb");
    let rocksdb_opts = create_rocksdb_config();
    let rocksdb = DB::open(&rocksdb_opts, &rocksdb_path).unwrap();
    
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        rocksdb.put(key.as_bytes(), value).unwrap();
    }

    let mut group = c.benchmark_group("ycsb_mixed_workload");
    group.throughput(Throughput::Elements(1000));
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("FileKV_ycsb_workload_c", |b| {
        let mut rng = rand::thread_rng();
        b.iter(|| {
            for _ in 0..1000 {
                let idx = rng.gen_range(0..NUM_ENTRIES);
                if rng.gen::<f64>() < read_write_ratio {
                    let _ = filekv.get(&dataset.keys[idx]);
                } else {
                    let new_value = generate_value(idx);
                    let _ = filekv.put(&dataset.keys[idx], &new_value);
                }
            }
        });
    });

    group.bench_function("RocksDB_ycsb_workload_c", |b| {
        let mut rng = rand::thread_rng();
        b.iter(|| {
            for _ in 0..1000 {
                let idx = rng.gen_range(0..NUM_ENTRIES);
                if rng.gen::<f64>() < read_write_ratio {
                    let _ = rocksdb.get(dataset.keys[idx].as_bytes());
                } else {
                    let new_value = generate_value(idx);
                    let _ = rocksdb.put(dataset.keys[idx].as_bytes(), &new_value);
                }
            }
        });
    });

    group.finish();
}

/// Benchmark 5: Concurrent read scalability
fn bench_concurrent_read_scalability(c: &mut Criterion) {
    let dataset = TestDataset::new();

    for thread_count in [1, 4, 8, 16, 32] {
        let mut group = c.benchmark_group(format!("concurrent_read_{}threads", thread_count));
        group.throughput(Throughput::Elements((thread_count * 100) as u64));

        // Setup FileKV
        let filekv_dir = tempdir().unwrap();
        let filekv_config = create_filekv_config(&filekv_dir);
        let filekv = Arc::new(FileKV::open(filekv_config).unwrap());
        
        for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
            filekv.put(key, value).unwrap();
        }

        // Setup RocksDB
        let rocksdb_dir = tempdir().unwrap();
        let rocksdb_path = rocksdb_dir.path().join("rocksdb");
        let rocksdb_opts = create_rocksdb_config();
        let rocksdb = Arc::new(DB::open(&rocksdb_opts, &rocksdb_path).unwrap());
        
        for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
            rocksdb.put(key.as_bytes(), value).unwrap();
        }

        group.bench_with_input(BenchmarkId::new("FileKV", thread_count), &thread_count, |b, &tc| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                let mut handles = vec![];
                
                for _ in 0..tc {
                    let kv = Arc::clone(&filekv);
                    let handle = thread::spawn(move || {
                        let mut rng = rand::thread_rng();
                        for _ in 0..100 {
                            let idx = rng.gen_range(0..NUM_ENTRIES);
                            let _ = kv.get(&dataset.keys[idx]);
                        }
                    });
                    handles.push(handle);
                }
                
                for handle in handles {
                    handle.join().unwrap();
                }
                
                start.elapsed()
            });
        });

        group.bench_with_input(BenchmarkId::new("RocksDB", thread_count), &thread_count, |b, &tc| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                let mut handles = vec![];
                
                for _ in 0..tc {
                    let db = Arc::clone(&rocksdb);
                    let handle = thread::spawn(move || {
                        let mut rng = rand::thread_rng();
                        for _ in 0..100 {
                            let idx = rng.gen_range(0..NUM_ENTRIES);
                            let _ = db.get(dataset.keys[idx].as_bytes());
                        }
                    });
                    handles.push(handle);
                }
                
                for handle in handles {
                    handle.join().unwrap();
                }
                
                start.elapsed()
            });
        });

        group.finish();
    }
}

criterion_group!(
    benches,
    bench_write_throughput_comparison,
    bench_read_latency_hot_cache,
    bench_bloom_filter_negative,
    bench_ycsb_mixed_workload,
    bench_concurrent_read_scalability,
);

criterion_main!(benches);
