//! FileKV Performance Example
//!
//! This example demonstrates performance-oriented features:
//! - Write coalescing for batching rapid writes
//! - Bloom filter for fast negative lookups
//! - Block cache for hot data
//! - Zone map for segment pruning
//! - Amplification statistics monitoring

use std::fs;
use std::time::Instant;
use tempfile::tempdir;

fn main() -> anyhow::Result<()> {
    println!("=== FileKV Performance Example ===\n");

    // Setup temporary directory
    let temp_dir = tempdir()?;
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");

    fs::create_dir_all(&segment_dir)?;
    fs::create_dir_all(&index_dir)?;
    fs::create_dir_all(&wal_dir)?;

    // Configure with performance features enabled
    let config = tokitai_filekv::FileKVConfig {
        memtable: tokitai_filekv::MemTableConfig {
            flush_threshold_bytes: 32 * 1024 * 1024,
            max_entries: 500_000,
            max_memory_bytes: 128 * 1024 * 1024,
            shards: 32,
            enable_async_flush: false,
            max_immutable_memtables: 1,
            immutable_flush_threshold_bytes: 32 * 1024 * 1024,
        },
        segment_dir,
        enable_wal: true,
        wal_dir,
        index_dir,
        cache: tokitai_filekv::cache::block_cache::BlockCacheConfig {
            max_items: 100_000,
            max_memory_bytes: 256 * 1024 * 1024,
            frequency_aware: false,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        compaction: tokitai_filekv::compaction::CompactionConfig {
            min_segments: 4,
            auto_compact: false, // Manual compaction for benchmark
            check_interval: 100,
            max_segment_size_bytes: 128 * 1024 * 1024,
            target_segment_size_bytes: 64 * 1024 * 1024,
            async_compaction_enabled: false,
            leveled_compaction_enabled: true,
            level_size_multiplier: 10,
            max_level: 3,
            l0_file_count_threshold: 4,
            parallel_compaction_enabled: false,
            streaming_compaction_enabled: true,
            write_amplification_threshold: 3.0,
            max_background_compaction_threads: 1,
            l0_size_bytes_threshold: 64 * 1024 * 1024,
            l0_compaction_strategy: tokitai_filekv::compaction::CompactionStrategy::Leveled,
            l0_stcs_min_segments: 3,
            l0_stcs_size_ratio: 2.0,
        },
        segment_preallocate_size: 64 * 1024 * 1024,
        block_size: 8192,
        block_compression: tokitai_filekv::BlockCompressionConfig::default(),
        wal_max_size_bytes: 1024 * 1024 * 1024,
        wal_max_files: 10,
        cache_warming_enabled: true,
        compression: tokitai_filekv::DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 8,
        async_io_max_queue_depth: 4096,
        async_io_write_timeout_ms: 5000,
        async_io_enable_coalescing: false,
        async_io_coalesce_window_ms: 10,
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        audit_log: tokitai_filekv::ops::audit_log::AuditLogConfig {
            log_dir: temp_dir.path().join("audit_logs"),
            enabled: false,
            rotation_interval_hours: 24,
            retention_days: 30,
        },
        aggressive: tokitai_filekv::AggressiveConfig::performance(),
        enable_adaptive_bloom_cache: true,
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
        fs: std::sync::Arc::new(tokitai_filekv::io::StdFs),
        enable_multi_level_cache: true,
        l2_cache_max_bytes: 4 * 1024 * 1024 * 1024,
        l2_to_l1_threshold: 5,
        enable_wal_channel: false,
        wal_channel_interval_ms: 2,
        wal_channel_max_entries: 1000,
        wal_channel_capacity: 10_000,
    };

    let kv = tokitai_filekv::FileKV::open(config)?;
    println!("✓ FileKV opened with performance features enabled\n");

    // Benchmark 1: Write throughput
    println!("--- Benchmark 1: Write Throughput ---");
    let num_writes = 10_000;
    let value = vec![0u8; 100]; // 100 byte values

    let start = Instant::now();
    for i in 0..num_writes {
        let key = format!("key_{:06}", i);
        kv.put(&key, &value)?;
    }
    let duration = start.elapsed();
    let writes_per_sec = num_writes as f64 / duration.as_secs_f64();

    println!("  Writes: {}", num_writes);
    println!("  Duration: {:.3}s", duration.as_secs_f64());
    println!("  Throughput: {:.0} writes/sec", writes_per_sec);

    // Benchmark 2: Read latency (hot cache)
    println!("\n--- Benchmark 2: Read Latency (Hot Cache) ---");
    let num_reads = 10_000;

    // Warm up cache by reading some keys
    for i in (0..num_writes).step_by(10) {
        let key = format!("key_{:06}", i);
        let _ = kv.get(&key);
    }

    let start = Instant::now();
    let mut found_count = 0;
    for i in 0..num_reads {
        let key = format!("key_{:06}", i % num_writes);
        if kv.get(&key)?.is_some() {
            found_count += 1;
        }
    }
    let duration = start.elapsed();
    let reads_per_sec = num_reads as f64 / duration.as_secs_f64();
    let avg_latency_us = duration.as_micros() as f64 / num_reads as f64;

    println!("  Reads: {}", num_reads);
    println!("  Found: {}", found_count);
    println!("  Duration: {:.3}s", duration.as_secs_f64());
    println!("  Throughput: {:.0} reads/sec", reads_per_sec);
    println!("  Avg latency: {:.2} μs", avg_latency_us);

    // Benchmark 3: Bloom filter negative lookup
    println!("\n--- Benchmark 3: Bloom Filter (Negative Lookup) ---");
    let num_negative_lookups = 10_000;

    let start = Instant::now();
    for i in 0..num_negative_lookups {
        let key = format!("nonexistent_{:06}", i);
        let _ = kv.get(&key);
    }
    let duration = start.elapsed();
    let lookups_per_sec = num_negative_lookups as f64 / duration.as_secs_f64();
    let avg_latency_us = duration.as_micros() as f64 / num_negative_lookups as f64;

    println!("  Negative lookups: {}", num_negative_lookups);
    println!("  Duration: {:.3}s", duration.as_secs_f64());
    println!("  Throughput: {:.0} lookups/sec", lookups_per_sec);
    println!("  Avg latency: {:.2} μs", avg_latency_us);
    println!("  (Bloom filter quickly rejects non-existent keys)");

    // Flush and view statistics
    println!("\n--- Flushing Memtable ---");
    kv.flush_memtable()?;
    println!("✓ Memtable flushed");

    // View detailed amplification statistics
    println!("\n--- Amplification Statistics ---");
    let stats = kv.get_stats();

    println!("\n  Write Statistics:");
    println!("    Total writes: {}", stats.write_count);
    println!(
        "    User bytes written: {:.2} KB",
        stats.user_bytes_written as f64 / 1024.0
    );
    println!(
        "    Total bytes written (all layers): {:.2} KB",
        stats.total_bytes_written_all as f64 / 1024.0
    );
    println!(
        "    Write amplification factor: {:.2}x",
        stats.write_amplification_factor
    );

    println!("\n  Read Statistics:");
    println!("    Total reads: {}", stats.read_count);
    println!("    I/O operations: {}", stats.read_io_operations);
    println!("    Read amplification factor: {:.2}x", stats.read_amplification_factor);

    println!("\n  Space Statistics:");
    println!(
        "    Total size on disk: {:.2} KB",
        stats.total_size_bytes as f64 / 1024.0
    );
    println!(
        "    Space amplification factor: {:.2}x",
        stats.space_amplification_factor
    );

    println!("\n  Cache Statistics:");
    println!("    Cache hits: {}", stats.cache_hits);
    println!("    Cache misses: {}", stats.cache_misses);
    let total_lookups = stats.cache_hits + stats.cache_misses;
    if total_lookups > 0 {
        let hit_rate = stats.cache_hits as f64 / total_lookups as f64 * 100.0;
        println!("    Cache hit rate: {:.1}%", hit_rate);
    }

    println!("\n  Bloom Filter Statistics:");
    println!("    Bloom filtered lookups: {}", stats.bloom_filtered);

    println!("\n✓ Performance benchmarks completed!");

    Ok(())
}
