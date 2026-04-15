//! ReadEngine - handles all read-path operations for FileKV
//!
//! Responsibilities:
//! - `get()` - KV lookup across memtable, block cache, segments
//! - Bloom filter lookups and loading
//! - Zone Map pruning
//! - Sequential prefetch
//! - Feature flag checks for INNO-001/INNO-002
//! - Memory tracking
//! - Bloom migration tracking

use std::sync::Arc;

use bytes::Bytes;
use parking_lot::RwLock;
use tracing::debug;
use bloom::ASMS;

use crate::engine::EngineState;
use crate::ops::feature_flag::FeatureFlag;
use crate::cache::block_cache::BlockCacheAsPrefetchCache;
use crate::cache::prefetch::SequentialPrefetcher;
use crate::bloom::migration::MigrationDecision;

// Phase 2: Re-export state types for direct use if needed
pub use crate::engine::state::{CacheState, MemTableState, SegmentState, IndexState, StatsState};

// Re-export for backward compatibility
pub use crate::engine::types::CacheLookupResult;

/// Read-only engine for KV lookup
pub struct ReadEngine {
    pub state: Arc<EngineState>,
    /// Feature flag controller for INNO-001/INNO-002
    feature_flag_controller: Arc<crate::ops::feature_flag::FeatureFlagController>,
    /// Range query pruner for Zone Map-based block pruning (INNO-002)
    range_query_pruner: Option<Arc<crate::query::pruner::RangeQueryPruner>>,
    /// Sequential prefetcher for range queries (INNO-002)
    sequential_prefetcher: Option<Arc<parking_lot::RwLock<SequentialPrefetcher<BlockCacheAsPrefetchCache>>>>,
    /// Memory tracker for monitoring
    memory_tracker: Arc<crate::ops::memory_tracker::MemoryTracker>,
    /// Bloom filter layer migration controller (INNO-001)
    bloom_migration_controller: Arc<crate::bloom::migration::MigrationController>,
    /// Dictionary compressor for decompression on read path (S2-1)
    compressor: Option<Arc<parking_lot::Mutex<crate::compression::dictionary::DictionaryCompressor>>>,
}

impl ReadEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: Arc<EngineState>,
        feature_flag_controller: Arc<crate::ops::feature_flag::FeatureFlagController>,
        range_query_pruner: Option<Arc<crate::query::pruner::RangeQueryPruner>>,
        sequential_prefetcher: Option<Arc<parking_lot::RwLock<SequentialPrefetcher<BlockCacheAsPrefetchCache>>>>,
        memory_tracker: Arc<crate::ops::memory_tracker::MemoryTracker>,
        bloom_migration_controller: Arc<crate::bloom::migration::MigrationController>,
        compressor: Option<Arc<parking_lot::Mutex<crate::compression::dictionary::DictionaryCompressor>>>,
    ) -> Self {
        Self {
            state,
            feature_flag_controller,
            range_query_pruner,
            sequential_prefetcher,
            memory_tracker,
            bloom_migration_controller,
            compressor,
        }
    }

    /// Check if INNO-002 Zone Map pruning is enabled at runtime
    pub fn is_zone_map_pruning_enabled(&self) -> bool {
        self.feature_flag_controller
            .is_enabled(FeatureFlag::Inno002ZoneMapPruning)
    }

    /// Check if INNO-002 Sequential Prefetch is enabled at runtime
    pub fn is_sequential_prefetch_enabled(&self) -> bool {
        self.feature_flag_controller
            .is_enabled(FeatureFlag::Inno002SequentialPrefetch)
    }

    /// Check if INNO-001 Adaptive Bloom Cache is enabled at runtime
    pub fn is_adaptive_bloom_cache_enabled(&self) -> bool {
        self.feature_flag_controller
            .is_enabled(FeatureFlag::Inno001AdaptiveBloomCache)
    }

    /// Get range query pruner reference
    pub fn get_range_query_pruner(&self) -> Option<&crate::query::pruner::RangeQueryPruner> {
        if !self.is_zone_map_pruning_enabled() {
            return None;
        }
        self.range_query_pruner.as_ref().map(|arc| arc.as_ref())
    }

    /// Get sequential prefetcher reference
    pub fn get_sequential_prefetcher(&self) -> Option<&Arc<RwLock<SequentialPrefetcher<BlockCacheAsPrefetchCache>>>> {
        if !self.is_sequential_prefetch_enabled() {
            return None;
        }
        self.sequential_prefetcher.as_ref()
    }

    /// Load bloom filter for a segment (supports v1, v2, and v3 formats)
    ///
    /// V3 format loads bit vector directly, eliminating rebuild overhead.
    /// V2 format includes num_bits and num_hashes metadata for faster reconstruction.
    pub fn load_bloom_filter(&self, segment_id: u64) -> anyhow::Result<Option<(crate::BloomFilter, Vec<String>)>> {
        let bloom_path = self.state.config.index_dir.join(format!("bloom_{:06}.bin", segment_id));

        if !self.state.config.fs.file_exists(&bloom_path) {
            return Ok(None);
        }

        let mut file = self.state.config.fs.open_file(&bloom_path, true, false, false)?;

        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;

        let magic = u32::from_le_bytes(header[0..4].try_into()?);
        if magic != crate::core::types::BLOOM_MAGIC {
            return Err(anyhow::anyhow!("Invalid bloom filter magic"));
        }

        let version = u32::from_le_bytes(header[4..8].try_into()?);

        // Read num_bits and num_hashes (present in v2+)
        let mut num_bits_buf = [0u8; 4];
        file.read_exact(&mut num_bits_buf)?;
        let num_bits = u32::from_le_bytes(num_bits_buf);

        let mut num_hashes_buf = [0u8; 4];
        file.read_exact(&mut num_hashes_buf)?;
        let num_hashes = u32::from_le_bytes(num_hashes_buf);

        // Parse based on version
        if version == 3 {
            // POL-003: V3 format - load bit vector directly (no rebuild needed)
            let mut bitvec_len_buf = [0u8; 4];
            file.read_exact(&mut bitvec_len_buf)?;
            let bitvec_len = u32::from_le_bytes(bitvec_len_buf) as usize;

            let mut bitvec_bytes = vec![0u8; bitvec_len];
            file.read_exact(&mut bitvec_bytes)?;

            // Reconstruct BloomFilter from bit vector
            let bf: crate::BloomFilter = crate::BloomFilter::from_bytes(num_bits as usize, num_hashes, &bitvec_bytes);
            
            // V3 doesn't store keys, return empty vec
            Ok(Some((bf, Vec::new())))
        } else if version == 2 {
            // V2 format: has num_keys and keys list
            let mut num_keys_buf = [0u8; 8];
            file.read_exact(&mut num_keys_buf)?;
            let num_keys = u64::from_le_bytes(num_keys_buf) as usize;

            let mut keys = Vec::with_capacity(num_keys);
            for _ in 0..num_keys {
                let mut key_len_buf = [0u8; 4];
                file.read_exact(&mut key_len_buf)?;
                let key_len = u32::from_le_bytes(key_len_buf) as usize;

                let mut key_bytes = vec![0u8; key_len];
                file.read_exact(&mut key_bytes)?;

                let key = String::from_utf8_lossy(&key_bytes).to_string();
                keys.push(key);
            }

            // V2 fast path: use stored metadata for faster construction
            let mut bf = crate::BloomFilter::with_size(num_bits as usize, num_hashes);
            for key in &keys {
                bf.insert(key);
            }
            Ok(Some((bf, keys)))
        } else if version == 1 {
            // V1 format: no metadata, just num_keys
            // For v1, the format is: [magic 4B][version 4B][num_keys 8B][keys...]
            // We already read num_bits and num_hashes which don't exist in v1
            // Need to re-read from after magic+version
            
            // Reopen file and read from start
            drop(file);
            let mut file = self.state.config.fs.open_file(&bloom_path, true, false, false)?;
            file.read_exact(&mut header)?; // skip magic+version
            
            let mut num_keys_buf = [0u8; 8];
            file.read_exact(&mut num_keys_buf)?;
            let num_keys = u64::from_le_bytes(num_keys_buf) as usize;

            let mut keys = Vec::with_capacity(num_keys);
            for _ in 0..num_keys {
                let mut key_len_buf = [0u8; 4];
                file.read_exact(&mut key_len_buf)?;
                let key_len = u32::from_le_bytes(key_len_buf) as usize;

                let mut key_bytes = vec![0u8; key_len];
                file.read_exact(&mut key_bytes)?;

                let key = String::from_utf8_lossy(&key_bytes).to_string();
                keys.push(key);
            }

            // V1 fallback: estimate capacity from keys
            let mut bf = crate::BloomFilter::with_rate(crate::DEFAULT_BLOOM_FPR, num_keys as u32);
            for key in &keys {
                bf.insert(key);
            }
            Ok(Some((bf, keys)))
        } else {
            Err(anyhow::anyhow!("Unsupported bloom filter version: {}", version))
        }
    }

    /// Read key-value pair
    ///
    /// Lookup order:
    /// 1. MemTable (fastest, in-memory)
    /// 2. Block Cache (O(1) DashMap lookup)
    /// 3. Global Key Index (O(log n) segment location) - V0.6.0
    /// 4. Segments with Bloom Filter + Zone Map pruning (fallback)
    /// 5. Sparse Index O(1) lookup (now using HashMap)
    pub fn get(&self, key: &str) -> anyhow::Result<(Option<Bytes>, CacheLookupResult)> {
        self.state.stats_state.stats.read_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Check MemTable first (fastest path)
        if let Some((value, _pointer, deleted)) = self.state.memtable_state.memtable.get(key) {
            if deleted {
                return Ok((None, CacheLookupResult::MemTableHit)); // Tombstone
            } else if let Some(v) = value {
                return Ok((Some(v), CacheLookupResult::MemTableHit));
            }
        }

        // FIX-001: Check prefetch cache before BlockCache
        // SequentialPrefetcher stores prefetched KV pairs with key format "prefetch:key:<key>"
        // This enables get() to benefit from range query prefetching
        if let Some(value) = self.state.cache_state.block_cache.get_prefetch(key) {
            self.state.stats_state.stats.prefetch_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok((Some(value), CacheLookupResult::BlockCacheHit));
        }

        // Check block cache (O(1) DashMap lookup)
        if let Some(cached) = self.state.cache_state.block_cache.get_by_key(key) {
            return Ok((Some(cached), CacheLookupResult::BlockCacheHit));
        }

        // V0.6.0: Try global key index for O(log n) segment lookup
        // This avoids traversing all L0 segments when the key is indexed
        if let Some(location) = self.state.global_index_state.global_index.get(key.as_bytes()) {
            debug!("Global index returned segment {} for key {}", location.segment_id, key);
            let segments_snapshot = self.state.segment_state.segments.load();
            if let Some(segment) = segments_snapshot.get(&location.segment_id) {
                // Get sparse index and zone map for this segment
                let (sparse_idx, zone_map) = {
                    let index_manager = self.state.index_state.index_manager.read();
                    (index_manager.get_index(segment.id), index_manager.get_zone_map(segment.id))
                };

                // Direct segment lookup using known location
                if let Some(value) = self.search_segment(
                    segment,
                    key,
                    sparse_idx.as_ref().map(|arc| arc.as_ref()) as Option<&crate::core::sparse_index::SparseIndex>,
                    zone_map.as_ref() as Option<&crate::query::zone_map::ZoneMapIndex>,
                )? {
                    debug!("Found key {} in segment {} via global index", key, location.segment_id);
                    return Ok((Some(value), CacheLookupResult::DiskHit));
                }
                debug!("Key {} not found in segment {} (global index stale), falling through", key, location.segment_id);
                // If global index pointed to wrong segment (stale), fall through to full traversal
            }
        }

        // Fallback: Level-aware segment traversal (when global index misses or is stale)
        let segments_snapshot = self.state.segment_state.segments.load();

        // ENG-004: Clone only the data we need, then release lock before processing
        // Collect segment Arcs, SparseIndex (Arc clone), and ZoneMapIndex from index_manager
        #[allow(clippy::type_complexity)]
        let segment_data: Vec<(
            Arc<crate::core::segment::SegmentFile>,
            Option<std::sync::Arc<crate::core::sparse_index::SparseIndex>>,
            Option<crate::query::zone_map::ZoneMapIndex>,
        )> = {
            let index_manager = self.state.index_state.index_manager.read();
            segments_snapshot
                .values()
                .map(|seg| {
                    let idx = index_manager.get_index(seg.id);
                    let zm = index_manager.get_zone_map(seg.id);
                    (Arc::clone(seg), idx, zm)
                })
                .collect()
        }; // index_manager read lock released here

        // Group segments by level
        let mut by_level: std::collections::BTreeMap<u8, Vec<usize>> = std::collections::BTreeMap::new();
        for (i, (segment, _, _)) in segment_data.iter().enumerate() {
            by_level.entry(segment.level).or_default().push(i);
        }

        // L0: Search newest to oldest (key ranges may overlap, must check all)
        if let Some(l0_indices) = by_level.get(&0) {
            let mut l0_sorted: Vec<_> = l0_indices.clone();
            l0_sorted.sort_by(|&a, &b| segment_data[b].0.id.cmp(&segment_data[a].0.id));  // Newest first

            for &idx in &l0_sorted {
                let (segment, sparse_idx, zone_map) = &segment_data[idx];
                if let Some(value) = self.search_segment(
                    segment,
                    key,
                    sparse_idx.as_ref().map(|arc| arc.as_ref()) as Option<&crate::core::sparse_index::SparseIndex>,
                    zone_map.as_ref() as Option<&crate::query::zone_map::ZoneMapIndex>,
                )? {
                    return Ok((Some(value), CacheLookupResult::DiskHit));
                }
            }
        }

        // L1+: Search by level order, within each level use key range to find target segment
        for level in 1..=3 {  // L1, L2, L3
            if let Some(level_indices) = by_level.get(&level) {
                // L1+ segments have non-overlapping key ranges, find the one that might contain key
                for &idx in level_indices {
                    let (segment, sparse_idx, zone_map) = &segment_data[idx];

                    // Use min_key/max_key for fast range check
                    // Use inner scope to automatically drop MutexGuards after check
                    let key_in_range = {
                        let min_key = segment.min_key.lock();
                        let max_key = segment.max_key.lock();
                        if let (Some(ref min_key), Some(ref max_key)) = (&*min_key, &*max_key) {
                            key >= min_key.as_str() && key <= max_key.as_str()
                        } else {
                            true  // No key range set, search segment
                        }
                    };  // min_key and max_key MutexGuards automatically dropped here

                    if !key_in_range {
                        continue;  // Key out of range, skip this segment
                    }

                    // Key in range, search segment (pass pre-fetched index and zone_map)
                    if let Some(value) = self.search_segment(
                        segment,
                        key,
                        sparse_idx.as_ref().map(|arc| arc.as_ref()) as Option<&crate::core::sparse_index::SparseIndex>,
                        zone_map.as_ref() as Option<&crate::query::zone_map::ZoneMapIndex>,
                    )? {
                        return Ok((Some(value), CacheLookupResult::DiskHit));
                    }
                }
            }
        }

        Ok((None, CacheLookupResult::CacheMiss))
    }

    /// Search a single segment for a key
    /// Extracted from get() to support level-aware reading
    /// ENG-004: Accept pre-fetched sparse_index and zone_map to avoid holding index_manager lock
    ///
    /// POL-004 OPTIMIZATION: Dense index fast path avoids expensive bloom/zone map overhead.
    /// When dense index is enabled and can definitively answer, we skip all bloom filter
    /// loading and zone map checks, reducing get() latency by 20%+.
    fn search_segment(
        &self,
        segment: &Arc<crate::core::segment::SegmentFile>,
        key: &str,
        sparse_index: Option<&crate::core::sparse_index::SparseIndex>,
        zone_map: Option<&crate::query::zone_map::ZoneMapIndex>,
    ) -> anyhow::Result<Option<bytes::Bytes>> {
        use tracing::debug;

        // POL-004: Try dense index fast path FIRST (before expensive bloom/zone map checks)
        // If dense index can definitively answer that key EXISTS, we skip all subsequent overhead.
        // However, if dense index says key doesn't exist, we still continue to bloom/zone map path
        // as a safety measure (dense index may be incomplete or stale).
        if let Some(key_exists) = segment.key_might_exist_in_dense_index(key) {
            if key_exists {
                // Key exists in dense index, read directly (skip bloom/zone map overhead)
                if let Some(raw_value) = segment.get_by_key(key)? {
                    let value_bytes = if let Some(ref compressor) = self.compressor {
                        let compressor_guard = compressor.lock();
                        match compressor_guard.decompress(&raw_value) {
                            Ok(decompressed) => Bytes::from(decompressed),
                            Err(e) => {
                                tracing::warn!("Dictionary decompression failed for key '{}': {}, using raw value", key, e);
                                Bytes::from(raw_value)
                            }
                        }
                    } else {
                        Bytes::from(raw_value)
                    };

                    self.state.cache_state.block_cache.insert_by_key(key.to_string(), value_bytes.clone());
                    self.record_sequential_access(key, segment.id);
                    return Ok(Some(value_bytes));
                }
                // Dense index said key exists but read failed (stale index), fall through to bloom path
            }
            // If dense index says key doesn't exist, we continue to bloom/zone map path as safety measure
        }

        // Record I/O operation for segment lookup (only if we didn't take the dense index fast path)
        self.state.stats_state.stats.read_io_operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Record segment access for bloom filter layer migration and process migration decisions
        if let Some(decision) = self.bloom_migration_controller.record_access(segment.id) {
            if let Some(adaptive_cache) = &self.state.cache_state.adaptive_bloom_cache {
                match decision {
                    MigrationDecision::UpgradeToL1 => {
                        tracing::debug!("Bloom migration: segment {} upgrade to L1 (hot)", segment.id);
                        adaptive_cache.migrate_l2_to_l1(segment.id);
                    }
                    MigrationDecision::UpgradeToL2 => {
                        tracing::debug!("Bloom migration: segment {} upgrade to L2 (warm)", segment.id);
                    }
                    MigrationDecision::DowngradeToL2 => {
                        tracing::debug!("Bloom migration: segment {} downgrade to L2 (cooldown)", segment.id);
                        adaptive_cache.migrate_l1_to_l2(segment.id);
                    }
                    MigrationDecision::DowngradeToL3 => {
                        tracing::debug!("Bloom migration: segment {} downgrade to L3 (evict)", segment.id);
                        adaptive_cache.remove(segment.id);
                    }
                    MigrationDecision::Stay => {}
                }
            }
        }

        // BLOOM FILTER: Fast negative lookup (O(1), 99% accuracy)
        let bloom_found = if self.is_adaptive_bloom_cache_enabled() {
            if let Some(adaptive_cache) = &self.state.cache_state.adaptive_bloom_cache {
                match adaptive_cache.get(segment.id, &|sid| {
                    self.load_bloom_filter(sid)
                        .map_err(|e| crate::core::error::FileKVError::Fatal(crate::core::error::FatalError::Corruption(e.to_string())))
                }) {
                    Ok(Some(filter)) => {
                        let contains = filter.contains(&key);
                        if !contains {
                            None  // Bloom says key doesn't exist, skip segment
                        } else {
                            Some(true)  // Bloom says key might exist, continue
                        }
                    }
                    Ok(None) => Some(true),  // No bloom filter, continue
                    Err(e) => {
                        tracing::warn!("AdaptiveBloomCache error for segment {}: {}", segment.id, e);
                        Some(true)  // On error, continue
                    }
                }
            } else {
                Some(true)  // No adaptive cache, fall back to regular
            }
        } else {
            // 使用原来的 BloomFilterCache
            if let Some(bloom_result) = self.state.cache_state.bloom_filter_cache.get(segment.id, &|sid| {
                self.load_bloom_filter(sid)
                    .map(|opt| opt.map(|(bloom, _keys)| bloom))
                    .map_err(|e| crate::core::error::FileKVError::Fatal(crate::core::error::FatalError::Corruption(e.to_string())))
            })? {
                if !bloom_result.contains(&key) {
                    None  // Bloom says key doesn't exist, skip segment
                } else {
                    Some(true)  // Bloom says key might exist, continue
                }
            } else {
                Some(true)  // No bloom filter, continue
            }
        };

        if bloom_found.is_none() {
            return Ok(None);  // Bloom negative, skip segment
        }

        // Zone Map: Skip segments where key can't exist
        if let Some(index) = sparse_index {
            if !index.key_might_exist(key) {
                return Ok(None);
            }
        }

        // S1-3: RangeQueryPruner block-level pruning for point queries
        if self.is_zone_map_pruning_enabled() {
            if let Some(zm) = zone_map {
                let blocks_to_scan = if let Some(ref pruner) = self.range_query_pruner {
                    pruner.find_blocks_to_scan(zm, key, key)
                } else {
                    zm.find_overlapping_blocks(key, key)
                };

                if blocks_to_scan.is_empty() {
                    debug!(
                        "Zone Map block pruning (get): key='{}' not in any block of segment {}",
                        key, segment.id
                    );
                    return Ok(None);
                }
            }
        }

        // Fallback to sparse index + read_at with Zone Map block-level pruning
        // (Only reached if dense index fast path didn't apply or returned stale result)
        if let Some(index) = sparse_index {
            if let Some(pos) = index.find(key) {
                let should_read = if self.is_zone_map_pruning_enabled() {
                    if let Some(zm) = zone_map {
                        let blocks_to_scan = if let Some(ref pruner) = self.range_query_pruner {
                            pruner.find_blocks_to_scan(zm, key, key)
                        } else {
                            zm.find_overlapping_blocks(key, key)
                        };

                        if blocks_to_scan.is_empty() {
                            debug!(
                                "Zone Map block pruning: key='{}' not in any block of segment {}",
                                key, segment.id
                            );
                            false
                        } else {
                            let offset_in_block = blocks_to_scan.iter().any(|&block_id| {
                                zm.entries().iter().find(|e| e.block_id == block_id).is_some_and(|entry| {
                                    pos >= entry.offset && pos < entry.offset + entry.size_bytes as u64
                                })
                            });
                            offset_in_block
                        }
                    } else {
                        true
                    }
                } else {
                    true
                };

                if should_read {
                    if let Ok(value) = segment.read_at(pos, 0) {
                        if !value.is_empty() {
                            let value_bytes = bytes::Bytes::from(value);
                            self.state.cache_state.block_cache.insert_by_key(key.to_string(), value_bytes.clone());
                            self.record_sequential_access(key, segment.id);
                            return Ok(Some(value_bytes));
                        } else {
                            return Ok(None); // Tombstone
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// GAP-C4: Record key access for sequential prefetch detection
    ///
    /// When sequential access pattern is detected, triggers prefetching
    /// of subsequent blocks to improve cache hit rate for range queries.
    ///
    /// OPT-001: Minimize lock contention by checking conditions before acquiring locks.
    fn record_sequential_access(&self, key: &str, segment_id: u64) {
        // Check if sequential prefetch is enabled and prefetcher exists
        let Some(ref prefetcher) = self.sequential_prefetcher else {
            return;
        };

        // OPT-001: Only acquire lock when needed for dense index lookup
        let index_manager = self.state.index_state.index_manager.read();
        let Some(dense_index) = index_manager.all_dense_indexes().get(&segment_id) else {
            return;
        };
        let Some(entry) = dense_index.entries.get(key) else {
            return;
        };

        let block_id = entry.block_id;
        // Release locks before recording
        drop(index_manager);

        let mut prefetcher = prefetcher.write();
        // GAP-C4: Pass segment_id to record_access
        prefetcher.record_access(key, segment_id, block_id);
    }

    /// Get feature flag controller
    pub fn get_feature_flag_controller(&self) -> Arc<crate::ops::feature_flag::FeatureFlagController> {
        Arc::clone(&self.feature_flag_controller)
    }

    /// Enable INNO-002 (both Zone Map pruning and Sequential prefetch)
    pub fn enable_inno002(&self) {
        self.feature_flag_controller.enable_inno002();
        debug!("INNO-002 enabled via runtime feature flag");
    }

    /// Disable INNO-002 (both Zone Map pruning and Sequential prefetch)
    pub fn disable_inno002(&self) {
        self.feature_flag_controller.disable_inno002();
        debug!("INNO-002 disabled via runtime feature flag");
    }

    /// Enable INNO-001 Adaptive Bloom Cache
    pub fn enable_inno001(&self) {
        self.feature_flag_controller.enable_inno001();
        debug!("INNO-001 enabled via runtime feature flag");
    }

    /// Disable INNO-001 Adaptive Bloom Cache
    pub fn disable_inno001(&self) {
        self.feature_flag_controller.disable_inno001();
        debug!("INNO-001 disabled via runtime feature flag");
    }

    /// Get feature flag statistics
    pub fn get_feature_flag_stats(&self) -> crate::ops::feature_flag::FeatureFlagStats {
        self.feature_flag_controller.get_stats()
    }

    /// Generate feature flag report
    pub fn generate_feature_flag_report(&self) -> crate::ops::feature_flag::FeatureReport {
        self.feature_flag_controller.generate_report()
    }

    /// Get bloom migration stats
    pub fn get_bloom_migration_stats(&self) -> crate::bloom::migration::MigrationStats {
        self.bloom_migration_controller.stats()
    }

    /// 4.1 OPTIMIZATION: Get memory usage snapshot
    pub fn get_memory_usage(&self) -> crate::ops::memory_tracker::MemoryUsage {
        // Update memory tracker with current values
        let cache_bytes = self.state.cache_state.block_cache.memory_usage();
        self.memory_tracker.set_block_cache_bytes(cache_bytes);

        // Estimate memtable bytes
        let memtable_bytes = self.state.memtable_state.memtable.approximate_memory_bytes();
        self.memory_tracker.set_memtable_bytes(memtable_bytes);

        // Estimate dense index and mmap bytes from segments
        let segments = self.state.segment_state.segments.load();
        let dense_index_bytes: u64 = segments.values()
            .filter_map(|s| s.dense_index_memory_bytes())
            .sum();
        let mmap_bytes: u64 = segments.values()
            .filter(|s| s.use_persistent_mmap())
            .map(|s| s.size())
            .sum();

        self.memory_tracker.set_dense_index_bytes(dense_index_bytes);
        self.memory_tracker.set_mmap_bytes(mmap_bytes);

        self.memory_tracker.get_usage()
    }
}

// ============================================================================
// Phase 1: ReadEngineAPI trait implementation
// ============================================================================

impl crate::engine::traits::ReadEngineAPI for ReadEngine {
    fn get(&self, key: &str) -> anyhow::Result<(Option<Bytes>, crate::engine::traits::CacheLookupResult)> {
        let (value, cache_result) = ReadEngine::get(self, key)?;
        // Both now use the same shared CacheLookupResult type
        Ok((value, cache_result))
    }

    fn get_stats(&self) -> crate::engine::traits::ReadStats {
        let cache_hits = self.state.stats_state.stats.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        let cache_misses = self.state.stats_state.stats.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = cache_hits + cache_misses;
        let cache_hit_rate = if total > 0 {
            cache_hits as f64 / total as f64
        } else {
            0.0
        };

        crate::engine::traits::ReadStats {
            read_count: self.state.stats_state.stats.read_count.load(std::sync::atomic::Ordering::Relaxed),
            read_io_operations: self.state.stats_state.stats.read_io_operations.load(std::sync::atomic::Ordering::Relaxed),
            cache_hit_rate,
        }
    }

    fn get_memory_usage(&self) -> crate::ops::memory_tracker::MemoryUsage {
        ReadEngine::get_memory_usage(self)
    }
}
