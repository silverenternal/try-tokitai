//! LifecycleManager - handles FileKV lifecycle operations
//!
//! Responsibilities:
//! - `open()` - Create/open FileKV storage
//! - `recover()` - WAL recovery
//! - `rebuild_bloom_filters()` - Rebuild bloom filters for all segments
//! - Checkpoint management
//! - Audit logging
//! - Metrics (Prometheus)
//! - Timeout configuration
//! - Feature flag initialization

use std::collections::BTreeMap;
use std::sync::Arc;

use tracing::{debug, info, warn};
use bloom::ASMS;

use crate::engine::EngineState;
use crate::core::segment::SegmentFile;
use crate::core::sparse_index::IndexManager;
use crate::cache::warmup::{CacheWarmer, CacheWarmingConfig};

/// Lifecycle manager for FileKV
pub struct LifecycleManager {
    pub state: Arc<EngineState>,
    /// Incremental checkpoint manager
    checkpoint_manager: parking_lot::Mutex<crate::checkpoint::IncrementalCheckpointManager>,
    /// Audit logger
    audit_logger: Option<Arc<crate::ops::audit_log::AuditLogger>>,
    /// Prometheus metrics (feature-gated)
    #[cfg(feature = "metrics")]
    metrics: Arc<crate::ops::metrics::FileKVMetrics>,
    /// Timeout configuration
    timeout_config: parking_lot::Mutex<crate::ops::timeout_control::TimeoutConfig>,
    /// Timeout statistics
    timeout_stats: parking_lot::Mutex<crate::ops::timeout_control::TimeoutStats>,
    /// Write coalescer (needed for open() initialization)
    write_coalescer: Option<Arc<crate::core::write_coalescer::WriteCoalescer>>,
    /// Flush trigger
    flush_trigger: crate::core::flush::FlushTrigger,
    /// Compaction manager
    compaction_manager: Arc<crate::compaction::CompactionManager>,
}

impl LifecycleManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: Arc<EngineState>,
        checkpoint_manager: parking_lot::Mutex<crate::checkpoint::IncrementalCheckpointManager>,
        audit_logger: Option<Arc<crate::ops::audit_log::AuditLogger>>,
        #[cfg(feature = "metrics")] metrics: Arc<crate::ops::metrics::FileKVMetrics>,
        timeout_config: crate::ops::timeout_control::TimeoutConfig,
        write_coalescer: Option<Arc<crate::core::write_coalescer::WriteCoalescer>>,
        flush_trigger: crate::core::flush::FlushTrigger,
        compaction_manager: Arc<crate::compaction::CompactionManager>,
    ) -> Self {
        Self {
            state,
            checkpoint_manager,
            audit_logger,
            #[cfg(feature = "metrics")]
            metrics,
            timeout_config: parking_lot::Mutex::new(timeout_config),
            timeout_stats: parking_lot::Mutex::new(crate::ops::timeout_control::TimeoutStats::default()),
            write_coalescer,
            flush_trigger,
            compaction_manager,
        }
    }

    /// Create or open FileKV storage
    ///
    /// This is the main initialization entry point. It:
    /// 1. Validates config and creates directories
    /// 2. Cleans up leftover temp files
    /// 3. Opens existing segment files
    /// 4. Loads indexes
    /// 5. Initializes caches and returns EngineState
    pub fn open(config: crate::core::types::FileKVConfig) -> anyhow::Result<Arc<EngineState>> {
        Self::validate_and_create_dirs(&config)?;
        Self::cleanup_temp_files(&config)?;
        let (segments, max_id) = Self::load_segments(&config)?;
        let index_manager = Self::load_indexes(&config)?;
        let engine_state = Self::initialize_caches(config, segments, max_id, index_manager)?;

        // Update stats
        {
            // Use atomic counters for segment stats
            engine_state.stats_state.stats.segment_count.store(
                engine_state.segment_state.segment_count.load(std::sync::atomic::Ordering::Relaxed),
                std::sync::atomic::Ordering::Relaxed,
            );
            engine_state.stats_state.stats.total_size_bytes.store(
                engine_state.segment_state.total_size_bytes.load(std::sync::atomic::Ordering::Relaxed),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        Ok(engine_state)
    }

    /// Validate configuration and create required directories
    fn validate_and_create_dirs(config: &crate::core::types::FileKVConfig) -> anyhow::Result<()> {
        let validation = config.validate();
        if !validation.errors.is_empty() {
            return Err(anyhow::anyhow!("Invalid config: {}", validation.errors[0]));
        }

        for warning in &validation.warnings {
            warn!("{}", warning);
        }

        config.fs.create_dir_all(&config.segment_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create segment dir: {}", e))?;

        if config.enable_wal {
            config.fs.create_dir_all(&config.wal_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create WAL dir: {}", e))?;
        }

        config.fs.create_dir_all(&config.index_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create index dir: {}", e))?;

        Ok(())
    }

    /// Clean up leftover temporary files from previous crashes
    fn cleanup_temp_files(config: &crate::core::types::FileKVConfig) -> anyhow::Result<()> {
        for path in config.fs.read_dir(&config.segment_dir)? {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with(".segment_") && name.ends_with(".log.tmp") {
                    tracing::warn!("Cleaning up leftover temp file: {}", path.display());
                    let _ = config.fs.remove_file(&path);
                }
            }
        }
        Ok(())
    }

    /// Load existing segment files from disk
    fn load_segments(
        config: &crate::core::types::FileKVConfig,
    ) -> anyhow::Result<(BTreeMap<u64, Arc<SegmentFile>>, u64)> {
        let mut segments = BTreeMap::new();
        let mut max_id = 0u64;

        for path in config.fs.read_dir(&config.segment_dir)? {
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(id_str) = name.strip_prefix("segment_") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            let level = 0u8;
                            let segment = SegmentFile::open(
                                config.fs.clone(),
                                id,
                                level,
                                &path,
                                config.aggressive.persistent_mmap_enabled,
                                config.aggressive.readahead_multiplier,
                                config.aggressive.dense_index_enabled,
                            )?;
                            max_id = max_id.max(id);
                            segments.insert(id, Arc::new(segment));
                        }
                    }
                }
            }
        }

        Ok((segments, max_id))
    }

    /// Load indexes from disk
    fn load_indexes(config: &crate::core::types::FileKVConfig) -> anyhow::Result<IndexManager> {
        let mut index_manager = IndexManager::new(&config.index_dir)?;
        index_manager.load_all_indexes()?;
        Ok(index_manager)
    }

    /// Initialize caches and construct the EngineState
    fn initialize_caches(
        config: crate::core::types::FileKVConfig,
        segments: BTreeMap<u64, Arc<SegmentFile>>,
        max_id: u64,
        index_manager: IndexManager,
    ) -> anyhow::Result<Arc<EngineState>> {
        // Configure BlockCache
        let mut cache_config = config.cache.clone();
        if config.aggressive.cache_max_memory_bytes > 0 {
            cache_config.max_memory_bytes = config.aggressive.cache_max_memory_bytes as u64;
            cache_config.max_items = std::cmp::max(
                cache_config.max_items,
                config.aggressive.cache_max_memory_bytes / 4096,
            );
        }
        let block_cache = Arc::new(crate::cache::block_cache::BlockCache::new(cache_config));
        let memtable = Arc::new(crate::core::memtable::MemTable::new(config.memtable.clone()));
        let _compaction_manager = Arc::new(crate::compaction::CompactionManager::new(config.compaction.clone()));

        let bloom_filter_cache = Arc::new(crate::bloom::filter_cache::BloomFilterCache::new(
            crate::bloom::filter_cache::BloomFilterCacheConfig::default(),
            config.index_dir.clone(),
        ));

        let stats = Arc::new(crate::core::types::FileKVStats::default());

        // Build global key index from existing segments (V0.6.0: enables O(log n) lookups)
        let global_index = Arc::new(crate::core::global_index::GlobalKeyIndex::new());
        if !segments.is_empty() {
            global_index.rebuild_from_segments(&segments)?;
            debug!("Global key index rebuilt with {} keys from {} segments", global_index.len(), segments.len());
        }

        // ENG-007: Use builder pattern instead of 10+ parameter constructor
        let engine_state = Arc::new(EngineState::builder(config)
            .segments(segments)
            .next_segment_id(max_id + 1)
            .index_manager(index_manager)
            .stats(stats)
            .memtable(memtable)
            .bloom_filter_cache(bloom_filter_cache)
            .block_cache(block_cache)
            .global_index(global_index)
            .build());

        Ok(engine_state)
    }

    /// Get configuration
    pub fn get_config(&self) -> &crate::core::types::FileKVConfig {
        &self.state.config
    }

    /// Get timeout configuration
    pub fn get_timeout_config(&self) -> parking_lot::MutexGuard<'_, crate::ops::timeout_control::TimeoutConfig> {
        self.timeout_config.lock()
    }

    /// Set timeout configuration
    pub fn set_timeout_config(&self, config: crate::ops::timeout_control::TimeoutConfig) {
        *self.timeout_config.lock() = config;
    }

    /// Get timeout statistics snapshot
    pub fn get_timeout_stats(&self) -> crate::ops::timeout_control::TimeoutStats {
        self.timeout_stats.lock().clone()
    }

    /// Reset timeout statistics
    pub fn reset_timeout_stats(&self) {
        *self.timeout_stats.lock() = crate::ops::timeout_control::TimeoutStats::default();
    }

    /// Get segments reference
    pub fn segments(&self) -> &arc_swap::ArcSwap<BTreeMap<u64, Arc<SegmentFile>>> {
        &self.state.segment_state.segments
    }

    /// Rebuild bloom filters for all segments
    pub fn rebuild_bloom_filters(&self) -> anyhow::Result<usize> {
        let segments = self.state.segment_state.segments.load();
        let mut rebuilt = 0usize;

        for (&segment_id, segment) in segments.iter() {
            let bloom_path = self.state.config.index_dir.join(format!("bloom_{:06}.bin", segment_id));

            // Collect all keys from segment
            let mut keys = Vec::new();
            segment.iterate_all(|key: &str, _value: &[u8], _deleted: bool| {
                keys.push(key.to_string());
                Ok(())
            })?;

            if keys.is_empty() {
                continue;
            }

            // Create bloom filter
            let mut bloom = crate::BloomFilter::with_rate(crate::DEFAULT_BLOOM_FPR, keys.len() as u32);
            for key in &keys {
                bloom.insert(key);
            }

            // Save atomically
            let temp_path = self.state.config.index_dir.join(format!(".bloom_{:06}.bin.tmp", segment_id));
            let mut file = self.state.config.fs.create_file(&temp_path)?;

            // Write header
            file.write_all(&crate::core::types::BLOOM_MAGIC.to_le_bytes())?;
            file.write_all(&crate::core::types::BLOOM_VERSION.to_le_bytes())?;
            file.write_all(&(keys.len() as u64).to_le_bytes())?;

            // Write keys
            for key in &keys {
                let key_bytes = key.as_bytes();
                file.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
                file.write_all(key_bytes)?;
            }

            file.flush()?;
            file.sync_all()?;
            drop(file);

            self.state.config.fs.rename(&temp_path, &bloom_path)?;

            // Insert into bloom filter cache (without keys list)
            self.state.cache_state.bloom_filter_cache.insert(segment_id, bloom);

            rebuilt += 1;
        }

        if rebuilt > 0 {
            info!("Rebuilt {} bloom filters", rebuilt);
        }

        Ok(rebuilt)
    }

    /// WAL recovery - unified recovery entry point
    ///
    /// GAP-M1 FIX: Migrated actual recovery logic from recovery.rs into here.
    /// This is now the single source of truth for WAL recovery.
    /// FileKV::recover() should delegate to this method.
    ///
    /// T-004: Validates WAL integrity before reading entries.
    /// T-018: Validates WAL sequence number continuity during recovery.
    pub fn recover_from_wal(&self, wal: &parking_lot::Mutex<crate::core::wal::WalManager>) -> anyhow::Result<usize> {
        use crate::core::wal::WalOperation;
        use tracing::{info, warn, error};

        let mut wal_guard = wal.lock();

        // T-004: Validate WAL integrity before reading entries
        wal_guard.validate_wal_integrity()
            .map_err(|e| anyhow::anyhow!("WAL integrity check failed: {}", e))?;

        // T-018: Validate sequence number continuity and collect entries
        let entries = wal_guard.read_entries()?;
        let (entries, sequence_warnings) = Self::validate_wal_sequence_continuity(&entries);
        for warning in &sequence_warnings {
            warn!("{}", warning);
        }
        let total_count = entries.len();

        if total_count == 0 {
            return Ok(0);
        }

        info!("Starting WAL recovery with {} entries", total_count);

        let mut recovered_count = 0;
        let mut failed_count = 0;

        for (idx, entry) in entries.iter().enumerate() {
            match &entry.operation {
                WalOperation::Add { session: key, hash: _, layer: _ } => {
                    if let Some(payload) = &entry.payload {
                        // PERF-005 FIX: Parse binary payload format
                        // Format: [8 bytes length][8 bytes hash][value bytes]
                        if payload.len() < 16 {
                            warn!("WAL entry {} for key '{}' has payload too small, skipping", idx, key);
                            failed_count += 1;
                            continue;
                        }

                        let len_bytes: [u8; 8] = payload[0..8].try_into()
                            .map_err(|e| anyhow::anyhow!("WAL data corrupted: {}", e))?;
                        let value_len = u64::from_le_bytes(len_bytes) as usize;

                        // Check if we have enough data
                        if payload.len() < 16 + value_len {
                            warn!("WAL entry {} for key '{}' has truncated payload, skipping", idx, key);
                            failed_count += 1;
                            continue;
                        }

                        let value_bytes = &payload[16..16 + value_len];

                        // Empty value means delete (tombstone)
                        if value_len == 0 {
                            if self.state.memtable_state.memtable.delete(key).is_none() {
                                let _ = self.state.memtable_state.memtable.insert_tombstone(key.clone());
                            }
                            info!("Replayed Delete for key: {}", key);
                        } else {
                            let _ = self.state.memtable_state.memtable.insert(key.clone(), value_bytes);
                            info!("Replayed Add for key: {}", key);
                        }
                        recovered_count += 1;
                    } else {
                        warn!("WAL entry {} for key '{}' has no payload, skipping", idx, key);
                        failed_count += 1;
                    }
                }
                WalOperation::Delete { session: key, .. } => {
                    let _ = self.state.memtable_state.memtable.delete(key);
                    info!("Replayed Delete for key: {}", key);
                    recovered_count += 1;
                }
                WalOperation::BatchAdd { entries } => {
                    // Replay batch add atomically
                    let batch_entries: Vec<(String, Vec<u8>)> = entries
                        .iter()
                        .map(|batch_entry| {
                            (batch_entry.key.clone(), batch_entry.value.clone())
                        })
                        .collect();

                    if !batch_entries.is_empty() {
                        let (_, start_seq) = self.state.memtable_state.memtable.insert_batch(&batch_entries);
                        info!("Replayed BatchAdd for {} keys, starting seq={}", batch_entries.len(), start_seq);
                        recovered_count += batch_entries.len();
                    }
                }
            }
        }

        wal_guard.clear()?;

        // Report recovery statistics
        if failed_count > 0 {
            error!(
                "WAL recovery completed: {} total, {} recovered, {} failed",
                total_count, recovered_count, failed_count
            );
        } else {
            info!(
                "WAL recovery completed successfully: {} entries replayed",
                recovered_count
            );
        }

        Ok(recovered_count)
    }

    /// T-018: Validate WAL sequence number continuity and return valid entries with warnings
    ///
    /// This method checks that WAL entries have monotonically increasing sequence numbers.
    /// Out-of-order entries are skipped with a warning, which can indicate partial writes
    /// or corruption.
    fn validate_wal_sequence_continuity(
        entries: &[crate::core::wal::WalEntry],
    ) -> (Vec<crate::core::wal::WalEntry>, Vec<String>) {
        use tracing::warn;
        let mut valid_entries = Vec::with_capacity(entries.len());
        let mut warnings = Vec::new();
        let mut expected_seq: Option<u64> = None;

        for (idx, entry) in entries.iter().enumerate() {
            if let Some(prev_seq) = expected_seq {
                if entry.sequence_number != prev_seq + 1 {
                    warnings.push(format!(
                        "WAL entry {} has unexpected sequence_number={} (expected={}), possible gap or corruption - skipping",
                        idx, entry.sequence_number, prev_seq + 1
                    ));
                    warn!(
                        "WAL entry {} has unexpected sequence_number={} (expected={}), skipping",
                        idx, entry.sequence_number, prev_seq + 1
                    );
                    continue;
                }
            }
            expected_seq = Some(entry.sequence_number);
            valid_entries.push(entry.clone());
        }

        if !warnings.is_empty() {
            warn!(
                "WAL sequence validation: {} entries skipped due to sequence gaps",
                warnings.len()
            );
        }

        (valid_entries, warnings)
    }

    /// Cache warming - warm block cache from segments
    pub fn warm_cache(&self) -> anyhow::Result<()> {
        let segments = self.state.segment_state.segments.load();
        let segments_vec: Vec<Arc<SegmentFile>> = segments.values().cloned().collect();
        if segments_vec.is_empty() {
            return Ok(());
        }

        let cache_warmer = CacheWarmer::new(
            CacheWarmingConfig::default(),
            self.state.cache_state.block_cache.clone(),
        );
        let _ = cache_warmer.warm(&segments_vec);
        Ok(())
    }

    /// Get checkpoint manager reference
    pub fn checkpoint_manager(&self) -> &parking_lot::Mutex<crate::checkpoint::IncrementalCheckpointManager> {
        &self.checkpoint_manager
    }

    /// Get audit logger reference
    pub fn audit_logger(&self) -> Option<&Arc<crate::ops::audit_log::AuditLogger>> {
        self.audit_logger.as_ref()
    }

    /// Get Prometheus metrics (feature-gated)
    #[cfg(feature = "metrics")]
    pub fn metrics(&self) -> &Arc<crate::ops::metrics::FileKVMetrics> {
        &self.metrics
    }

    /// Get flush trigger reference
    pub fn flush_trigger(&self) -> &crate::core::flush::FlushTrigger {
        &self.flush_trigger
    }

    /// Get compaction manager reference
    pub fn compaction_manager(&self) -> &Arc<crate::compaction::CompactionManager> {
        &self.compaction_manager
    }

    /// Get write coalescer reference
    pub fn write_coalescer(&self) -> Option<&Arc<crate::core::write_coalescer::WriteCoalescer>> {
        self.write_coalescer.as_ref()
    }
}

// ============================================================================
// Phase 1: LifecycleManagerAPI trait implementation
// ============================================================================

impl crate::engine::traits::LifecycleManagerAPI for LifecycleManager {
    fn open(config: crate::core::types::FileKVConfig) -> anyhow::Result<Arc<crate::engine::EngineState>> {
        LifecycleManager::open(config)
    }

    fn recover_from_wal(&self, wal: &parking_lot::Mutex<crate::core::wal::WalManager>) -> anyhow::Result<usize> {
        LifecycleManager::recover_from_wal(self, wal)
    }

    fn rebuild_bloom_filters(&self) -> anyhow::Result<usize> {
        LifecycleManager::rebuild_bloom_filters(self)
    }

    fn warm_cache(&self) -> anyhow::Result<()> {
        LifecycleManager::warm_cache(self)
    }

    fn get_config(&self) -> &crate::core::types::FileKVConfig {
        LifecycleManager::get_config(self)
    }

    fn get_timeout_config(&self) -> parking_lot::MutexGuard<'_, crate::ops::timeout_control::TimeoutConfig> {
        LifecycleManager::get_timeout_config(self)
    }

    fn set_timeout_config(&self, config: crate::ops::timeout_control::TimeoutConfig) {
        LifecycleManager::set_timeout_config(self, config)
    }

    fn get_timeout_stats(&self) -> crate::ops::timeout_control::TimeoutStats {
        LifecycleManager::get_timeout_stats(self)
    }

    fn reset_timeout_stats(&self) {
        LifecycleManager::reset_timeout_stats(self)
    }

    fn checkpoint_manager(&self) -> &parking_lot::Mutex<crate::checkpoint::IncrementalCheckpointManager> {
        LifecycleManager::checkpoint_manager(self)
    }
}
