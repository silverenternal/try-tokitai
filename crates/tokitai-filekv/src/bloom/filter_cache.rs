//! Bloom Filter Cache module
//!
//! Implements on-demand loading of bloom filters with CLOCK eviction to reduce memory usage.
//!
//! # Features
//! - On-demand loading: Bloom filters are loaded only when accessed
//! - CLOCK eviction: Approximate LRU with O(1) lock-free access path
//! - Sharded design: 16 independent CLOCK shards to reduce contention
//! - Configurable cache size: Limit memory usage for bloom filter cache
//! - Thread-safe: Uses DashMap for concurrent access
//! - Statistics: Track cache hits, misses, and memory usage
//!
//! # Benefits over resident approach
//! - Reduced memory footprint for large datasets with many segments
//! - Faster startup time (no need to load all filters at startup)
//! - Automatic memory management with configurable limits

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

use super::custom_bloom::CustomBloom;
use crate::core::error::FileKVResult;
use ::bloom::{BloomFilter, ASMS};

/// OPT-002: Unified wrapper for bloom filters in filter cache
/// Supports both legacy ::bloom::BloomFilter and high-performance CustomBloom (V3 format).
pub enum FilterWrapper {
    /// Legacy bloom filter (V1/V2 format)
    Bloom(BloomFilter),
    /// Custom bloom filter (V3 format, uses deterministic XXH3 hashing)
    Custom(CustomBloom),
}

impl std::fmt::Debug for FilterWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterWrapper::Bloom(_) => f.debug_tuple("Bloom").field(&"<bloom::BloomFilter>").finish(),
            FilterWrapper::Custom(cb) => f.debug_tuple("Custom").field(cb).finish(),
        }
    }
}

impl FilterWrapper {
    /// Check if a key might be in the filter
    pub fn contains(&self, key: &str) -> bool {
        match self {
            FilterWrapper::Bloom(bf) => bf.contains(&key.to_string()),
            FilterWrapper::Custom(cb) => cb.contains(key.as_bytes()),
        }
    }

    /// Estimate memory size for the wrapped filter
    pub fn estimate_memory_size(&self) -> usize {
        match self {
            FilterWrapper::Bloom(bf) => {
                let num_bits = bf.num_bits();
                let bitvec_bytes = num_bits.div_ceil(8);
                let bitvec_aligned = (bitvec_bytes + 7) & !7;
                bitvec_aligned + 64
            }
            FilterWrapper::Custom(cb) => cb.memory_usage(),
        }
    }
}

/// CLOCK algorithm cache entry
///
/// Each entry has a reference bit that is set atomically on access.
/// During eviction, the CLOCK algorithm scans entries and clears reference bits.
struct ClockEntry {
    key: u64,
    reference_bit: AtomicBool,
}

impl ClockEntry {
    fn new(key: u64) -> Self {
        Self {
            key,
            reference_bit: AtomicBool::new(true), // New entries start as referenced
        }
    }

    /// Set reference bit on access (lock-free atomic operation)
    fn set_referenced(&self) {
        self.reference_bit.store(true, Ordering::Relaxed);
    }

    /// Test and clear reference bit (used during CLOCK scan)
    /// Returns true if the bit was set (entry was referenced)
    fn test_and_clear(&self) -> bool {
        self.reference_bit.swap(false, Ordering::Relaxed)
    }
}

/// Single shard of the CLOCK cache
///
/// Uses parking_lot::RwLock to allow concurrent get() operations (read lock)
/// while insert/evict/remove operations acquire exclusive write lock.
///
/// T-005 Optimization: The fast path (get/set_referenced) only needs to set
/// an atomic reference bit, but we still need the read lock to find the entry.
/// A more aggressive optimization would use a separate DashMap for O(1) lookup,
/// but that adds memory overhead. Current design balances performance and memory.
struct ClockShard {
    entries: parking_lot::RwLock<Vec<Option<ClockEntry>>>,
    clock_hand: AtomicUsize,
    capacity: usize,
    count: AtomicUsize,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl ClockShard {
    fn new(capacity: usize) -> Self {
        Self {
            entries: parking_lot::RwLock::new((0..capacity).map(|_| None).collect()),
            clock_hand: AtomicUsize::new(0),
            capacity,
            count: AtomicUsize::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Check if a key exists in this shard. If found, sets reference bit.
    /// Uses read lock for concurrent access.
    fn get(&self, key: u64) -> bool {
        let entries = self.entries.read();
        for entry in entries.iter().flatten() {
            if entry.key == key {
                entry.set_referenced();
                self.hits.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        false
    }

    /// Insert a key. If full, evicts one entry first. Returns evicted key if any.
    /// Uses write lock for exclusive access.
    fn insert(&self, key: u64) -> Option<u64> {
        let mut entries = self.entries.write();

        // Check if we need to evict
        let current_count = self.count.load(Ordering::Relaxed);
        if current_count >= self.capacity {
            // Must evict
            return self.evict_one_internal(&mut entries, key);
        }

        // Find free slot and insert
        if let Some(slot) = entries.iter_mut().find(|s| s.is_none()) {
            *slot = Some(ClockEntry::new(key));
            self.count.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        // Should have been caught by the eviction check above
        self.evict_one_internal(&mut entries, key)
    }

    fn evict_one_internal(&self, entries: &mut [Option<ClockEntry>], key: u64) -> Option<u64> {
        let evicted = self.evict_one_locked(entries);
        // Insert in the freed slot
        if let Some(slot) = entries.iter_mut().find(|s| s.is_none()) {
            *slot = Some(ClockEntry::new(key));
            return evicted;
        }
        evicted
    }

    /// Evict one entry using CLOCK algorithm (must hold entries lock)
    fn evict_one_locked(&self, entries: &mut [Option<ClockEntry>]) -> Option<u64> {
        let mut scanned = 0;
        while scanned < self.capacity {
            let hand = self.clock_hand.load(Ordering::Relaxed);
            let idx = (hand + scanned) % self.capacity;

            if let Some(ref entry) = entries[idx] {
                if entry.test_and_clear() {
                    // Was referenced, skip
                    scanned += 1;
                    continue;
                } else {
                    // Not referenced, evict
                    let entry = entries[idx].take().unwrap();
                    self.count.fetch_sub(1, Ordering::Relaxed);
                    self.clock_hand.store((idx + 1) % self.capacity, Ordering::Relaxed);
                    return Some(entry.key);
                }
            } else {
                // Empty slot, skip
                scanned += 1;
            }
        }

        // Force evict from hand position if all were referenced
        let hand = self.clock_hand.load(Ordering::Relaxed);
        for i in 0..self.capacity {
            let idx = (hand + i) % self.capacity;
            if let Some(entry) = entries[idx].take() {
                self.count.fetch_sub(1, Ordering::Relaxed);
                self.clock_hand.store((idx + 1) % self.capacity, Ordering::Relaxed);
                return Some(entry.key);
            }
        }

        None
    }

    /// Evict one entry using CLOCK algorithm
    fn evict_one(&self) -> Option<u64> {
        let mut entries = self.entries.write();
        self.evict_one_locked(&mut entries)
    }

    /// Remove a specific key
    fn remove(&self, key: u64) -> bool {
        let mut entries = self.entries.write();
        for entry_opt in entries.iter_mut() {
            if let Some(entry) = entry_opt {
                if entry.key == key {
                    *entry_opt = None;
                    self.count.fetch_sub(1, Ordering::Relaxed);
                    return true;
                }
            }
        }
        false
    }

    /// Clear all entries
    fn clear(&self) {
        let mut entries = self.entries.write();
        for entry_opt in entries.iter_mut() {
            *entry_opt = None;
        }
        self.count.store(0, Ordering::Relaxed);
        self.clock_hand.store(0, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }
}

/// Sharded CLOCK cache for bloom filters
///
/// Uses 16 independent shards to reduce contention.
/// Each shard manages its own CLOCK queue with:
/// - RwLock-protected entries vector
/// - Atomic clock hand pointer
/// - Atomic hit/miss counters
struct ShardedClockCache {
    shards: Vec<ClockShard>,
    shard_mask: usize,
}

impl ShardedClockCache {
    /// Create a new sharded CLOCK cache
    /// capacity: total entries across all shards
    /// num_shards: must be power of 2
    fn new(capacity: usize, num_shards: usize) -> Self {
        debug_assert!(num_shards.is_power_of_two(), "num_shards must be power of 2");
        let shard_capacity = (capacity / num_shards).max(1);
        let shards = (0..num_shards).map(|_| ClockShard::new(shard_capacity)).collect();

        Self {
            shards,
            shard_mask: num_shards - 1,
        }
    }

    #[inline]
    fn shard_index(&self, key: u64) -> usize {
        (key as usize) & self.shard_mask
    }

    /// Check if key exists (sets reference bit on hit)
    fn get(&self, key: u64) -> bool {
        let idx = self.shard_index(key);
        self.shards[idx].get(key)
    }

    /// Insert key. Returns evicted key if any.
    fn insert(&self, key: u64) -> Option<u64> {
        let idx = self.shard_index(key);
        self.shards[idx].insert(key)
    }

    /// Remove a specific key
    fn remove(&self, key: u64) -> bool {
        let idx = self.shard_index(key);
        self.shards[idx].remove(key)
    }

    /// Clear all entries
    fn clear(&self) {
        for shard in &self.shards {
            shard.clear();
        }
    }

    /// Evict one entry from any shard. Returns the evicted key if any.
    fn evict_one(&self) -> Option<u64> {
        for shard in &self.shards {
            if let Some(evicted) = shard.evict_one() {
                return Some(evicted);
            }
        }
        None
    }
}

/// Bloom filter cache configuration
#[derive(Debug, Clone)]
pub struct BloomFilterCacheConfig {
    /// Maximum number of bloom filters to cache
    pub max_filters: usize,
    /// Maximum memory usage for bloom filter cache (bytes)
    pub max_memory_bytes: usize,
    /// Enable on-demand loading (if false, all filters are loaded at startup)
    pub on_demand_enabled: bool,
}

impl Default for BloomFilterCacheConfig {
    fn default() -> Self {
        Self {
            max_filters: 1000, // Cache up to 1000 filters (optimized for large datasets with many segments)
            max_memory_bytes: 256 * 1024 * 1024, // 256MB max (increased from 64MB to reduce cache evictions)
            on_demand_enabled: true,
        }
    }
}

/// Statistics for bloom filter cache
#[derive(Debug, Clone, Default)]
pub struct BloomFilterCacheStats {
    /// Cache hits (filter found in cache)
    pub hits: u64,
    /// Cache misses (filter had to be loaded)
    pub misses: u64,
    /// Hit rate (0.0-1.0)
    pub hit_rate: f64,
    /// Number of filters currently in cache
    pub filters_cached: usize,
    /// Memory used by cached filters (bytes)
    pub memory_used: usize,
    /// Number of filters evicted
    pub evictions: u64,
    /// Number of filters loaded from disk
    pub loads: u64,
}

impl BloomFilterCacheStats {
    /// Get hit rate as percentage
    pub fn hit_rate_percent(&self) -> f64 {
        self.hit_rate * 100.0
    }

    /// Get memory used in MB
    pub fn memory_used_mb(&self) -> f64 {
        self.memory_used as f64 / (1024.0 * 1024.0)
    }

    /// Get memory used in KB
    pub fn memory_used_kb(&self) -> f64 {
        self.memory_used as f64 / 1024.0
    }
}

/// Cached bloom filter with metadata (wrapped in Arc for sharing)
///
/// OPT-002: Uses FilterWrapper to support both legacy BloomFilter and CustomBloom.
struct CachedBloomFilter {
    /// The bloom filter (wrapped in FilterWrapper for unified interface)
    filter: Arc<FilterWrapper>,
    /// Estimated memory size of the filter (bytes)
    memory_size: usize,
}

impl CachedBloomFilter {
    fn new(filter: FilterWrapper) -> Self {
        let memory_size = filter.estimate_memory_size();

        Self {
            filter: Arc::new(filter),
            memory_size,
        }
    }
}

/// Bloom Filter Cache with on-demand loading and CLOCK eviction
pub struct BloomFilterCache {
    /// Cache of loaded bloom filters
    cache: DashMap<u64, CachedBloomFilter>,
    /// CLOCK queue for eviction tracking
    clock_queue: Arc<ShardedClockCache>,
    /// Configuration
    config: BloomFilterCacheConfig,
    /// Dynamic max memory limit (can be adjusted at runtime via grow_max_memory/shrink_to_memory).
    /// When None, uses config.max_memory_bytes.
    dynamic_max_memory_bytes: parking_lot::Mutex<Option<usize>>,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    loads: AtomicU64,
    memory_used: AtomicUsize,
}

impl BloomFilterCache {
    /// Create a new bloom filter cache
    pub fn new(config: BloomFilterCacheConfig, _index_dir: PathBuf) -> Self {
        const NUM_SHARDS: usize = 16;
        let clock_queue = Arc::new(ShardedClockCache::new(config.max_filters, NUM_SHARDS));

        Self {
            cache: DashMap::new(),
            clock_queue,
            config,
            dynamic_max_memory_bytes: parking_lot::Mutex::new(None),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            loads: AtomicU64::new(0),
            memory_used: AtomicUsize::new(0),
        }
    }

    /// Get a bloom filter for a segment (loads on-demand if not cached)
    ///
    /// OPT-002: Returns Arc<FilterWrapper> to support both legacy BloomFilter
    /// and high-performance CustomBloom (V3 format).
    pub fn get(
        &self,
        segment_id: u64,
        loader: &dyn Fn(u64) -> FileKVResult<Option<FilterWrapper>>,
    ) -> FileKVResult<Option<Arc<FilterWrapper>>> {
        // Check if filter is already cached
        if let Some(cached) = self.cache.get(&segment_id) {
            self.hits.fetch_add(1, Ordering::Relaxed);

            // Mark as referenced in CLOCK queue (approximate LRU)
            self.clock_queue.get(segment_id);

            return Ok(Some(cached.filter.clone()));
        }

        // Filter not in cache, load on-demand
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.loads.fetch_add(1, Ordering::Relaxed);

        // Use loader to load the filter
        match loader(segment_id)? {
            Some(filter) => {
                // Cache the loaded filter
                self.cache_and_promote(segment_id, filter);
                // Get the cached filter and return Arc
                if let Some(cached) = self.cache.get(&segment_id) {
                    Ok(Some(cached.filter.clone()))
                } else {
                    Ok(None)
                }
            }
            None => {
                // Filter doesn't exist on disk
                Ok(None)
            }
        }
    }

    /// Insert a bloom filter into the cache
    ///
    /// OPT-002: Accepts FilterWrapper to support both legacy and V3 formats.
    pub fn insert(&self, segment_id: u64, filter: FilterWrapper) {
        self.cache_and_promote(segment_id, filter);
    }

    /// Check if a key exists in a segment's bloom filter (convenience method)
    pub fn contains(
        &self,
        segment_id: u64,
        key: &str,
        loader: &dyn Fn(u64) -> FileKVResult<Option<FilterWrapper>>,
    ) -> FileKVResult<Option<bool>> {
        match self.get(segment_id, loader)? {
            Some(filter) => Ok(Some(filter.contains(key))),
            None => Ok(None),
        }
    }

    /// Remove a bloom filter from the cache
    pub fn remove(&self, segment_id: u64) -> Option<Arc<FilterWrapper>> {
        if let Some((_, cached)) = self.cache.remove(&segment_id) {
            self.memory_used.fetch_sub(cached.memory_size, Ordering::Relaxed);

            // Remove from CLOCK queue
            self.clock_queue.remove(segment_id);

            Some(cached.filter)
        } else {
            None
        }
    }

    /// Clear all cached filters
    pub fn clear(&self) {
        self.cache.clear();
        self.clock_queue.clear();
        self.memory_used.store(0, Ordering::Relaxed);
    }

    /// Get cache statistics
    pub fn stats(&self) -> BloomFilterCacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let filters_cached = self.cache.len();
        let memory_used = self.memory_used.load(Ordering::Relaxed);
        let evictions = self.evictions.load(Ordering::Relaxed);
        let loads = self.loads.load(Ordering::Relaxed);

        BloomFilterCacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            filters_cached,
            memory_used,
            evictions,
            loads,
        }
    }

    /// Get number of cached filters
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Cache a filter and update CLOCK queue (internal helper)
    ///
    /// OPT-002: Accepts FilterWrapper to support both legacy and V3 formats.
    fn cache_and_promote(&self, segment_id: u64, filter: FilterWrapper) {
        let cached = CachedBloomFilter::new(filter);
        let memory_delta = cached.memory_size;

        // Check memory limit and evict if necessary
        let max_memory = self.effective_max_memory_bytes();
        let current_memory = self.memory_used.load(Ordering::Relaxed);
        if current_memory + memory_delta > max_memory {
            self.evict_to_fit(memory_delta);
        }

        // Insert into cache
        if let Some(old_cached) = self.cache.insert(segment_id, cached) {
            self.memory_used.fetch_sub(old_cached.memory_size, Ordering::Relaxed);
        }

        self.memory_used.fetch_add(memory_delta, Ordering::Relaxed);

        // Insert into CLOCK queue (may trigger eviction)
        if let Some(evicted_id) = self.clock_queue.insert(segment_id) {
            // Remove evicted entry from cache if it's still there
            if let Some((_, evicted)) = self.cache.remove(&evicted_id) {
                self.memory_used.fetch_sub(evicted.memory_size, Ordering::Relaxed);
                self.evictions.fetch_add(1, Ordering::Relaxed);
                debug!("Evicted bloom filter for segment {} (CLOCK eviction)", evicted_id);
            }
        }
    }

    /// Get the effective max memory limit (dynamic if set, otherwise from config)
    fn effective_max_memory_bytes(&self) -> usize {
        let guard = self.dynamic_max_memory_bytes.lock();
        guard.unwrap_or(self.config.max_memory_bytes)
    }

    /// Evict filters to make room for new data using CLOCK algorithm
    fn evict_to_fit(&self, needed_memory: usize) {
        let max_memory = self.effective_max_memory_bytes();
        let target_memory = max_memory.saturating_sub(needed_memory);

        while self.memory_used.load(Ordering::Relaxed) > target_memory {
            if let Some(evict_id) = self.clock_queue.evict_one() {
                if let Some((_, cached)) = self.cache.remove(&evict_id) {
                    self.memory_used.fetch_sub(cached.memory_size, Ordering::Relaxed);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    debug!("Evicted bloom filter for segment {} (CLOCK eviction)", evict_id);
                }
            } else {
                // No more entries to evict
                break;
            }
        }
    }

    /// Shrink the bloom cache to fit within a target memory limit.
    /// Evicts CLOCK entries until current memory usage is at or below `target_memory_bytes`.
    /// Returns the number of entries evicted.
    pub fn shrink_to_memory(&self, target_memory_bytes: usize) -> u64 {
        let before_evictions = self.evictions.load(Ordering::Relaxed);
        let current_memory = self.memory_used.load(Ordering::Relaxed);

        if current_memory <= target_memory_bytes {
            debug!(
                "BloomFilterCache: already within target memory ({} <= {}), no eviction needed",
                current_memory, target_memory_bytes
            );
            return 0;
        }

        while self.memory_used.load(Ordering::Relaxed) > target_memory_bytes {
            if let Some(evict_id) = self.clock_queue.evict_one() {
                if let Some((_, cached)) = self.cache.remove(&evict_id) {
                    self.memory_used.fetch_sub(cached.memory_size, Ordering::Relaxed);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    debug!("Evicted bloom filter for segment {} (rebalance shrink)", evict_id);
                }
            } else {
                break;
            }
        }

        let evicted = self.evictions.load(Ordering::Relaxed) - before_evictions;
        let after_memory = self.memory_used.load(Ordering::Relaxed);
        info!(
            "BloomFilterCache: shrunk from {}KB to {}KB, evicted {} entries",
            current_memory / 1024,
            after_memory / 1024,
            evicted
        );
        evicted
    }

    /// Increase the dynamic max memory limit for the bloom cache.
    /// This allows more filters to be cached before eviction kicks in.
    /// Returns the previous effective max memory limit.
    pub fn grow_max_memory(&self, new_max_memory_bytes: usize) -> usize {
        let prev = self.effective_max_memory_bytes();
        *self.dynamic_max_memory_bytes.lock() = Some(new_max_memory_bytes);
        info!(
            "BloomFilterCache: max memory limit increased from {}KB to {}KB (advisory)",
            prev / 1024,
            new_max_memory_bytes / 1024
        );
        prev
    }
}

/// Helper to load a bloom filter from disk
///
/// OPT-002: Returns FilterWrapper to support both legacy BloomFilter and CustomBloom.
pub fn load_bloom_filter_from_disk(index_dir: &Path, segment_id: u64) -> FileKVResult<Option<FilterWrapper>> {
    use super::migration::{BloomFilterMigrator, MigrationResult};
    use tracing::{info, warn};

    let migrator = BloomFilterMigrator::new(index_dir.to_path_buf());

    match migrator.load_with_migration(segment_id) {
        Ok(Some((bloom, _keys, migration_result))) => {
            match migration_result {
                MigrationResult::Migrated {
                    from_version,
                    to_version,
                } => {
                    info!(
                        "Migrated bloom filter for segment {} from v{} to v{}",
                        segment_id, from_version, to_version
                    );
                }
                MigrationResult::UnsupportedVersion { version } => {
                    warn!(
                        "Bloom filter for segment {} has unsupported version {}, skipping",
                        segment_id, version
                    );
                    return Ok(None);
                }
                MigrationResult::FutureVersion { version } => {
                    warn!(
                        "Bloom filter for segment {} has future version {}, may have compatibility issues",
                        segment_id, version
                    );
                }
                MigrationResult::NoMigrationNeeded => {}
            }
            Ok(Some(FilterWrapper::Bloom(bloom)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(crate::core::error::FileKVError::from(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_BLOOM_FPR;
    use tempfile::TempDir;

    #[test]
    fn test_bloom_filter_cache_config_default() {
        let config = BloomFilterCacheConfig::default();
        assert_eq!(config.max_filters, 1000);
        assert_eq!(config.max_memory_bytes, 256 * 1024 * 1024);
        assert!(config.on_demand_enabled);
    }

    #[test]
    fn test_bloom_filter_cache_basic() {
        let temp_dir = TempDir::new().unwrap();
        let config = BloomFilterCacheConfig::default();
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        // Create a test bloom filter
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test_key".to_string());

        // Insert into cache
        cache.insert(1, FilterWrapper::Bloom(filter));

        // Retrieve from cache
        let loader = |_id: u64| -> FileKVResult<Option<FilterWrapper>> { Ok(None) };
        let cached = cache.get(1, &loader).unwrap();
        assert!(cached.is_some());
        assert!(cached.unwrap().contains("test_key"));
    }

    #[test]
    fn test_bloom_filter_cache_on_demand() {
        let temp_dir = TempDir::new().unwrap();
        let config = BloomFilterCacheConfig::default();
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        // Simulate on-demand loading with a static response after first load
        let loader = |id: u64| -> FileKVResult<Option<FilterWrapper>> {
            if id == 1 {
                let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
                filter.insert(&"loaded_key".to_string());
                Ok(Some(FilterWrapper::Bloom(filter)))
            } else {
                Ok(None)
            }
        };

        // First access (cache miss, should load)
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("loaded_key"));

        // Second access (cache hit, should use cached)
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.loads, 1);
    }

    #[test]
    fn test_bloom_filter_cache_eviction() {
        let temp_dir = TempDir::new().unwrap();
        // Use a larger max_filters so sharding works properly (16 shards need at least 16 per shard for meaningful eviction)
        let config = BloomFilterCacheConfig {
            max_filters: 16,             // At least 1 per shard
            max_memory_bytes: 1024 * 16, // 16KB - each filter ~1KB, so about 16 filters max
            on_demand_enabled: true,
        };
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        // Insert many filters with sequential IDs (should distribute across shards and trigger eviction)
        for i in 1..=100 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            filter.insert(&format!("key_{}", i));
            cache.insert(i, FilterWrapper::Bloom(filter));
        }

        // Cache should have evicted some filters (CLOCK queue should have triggered eviction)
        let stats = cache.stats();
        assert!(stats.evictions > 0, "Should have evictions, got {}", stats.evictions);
        assert!(stats.filters_cached <= 16);
    }

    #[test]
    fn test_bloom_filter_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = BloomFilterCacheConfig::default();
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test".to_string());
        cache.insert(1, FilterWrapper::Bloom(filter));

        let loader = |_id: u64| -> FileKVResult<Option<FilterWrapper>> { Ok(None) };

        cache.get(1, &loader).unwrap(); // hit
        cache.get(1, &loader).unwrap(); // hit
        cache.get(2, &loader).unwrap(); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.66).abs() < 0.02);
    }
}
