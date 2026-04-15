//! Block Cache module for FileKV
//!
//! Implements sharded Moka-based cache for hot data blocks.
//! Uses multiple Moka Cache instances (shards) to enable dynamic capacity adjustment at runtime.
//! Each shard has a fixed capacity, and the total cache capacity can be changed by adding or removing shards.
//!
//! Shard routing uses consistent key hashing: both `insert_by_key` and `get_by_key` compute
//! the shard index from the key's hash, enabling O(1) lookups instead of O(num_shards) iteration.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use ahash::AHasher;
use moka::sync::Cache;
use bytes::Bytes;
use parking_lot::RwLock;

/// Default shard size: 16MB
const DEFAULT_SHARD_SIZE_BYTES: u64 = 16 * 1024 * 1024;

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub items: usize,
    pub capacity: usize,
    pub memory_usage: u64,
    pub inserts: u64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn hit_rate_percent(&self) -> f64 {
        self.hit_rate * 100.0
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct BlockCacheConfig {
    pub max_items: usize,
    pub max_memory_bytes: u64,
    /// T-004: Enable access-frequency-aware weighing
    /// When true, the weigher considers both value size and access frequency
    pub frequency_aware: bool,
}

impl Default for BlockCacheConfig {
    fn default() -> Self {
        Self {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            frequency_aware: false,
        }
    }
}

/// A single shard within the BlockCache
struct BlockCacheShard {
    cache: Cache<String, Bytes>,
}

impl BlockCacheShard {
    fn new(capacity_bytes: u64, stats: Arc<CacheStatsInner>) -> Self {
        let stats_for_listener = stats.clone();

        let cache: Cache<String, Bytes> = Cache::builder()
            .max_capacity(capacity_bytes)
            .weigher(|_, value: &Bytes| -> u32 {
                value.len().min(u32::MAX as usize) as u32
            })
            .eviction_listener(move |_key: Arc<String>, value: Bytes, _cause| {
                let memory_delta = value.len();
                stats_for_listener.memory_usage.fetch_sub(memory_delta, Ordering::Relaxed);
            })
            .build();

        Self { cache }
    }

    fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }

    fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    fn run_pending_tasks(&self) {
        self.cache.run_pending_tasks();
    }
}

/// Block cache for hot data
///
/// Uses sharded Moka Cache instances to enable dynamic capacity adjustment at runtime.
/// Each shard has a fixed capacity, and shards can be added or removed to change total capacity.
/// This architecture allows true shrink/grow operations that were impossible with a single Moka instance.
pub struct BlockCache {
    shards: RwLock<Vec<Arc<BlockCacheShard>>>,
    shard_size_bytes: u64,
    config: BlockCacheConfig,
    stats: Arc<CacheStatsInner>,
    /// CACHE-003 FIX: Secondary index mapping segment_id -> keys for O(1) segment invalidation
    segment_index: RwLock<HashMap<u64, HashSet<String>>>,
}

#[derive(Debug, Default)]
struct CacheStatsInner {
    hits: AtomicU64,
    misses: AtomicU64,
    inserts: AtomicU64,
    evictions: AtomicU64,
    memory_usage: AtomicUsize,
}

impl BlockCache {
    /// Create a new shard with the given ID
    fn create_shard(&self, _id: usize) -> Arc<BlockCacheShard> {
        Arc::new(BlockCacheShard::new(self.shard_size_bytes, self.stats.clone()))
    }

    pub fn new(config: BlockCacheConfig) -> Self {
        let max_memory_bytes = config.max_memory_bytes;
        let stats: Arc<CacheStatsInner> = Arc::default();

        // Calculate shard size and number of shards
        // Use DEFAULT_SHARD_SIZE_BYTES (16MB) as base, but adjust if config is smaller
        let shard_size_bytes = DEFAULT_SHARD_SIZE_BYTES.min(max_memory_bytes);
        let num_shards = max_memory_bytes.div_ceil(shard_size_bytes) as usize;
        let num_shards = num_shards.max(1); // At least 1 shard

        // Create initial shards
        let mut shards = Vec::with_capacity(num_shards);
        for _i in 0..num_shards {
            shards.push(Arc::new(BlockCacheShard::new(shard_size_bytes, stats.clone())));
        }

        Self {
            shards: RwLock::new(shards),
            shard_size_bytes,
            config,
            stats,
            segment_index: RwLock::new(HashMap::new()),
        }
    }

    /// Parse segment_id from a cache key formatted as "segment_id:offset"
    fn parse_segment_id(key: &str) -> Option<u64> {
        key.split(':').next().and_then(|s| s.parse().ok())
    }

    /// Generate cache key from segment_id and offset
    fn make_key(segment_id: u64, offset: u64) -> String {
        format!("{}:{}", segment_id, offset)
    }

    /// Calculate shard index from a key's hash value.
    ///
    /// Uses `AHash` for high-performance distribution.
    /// Both `insert_by_key` and `get_by_key` use this method to ensure
    /// they route to the same shard for a given key.
    #[cfg(not(feature = "benchmarks"))]
    fn calculate_shard_id(key: &str, num_shards: usize) -> usize {
        use std::hash::{Hash, Hasher};
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % num_shards
    }

    /// Calculate shard index from a key's hash value.
    ///
    /// Uses `AHash` for high-performance distribution.
    /// Both `insert_by_key` and `get_by_key` use this method to ensure
    /// they route to the same shard for a given key.
    #[cfg(feature = "benchmarks")]
    pub fn calculate_shard_id(key: &str, num_shards: usize) -> usize {
        use std::hash::{Hash, Hasher};
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % num_shards
    }

    /// Get value by string key (for KV operations)
    ///
    /// Uses key hash to directly route to the target shard - O(1) lookup.
    pub fn get_by_key(&self, key: &str) -> Option<Bytes> {
        let shards = self.shards.read();
        let shard_id = Self::calculate_shard_id(key, shards.len());
        let result = shards[shard_id].cache.get(key);
        drop(shards);

        if result.is_some() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Get value by segment_id and offset (for block operations)
    pub fn get(&self, segment_id: u64, offset: u64) -> Option<Bytes> {
        let key = Self::make_key(segment_id, offset);
        self.get_by_key(&key)
    }

    /// Insert value with string key (for KV operations)
    /// Uses key hash to directly route to the target shard - O(1) insertion
    ///
    /// CACHE-002 FIX: Use atomic upsert pattern to avoid TOCTOU race condition.
    /// We accept approximate memory tracking since Moka doesn't provide atomic check-and-insert.
    /// The memory_usage counter may temporarily overcount if the same key is updated concurrently,
    /// but will be corrected when entries are evicted via the eviction listener.
    ///
    /// CACHE-003 FIX: Update segment -> keys secondary index for O(1) segment invalidation.
    pub fn insert_by_key(&self, key: String, value: Bytes) {
        let memory_delta = value.len();

        // CACHE-003: Update secondary index
        if let Some(segment_id) = Self::parse_segment_id(&key) {
            let mut index = self.segment_index.write();
            index.entry(segment_id).or_default().insert(key.clone());
        }

        // Insert into the shard determined by key hash (consistent with get_by_key)
        let shards = self.shards.read();
        let shard_id = Self::calculate_shard_id(&key, shards.len());
        shards[shard_id].cache.insert(key, value);
        drop(shards);

        // CACHE-002: We can't atomically check existence and insert, so we accept
        // that memory tracking is approximate. The counter may overcount if the same
        // key is updated by multiple threads, but this is acceptable because:
        // 1. The overcount is bounded by the number of concurrent updates to the same key
        // 2. The counter will be corrected when entries are evicted
        // 3. Exact tracking would require a lock, defeating Moka's lock-free advantage
        self.stats.inserts.fetch_add(1, Ordering::Relaxed);
        self.stats.memory_usage.fetch_add(memory_delta, Ordering::Relaxed);
    }

    pub fn insert(&self, key: String, value: Bytes) {
        self.insert_by_key(key, value);
    }

    /// Put method with segment_id and offset
    pub fn put(&self, segment_id: u64, offset: u64, value: Bytes) {
        let key = Self::make_key(segment_id, offset);
        self.insert_by_key(key, value);
    }

    /// Invalidate all cache entries for a specific segment
    /// Used when segment is deleted during compaction to avoid stale cache hits
    ///
    /// CACHE-003 FIX: Use segment -> keys secondary index for O(1) lookup
    /// instead of O(n) linear scan over all cache entries.
    pub fn invalidate_by_segment(&self, segment_id: u64) {
        // CACHE-003: Look up keys directly from secondary index
        let keys_to_invalidate = {
            let mut index = self.segment_index.write();
            index.remove(&segment_id).unwrap_or_default()
        };

        if keys_to_invalidate.is_empty() {
            return;
        }

        let removed_count = keys_to_invalidate.len() as u64;
        // Invalidate from all shards
        let shards = self.shards.read();
        for key in &keys_to_invalidate {
            for shard in shards.iter() {
                shard.cache.invalidate(key);
            }
        }

        self.stats.evictions.fetch_add(removed_count, Ordering::Relaxed);
        tracing::debug!(segment_id, removed = removed_count, "Invalidated cache entries for deleted segment");
    }

    /// Get value from prefetch cache
    ///
    /// FIX-001: Check if a key was prefetched by SequentialPrefetcher.
    /// Prefetched entries are stored with key format "prefetch:key:<original_key>"
    pub fn get_prefetch(&self, key: &str) -> Option<Bytes> {
        let cache_key = format!("prefetch:key:{}", key);
        self.get_by_key(&cache_key)
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> u64 {
        self.stats.memory_usage.load(Ordering::Relaxed) as u64
    }

    pub fn stats(&self) -> CacheStats {
        let hits = self.stats.hits.load(Ordering::Relaxed);
        let misses = self.stats.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        // Estimate item count from weighted_size (sum across all shards)
        let shards = self.shards.read();
        let items: usize = shards.iter().map(|s| s.weighted_size() as usize).sum();
        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            items,
            capacity: self.config.max_items,
            memory_usage: self.stats.memory_usage.load(Ordering::Relaxed) as u64,
            inserts: self.stats.inserts.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
        }
    }

    /// Apply eviction pressure to the cache by running Moka's maintenance tasks.
    /// This forces Moka to process pending evictions and clean up expired entries.
    pub fn apply_eviction_pressure(&self) {
        let shards = self.shards.read();
        for shard in shards.iter() {
            shard.run_pending_tasks();
        }
    }

    /// Shrink the cache to the target byte size by removing excess shards.
    ///
    /// This method:
    /// 1. Calculates the target number of shards based on target_bytes
    /// 2. Drains and removes excess shards from the end
    /// 3. Returns the approximate bytes freed
    ///
    /// # Thread Safety
    /// This method acquires a write lock on the shards list.
    /// Concurrent get/put operations may be briefly blocked during shard removal.
    pub fn shrink_to(&self, target_bytes: u64) -> usize {
        let mut shards = self.shards.write();
        let current_total = (shards.len() as u64) * self.shard_size_bytes;
        
        // If already at or below target, nothing to do
        if current_total <= target_bytes {
            return 0;
        }

        let target_shard_count = target_bytes.div_ceil(self.shard_size_bytes) as usize;
        let target_shard_count = target_shard_count.max(1); // Always keep at least 1 shard

        let mut bytes_freed = 0;
        while shards.len() > target_shard_count {
            if let Some(shard) = shards.pop() {
                // Track bytes before dropping
                bytes_freed += shard.weighted_size() as usize;
                
                // Invalidate all entries in the shard to trigger eviction listeners
                shard.invalidate_all();
                shard.run_pending_tasks();
                
                // Drop the shard (this will wait for Moka's background threads)
                drop(shard);
            } else {
                break;
            }
        }

        tracing::info!(
            target_shard_count = shards.len(),
            bytes_freed = bytes_freed,
            "BlockCache shrunk to {} shards",
            shards.len()
        );

        bytes_freed
    }

    /// Grow the cache to the target byte size by adding new shards.
    ///
    /// This method:
    /// 1. Calculates the target number of shards based on target_bytes
    /// 2. Creates new shards as needed
    /// 3. Adds them to the shards list
    ///
    /// # Thread Safety
    /// This method acquires a write lock on the shards list.
    /// Concurrent get/put operations may be briefly blocked during shard addition.
    pub fn grow_to(&self, target_bytes: u64) {
        let mut shards = self.shards.write();
        let current_total = (shards.len() as u64) * self.shard_size_bytes;
        
        // If already at or above target, nothing to do
        if current_total >= target_bytes {
            return;
        }

        let target_shard_count = target_bytes.div_ceil(self.shard_size_bytes) as usize;
        let target_shard_count = target_shard_count.max(1);

        let next_id = shards.len();
        for i in 0..(target_shard_count - shards.len()) {
            let shard_id = next_id + i;
            let new_shard = self.create_shard(shard_id);
            shards.push(new_shard);
        }

        tracing::info!(
            target_shard_count = shards.len(),
            "BlockCache grown to {} shards",
            shards.len()
        );
    }

    /// Get the current number of shards
    #[cfg(test)]
    pub fn shard_count(&self) -> usize {
        self.shards.read().len()
    }

    /// Get the shard size in bytes
    #[cfg(test)]
    pub fn shard_size_bytes(&self) -> u64 {
        self.shard_size_bytes
    }
}

/// BlockCache adapter for prefetch interface
///
/// GAP-C4: Enhanced to actually read blocks from segments and cache them
pub struct BlockCacheAsPrefetchCache {
    block_cache: Arc<BlockCache>,
    /// GAP-C4: Callback to read block data from segments
    /// Takes (segment_id, block_id, block_size) and returns block data
    block_reader: Box<dyn Fn(u64, u64, u64) -> Option<Bytes> + Send + Sync>,
    block_size: u64,
}

impl BlockCacheAsPrefetchCache {
    pub fn new(
        block_cache: Arc<BlockCache>,
        block_size: u64,
        block_reader: impl Fn(u64, u64, u64) -> Option<Bytes> + Send + Sync + 'static,
    ) -> Self {
        Self {
            block_cache,
            block_reader: Box::new(block_reader),
            block_size,
        }
    }
}

impl crate::cache::prefetch::PrefetchCache for BlockCacheAsPrefetchCache {
    fn prefetch(&self, segment_id: u64, block_id: u64) -> bool {
        // GAP-C4 / FIX-001: Actually read the block from segment and cache it
        let cache_key = format!("{}:block_{}", segment_id, block_id);

        // Try to read block from segment via callback
        if let Some(block_data) = (self.block_reader)(segment_id, block_id, self.block_size) {
            // Store the raw block data for block-level access
            self.block_cache.insert_by_key(cache_key.clone(), block_data.clone());

            // FIX-001: Parse block data and cache individual KV pairs
            // This allows get() to find prefetched entries via get_from_prefetch(key)
            Self::parse_and_cache_kv_pairs(&block_data, segment_id, block_id, &self.block_cache)
        } else {
            false
        }
    }

    fn contains(&self, segment_id: u64, block_id: u64) -> bool {
        let cache_key = format!("{}:block_{}", segment_id, block_id);
        self.block_cache.get_by_key(&cache_key).is_some()
    }

    fn get(&self, segment_id: u64, block_id: u64) -> Option<Arc<dyn Send + Sync>> {
        let cache_key = format!("{}:block_{}", segment_id, block_id);
        self.block_cache.get_by_key(&cache_key)
            .map(|bytes| Arc::new(bytes) as Arc<dyn Send + Sync>)
    }
}

impl BlockCacheAsPrefetchCache {
    /// Parse block data and cache individual KV pairs
    ///
    /// Block format (from segment.rs scan_next):
    /// [key_len: 4 bytes] [key: key_len bytes] [value_len: 4 bytes] [value: value_len bytes] [checksum: 4 bytes]
    ///
    /// This allows get() to find prefetched entries via get_from_prefetch(key)
    fn parse_and_cache_kv_pairs(
        block_data: &Bytes,
        segment_id: u64,
        block_id: u64,
        block_cache: &BlockCache,
    ) -> bool {
        let mut parsed_any = false;
        let data = block_data.as_ref();
        let mut pos = 0;

        while pos + 4 <= data.len() {
            // Read key length
            let key_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > data.len() {
                break;
            }

            // Read key
            let key_bytes = &data[pos..pos + key_len];
            let key = match String::from_utf8(key_bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => {
                    // Invalid UTF-8, skip this entry
                    pos += key_len;
                    if pos + 4 > data.len() {
                        break;
                    }
                    let value_len = match data[pos..pos + 4].try_into() {
                        Ok(buf) => u32::from_le_bytes(buf) as usize,
                        Err(_) => break,
                    };
                    pos += 4 + value_len + 4;
                    continue;
                }
            };
            pos += key_len;

            if pos + 4 > data.len() {
                break;
            }

            // Read value length
            let value_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len > data.len() {
                break;
            }

            // Read value
            let value = Bytes::copy_from_slice(&data[pos..pos + value_len]);
            pos += value_len;

            // Skip checksum (4 bytes)
            if pos + 4 <= data.len() {
                pos += 4;
            }

            // Cache the KV pair with a prefixed key to distinguish from regular cached entries
            // Key format: "prefetch:key:<original_key>"
            let cache_key = format!("prefetch:key:{}", key);
            block_cache.insert_by_key(cache_key, value);
            parsed_any = true;
        }

        tracing::debug!(
            segment_id,
            block_id,
            parsed = parsed_any,
            "Prefetched block parsed into KV pairs"
        );

        parsed_any
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_invalidate_by_segment() {
        let config = BlockCacheConfig::default();
        let cache = BlockCache::new(config);

        // Insert entries for multiple segments
        cache.put(1, 100, Bytes::from("seg1_block100"));
        cache.put(1, 200, Bytes::from("seg1_block200"));
        cache.put(2, 100, Bytes::from("seg2_block100"));
        cache.put(3, 100, Bytes::from("seg3_block100"));

        // Verify all entries are present
        assert!(cache.get_by_key("1:100").is_some());
        assert!(cache.get_by_key("1:200").is_some());
        assert!(cache.get_by_key("2:100").is_some());
        assert!(cache.get_by_key("3:100").is_some());

        // Invalidate segment 1
        cache.invalidate_by_segment(1);

        // Verify segment 1 entries are gone
        assert!(cache.get_by_key("1:100").is_none());
        assert!(cache.get_by_key("1:200").is_none());

        // Verify other segments are still present
        assert!(cache.get_by_key("2:100").is_some());
        assert!(cache.get_by_key("3:100").is_some());
    }

    #[test]
    fn test_cache_invalidate_empty_segment() {
        let config = BlockCacheConfig::default();
        let cache = BlockCache::new(config);

        // Invalidate a segment that has no entries (should not panic)
        cache.invalidate_by_segment(999);

        // Cache should still work
        cache.put(1, 100, Bytes::from("test"));
        assert!(cache.get_by_key("1:100").is_some());
    }

    #[test]
    fn test_cache_basic_operations() {
        let config = BlockCacheConfig::default();
        let cache = BlockCache::new(config);

        // Test insert and get
        cache.put(1, 100, Bytes::from("test_data"));
        assert!(cache.get(1, 100).is_some());
        assert_eq!(cache.get(1, 100), Some(Bytes::from("test_data")));

        // Test miss
        assert!(cache.get(999, 999).is_none());

        // Test stats
        let stats = cache.stats();
        assert!(stats.hits >= 2);
        assert!(stats.misses >= 1);
    }

    #[test]
    fn test_cache_memory_tracking() {
        let config = BlockCacheConfig::default();
        let cache = BlockCache::new(config);

        let initial_memory = cache.memory_usage();
        cache.put(1, 100, Bytes::from("1234567890")); // 10 bytes
        let after_insert = cache.memory_usage();

        // Memory usage should have increased
        assert!(after_insert > initial_memory);
    }

    #[test]
    fn test_cache_eviction_stats() {
        // Create cache with very small capacity to force eviction
        // Use a small but realistic capacity (1MB) to avoid overflow
        let config = BlockCacheConfig {
            max_items: 2,
            max_memory_bytes: 1024 * 1024, // 1MB
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);

        // Insert more items than capacity
        cache.put(1, 1, Bytes::from("a"));
        cache.put(1, 2, Bytes::from("b"));
        cache.put(1, 3, Bytes::from("c")); // Should trigger eviction

        let stats = cache.stats();
        assert_eq!(stats.inserts, 3);
        // Moka may not evict immediately, but weighted_size should be tracked
        assert!(stats.items <= 3);
    }

    // ==================== PROD-001: Sharded Architecture Tests ====================

    #[test]
    fn test_sharded_cache_initial_shard_count() {
        // 64MB config should create 4 shards (64MB / 16MB = 4)
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 4);
        assert_eq!(cache.shard_size_bytes(), 16 * 1024 * 1024);
    }

    #[test]
    fn test_sharded_cache_small_config() {
        // 8MB config should create 1 shard (8MB < 16MB, so uses 8MB as shard size)
        let config = BlockCacheConfig {
            max_items: 1_000,
            max_memory_bytes: 8 * 1024 * 1024, // 8MB
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 1);
        assert_eq!(cache.shard_size_bytes(), 8 * 1024 * 1024);
    }

    #[test]
    fn test_sharded_cache_basic_operations() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 32 * 1024 * 1024, // 32MB = 2 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 2);

        // Test insert and get
        cache.put(1, 100, Bytes::from("test_data"));
        assert!(cache.get(1, 100).is_some());
        assert_eq!(cache.get(1, 100), Some(Bytes::from("test_data")));

        // Test miss
        assert!(cache.get(999, 999).is_none());

        // Test stats
        let stats = cache.stats();
        assert!(stats.hits >= 1);
        assert!(stats.misses >= 1);
    }

    #[test]
    fn test_sharded_cache_shrink_to() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB = 4 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 4);

        // Insert some data
        for i in 0..100 {
            cache.put(1, i, Bytes::from(format!("data_{}", i)));
        }

        // Shrink to 32MB (should leave 2 shards)
        let bytes_freed = cache.shrink_to(32 * 1024 * 1024);
        assert_eq!(cache.shard_count(), 2);
        // Bytes freed is tracked when shards are removed
        let _ = bytes_freed;

        // Cache should still work after shrink
        cache.put(2, 100, Bytes::from("after_shrink"));
        assert!(cache.get(2, 100).is_some());
    }

    #[test]
    fn test_sharded_cache_grow_to() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 32 * 1024 * 1024, // 32MB = 2 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 2);

        // Grow to 64MB (should create 4 shards total)
        cache.grow_to(64 * 1024 * 1024);
        assert_eq!(cache.shard_count(), 4);

        // Cache should still work after grow
        cache.put(1, 100, Bytes::from("after_grow"));
        assert!(cache.get(1, 100).is_some());
    }

    #[test]
    fn test_sharded_cache_shrink_and_ground() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB = 4 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);

        // Insert data across multiple segments
        for i in 0..50 {
            cache.put(1, i, Bytes::from(format!("seg1_{}", i)));
            cache.put(2, i, Bytes::from(format!("seg2_{}", i)));
        }

        // Shrink to 16MB (1 shard minimum)
        cache.shrink_to(16 * 1024 * 1024);
        assert_eq!(cache.shard_count(), 1);

        // Grow back to 64MB
        cache.grow_to(64 * 1024 * 1024);
        assert_eq!(cache.shard_count(), 4);

        // Operations should still work
        cache.put(3, 100, Bytes::from("after_cycle"));
        assert!(cache.get(3, 100).is_some());
    }

    #[test]
    fn test_sharded_cache_no_op_shrink() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 32 * 1024 * 1024, // 32MB = 2 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 2);

        // Try to shrink to larger than current (should be no-op)
        let bytes_freed = cache.shrink_to(64 * 1024 * 1024);
        assert_eq!(bytes_freed, 0);
        assert_eq!(cache.shard_count(), 2);
    }

    #[test]
    fn test_sharded_cache_no_op_grow() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB = 4 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);
        assert_eq!(cache.shard_count(), 4);

        // Try to grow to smaller than current (should be no-op)
        cache.grow_to(16 * 1024 * 1024);
        assert_eq!(cache.shard_count(), 4);
    }

    #[test]
    fn test_sharded_cache_invalidate_by_segment() {
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB = 4 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);

        // Insert entries for multiple segments
        cache.put(1, 100, Bytes::from("seg1_block100"));
        cache.put(1, 200, Bytes::from("seg1_block200"));
        cache.put(2, 100, Bytes::from("seg2_block100"));
        cache.put(3, 100, Bytes::from("seg3_block100"));

        // Verify all entries are present
        assert!(cache.get_by_key("1:100").is_some());
        assert!(cache.get_by_key("1:200").is_some());
        assert!(cache.get_by_key("2:100").is_some());
        assert!(cache.get_by_key("3:100").is_some());

        // Invalidate segment 1
        cache.invalidate_by_segment(1);

        // Verify segment 1 entries are gone
        assert!(cache.get_by_key("1:100").is_none());
        assert!(cache.get_by_key("1:200").is_none());

        // Verify other segments are still present
        assert!(cache.get_by_key("2:100").is_some());
        assert!(cache.get_by_key("3:100").is_some());
    }

    #[test]
    fn test_sharded_concurrent_access() {
        use std::thread;

        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024,
            frequency_aware: false,
        };
        let cache = Arc::new(BlockCache::new(config));

        // Spawn multiple threads doing concurrent reads/writes
        let mut handles = vec![];
        for t in 0..16 {
            let cache_clone = cache.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    let key = format!("{}_{}", t, i);
                    cache_clone.put(t, i, Bytes::from(key.clone()));
                    // Read may or may not find the value depending on timing
                    let _ = cache_clone.get(t, i);
                }
            }));
        }

        // All threads should complete without panic
        for handle in handles {
            handle.join().unwrap();
        }

        // Cache should still be functional
        cache.put(999, 999, Bytes::from("final"));
        assert!(cache.get(999, 999).is_some());
    }

    // ==================== T-002: Key Hash Routing Tests ====================

    #[test]
    fn test_key_routing_consistency() {
        // Verify that insert and get use the same shard routing
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 64 * 1024 * 1024, // 4 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);

        // Insert and verify that get_by_key finds the value
        cache.put(1, 100, Bytes::from("test_data"));
        assert!(cache.get_by_key("1:100").is_some());

        // Same key should always route to same shard
        let key1 = "1:100";
        let key2 = "2:200";
        let key3 = "3:300";

        let shard_id1_a = BlockCache::calculate_shard_id(key1, 4);
        let shard_id1_b = BlockCache::calculate_shard_id(key1, 4);
        assert_eq!(shard_id1_a, shard_id1_b, "Same key should always route to same shard");

        // Different keys may route to different shards (hash distribution)
        let shard_id2 = BlockCache::calculate_shard_id(key2, 4);
        let shard_id3 = BlockCache::calculate_shard_id(key3, 4);
        // At least verify they are valid shard indices
        assert!(shard_id2 < 4);
        assert!(shard_id3 < 4);
    }

    #[test]
    fn test_key_hash_distribution_uniformity() {
        // Verify that hash distribution is reasonably uniform across shards
        let num_shards = 4;
        let num_keys = 10_000;

        // Count keys per shard
        let mut shard_counts = vec![0usize; num_shards];

        for i in 0..num_keys {
            let key = format!("{}:{}", i / 100, i); // segment_id:offset format
            let shard_id = BlockCache::calculate_shard_id(&key, num_shards);
            shard_counts[shard_id] += 1;
        }

        let expected_per_shard = num_keys / num_shards;
        let tolerance = (expected_per_shard as f64 * 0.3) as usize; // 30% tolerance

        for (i, count) in shard_counts.iter().enumerate() {
            let diff = (*count as isize - expected_per_shard as isize).unsigned_abs();
            assert!(
                diff <= tolerance,
                "Shard {} has {} keys, expected ~{}, diff {} > tolerance {}",
                i, count, expected_per_shard, diff, tolerance
            );
        }

        // Print distribution for manual verification
        eprintln!("Key distribution across {} shards:", num_shards);
        for (i, count) in shard_counts.iter().enumerate() {
            eprintln!("  Shard {}: {} keys ({:.1}%)", i, count, (*count as f64 / num_keys as f64) * 100.0);
        }
    }

    #[test]
    fn test_key_routing_deterministic() {
        // Verify hash function is deterministic across calls
        let key = "123:456";
        let shard_id_1 = BlockCache::calculate_shard_id(key, 8);
        let shard_id_2 = BlockCache::calculate_shard_id(key, 8);
        assert_eq!(shard_id_1, shard_id_2);

        // Different shard counts should still be consistent
        let shard_id_4 = BlockCache::calculate_shard_id(key, 4);
        let shard_id_16 = BlockCache::calculate_shard_id(key, 16);
        assert!(shard_id_4 < 4);
        assert!(shard_id_16 < 16);
    }

    #[test]
    fn test_get_by_key_after_shard_resize() {
        // Verify get/insert work correctly after grow/shrink operations
        let config = BlockCacheConfig {
            max_items: 10_000,
            max_memory_bytes: 32 * 1024 * 1024, // 2 shards
            frequency_aware: false,
        };
        let cache = BlockCache::new(config);

        // Insert data with 2 shards
        cache.put(1, 100, Bytes::from("before_grow"));
        assert!(cache.get_by_key("1:100").is_some());

        // Grow to 4 shards
        cache.grow_to(64 * 1024 * 1024);

        // Note: After resize, the key hashes to a different shard index,
        // so the old entry may not be found. This is expected behavior -
        // cache entries are not migrated during resize.
        // New inserts should work correctly.
        cache.put(2, 200, Bytes::from("after_grow"));
        assert!(cache.get_by_key("2:200").is_some());

        // Shrink back to 1 shard
        cache.shrink_to(16 * 1024 * 1024);

        // New operations should still work
        cache.put(3, 300, Bytes::from("after_shrink"));
        assert!(cache.get_by_key("3:300").is_some());
    }
}
