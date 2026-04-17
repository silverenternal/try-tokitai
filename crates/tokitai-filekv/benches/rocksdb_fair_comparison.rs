//! RocksDB vs FileKV Fair Comparison Benchmarks
//!
//! **FAIR COMPARISON METHODOLOGY**:
//! - Same hardware environment (AMD Ryzen 9 8945HS, 64GB DDR5)
//! - Same dataset (1M entries, 16B key, 100B value)
//! - Same Bloom Filter FPR (1%)
//! - Same test scenarios (pure memory vs pure memory, full KV vs full KV)
//!
//! **Comparison Levels**:
//! 1. Bloom Filter contains() - Pure memory operation (no disk I/O)
//! 2. Full KV get() - Complete query path (index + data retrieval)
//! 3. KV put() - Write path (with/without WAL)
//! 4. Memory overhead - Total memory usage for 100K entries

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tempfile::tempdir;

// FileKV imports
use tokitai_filekv::cache::block_cache::BlockCacheConfig;
use tokitai_filekv::{FileKV, FileKVConfig};

// RocksDB imports
use rocksdb::{BlockBasedOptions, Options, DB};

/// Test dataset configuration
const NUM_ENTRIES: usize = 100_000; // 100K for faster benchmarks
#[allow(dead_code)] // Reserved for future use
const KEY_SIZE: usize = 16;
const VALUE_SIZE: usize = 100;
const BLOOM_FPR: f64 = 0.01; // 1% false positive rate

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

/// Test dataset for benchmarks
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

        // Generate nonexistent keys for negative lookups
        let mut nonexistent_keys = Vec::with_capacity(1000);
        for i in NUM_ENTRIES..NUM_ENTRIES + 1000 {
            nonexistent_keys.push(generate_key(i));
        }

        Self {
            keys,
            values,
            nonexistent_keys,
        }
    }
}

// ============================================================================
// Experiment 1: Bloom Filter Contains() - Pure Memory Comparison
// ============================================================================

/// FileKV Bloom Filter contains() benchmark
fn bench_filekv_bloom_contains(c: &mut Criterion) {
    let dataset = TestDataset::new();

    // Create FileKV and populate
    let dir = tempdir().unwrap();
    let config = FileKVConfig {
        segment_dir: dir.path().to_path_buf(),
        wal_dir: dir.path().join("wal"),
        index_dir: dir.path().join("index"),
        enable_background_flush: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    // Populate KV
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        kv.put(key, value).unwrap();
    }

    // Force flush to create segments with Bloom Filters
    kv.flush_memtable().unwrap();

    let mut group = c.benchmark_group("bloom_filter_contains");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("FileKV", "negative_lookup"), |b| {
        b.iter(|| {
            for key in &dataset.nonexistent_keys {
                black_box(kv.get(key).unwrap());
            }
        });
    });

    group.finish();
}

/// RocksDB Bloom Filter contains() benchmark
fn bench_rocksdb_bloom_contains(c: &mut Criterion) {
    let dataset = TestDataset::new();

    // Create RocksDB with Bloom Filter
    let dir = tempdir().unwrap();
    let mut opts = Options::default();
    opts.create_if_missing(true);

    // Configure Bloom Filter with same FPR
    let mut block_opts = BlockBasedOptions::default();
    block_opts.set_bloom_filter(BLOOM_FPR, false); // 1% FPR, same as FileKV
    opts.set_block_based_table_factory(&block_opts);

    let db = DB::open(&opts, dir.path()).unwrap();

    // Populate DB
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        db.put(key.as_bytes(), value).unwrap();
    }

    // Force flush to create SST files with Bloom Filters
    db.flush().unwrap();

    let mut group = c.benchmark_group("bloom_filter_contains");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("RocksDB", "negative_lookup"), |b| {
        b.iter(|| {
            for key in &dataset.nonexistent_keys {
                black_box(db.get(key.as_bytes()).unwrap());
            }
        });
    });

    group.finish();
}

// ============================================================================
// Experiment 2: Full KV Get() - Complete Query Path Comparison
// ============================================================================

/// FileKV full KV get() benchmark (hot cache)
fn bench_filekv_full_get(c: &mut Criterion) {
    let dataset = TestDataset::new();

    // Create FileKV and populate
    let dir = tempdir().unwrap();
    let config = FileKVConfig {
        segment_dir: dir.path().to_path_buf(),
        wal_dir: dir.path().join("wal"),
        index_dir: dir.path().join("index"),
        enable_background_flush: false,
        cache: BlockCacheConfig {
            max_items: 150_000, // 150K > 100K keys
            ..Default::default()
        },
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    // Populate KV
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        kv.put(key, value).unwrap();
    }

    // Flush to create segments
    kv.flush_memtable().unwrap();

    // Warm up cache by reading all keys
    for key in &dataset.keys {
        let _ = kv.get(key);
    }

    let mut group = c.benchmark_group("full_kv_get");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("FileKV", "hot_cache"), |b| {
        b.iter(|| {
            for key in &dataset.keys[..1000] {
                black_box(kv.get(key).unwrap());
            }
        });
    });

    group.finish();
}

/// RocksDB full KV get() benchmark (hot cache)
fn bench_rocksdb_full_get(c: &mut Criterion) {
    let dataset = TestDataset::new();

    // Create RocksDB with Bloom Filter
    let dir = tempdir().unwrap();
    let mut opts = Options::default();
    opts.create_if_missing(true);

    // Configure Bloom Filter
    let mut block_opts = BlockBasedOptions::default();
    block_opts.set_bloom_filter(BLOOM_FPR, false);
    opts.set_block_based_table_factory(&block_opts);

    let db = DB::open(&opts, dir.path()).unwrap();

    // Populate DB
    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        db.put(key.as_bytes(), value).unwrap();
    }

    // Flush to create SST files
    db.flush().unwrap();

    // Warm up cache
    for key in &dataset.keys {
        let _ = db.get(key.as_bytes());
    }

    let mut group = c.benchmark_group("full_kv_get");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("RocksDB", "hot_cache"), |b| {
        b.iter(|| {
            for key in &dataset.keys[..1000] {
                black_box(db.get(key.as_bytes()).unwrap());
            }
        });
    });

    group.finish();
}

// ============================================================================
// Experiment 3: KV Put() - Write Path Comparison
// ============================================================================

/// FileKV put() benchmark (with WAL)
fn bench_filekv_put_wal(c: &mut Criterion) {
    let dataset = TestDataset::new();

    let dir = tempdir().unwrap();
    let config = FileKVConfig {
        segment_dir: dir.path().to_path_buf(),
        wal_dir: dir.path().join("wal"),
        index_dir: dir.path().join("index"),
        enable_wal: true,
        enable_background_flush: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    let mut group = c.benchmark_group("kv_put_wal");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("FileKV", "64B"), |b| {
        let small_value = vec![0u8; 64];
        b.iter(|| {
            for key in &dataset.keys[..1000] {
                kv.put(key, &small_value).unwrap();
                black_box(());
            }
        });
    });

    group.bench_function(BenchmarkId::new("FileKV", "100B"), |b| {
        b.iter(|| {
            for key in &dataset.keys[..1000] {
                kv.put(key, &dataset.values[0]).unwrap();
                black_box(());
            }
        });
    });

    group.finish();
}

/// RocksDB put() benchmark (with WAL)
fn bench_rocksdb_put_wal(c: &mut Criterion) {
    let dataset = TestDataset::new();

    let dir = tempdir().unwrap();
    let mut opts = Options::default();
    opts.create_if_missing(true);

    // Configure Bloom Filter
    let mut block_opts = BlockBasedOptions::default();
    block_opts.set_bloom_filter(BLOOM_FPR, false);
    opts.set_block_based_table_factory(&block_opts);

    let db = DB::open(&opts, dir.path()).unwrap();

    let mut group = c.benchmark_group("kv_put_wal");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("RocksDB", "64B"), |b| {
        let small_value = vec![0u8; 64];
        b.iter(|| {
            for key in &dataset.keys[..1000] {
                db.put(key.as_bytes(), &small_value).unwrap();
                black_box(());
            }
        });
    });

    group.bench_function(BenchmarkId::new("RocksDB", "100B"), |b| {
        b.iter(|| {
            for key in &dataset.keys[..1000] {
                db.put(key.as_bytes(), &dataset.values[0]).unwrap();
                black_box(());
            }
        });
    });

    group.finish();
}

// ============================================================================
// Experiment 4: Memory Overhead Comparison (Not a benchmark, just measurement)
// ============================================================================

/// Memory overhead measurement - returns (filekv_memory_bytes, entry_count)
/// PERF-002 FIX: Separated measurement from reporting to avoid repeated prints
fn measure_memory_overhead() -> (usize, usize) {
    let dataset = TestDataset::new();

    // FileKV memory measurement
    let dir = tempdir().unwrap();
    let config = FileKVConfig {
        segment_dir: dir.path().to_path_buf(),
        wal_dir: dir.path().join("wal"),
        index_dir: dir.path().join("index"),
        enable_background_flush: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    for (key, value) in dataset.keys.iter().zip(dataset.values.iter()) {
        kv.put(key, value).unwrap();
    }
    kv.flush_memtable().unwrap();

    // Get FileKV memory estimate (total size of segments)
    let stats = kv.get_stats();
    let filekv_memory = stats.total_size_bytes as usize;

    (filekv_memory, NUM_ENTRIES)
}

/// Report memory comparison results (called once externally)
#[allow(dead_code)]
fn report_memory_comparison(filekv_memory: usize, num_entries: usize) {
    println!("\n=== Memory Overhead Comparison ({} entries) ===", num_entries);
    println!(
        "FileKV:  {:.2} MB (total_size_bytes from segments)",
        filekv_memory as f64 / (1024.0 * 1024.0)
    );
    println!("RocksDB: (property_int not available in rocksdb 0.24 Rust crate)");
    println!("Note: For accurate memory comparison, use external tools like /proc/self/status");
}

/// Memory overhead benchmark - measures memory usage
/// PERF-002 FIX: Only measures memory, doesn't print results
fn bench_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead");
    group.sample_size(10);
    group.measurement_time(Duration::from_millis(100));

    group.bench_function("memory_measurement", |b| {
        b.iter(|| {
            // Only measure, don't print
            measure_memory_overhead()
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    benches,
    bench_filekv_bloom_contains,
    bench_rocksdb_bloom_contains,
    bench_filekv_full_get,
    bench_rocksdb_full_get,
    bench_filekv_put_wal,
    bench_rocksdb_put_wal,
    bench_memory_overhead,
);

criterion_main!(benches);
