//! Bloom Filter Cache module
//!
//! Implements on-demand loading of bloom filters with LRU eviction to reduce memory usage.
//!
//! # Features
//! - On-demand loading: Bloom filters are loaded only when accessed
//! - LRU eviction: Least recently used filters are evicted when cache is full
//! - Configurable cache size: Limit memory usage for bloom filter cache
//! - Thread-safe: Uses DashMap for concurrent access
//! - Statistics: Track cache hits, misses, and memory usage
//!
//! # Benefits over resident approach
//! - Reduced memory footprint for large datasets with many segments
//! - Faster startup time (no need to load all filters at once)
//! - Automatic memory management with configurable limits

use std::num::NonZero;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use tracing::{debug, warn};

use crate::error::{ContextResult, ContextError};
use bloom::{BloomFilter, ASMS};

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
            max_filters: 100, // Cache up to 100 filters
            max_memory_bytes: 64 * 1024 * 1024, // 64MB max
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
struct CachedBloomFilter {
    /// The bloom filter (Arc-wrapped for sharing)
    filter: Arc<BloomFilter>,
    /// Estimated memory size of the filter (bytes)
    memory_size: usize,
    /// Segment ID this filter belongs to
    segment_id: u64,
}

impl CachedBloomFilter {
    fn new(filter: BloomFilter, segment_id: u64) -> Self {
        // Estimate memory size: Bloom filter uses ~10 bits per element
        // Plus overhead for the filter structure
        // Note: bloom filter crate doesn't expose size, so we estimate
        let memory_size = 1024 * 10; // ~10KB per filter estimate

        Self {
            filter: Arc::new(filter),
            segment_id,
            memory_size,
        }
    }
}

/// Bloom Filter Cache with on-demand loading and LRU eviction
pub struct BloomFilterCache {
    /// Cache of loaded bloom filters
    cache: DashMap<u64, CachedBloomFilter>,
    /// LRU queue for eviction
    lru_queue: Mutex<LruCache<u64, ()>>,
    /// Configuration
    config: BloomFilterCacheConfig,
    /// Index directory where bloom filters are stored
    index_dir: PathBuf,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    loads: AtomicU64,
    memory_used: AtomicUsize,
}

impl BloomFilterCache {
    /// Create a new bloom filter cache
    pub fn new(config: BloomFilterCacheConfig, index_dir: PathBuf) -> Self {
        let cap = NonZero::new(config.max_filters)
            .expect("BloomFilterCache max_filters must be non-zero");
        let lru_queue = Mutex::new(LruCache::new(cap));

        Self {
            cache: DashMap::new(),
            lru_queue,
            config,
            index_dir,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            loads: AtomicU64::new(0),
            memory_used: AtomicUsize::new(0),
        }
    }

    /// Get a bloom filter for a segment (loads on-demand if not cached)
    pub fn get(&self, segment_id: u64, loader: &dyn Fn(u64) -> ContextResult<Option<BloomFilter>>) -> ContextResult<Option<Arc<BloomFilter>>> {
        // Check if filter is already cached
        if let Some(cached) = self.cache.get(&segment_id) {
            self.hits.fetch_add(1, Ordering::Relaxed);

            // Update LRU (promote to most recently used)
            {
                let mut lru = self.lru_queue.lock();
                lru.promote(&segment_id);
            }

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
    pub fn insert(&self, segment_id: u64, filter: BloomFilter) {
        self.cache_and_promote(segment_id, filter);
    }

    /// Check if a key exists in a segment's bloom filter (convenience method)
    pub fn contains(&self, segment_id: u64, key: &str, loader: &dyn Fn(u64) -> ContextResult<Option<BloomFilter>>) -> ContextResult<Option<bool>> {
        match self.get(segment_id, loader)? {
            Some(filter) => Ok(Some(filter.contains(&key.to_string()))),
            None => Ok(None),
        }
    }

    /// Remove a bloom filter from the cache
    pub fn remove(&self, segment_id: u64) -> Option<Arc<BloomFilter>> {
        if let Some((_, cached)) = self.cache.remove(&segment_id) {
            self.memory_used.fetch_sub(cached.memory_size, Ordering::Relaxed);
            
            // Remove from LRU
            {
                let mut lru = self.lru_queue.lock();
                lru.pop_entry(&segment_id);
            }
            
            Some(cached.filter)
        } else {
            None
        }
    }

    /// Clear all cached filters
    pub fn clear(&self) {
        self.cache.clear();
        let mut lru = self.lru_queue.lock();
        lru.clear();
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

    /// Cache a filter and update LRU (internal helper)
    fn cache_and_promote(&self, segment_id: u64, filter: BloomFilter) {
        let cached = CachedBloomFilter::new(filter, segment_id);
        let memory_delta = cached.memory_size;

        // Check memory limit and evict if necessary
        let current_memory = self.memory_used.load(Ordering::Relaxed);
        if current_memory + memory_delta > self.config.max_memory_bytes {
            self.evict_to_fit(memory_delta);
        }

        // Insert into cache
        if let Some(old_cached) = self.cache.insert(segment_id, cached) {
            self.memory_used.fetch_sub(old_cached.memory_size, Ordering::Relaxed);
        }

        self.memory_used.fetch_add(memory_delta, Ordering::Relaxed);

        // Update LRU
        {
            let mut lru = self.lru_queue.lock();
            lru.push(segment_id, ());
        }
    }

    /// Evict filters to make room for new data
    fn evict_to_fit(&self, needed_memory: usize) {
        let mut lru = self.lru_queue.lock();
        let target_memory = self.config.max_memory_bytes.saturating_sub(needed_memory);

        while self.memory_used.load(Ordering::Relaxed) > target_memory {
            if let Some((evict_id, _)) = lru.pop_lru() {
                if let Some((_, cached)) = self.cache.remove(&evict_id) {
                    self.memory_used.fetch_sub(cached.memory_size, Ordering::Relaxed);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    debug!("Evicted bloom filter for segment {}", evict_id);
                }
            } else {
                break;
            }
        }
    }
}

/// Helper to load a bloom filter from disk
pub fn load_bloom_filter_from_disk(index_dir: &Path, segment_id: u64) -> ContextResult<Option<BloomFilter>> {
    use super::bloom_migration::{BloomFilterMigrator, MigrationResult};
    use tracing::{warn, info};

    let migrator = BloomFilterMigrator::new(index_dir.to_path_buf());
    
    match migrator.load_with_migration(segment_id) {
        Ok(Some((bloom, _keys, migration_result))) => {
            match migration_result {
                MigrationResult::Migrated { from_version, to_version } => {
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
            Ok(Some(bloom))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bloom_filter_cache_config_default() {
        let config = BloomFilterCacheConfig::default();
        assert_eq!(config.max_filters, 100);
        assert_eq!(config.max_memory_bytes, 64 * 1024 * 1024);
        assert!(config.on_demand_enabled);
    }

    #[test]
    fn test_bloom_filter_cache_basic() {
        let temp_dir = TempDir::new().unwrap();
        let config = BloomFilterCacheConfig::default();
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        // Create a test bloom filter
        let mut filter = BloomFilter::with_rate(0.01, 100);
        filter.insert(&"test_key".to_string());

        // Insert into cache
        cache.insert(1, filter);

        // Retrieve from cache
        let loader = |_id: u64| -> ContextResult<Option<BloomFilter>> { Ok(None) };
        let cached = cache.get(1, &loader).unwrap();
        assert!(cached.is_some());
        assert!(cached.unwrap().contains(&"test_key".to_string()));
    }

    #[test]
    fn test_bloom_filter_cache_on_demand() {
        let temp_dir = TempDir::new().unwrap();
        let config = BloomFilterCacheConfig::default();
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        // Simulate on-demand loading with a static response after first load
        let loader = |id: u64| -> ContextResult<Option<BloomFilter>> {
            if id == 1 {
                let mut filter = BloomFilter::with_rate(0.01, 100);
                filter.insert(&"loaded_key".to_string());
                Ok(Some(filter))
            } else {
                Ok(None)
            }
        };

        // First access (cache miss, should load)
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains(&"loaded_key".to_string()));

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
        let config = BloomFilterCacheConfig {
            max_filters: 3,
            max_memory_bytes: 1024 * 10, // Small limit for testing
            on_demand_enabled: true,
        };
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        // Insert multiple filters
        for i in 1..=5 {
            let mut filter = BloomFilter::with_rate(0.01, 100);
            filter.insert(&format!("key_{}", i));
            cache.insert(i, filter);
        }

        // Cache should have evicted some filters
        let stats = cache.stats();
        assert!(stats.evictions > 0);
        assert!(stats.filters_cached <= 3);
    }

    #[test]
    fn test_bloom_filter_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = BloomFilterCacheConfig::default();
        let cache = BloomFilterCache::new(config, temp_dir.path().to_path_buf());

        let mut filter = BloomFilter::with_rate(0.01, 100);
        filter.insert(&"test".to_string());
        cache.insert(1, filter);

        let loader = |_id: u64| -> ContextResult<Option<BloomFilter>> { Ok(None) };

        cache.get(1, &loader).unwrap(); // hit
        cache.get(1, &loader).unwrap(); // hit
        cache.get(2, &loader).unwrap(); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.66).abs() < 0.02);
    }
}
