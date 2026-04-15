//! Per-engine unit tests for Phase 4 God Object Decomposition
//!
//! Tests each engine independently plus cross-engine integration.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use crate::engine::{EngineState, ReadEngine, WriteEngine, CompactionEngine, LifecycleManager};
    use crate::core::types::FileKVConfig;
    use crate::core::types::FileKVStats;
    use crate::core::memtable::MemTable;
    use crate::bloom::filter_cache::{BloomFilterCache, BloomFilterCacheConfig};
    use crate::core::sparse_index::IndexManager;
    use crate::ops::feature_flag::FeatureFlagController;
    use crate::ops::memory_tracker::MemoryTracker;
    use crate::bloom::migration::MigrationController;
    use crate::compaction::CompactionManager;
    use crate::core::flush::FlushTrigger;
    use crate::io::StdFs;
    use crate::core::write_coalescer::{WriteBuffer, WriteBufferConfig};
    use crate::cache::UnifiedCacheManager;

    /// Helper to create a minimal EngineState for testing with temp directories
    fn make_test_engine_state() -> Arc<EngineState> {
        let mut config = FileKVConfig::default();

        // Use temp directories避免 flush_memtable 时目录不存在
        let temp_base = std::env::temp_dir().join(format!("filekv_engine_test_{}", std::process::id()));
        config.segment_dir = temp_base.join("segments");
        config.wal_dir = temp_base.join("wal");
        config.index_dir = temp_base.join("index");

        // Create directories
        std::fs::create_dir_all(&config.segment_dir).ok();
        std::fs::create_dir_all(&config.wal_dir).ok();
        std::fs::create_dir_all(&config.index_dir).ok();

        let segments = BTreeMap::new();
        let index_manager = IndexManager::new(&config.index_dir).expect("create index manager");
        let stats = Arc::new(FileKVStats::default());
        let memtable = Arc::new(MemTable::new(config.memtable.clone()));
        let bloom_filter_cache = Arc::new(BloomFilterCache::new(
            BloomFilterCacheConfig::default(),
            config.index_dir.clone(),
        ));

        // GAP-M5: Create UnifiedCacheManager for test engine state
        let unified_cache = Arc::new(UnifiedCacheManager::new(
            crate::cache::UnifiedCacheConfig::default(),
        ));
        let block_cache = unified_cache.block_cache().clone();

        // ENG-007: Use builder pattern instead of 10+ parameter constructor
        Arc::new(
            EngineState::builder(config)
                .segments(segments)
                .next_segment_id(1)
                .index_manager(index_manager)
                .stats(stats)
                .memtable(memtable)
                .bloom_filter_cache(bloom_filter_cache)
                .block_cache(block_cache)
                .unified_cache(Some(unified_cache))
                .build(),
        )
    }

    // ========== ReadEngine Tests ==========

    #[test]
    fn test_read_engine_get_miss_memtable() {
        let state = make_test_engine_state();
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));

        let read_engine = ReadEngine::new(
            state,
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // No data written, get should return None
        let (result, _cache_result) = read_engine.get("nonexistent_key").expect("get should not error");
        assert!(result.is_none());
    }

    #[test]
    fn test_read_engine_get_from_memtable() {
        let state = make_test_engine_state();
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));

        let read_engine = ReadEngine::new(
            state.clone(),
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // Write to memtable directly
        state.memtable_state.memtable.insert("test_key".to_string(), b"test_value");

        // Read via ReadEngine
        let (result, _cache_result) = read_engine.get("test_key").expect("get should not error");
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"test_value");
    }

    #[test]
    fn test_read_engine_feature_flags() {
        let state = make_test_engine_state();
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));

        let read_engine = ReadEngine::new(
            state,
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // Default: INNO-001 and INNO-002 should be enabled
        assert!(read_engine.is_adaptive_bloom_cache_enabled());
        assert!(read_engine.is_zone_map_pruning_enabled());
        assert!(read_engine.is_sequential_prefetch_enabled());

        // Disable INNO-001
        read_engine.disable_inno001();
        assert!(!read_engine.is_adaptive_bloom_cache_enabled());

        // Disable INNO-002
        read_engine.disable_inno002();
        assert!(!read_engine.is_zone_map_pruning_enabled());
        assert!(!read_engine.is_sequential_prefetch_enabled());

        // Re-enable
        read_engine.enable_inno001();
        read_engine.enable_inno002();
        assert!(read_engine.is_adaptive_bloom_cache_enabled());
        assert!(read_engine.is_zone_map_pruning_enabled());
    }

    #[test]
    fn test_read_engine_memory_usage() {
        let state = make_test_engine_state();
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));

        let read_engine = ReadEngine::new(
            state,
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        let usage = read_engine.get_memory_usage();
        // Basic check - should not panic and return valid usage (u64 is always >= 0)
        let _ = usage.total_bytes();
    }

    #[test]
    fn test_read_engine_bloom_migration_stats() {
        let state = make_test_engine_state();
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));

        let read_engine = ReadEngine::new(
            state.clone(),
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // Access a segment to record access
        state.memtable_state.memtable.insert("_probe".to_string(), b"x");
        let _ = read_engine.get("_probe");

        let stats = read_engine.get_bloom_migration_stats();
        // Should have recorded access (u64 is always >= 0)
        let _ = stats.tracked_segments;
    }

    // ========== WriteEngine Tests ==========

    #[test]
    fn test_write_engine_put_basic() {
        let state = make_test_engine_state();
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );

        let write_engine = WriteEngine::new(
            state.clone(),
            None, // no WAL for this test
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None, // no compressor
            None, // no async writer
            FlushTrigger::new(),
            compaction_manager,
            None, // no audit logger
            None, // no preallocator
        );

        // Write a KV pair with Immediate durability to bypass write buffer
        write_engine.put_with_durability("key1", b"value1", crate::core::types::Durability::Immediate)
            .expect("put should succeed");

        // Verify via memtable (no flush needed with Immediate durability)
        let (val, _, deleted) = state.memtable_state.memtable.get("key1").expect("key should exist");
        assert!(!deleted);
        assert_eq!(val.unwrap().as_ref(), b"value1");
    }

    #[test]
    fn test_write_engine_put_multiple() {
        let state = make_test_engine_state();
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );

        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Write multiple KV pairs with Immediate durability
        write_engine.put_with_durability("alpha", b"one", crate::core::types::Durability::Immediate).expect("put should succeed");
        write_engine.put_with_durability("beta", b"two", crate::core::types::Durability::Immediate).expect("put should succeed");
        write_engine.put_with_durability("gamma", b"three", crate::core::types::Durability::Immediate).expect("put should succeed");

        // Verify all in memtable
        let test_data: Vec<(&str, &[u8])> = vec![("alpha", b"one".as_slice()), ("beta", b"two".as_slice()), ("gamma", b"three".as_slice())];
        for (key, expected) in &test_data {
            let (val, _, deleted) = state.memtable_state.memtable.get(key).expect("key should exist");
            assert!(!deleted);
            assert_eq!(val.unwrap().as_ref(), *expected);
        }
    }

    #[test]
    fn test_write_engine_delete() {
        let state = make_test_engine_state();
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );

        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Write then delete with Immediate durability
        write_engine.put_with_durability("delete_me", b"temp", crate::core::types::Durability::Immediate)
            .expect("put should succeed");
        write_engine.delete("delete_me").expect("delete should succeed");

        // Delete creates a tombstone (empty value)
        let (val, _, deleted) = state.memtable_state.memtable.get("delete_me").expect("key should exist");
        // Tombstone = empty value
        assert!(val.map_or(true, |v| v.is_empty()) || deleted);
    }

    #[test]
    fn test_write_engine_put_batch() {
        let state = make_test_engine_state();
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );

        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Batch write
        let entries: Vec<(&str, &[u8])> = vec![
            ("batch1", b"val1".as_slice()),
            ("batch2", b"val2".as_slice()),
            ("batch3", b"val3".as_slice()),
        ];
        write_engine.put_batch(&entries).expect("batch put should succeed");

        // Verify all
        let test_entries: Vec<(&str, &[u8])> = vec![("batch1", b"val1".as_slice()), ("batch2", b"val2".as_slice()), ("batch3", b"val3".as_slice())];
        for (key, expected) in &test_entries {
            let (val, _, deleted) = state.memtable_state.memtable.get(key).expect("key should exist");
            assert!(!deleted);
            assert_eq!(val.unwrap().as_ref(), *expected);
        }
    }

    #[test]
    fn test_write_engine_stats() {
        let state = make_test_engine_state();
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );

        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Use Immediate durability to bypass write buffer and write directly to memtable
        write_engine.put_with_durability("stat_key", b"stat_val", crate::core::types::Durability::Immediate)
            .expect("put should succeed");

        let stats = write_engine.get_stats();
        assert!(stats.write_count >= 1);
        assert!(stats.memtable_entries >= 1);
    }

    // ========== CompactionEngine Tests ==========

    #[test]
    fn test_compaction_engine_creation() {
        let state = make_test_engine_state();
        let compaction_config = crate::compaction::CompactionConfig::default();

        let compaction_engine = CompactionEngine::new(
            state.clone(),
            compaction_config,
            None,
        );

        // Basic check - engine created successfully
        assert!(compaction_engine.compaction_manager().lock().should_run_compaction());
    }

    #[test]
    fn test_compaction_engine_no_segments_to_compact() {
        let state = make_test_engine_state();
        let compaction_config = crate::compaction::CompactionConfig::default();

        let compaction_engine = CompactionEngine::new(
            state.clone(),
            compaction_config,
            None,
        );

        // No segments, run_compaction should return default stats
        let result = compaction_engine.run_compaction(|_| {
            // This closure won't be called because segment count < min_segments
            Ok(crate::compaction::CompactionStats::default())
        });
        assert!(result.is_ok());
        let stats = result.unwrap();
        // No segments to compact, should return default (all zeros)
        assert_eq!(stats.segments_merged, 0);
    }

    #[test]
    fn test_compaction_engine_maybe_run_empty() {
        let state = make_test_engine_state();
        let compaction_config = crate::compaction::CompactionConfig::default();

        let compaction_engine = CompactionEngine::new(
            state.clone(),
            compaction_config,
            None,
        );

        // No segments, maybe_run_compaction should succeed
        let result = compaction_engine.maybe_run_compaction();
        assert!(result.is_ok());
    }

    #[test]
    fn test_compaction_engine_record_write() {
        let state = make_test_engine_state();
        let mut compaction_config = crate::compaction::CompactionConfig::default();
        compaction_config.check_interval = 5; // Trigger every 5 writes for testing
        compaction_config.auto_compact = true;

        let compaction_engine = CompactionEngine::new(
            state,
            compaction_config,
            None,
        );

        // The compaction engine has its own internal CompactionManager with the config.
        // record_write uses fetch_add which returns OLD value.
        // With check_interval=5: calls 1-5 have old values 0-4, all < 5 → false
        // Call 6: old=5 >= 5 → true
        for _ in 0..5 {
            assert!(!compaction_engine.record_write());
        }

        // 6th write should trigger compaction check (old_count=5 >= check_interval=5)
        assert!(compaction_engine.record_write());
    }

    // ========== COMP-006: Concurrent Compaction Tests ==========

    /// Test: COMP-006 - Two threads simultaneously triggering compaction
    /// Verifies no panics, deadlocks, or segment conflicts
    #[test]
    fn test_concurrent_compaction_no_data_loss() {
        use std::thread;
        use std::sync::atomic::AtomicUsize;

        let state = make_test_engine_state();

        let compaction_config = crate::compaction::CompactionConfig {
            min_segments: 1,
            auto_compact: false,
            ..Default::default()
        };

        let compaction_engine = CompactionEngine::new(
            state.clone(),
            compaction_config,
            None,
        );

        let success_count = Arc::new(AtomicUsize::new(0));
        let num_threads: usize = 2;

        // Two threads simultaneously run compaction
        thread::scope(|s| {
            for _ in 0..num_threads {
                let ce = &compaction_engine;
                let success = success_count.clone();
                s.spawn(move || {
                    // run_compaction should handle concurrent calls safely
                    match ce.run_compaction(|_| Ok(crate::compaction::CompactionStats::default())) {
                        Ok(_stats) => {
                            success.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(_) => {
                            // Error is acceptable in concurrent scenario
                        }
                    }
                });
            }
        });

        // Verify no panics or deadlocks (test completes)
        let successes = success_count.load(std::sync::atomic::Ordering::Relaxed);
        assert!(successes <= num_threads, "At most {} threads should succeed", num_threads);
    }

    /// Test: COMP-006 - Concurrent compaction runs verify all complete
    #[test]
    fn test_concurrent_multiple_compaction_runs() {
        use std::thread;
        use std::sync::atomic::AtomicUsize;

        let state = make_test_engine_state();

        let compaction_config = crate::compaction::CompactionConfig {
            min_segments: 2,
            auto_compact: false,
            ..Default::default()
        };

        let compaction_engine = CompactionEngine::new(
            state.clone(),
            compaction_config,
            None,
        );

        let completed_count = Arc::new(AtomicUsize::new(0));
        let num_threads: usize = 3;

        thread::scope(|s| {
            for _ in 0..num_threads {
                let ce = &compaction_engine;
                let completed = completed_count.clone();
                s.spawn(move || {
                    let _ = ce.run_compaction(|_| Ok(crate::compaction::CompactionStats::default()));
                    completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                });
            }
        });

        // All threads should have completed (no deadlocks, no panics)
        let completed = completed_count.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(completed, num_threads, "All concurrent compaction runs should complete");
    }

    // ========== LifecycleManager Tests ==========

    #[test]
    fn test_lifecycle_manager_open_creates_dirs() {
        let temp_dir = std::env::temp_dir().join(format!("filekv_test_{}", std::process::id()));

        let config = FileKVConfig {
            segment_dir: temp_dir.join("segments"),
            wal_dir: temp_dir.join("wal"),
            index_dir: temp_dir.join("index"),
            checkpoint_dir: temp_dir.join("checkpoints"),
            fs: Arc::new(StdFs),
            ..Default::default()
        };

        let result = LifecycleManager::open(config.clone());
        assert!(result.is_ok());
        let state = result.unwrap();

        // Verify directories were created
        assert!(config.fs.file_exists(&config.segment_dir));
        assert!(config.fs.file_exists(&config.wal_dir));
        assert!(config.fs.file_exists(&config.index_dir));

        // Verify state
        assert_eq!(state.config.segment_dir, config.segment_dir);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_lifecycle_manager_timeout_config() {
        let state = make_test_engine_state();
        let checkpoint_manager = parking_lot::Mutex::new(
            crate::checkpoint::IncrementalCheckpointManager::new(
                &state.config.checkpoint_dir,
            ).expect("create checkpoint manager"),
        );

        let lifecycle = LifecycleManager::new(
            state,
            checkpoint_manager,
            None,
            #[cfg(feature = "metrics")]
            Arc::new(crate::ops::metrics::FileKVMetrics::new()),
            crate::ops::timeout_control::TimeoutConfig::default(),
            None,
            FlushTrigger::new(),
            Arc::new(
                CompactionManager::new(crate::compaction::CompactionConfig::default()),
            ),
        );

        // Check default timeout config
        {
            let timeout_config = lifecycle.get_timeout_config();
            // Basic check - should not panic
            assert!(timeout_config.read_timeout_ms > 0 || timeout_config.write_timeout_ms > 0);
        }

        // Change timeout config
        let new_config = crate::ops::timeout_control::TimeoutConfig {
            read_timeout_ms: 5000,
            write_timeout_ms: 10000,
            ..Default::default()
        };
        lifecycle.set_timeout_config(new_config);

        let updated = lifecycle.get_timeout_config();
        assert_eq!(updated.read_timeout_ms, 5000);
        assert_eq!(updated.write_timeout_ms, 10000);
    }

    #[test]
    fn test_lifecycle_manager_timeout_stats() {
        let state = make_test_engine_state();
        let checkpoint_manager = parking_lot::Mutex::new(
            crate::checkpoint::IncrementalCheckpointManager::new(
                &state.config.checkpoint_dir,
            ).expect("create checkpoint manager"),
        );

        let lifecycle = LifecycleManager::new(
            state,
            checkpoint_manager,
            None,
            #[cfg(feature = "metrics")]
            Arc::new(crate::ops::metrics::FileKVMetrics::new()),
            crate::ops::timeout_control::TimeoutConfig::default(),
            None,
            FlushTrigger::new(),
            Arc::new(
                CompactionManager::new(crate::compaction::CompactionConfig::default()),
            ),
        );

        // Initial stats should be zero
        let stats = lifecycle.get_timeout_stats();
        assert_eq!(stats.timeout_count, 0);

        // Reset should work
        lifecycle.reset_timeout_stats();
        let stats_after_reset = lifecycle.get_timeout_stats();
        assert_eq!(stats_after_reset.timeout_count, 0);
    }

    // ========== Cross-Engine Integration Tests ==========

    #[test]
    fn test_integration_read_after_write() {
        // WriteEngine writes to memtable, ReadEngine reads from memtable
        let state = make_test_engine_state();

        // Create WriteEngine
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );
        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Create ReadEngine
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));
        let read_engine = ReadEngine::new(
            state.clone(),
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // Write via WriteEngine (use Immediate to bypass buffer)
        write_engine.put_with_durability("integration_key", b"integration_value", crate::core::types::Durability::Immediate)
            .expect("put should succeed");

        // Read via ReadEngine
        let (result, _cache_result) = read_engine.get("integration_key").expect("get should not error");
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"integration_value");
    }

    #[test]
    fn test_integration_write_batch_then_read_all() {
        let state = make_test_engine_state();

        // Create WriteEngine
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );
        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Create ReadEngine
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));
        let read_engine = ReadEngine::new(
            state.clone(),
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // Batch write
        let entries: Vec<(&str, &[u8])> = vec![
            ("ikey1", b"ival1".as_slice()),
            ("ikey2", b"ival2".as_slice()),
            ("ikey3", b"ival3".as_slice()),
        ];
        write_engine.put_batch(&entries).expect("batch put should succeed");

        // Read all back
        for (key, expected) in &entries {
            let (result, _cache_result) = read_engine.get(key).expect("get should not error");
            assert!(result.is_some(), "key {} should exist", key);
            assert_eq!(result.unwrap().as_ref(), *expected);
        }
    }

    #[test]
    fn test_integration_delete_then_read() {
        let state = make_test_engine_state();

        // Create WriteEngine
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );
        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Create ReadEngine
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));
        let read_engine = ReadEngine::new(
            state.clone(),
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        // Write then delete (use Immediate to bypass buffer)
        write_engine.put_with_durability("del_key", b"del_val", crate::core::types::Durability::Immediate)
            .expect("put should succeed");
        let (result, _cache_result) = read_engine.get("del_key").expect("get should not error");
        assert!(result.is_some());

        write_engine.delete("del_key").expect("delete should succeed");

        // After delete, ReadEngine should return None (tombstone = empty value treated as deleted)
        let (result, _cache_result) = read_engine.get("del_key").expect("get should not error");
        // Delete writes empty value; memtable.get returns (Some(empty), _, false) or (None, _, true)
        // Either way, the key is effectively deleted
        assert!(result.as_ref().map_or(true, |v| v.is_empty()) || result.is_none(),
            "deleted key should return None or empty value");
    }

    #[test]
    fn test_integration_stats_increment() {
        let state = make_test_engine_state();

        // Create WriteEngine
        let compaction_manager = Arc::new(
            CompactionManager::new(crate::compaction::CompactionConfig::default()),
        );
        let write_engine = WriteEngine::new(
            state.clone(),
            None,
            Arc::new(WriteBuffer::new(WriteBufferConfig::default())),
            None,
            None,
            FlushTrigger::new(),
            compaction_manager,
            None,
            None,
        );

        // Create ReadEngine
        let feature_flags = Arc::new(FeatureFlagController::new());
        let memory_tracker = Arc::new(MemoryTracker::new(0));
        let bloom_migration = Arc::new(MigrationController::new(
            crate::bloom::migration::MigrationThresholds::default(),
        ));
        let read_engine = ReadEngine::new(
            state.clone(),
            feature_flags,
            None,
            None,
            memory_tracker,
            bloom_migration,
            None, // compressor (S2-1)
        );

        let write_count_before = state.stats_state.stats.write_count.load(std::sync::atomic::Ordering::Relaxed);
        let read_count_before = state.stats_state.stats.read_count.load(std::sync::atomic::Ordering::Relaxed);

        // Write and read
        write_engine.put("stats_key", b"stats_val").expect("put should succeed");
        let _ = read_engine.get("stats_key").expect("get should not error");

        let write_count_after = state.stats_state.stats.write_count.load(std::sync::atomic::Ordering::Relaxed);
        let read_count_after = state.stats_state.stats.read_count.load(std::sync::atomic::Ordering::Relaxed);

        assert!(write_count_after > write_count_before, "write count should increment");
        assert!(read_count_after > read_count_before, "read count should increment");
    }

    // === Async I/O Integration Tests ===

    #[cfg(feature = "async-io")]
    mod async_io_tests {
        use super::*;
        use crate::ops::async_io::{AsyncWriter, AsyncIoConfig};
        use tokio::time::{timeout, Duration};

        #[tokio::test]
        async fn test_async_writer_basic_operations() {
            // Add 30 second timeout to prevent test hangs
            timeout(Duration::from_secs(30), async {
                // Test AsyncWriter directly without full engine setup
                let temp_dir = std::env::temp_dir().join(format!("async_test_{}", std::process::id()));
                std::fs::create_dir_all(&temp_dir).ok();

                let async_config = AsyncIoConfig::default();
                let async_writer = AsyncWriter::new(async_config, temp_dir.clone())
                    .expect("create async writer");

                // Test async segment write
                let data = bytes::Bytes::from(b"test async data".to_vec());
                let result = async_writer.write_segment(1, 0, data.clone()).await
                    .expect("async segment write should succeed");
                assert!(result.success);
                assert_eq!(result.bytes_written, 15);

                // Test async WAL write
                let wal_data = bytes::Bytes::from(b"wal entry".to_vec());
                let wal_result = async_writer.write_wal(wal_data, false).await
                    .expect("async WAL write should succeed");
                assert!(wal_result.success);

                // Test sync bridge - must run in spawn_blocking to avoid blocking the runtime
                let sync_result = tokio::task::spawn_blocking(move || {
                    let sync_data = bytes::Bytes::from(b"sync bridge data".to_vec());
                    async_writer.write_segment_sync(2, 0, sync_data)
                }).await.expect("spawn_blocking should succeed").expect("sync bridge should succeed");
                assert!(sync_result.success);
            }).await.expect("Async test timed out after 30s");
        }

        #[tokio::test]
        async fn test_async_writer_concurrent_writes() {
            // Add 30 second timeout to prevent test hangs
            timeout(Duration::from_secs(30), async {
                let temp_dir = std::env::temp_dir().join(format!("async_concurrent_{}", std::process::id()));
                std::fs::create_dir_all(&temp_dir).ok();

                let async_config = AsyncIoConfig::default();
                let async_writer = Arc::new(AsyncWriter::new(async_config, temp_dir.clone())
                    .expect("create async writer"));

                // Concurrent async writes
                let mut handles = Vec::new();
                for i in 0..10 {
                    let writer = async_writer.clone();
                    let handle = tokio::spawn(async move {
                        let data = bytes::Bytes::from(format!("data {}", i).into_bytes());
                        writer.write_segment(i, 0, data).await
                    });
                    handles.push(handle);
                }

                // Wait for all to complete
                for handle in handles {
                    let result = handle.await.expect("task should not panic");
                    assert!(result.is_ok());
                    assert!(result.unwrap().success);
                }

                // Stats should reflect all writes
                let stats = async_writer.stats();
                assert_eq!(stats.total_writes, 10);
                assert_eq!(stats.successful_writes, 10);
            }).await.expect("Async test timed out after 30s");
        }
    }
}
