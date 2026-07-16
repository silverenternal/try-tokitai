//! FileKV Basic Usage Example
//!
//! This example demonstrates the fundamental operations with FileKV:
//! - Opening a FileKV instance
//! - Inserting key-value pairs
//! - Reading values by key
//! - Deleting keys
//! - Flushing memtable to segments
//! - Running compaction
//! - Viewing amplification statistics

use std::fs;
use tempfile::tempdir;

fn main() -> anyhow::Result<()> {
    // Create temporary directory for storage
    let temp_dir = tempdir()?;
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");

    // Create directories
    fs::create_dir_all(&segment_dir)?;
    fs::create_dir_all(&index_dir)?;
    fs::create_dir_all(&wal_dir)?;

    // Configure FileKV
    let config = tokitai_filekv::FileKVConfig {
        memtable: tokitai_filekv::MemTableConfig {
            flush_threshold_bytes: 16 * 1024 * 1024, // 16MB
            max_entries: 100_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            shards: 32,
            enable_async_flush: false,
            max_immutable_memtables: 1,
            immutable_flush_threshold_bytes: 16 * 1024 * 1024,
        },
        segment_dir,
        enable_wal: true,
        wal_dir,
        index_dir,
        cache: tokitai_filekv::cache::block_cache::BlockCacheConfig {
            max_items: 50_000,
            max_memory_bytes: 128 * 1024 * 1024, // 128MB
            frequency_aware: false,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        compaction: tokitai_filekv::compaction::CompactionConfig {
            min_segments: 4,
            auto_compact: true,
            check_interval: 100,
            max_segment_size_bytes: 64 * 1024 * 1024,
            target_segment_size_bytes: 32 * 1024 * 1024,
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
        segment_preallocate_size: 32 * 1024 * 1024,
        block_size: 8192,
        block_compression: tokitai_filekv::BlockCompressionConfig::default(),
        wal_max_size_bytes: 512 * 1024 * 1024,
        wal_max_files: 10,
        cache_warming_enabled: false,
        compression: tokitai_filekv::DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 4,
        async_io_max_queue_depth: 1024,
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
        // Use defaults for remaining fields
        enable_multi_level_cache: true,
        l2_cache_max_bytes: 4 * 1024 * 1024 * 1024,
        l2_to_l1_threshold: 5,
        enable_wal_channel: false,
        wal_channel_interval_ms: 2,
        wal_channel_max_entries: 1000,
        wal_channel_capacity: 10_000,
    };

    // Open FileKV
    let kv = tokitai_filekv::FileKV::open(config)?;
    println!("✓ FileKV opened successfully");

    // Insert key-value pairs
    println!("\n--- Inserting key-value pairs ---");
    for i in 0..10 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i).into_bytes();
        kv.put(&key, &value)?;
        println!("  PUT {} -> {:?}", key, String::from_utf8_lossy(&value));
    }

    // Read values
    println!("\n--- Reading values ---");
    for i in 0..10 {
        let key = format!("key_{}", i);
        if let Some(value) = kv.get(&key)? {
            println!("  GET {} -> {:?}", key, String::from_utf8_lossy(&value));
        } else {
            println!("  GET {} -> NOT FOUND", key);
        }
    }

    // Try to read non-existent key
    println!("\n--- Reading non-existent key ---");
    match kv.get("nonexistent_key")? {
        Some(value) => println!("  Found: {:?}", String::from_utf8_lossy(&value)),
        None => println!("  Key not found (expected)"),
    }

    // Delete a key
    println!("\n--- Deleting a key ---");
    kv.delete("key_5")?;
    println!("  Deleted key_5");

    // Verify deletion
    match kv.get("key_5")? {
        Some(value) => println!("  After delete: Found {:?}", String::from_utf8_lossy(&value)),
        None => println!("  After delete: Key not found (expected)"),
    }

    // Flush memtable to segments
    println!("\n--- Flushing memtable ---");
    kv.flush_memtable()?;
    println!("✓ Memtable flushed to segments");

    // View amplification statistics
    println!("\n--- Amplification Statistics ---");
    let stats = kv.get_stats();
    println!("  Write count: {}", stats.write_count);
    println!("  Read count: {}", stats.read_count);
    println!("  User bytes written: {}", stats.user_bytes_written);
    println!("  Total bytes written (all layers): {}", stats.total_bytes_written_all);
    println!("  Write amplification factor: {:.2}x", stats.write_amplification_factor);
    println!("  Read amplification factor: {:.2}x", stats.read_amplification_factor);
    println!("  Space amplification factor: {:.2}x", stats.space_amplification_factor);
    println!("  Cache hits: {}", stats.cache_hits);
    println!("  Cache misses: {}", stats.cache_misses);
    if stats.cache_hits + stats.cache_misses > 0 {
        let hit_rate = stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64 * 100.0;
        println!("  Cache hit rate: {:.1}%", hit_rate);
    }

    // Run compaction if needed
    println!("\n--- Running compaction ---");
    kv.run_compaction()?;
    println!("✓ Compaction completed");

    // Final statistics
    let final_stats = kv.get_stats();
    println!("\n--- Final Statistics ---");
    println!("  Segments: {}", final_stats.segment_count);
    println!("  Total entries: {}", final_stats.total_entries);
    println!("  Compaction runs: {}", final_stats.compaction_runs);
    println!("  Tombstones removed: {}", final_stats.compaction_tombstones_removed);

    println!("\n✓ All operations completed successfully!");

    Ok(())
}
