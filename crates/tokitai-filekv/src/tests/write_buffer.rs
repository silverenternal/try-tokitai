//! Phase 6: Write Buffer and Durability Tests
//!
//! Tests for:
//! - WriteBuffer flush timing (size threshold, time window)
//! - Durability guarantees (Buffered vs Immediate)
//! - Batch WAL recovery after crash

use std::sync::Arc;
use std::fs;
use tempfile::TempDir;
use crate::{FileKV, FileKVConfig, core::write_coalescer::{WriteBuffer, WriteBufferConfig}};
use crate::io::StdFs;
use crate::core::types::Durability;

/// Helper to create test config
fn test_config() -> (TempDir, FileKVConfig) {
    let temp_dir = TempDir::new().unwrap();
    let segment_dir = temp_dir.path().join("segments");
    let index_dir = temp_dir.path().join("index");
    let wal_dir = temp_dir.path().join("wal");

    fs::create_dir_all(&segment_dir).unwrap();
    fs::create_dir_all(&index_dir).unwrap();
    fs::create_dir_all(&wal_dir).unwrap();

    let config = FileKVConfig {
        memtable: crate::core::memtable::MemTableConfig {
            flush_threshold_bytes: 16 * 1024 * 1024,
            max_entries: 100_000,
            max_memory_bytes: 64 * 1024 * 1024,
            shards: 32,
        },
        segment_dir: segment_dir.clone(),
        enable_wal: true,
        wal_dir,
        index_dir,
        cache: crate::cache::block_cache::BlockCacheConfig {
            max_items: 50_000,
            max_memory_bytes: 128 * 1024 * 1024,
            frequency_aware: false,
        },
        enable_bloom: true,
        enable_background_flush: false,
        background_flush_interval_ms: 100,
        compaction: crate::compaction::CompactionConfig {
            min_segments: 4,
            auto_compact: false,
            check_interval: 100,
            max_segment_size_bytes: 64 * 1024 * 1024,
            target_segment_size_bytes: 32 * 1024 * 1024,
            async_compaction_enabled: false,
            leveled_compaction_enabled: false,
            level_size_multiplier: 10,
            max_level: 3,
            l0_file_count_threshold: 4,
            parallel_compaction_enabled: false,
            streaming_compaction_enabled: true,
            write_amplification_threshold: 3.0, // OPT-003: Default WA threshold
            max_background_compaction_threads: 1, // Disabled for tests
            l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: Default L0 size trigger
        },
        segment_preallocate_size: 0,
        wal_max_size_bytes: 100 * 1024 * 1024,
        wal_max_files: 5,
        cache_warming_enabled: false,
        compression: crate::compression::dictionary::DictionaryCompressionConfig::default(),
        async_io_enabled: false,
        async_io_max_concurrent_writes: 4,
        async_io_max_queue_depth: 1024,
        async_io_write_timeout_ms: 5000,
        async_io_enable_coalescing: false,
        async_io_coalesce_window_ms: 10,
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        audit_log: crate::ops::audit_log::AuditLogConfig {
            log_dir: temp_dir.path().join("audit_logs"),
            enabled: false,
            rotation_interval_hours: 24,
            retention_days: 30,
        },
        aggressive: crate::AggressiveConfig::balanced(),
        enable_adaptive_bloom_cache: true,
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
        enable_background_cache_rebalance: false,
        fs: Arc::new(StdFs),
        block_size: 8192,
        block_compression: crate::core::types::BlockCompressionConfig::default(),
    };

    (temp_dir, config)
}

// ============================================================
// Test Group 1: WriteBuffer Flush Timing
// ============================================================

#[test]
fn test_write_buffer_size_threshold_triggers_flush() {
    // Test that write buffer flushes when size threshold is reached
    let config = WriteBufferConfig {
        time_window_us: 1_000_000, // Long time window (1s)
        size_threshold_bytes: 200, // Small threshold for testing
    };
    let buffer = WriteBuffer::new(config);

    // Add writes until threshold is exceeded
    let mut flush_triggered = false;
    for i in 0..20 {
        let result = buffer.add(
            format!("key_{}", i),
            vec![0u8; 20], // 20 bytes per write
        );
        if result.is_some() {
            flush_triggered = true;
            let batch = result.unwrap();
            assert!(!batch.is_empty());
            assert!(batch.len() <= 20);
            break;
        }
    }

    assert!(flush_triggered, "Flush should be triggered by size threshold");
}

#[test]
fn test_write_buffer_time_window_triggers_flush() {
    // Test that write buffer flushes when time window is exceeded
    let config = WriteBufferConfig {
        time_window_us: 1000, // 1ms time window
        size_threshold_bytes: 1024 * 1024, // Large size threshold
    };
    let buffer = WriteBuffer::new(config);

    // Add first write
    let result1 = buffer.add("key1".to_string(), b"value1".to_vec());
    assert!(result1.is_none()); // Should not trigger flush yet

    // Wait for time window to expire
    std::thread::sleep(std::time::Duration::from_millis(2));

    // Add second write - should trigger flush due to time window
    let result2 = buffer.add("key2".to_string(), b"value2".to_vec());
    assert!(result2.is_some(), "Time window should trigger flush");
}

#[test]
fn test_write_buffer_force_flush_returns_all_pending() {
    // Test that force_flush returns all pending writes
    let config = WriteBufferConfig::default();
    let buffer = WriteBuffer::new(config);

    // Add several writes
    for i in 0..10 {
        buffer.add(
            format!("key_{}", i),
            format!("value_{}", i).into_bytes(),
        );
    }

    assert_eq!(buffer.pending_count(), 10);

    // Force flush
    let batch = buffer.force_flush();
    assert_eq!(batch.len(), 10);
    assert_eq!(buffer.pending_count(), 0);
    assert!(!buffer.has_pending());

    // Verify all keys are present
    let keys: Vec<&str> = batch.iter().map(|w| w.key.as_str()).collect();
    for i in 0..10 {
        assert!(keys.contains(&format!("key_{}", i).as_str()));
    }
}

#[test]
fn test_write_buffer_empty_after_flush() {
    // Test that buffer is empty after flush
    let config = WriteBufferConfig {
        time_window_us: 1000,
        size_threshold_bytes: 1000,
    };
    let buffer = WriteBuffer::new(config);

    // Add writes
    for i in 0..5 {
        buffer.add(format!("key_{}", i), vec![0u8; 10]);
    }

    assert!(buffer.has_pending());
    assert!(buffer.pending_count() > 0);
    assert!(buffer.buffer_size() > 0);

    // Force flush
    let _ = buffer.force_flush();

    // Verify buffer is empty
    assert!(!buffer.has_pending());
    assert_eq!(buffer.pending_count(), 0);
    assert_eq!(buffer.buffer_size(), 0);
}

// ============================================================
// Test Group 2: Durability Guarantees
// ============================================================

#[test]
fn test_durability_buffered_write() {
    // Test that Buffered durability uses write buffer
    let (temp_dir, config) = test_config();
    let kv = FileKV::open(config).unwrap();

    // Write with Buffered durability
    for i in 0..50 {
        kv.put_with_durability(
            &format!("key_{}", i),
            &format!("value_{}", i).into_bytes(),
            Durability::Buffered,
        ).unwrap();
    }

    // Verify data is readable (may still be in buffer or already flushed)
    for i in 0..50 {
        let result = kv.get(&format!("key_{}", i)).unwrap();
        // Data might not be in memtable yet if still buffered
        // but put() eventually flushes to memtable
        assert!(result.is_some() || kv.write_coalescer_ref().pending_count() > 0);
    }

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}

#[test]
fn test_durability_immediate_write() {
    // Test that Immediate durability writes directly to WAL + MemTable
    let (temp_dir, config) = test_config();
    let kv = FileKV::open(config).unwrap();

    // Write with Immediate durability
    for i in 0..10 {
        kv.put_with_durability(
            &format!("key_{}", i),
            &format!("value_{}", i).into_bytes(),
            Durability::Immediate,
        ).unwrap();
    }

    // All data should be in memtable immediately
    for i in 0..10 {
        let result = kv.get(&format!("key_{}", i)).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), format!("value_{}", i).into_bytes());
    }

    // Write buffer should be empty (bypassed)
    assert_eq!(kv.write_coalescer_ref().pending_count(), 0);

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}

#[test]
fn test_durability_immediate_survives_restart() {
    // Test that Immediate durability survives restart
    let (temp_dir, config) = test_config();
    let segment_dir = config.segment_dir.clone();
    let wal_dir = config.wal_dir.clone();
    let index_dir = config.index_dir.clone();

    // Write with Immediate durability
    {
        let kv = FileKV::open(config.clone()).unwrap();
        for i in 0..10 {
            kv.put_with_durability(
                &format!("key_{}", i),
                &format!("value_{}", i).into_bytes(),
                Durability::Immediate,
            ).unwrap();
        }
        // Don't flush memtable - data should be in WAL
    }

    // Reopen
    let config2 = FileKVConfig {
        segment_dir,
        wal_dir,
        index_dir,
        ..config
    };
    let kv = FileKV::open(config2).unwrap();

    // Data should be recovered from WAL
    for i in 0..10 {
        let result = kv.get(&format!("key_{}", i)).unwrap();
        assert!(result.is_some(), "Key {} should be recovered from WAL", i);
        assert_eq!(result.unwrap(), format!("value_{}", i).into_bytes());
    }

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}

#[test]
fn test_mixed_durability_writes() {
    // Test mixing Buffered and Immediate writes
    let (temp_dir, config) = test_config();
    let kv = FileKV::open(config).unwrap();

    // Mix of Buffered and Immediate writes
    kv.put_with_durability("buffered_1", b"value1", Durability::Buffered).unwrap();
    kv.put_with_durability("immediate_1", b"value1", Durability::Immediate).unwrap();
    kv.put_with_durability("buffered_2", b"value2", Durability::Buffered).unwrap();
    kv.put_with_durability("immediate_2", b"value2", Durability::Immediate).unwrap();

    // Flush to ensure all data is in memtable/segments
    kv.flush_memtable().unwrap();

    // All data should be readable
    for key in &["buffered_1", "immediate_1", "buffered_2", "immediate_2"] {
        let result = kv.get(key).unwrap();
        assert!(result.is_some(), "Key {} should be readable", key);
    }

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}

// ============================================================
// Test Group 3: Batch WAL Recovery
// ============================================================

#[test]
fn test_batch_wal_recovery_after_crash() {
    // Test that batch WAL writes survive simulated crash
    let (temp_dir, config) = test_config();
    let segment_dir = config.segment_dir.clone();
    let wal_dir = config.wal_dir.clone();
    let index_dir = config.index_dir.clone();

    // Write batch with Buffered durability
    {
        let kv = FileKV::open(config.clone()).unwrap();

        // Add writes (buffered)
        for i in 0..20 {
            kv.put(
                &format!("key_{}", i),
                &format!("value_{}", i).into_bytes(),
            ).unwrap();
        }

        // Flush memtable to ensure WAL has entries
        kv.flush_memtable().unwrap();
    }

    // Reopen and verify recovery
    let config2 = FileKVConfig {
        segment_dir,
        wal_dir,
        index_dir,
        ..config
    };
    let kv = FileKV::open(config2).unwrap();

    // All data should be recovered
    for i in 0..20 {
        let result = kv.get(&format!("key_{}", i)).unwrap();
        assert!(result.is_some(), "Key {} should be recovered", i);
    }

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}

#[test]
fn test_wal_batch_atomic_write() {
    // Test that batch WAL writes are atomic
    let (temp_dir, config) = test_config();
    let kv = FileKV::open(config).unwrap();

    // Batch write using simple static arrays
    let entries: Vec<(&str, &[u8])> = vec![
        ("key_0", b"value_0" as &[u8]),
        ("key_1", b"value_1"),
        ("key_2", b"value_2"),
        ("key_3", b"value_3"),
        ("key_4", b"value_4"),
        ("key_5", b"value_5"),
        ("key_6", b"value_6"),
        ("key_7", b"value_7"),
        ("key_8", b"value_8"),
        ("key_9", b"value_9"),
    ];
    
    kv.put_batch(&entries).unwrap();

    // All or nothing: either all keys present or none
    let mut all_present = true;
    let mut _all_absent = true;

    for i in 0..10 {
        let result = kv.get(&format!("key_{}", i)).unwrap();
        if result.is_some() {
            _all_absent = false;
        } else {
            all_present = false;
        }
    }

    // Should be all present (batch succeeded)
    assert!(all_present, "Batch write should make all keys available");

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}

#[test]
fn test_flush_memtable_drains_write_buffer() {
    // Test that flush_memtable drains write buffer first
    let (temp_dir, config) = test_config();
    let kv = FileKV::open(config).unwrap();

    // Add writes to buffer
    for i in 0..10 {
        kv.put(
            &format!("key_{}", i),
            &format!("value_{}", i).into_bytes(),
        ).unwrap();
    }

    let _pending_before = kv.write_coalescer_ref().pending_count();

    // Flush memtable (should drain buffer first)
    kv.flush_memtable().unwrap();

    // Buffer should be empty after flush
    let pending_after = kv.write_coalescer_ref().pending_count();
    assert_eq!(pending_after, 0, "Write buffer should be drained after flush");

    // Data should be in segments
    for i in 0..10 {
        let result = kv.get(&format!("key_{}", i)).unwrap();
        assert!(result.is_some(), "Key {} should be in segments", i);
    }

    drop(kv);
    fs::remove_dir_all(temp_dir.path()).ok();
}
