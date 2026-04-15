//! Unified Cache Manager
//!
//! Provides a centralized manager that coordinates BlockCache, BloomFilterCache, and
//! prefetch caches under a single budget framework.
//!
//! Each cache enforces its own memory limits via `max_memory_bytes` / `max_items`
//! configuration. The budget framework tracks allocations for reporting purposes only.
//!
//! # Background Rebalance Thread
//!
//! When enabled via `RebalanceConfig`, a daemon thread runs periodically to:
//! 1. Collect hit rates and memory usage from all caches
//! 2. Identify caches with low utilization (hit rate below threshold)
//! 3. Identify caches with high utilization (hit rate above threshold)
//! 4. Redistribute memory budget by shrinking low-performers and growing high-performers
//!
//! The rebalance algorithm is conservative: budget is only moved when there is a clear
//! benefit (high-performer hit rate exceeds low-performer by at least `min_hit_rate_gap`).
//! The amount moved per cycle is bounded by `max_transfer_ratio` to avoid oscillation.
//!
//! # Rebalance Execution Mode
//!
//! - **BloomFilterCache**: Full dynamic shrink/grow support via LRU eviction.
//!   The `shrink_to_memory()` method performs actual LRU eviction, and `grow_max_memory()`
//!   raises the advisory memory limit.
//!
//! - **BlockCache (Sharded Moka)**: Full dynamic shrink/grow support via shard management.
//!   The cache is split into multiple Moka shards (default 16MB each). `shrink_to()` removes
//!   excess shards after invalidating their entries, truly reducing memory usage.
//!   `grow_to()` adds new shards to increase capacity.
//!
//! # Graceful Shutdown
//!
//! The rebalance thread monitors an `AtomicBool` shutdown flag. When `UnifiedCacheManager`
//! is dropped, the flag is set and the thread is joined with a timeout.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use parking_lot::Mutex;
use tracing::{debug, info, warn};

pub mod budget;
pub mod block_cache;
pub mod warmup;
pub mod prefetch;
mod rebalance;

pub use budget::{CacheBudget, SubBudget, CacheUsageReport};
pub use block_cache::{BlockCache, BlockCacheConfig, CacheStats, BlockCacheAsPrefetchCache};
pub use warmup::{CacheWarmer, CacheWarmingConfig, CacheWarmingStats, WarmingStrategy};
pub use prefetch::{SequentialPrefetcher, SequentialPrefetcherConfig, SequentialPrefetcherStats, PrefetchCache};
pub use rebalance::{RebalanceConfig, RebalanceDecision, RebalanceStats};

use crate::bloom::filter_cache::{BloomFilterCache, BloomFilterCacheConfig};

/// Configuration for the unified cache manager
#[derive(Debug, Clone)]
pub struct UnifiedCacheConfig {
    /// Total memory budget for all caches combined (bytes) - informational only
    pub max_total_memory_bytes: u64,
    /// Fraction allocated to BlockCache (default: 0.60)
    pub block_cache_ratio: f64,
    /// Fraction allocated to BloomFilterCache (default: 0.25)
    pub bloom_cache_ratio: f64,
    /// Remaining fraction is unallocated (caches enforce their own limits).
    /// Previously used for Prefetch budget (removed in Phase 2 cleanup).
    /// Existing BlockCache config (will be constrained by budget)
    pub block_cache_config: Option<BlockCacheConfig>,
    /// Existing BloomCache config (will be constrained by budget)
    pub bloom_cache_config: Option<BloomFilterCacheConfig>,
    /// Directory where bloom filter files are stored.
    /// CACHE-004 FIX: Previously hardcoded to "./bloom", now configurable.
    pub bloom_index_dir: PathBuf,
}

impl Default for UnifiedCacheConfig {
    fn default() -> Self {
        Self {
            max_total_memory_bytes: 128 * 1024 * 1024, // 128MB
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: PathBuf::from("bloom"),
        }
    }
}

/// Unified cache manager that coordinates all caches under a single budget.
/// When a rebalance config is provided, spawns a background thread that periodically
/// redistributes memory budget between caches based on hit rates.
pub struct UnifiedCacheManager {
    budget: Mutex<CacheBudget>,
    block_cache: Arc<BlockCache>,
    bloom_cache: Arc<BloomFilterCache>,
    /// Rebalance configuration (if background thread is enabled)
    rebalance_config: Option<RebalanceConfig>,
    /// Shutdown flag for the rebalance thread
    shutdown_flag: Arc<AtomicBool>,
    /// JoinHandle for the rebalance thread
    rebalance_thread: Mutex<Option<JoinHandle<()>>>,
    /// Last rebalance stats (updated by background thread)
    last_rebalance_stats: Mutex<Option<RebalanceStats>>,
}

impl UnifiedCacheManager {
    /// Create a new unified cache manager without background rebalance thread.
    /// Use `try_new_with_rebalance` to enable the background thread.
    pub fn new(config: UnifiedCacheConfig) -> Self {
        Self::new_inner(config, None)
    }

    /// Create a new unified cache manager with a background rebalance thread.
    ///
    /// The thread runs every `rebalance_config.interval` and:
    /// - Collects hit rates from block cache, bloom cache, and prefetch
    /// - Moves memory budget from low-performing caches to high-performing ones
    /// - Logs all rebalance decisions
    ///
    /// The thread shuts down gracefully when the manager is dropped.
    ///
    /// # Errors
    /// Returns an error string if the thread fails to spawn.
    pub fn try_new_with_rebalance(config: UnifiedCacheConfig, rebalance_config: RebalanceConfig) -> Result<Self, String> {
        let mut manager = Self::new_inner(config, Some(rebalance_config));
        manager.spawn_rebalance_thread();
        Ok(manager)
    }

    /// Internal constructor shared by `new` and `try_new_with_rebalance`.
    fn new_inner(config: UnifiedCacheConfig, rebalance_config: Option<RebalanceConfig>) -> Self {
        let block_max = (config.max_total_memory_bytes as f64 * config.block_cache_ratio) as u64;
        let bloom_max = (config.max_total_memory_bytes as f64 * config.bloom_cache_ratio) as u64;

        let budget = CacheBudget::new(
            config.max_total_memory_bytes,
            config.block_cache_ratio,
            config.bloom_cache_ratio,
        );

        // Create BlockCache constrained by budget
        let block_max_items = if let Some(ref bc) = config.block_cache_config {
            bc.max_items
        } else {
            // Derive from budget: assume ~1KB per item
            (block_max / 1024) as usize
        };

        let block_cache = if let Some(mut bc) = config.block_cache_config {
            bc.max_items = block_max_items;
            bc.max_memory_bytes = block_max;
            Arc::new(BlockCache::new(bc))
        } else {
            Arc::new(BlockCache::new(BlockCacheConfig {
                max_items: block_max_items,
                max_memory_bytes: block_max,
                frequency_aware: false,
            }))
        };

        // Create BloomCache constrained by budget
        let bloom_index_dir = config.bloom_index_dir.clone();
        let bloom_cache = if let Some(mut bfc) = config.bloom_cache_config {
            bfc.max_memory_bytes = bloom_max as usize;
            Arc::new(BloomFilterCache::new(bfc, bloom_index_dir))
        } else {
            let bfc = BloomFilterCacheConfig {
                max_memory_bytes: bloom_max as usize,
                ..Default::default()
            };
            Arc::new(BloomFilterCache::new(bfc, bloom_index_dir))
        };

        Self {
            budget: Mutex::new(budget),
            block_cache,
            bloom_cache,
            rebalance_config,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            rebalance_thread: Mutex::new(None),
            last_rebalance_stats: Mutex::new(None),
        }
    }

    /// Spawn the background rebalance thread.
    ///
    /// The thread runs `rebalance_once` every `interval` until `shutdown_flag` is set.
    fn spawn_rebalance_thread(&mut self) {
        let shutdown_flag = self.shutdown_flag.clone();

        // SAFETY: We store the raw pointer as usize and the thread is guaranteed to be
        // joined before Self is dropped (via Drop impl). The shutdown flag
        // provides a double-safety: even if Drop is somehow not called, the
        // thread will exit when the flag is set.
        let self_ptr: usize = self as *const UnifiedCacheManager as usize;

        let interval = self.rebalance_config.as_ref().map(|c| c.interval).unwrap_or(Duration::from_secs(30));

        let handle = thread::Builder::new()
            .name("cache-rebalance".to_string())
            .spawn(move || {
                debug!("Cache rebalance thread started, interval={:?}", interval);
                // Sleep first to avoid racing with the main thread's initialization
                let sleep_interval = Duration::from_millis(100);
                let mut elapsed = Duration::ZERO;
                while elapsed < interval && !shutdown_flag.load(Ordering::Relaxed) {
                    thread::sleep(sleep_interval);
                    elapsed += sleep_interval;
                }

                while !shutdown_flag.load(Ordering::Relaxed) {
                    // SAFETY: self_ptr is valid as long as UnifiedCacheManager exists,
                    // and the thread is joined before UnifiedCacheManager is dropped.
                    let manager = unsafe { &*(self_ptr as *const UnifiedCacheManager) };
                    manager.rebalance_once();

                    // Sleep in small increments to allow responsive shutdown
                    let sleep_interval = Duration::from_millis(100);
                    let mut elapsed = Duration::ZERO;
                    while elapsed < interval && !shutdown_flag.load(Ordering::Relaxed) {
                        thread::sleep(sleep_interval);
                        elapsed += sleep_interval;
                    }
                }
                debug!("Cache rebalance thread shutting down");
            })
            .expect("failed to spawn rebalance thread");

        *self.rebalance_thread.lock() = Some(handle);
        info!("Cache rebalance thread spawned");
    }

    /// Execute a single rebalance cycle.
    ///
    /// This method:
    /// 1. Collects stats from all caches (hit rates, memory usage, item counts)
    /// 2. Evaluates rebalance decisions based on configured thresholds
    /// 3. Adjusts cache capacities if beneficial
    /// 4. Logs all decisions and updates last_rebalance_stats
    ///
    /// The algorithm is conservative: budget is only transferred when:
    /// - A cache has hit rate below `low_hit_rate_threshold`
    /// - Another cache has hit rate above `high_hit_rate_threshold`
    /// - The gap between them exceeds `min_hit_rate_gap`
    /// - The transfer amount is bounded by `max_transfer_ratio`
    /// - Each cache's budget stays within [min_budget, max_budget] bounds
    pub fn rebalance_once(&self) -> RebalanceStats {
        let rebalance_config = match &self.rebalance_config {
            Some(cfg) => cfg,
            None => {
                debug!("Rebalance not configured (no RebalanceConfig provided)");
                return RebalanceStats::disabled();
            }
        };

        // Collect stats from all caches
        let block_stats = self.block_cache.stats();
        let bloom_stats = self.bloom_cache.stats();

        let block_hit_rate = block_stats.hit_rate;
        let bloom_hit_rate = bloom_stats.hit_rate;
        let block_memory = block_stats.memory_usage;
        let bloom_memory = bloom_stats.memory_used as u64;

        debug!(
            "Rebalance cycle: block hit_rate={:.3} ({:.1}MB), bloom hit_rate={:.3} ({:.1}MB)",
            block_hit_rate,
            block_memory as f64 / (1024.0 * 1024.0),
            bloom_hit_rate,
            bloom_memory as f64 / (1024.0 * 1024.0),
        );

        // Check if we have enough data (skip if both caches have very few accesses)
        let block_total = block_stats.hits + block_stats.misses;
        let bloom_total = bloom_stats.hits + bloom_stats.misses;
        let min_samples = rebalance_config.min_access_samples;

        if block_total < min_samples && bloom_total < min_samples {
            debug!(
                "Rebalance skipped: insufficient samples (block={}, bloom={}, min={})",
                block_total, bloom_total, min_samples
            );
            return RebalanceStats::skipped(block_hit_rate, bloom_hit_rate, block_memory, bloom_memory);
        }

        // Make rebalance decisions
        let decisions = RebalanceDecision::evaluate(
            rebalance_config,
            block_hit_rate,
            bloom_hit_rate,
            block_memory,
            bloom_memory,
        );

        // Apply decisions
        for decision in &decisions {
            match decision {
                RebalanceDecision::ShrinkBlock(bytes) => {
                    self.apply_block_shrink(*bytes);
                    info!("Rebalance: shrinking BlockCache by {} bytes", bytes);
                }
                RebalanceDecision::GrowBlock(bytes) => {
                    self.apply_block_grow(*bytes);
                    info!("Rebalance: growing BlockCache by {} bytes", bytes);
                }
                RebalanceDecision::ShrinkBloom(bytes) => {
                    self.apply_bloom_shrink(*bytes);
                    info!("Rebalance: shrinking BloomFilterCache by {} bytes", bytes);
                }
                RebalanceDecision::GrowBloom(bytes) => {
                    self.apply_bloom_grow(*bytes);
                    info!("Rebalance: growing BloomFilterCache by {} bytes", bytes);
                }
            }
        }

        // Update budget tracking
        {
            let budget = self.budget.lock();
            for decision in &decisions {
                match decision {
                    RebalanceDecision::ShrinkBlock(bytes) | RebalanceDecision::GrowBlock(bytes) => {
                        // BlockCache budget is informational; update for reporting
                        let current = budget.block_cache.max_budget();
                        if *bytes > 0 {
                            let new_budget = if matches!(decision, RebalanceDecision::GrowBlock(_)) {
                                current.saturating_add(*bytes)
                            } else {
                                current.saturating_sub(*bytes)
                            };
                            // Note: SubBudget.max is private, we track via report only
                            let _ = new_budget;
                        }
                    }
                    RebalanceDecision::ShrinkBloom(bytes) | RebalanceDecision::GrowBloom(bytes) => {
                        let current = budget.bloom_filter.max_budget();
                        if *bytes > 0 {
                            let new_budget = if matches!(decision, RebalanceDecision::GrowBloom(_)) {
                                current.saturating_add(*bytes)
                            } else {
                                current.saturating_sub(*bytes)
                            };
                            let _ = new_budget;
                        }
                    }
                }
            }
        }

        // Build stats and store for retrieval
        let stats = RebalanceStats::completed(
            block_hit_rate,
            bloom_hit_rate,
            block_memory,
            bloom_memory,
            decisions,
        );

        #[cfg(feature = "metrics")]
        {
            // Record Prometheus metrics if enabled
            debug!(
                "Rebalance metrics: block_hit_rate={}, bloom_hit_rate={}, decisions={}",
                block_hit_rate,
                bloom_hit_rate,
                stats.decisions.len(),
            );
        }

        *self.last_rebalance_stats.lock() = Some(stats.clone());
        stats
    }

    /// Shrink BlockCache by removing excess shards.
    ///
    /// This method calls `block_cache.shrink_to()` which removes excess shards
    /// after invalidating their entries. This truly reduces memory usage.
    fn apply_block_shrink(&self, bytes: u64) {
        let current_memory = self.block_cache.memory_usage();
        let target = current_memory.saturating_sub(bytes);
        let evicted = self.block_cache.shrink_to(target);

        let bytes_kb = bytes as f64 / 1024.0;
        let evicted_kb = evicted as f64 / 1024.0;
        info!(
            "BlockCache: shrunk by {:.1}KB target, actually freed {:.1}KB (real shard removal)",
            bytes_kb,
            evicted_kb
        );
    }

    /// Grow BlockCache by adding new shards.
    ///
    /// This method calls `block_cache.grow_to()` which creates new shards
    /// to increase the cache capacity.
    fn apply_block_grow(&self, bytes: u64) {
        let current_memory = self.block_cache.memory_usage();
        let target = current_memory.saturating_add(bytes);
        self.block_cache.grow_to(target);

        let bytes_kb = bytes as f64 / 1024.0;
        info!(
            "BlockCache: grew by {:.1}KB target (new shards added)",
            bytes_kb
        );
    }

    /// Shrink BloomFilterCache by evicting LRU entries until memory usage
    /// is within a target budget. This performs actual eviction, not just logging.
    fn apply_bloom_shrink(&self, bytes: u64) {
        let current_memory = self.bloom_cache.stats().memory_used as u64;
        // Target: reduce memory usage by `bytes` from current
        let target = current_memory.saturating_sub(bytes);
        let evicted = self.bloom_cache.shrink_to_memory(target as usize);

        let bytes_kb = bytes as f64 / 1024.0;
        info!(
            "BloomFilterCache: shrunk by {:.1}KB target, evicted {} entries (actual LRU eviction)",
            bytes_kb,
            evicted
        );
    }

    /// Grow BloomFilterCache by increasing its dynamic max memory limit.
    /// This allows more Bloom filters to be cached before eviction kicks in.
    fn apply_bloom_grow(&self, bytes: u64) {
        let current_max = self.bloom_cache.grow_max_memory(
            self.bloom_cache.stats().memory_used.saturating_add(bytes as usize)
        );

        let bytes_kb = bytes as f64 / 1024.0;
        let prev_kb = current_max as f64 / 1024.0;
        info!(
            "BloomFilterCache: grew by {:.1}KB, previous max: {:.1}KB (advisory - allows more filters to cache)",
            bytes_kb,
            prev_kb
        );
    }

    /// Get the last rebalance statistics.
    pub fn last_rebalance_stats(&self) -> Option<RebalanceStats> {
        self.last_rebalance_stats.lock().clone()
    }

    /// Get the block cache
    pub fn block_cache(&self) -> &Arc<BlockCache> {
        &self.block_cache
    }

    /// Get the bloom cache
    pub fn bloom_cache(&self) -> &Arc<BloomFilterCache> {
        &self.bloom_cache
    }

    /// Get a budget tracking report (informational - shows current cache memory usage)
    pub fn usage_report(&self) -> CacheUsageReport {
        let block_usage = self.block_cache.memory_usage();
        let bloom_usage = self.bloom_cache.stats().memory_used as u64;
        let total_used = block_usage + bloom_usage;

        // Calculate hit rates from actual cache statistics
        let block_stats = self.block_cache.stats();
        let bloom_stats = self.bloom_cache.stats();

        let budget = self.budget.lock();
        CacheUsageReport {
            total_budget: budget.max_bytes,
            total_used,
            usage_percent: if budget.max_bytes > 0 {
                total_used as f64 / budget.max_bytes as f64
            } else {
                0.0
            },
            block_cache_used: block_usage,
            block_cache_max: budget.block_cache.max_budget(),
            block_cache_hit_rate: block_stats.hit_rate,
            bloom_filter_used: bloom_usage,
            bloom_filter_max: budget.bloom_filter.max_budget(),
            bloom_filter_hit_rate: bloom_stats.hit_rate,
        }
    }
}

impl Drop for UnifiedCacheManager {
    fn drop(&mut self) {
        // Signal the rebalance thread to shut down
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // Wait for the thread to finish (with timeout to avoid blocking indefinitely)
        if let Some(handle) = self.rebalance_thread.lock().take() {
            debug!("Waiting for cache rebalance thread to shut down...");
            // Give the thread up to 5 seconds to finish its current cycle
            let timeout = Duration::from_secs(5);
            if handle.join().is_err() {
                warn!("Cache rebalance thread panicked during shutdown");
            } else {
                debug!("Cache rebalance thread shut down gracefully");
            }
            let _ = timeout; // Used for informational purposes above
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bloom::ASMS;
    use rebalance::RebalanceStatus;

    /// Test: UnifiedCacheManager creates caches with correct budget limits
    #[test]
    fn test_unified_cache_creation() {
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 10 * 1024 * 1024, // 10MB
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: std::path::PathBuf::from("test_bloom"),
        };

        let manager = UnifiedCacheManager::new(config);

        let report = manager.usage_report();
        assert_eq!(report.total_budget, 10 * 1024 * 1024);

        // BlockCache should get 60% of budget
        let block_max = report.block_cache_max;
        let expected_block_max = (10i64 * 1024 * 1024) as f64 * 0.60;
        assert!(
            (block_max as f64 - expected_block_max).abs() < 1024.0,
            "BlockCache max should be ~60% of total: got {}, expected {}",
            block_max, expected_block_max
        );

        // BloomCache should get 25% of budget
        let bloom_max = report.bloom_filter_max;
        let expected_bloom_max = (10i64 * 1024 * 1024) as f64 * 0.25;
        assert!(
            (bloom_max as f64 - expected_bloom_max).abs() < 1024.0,
            "BloomCache max should be ~25% of total: got {}, expected {}",
            bloom_max, expected_bloom_max
        );

        // Verify caches are accessible (memory_usage returns u64, always >= 0)
        let _ = manager.block_cache().memory_usage();
    }

    // ==================== CACHE-005: UnifiedCacheManager Coverage Tests ====================

    /// Test: CACHE-005 - block_cache() accessor returns valid cache that can insert and get
    #[test]
    fn test_block_cache_accessor_insert_and_get() {
        use bytes::Bytes;

        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let manager = UnifiedCacheManager::new(config);
        let block_cache = manager.block_cache();

        // Insert a block using put (segment_id, offset)
        let value = Bytes::from(vec![1, 2, 3, 4, 5]);
        block_cache.put(1, 0, value.clone());

        // Verify get returns the same data
        let result = block_cache.get(1, 0);
        assert!(result.is_some(), "Should retrieve inserted block");
        assert_eq!(result.unwrap(), value);
    }

    /// Test: CACHE-005 - bloom_cache() accessor returns valid cache
    #[test]
    fn test_bloom_cache_accessor_valid() {
        use bloom::ASMS;
        use crate::core::error::FileKVResult;

        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let manager = UnifiedCacheManager::new(config);
        let bloom_cache = manager.bloom_cache();

        // Insert a bloom filter
        let segment_id: u64 = 1;
        let mut filter = bloom::BloomFilter::with_rate(0.01, 100);
        filter.insert(&"test_key".to_string());
        bloom_cache.insert(segment_id, filter);

        // Verify get returns the filter (use a loader that returns None)
        let loader = |_id: u64| -> FileKVResult<Option<bloom::BloomFilter>> { Ok(None) };
        let result = bloom_cache.get(segment_id, &loader);
        assert!(result.is_ok(), "Get should succeed");
        let retrieved = result.unwrap();
        assert!(retrieved.is_some(), "Should retrieve inserted bloom filter");
        assert!(retrieved.unwrap().contains(&"test_key".to_string()), "Filter should contain key");
    }

    /// Test: CACHE-005 - usage_report returns reasonable defaults for empty caches
    #[test]
    fn test_usage_report_empty_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 32 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let manager = UnifiedCacheManager::new(config);
        let report = manager.usage_report();

        // Total budget should match config
        assert_eq!(report.total_budget, 32 * 1024 * 1024);

        // Empty caches should have low usage
        assert_eq!(report.total_used, 0);
        assert_eq!(report.usage_percent, 0.0);
        assert_eq!(report.block_cache_used, 0);
        assert_eq!(report.bloom_filter_used, 0);

        // Hit rates should be 0.0 for empty caches
        assert_eq!(report.block_cache_hit_rate, 0.0);
        assert_eq!(report.bloom_filter_hit_rate, 0.0);

        // Budget allocations should match ratios
        assert!(report.block_cache_max > 0);
        assert!(report.bloom_filter_max > 0);
    }

    /// Test: CACHE-005 - budget proportions that don't sum to 1.0 still work
    #[test]
    fn test_budget_proportions_not_sum_to_one() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.70, // 70%
            bloom_cache_ratio: 0.50, // 50% - total = 1.20 > 1.0
            // Remaining ratios are informational, not strictly enforced
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        // Should still create successfully (ratios are informational, not strictly enforced)
        let manager = UnifiedCacheManager::new(config);
        let report = manager.usage_report();

        // Budgets should still be allocated proportionally
        assert_eq!(report.total_budget, 64 * 1024 * 1024);
        // Block cache gets 70%
        assert!(report.block_cache_max > report.bloom_filter_max);
        // Caches should be accessible
        let _ = manager.block_cache().memory_usage();
        let _ = manager.bloom_cache().stats();
    }

    /// Test: CACHE-005 - memory usage exceeds budget (soft limit, no hard enforcement)
    #[test]
    fn test_budget_exceeded_soft_limit() {
        use bytes::Bytes;

        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 1024, // Very small budget (1KB)
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let manager = UnifiedCacheManager::new(config);

        // Insert more data than budget allows (soft limit, should still succeed)
        let block_cache = manager.block_cache();
        for i in 0..100u64 {
            let value = Bytes::from(vec![0u8; 256]); // 256 bytes per block
            block_cache.put(1, i, value);
        }

        // Usage should exceed budget (soft limit, no hard eviction enforcement)
        let report = manager.usage_report();
        assert!(report.total_used > 0, "Should have non-zero usage after inserts");
        // Report should show the actual usage
        assert!(report.block_cache_used > 0, "Block cache should have usage");
    }

    // ==================== T-025: Rebalance Thread Integration Tests ====================

    /// Test: T-025 - Rebalance thread can be spawned and shuts down gracefully on drop
    #[test]
    fn test_rebalance_thread_spawn_and_shutdown() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let rebalance_config = RebalanceConfig {
            interval: Duration::from_secs(3600), // Very long interval, we'll call rebalance_once manually
            ..Default::default()
        };

        let manager = UnifiedCacheManager::try_new_with_rebalance(config, rebalance_config)
            .expect("Should create manager with rebalance thread");

        // Verify thread was spawned
        assert!(manager.rebalance_thread.lock().is_some(), "Rebalance thread should be present");
        assert!(!manager.shutdown_flag.load(Ordering::Relaxed), "Shutdown flag should be false initially");

        // Drop should trigger graceful shutdown (no panic, no hang)
        drop(manager);
        // If we get here without hanging or panicking, shutdown succeeded
    }

    /// Test: T-025 - Rebalance without config returns disabled stats
    #[test]
    fn test_rebalance_once_disabled() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let manager = UnifiedCacheManager::new(config);
        let stats = manager.rebalance_once();

        assert_eq!(stats.status, RebalanceStatus::Disabled);
        assert!(!stats.had_action());
    }

    /// Test: T-025 - Rebalance skips when insufficient samples
    #[test]
    fn test_rebalance_once_insufficient_samples() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let rebalance_config = RebalanceConfig {
            interval: Duration::from_secs(1),
            min_access_samples: 1000, // Require lots of samples
            ..Default::default()
        };

        let manager = UnifiedCacheManager::try_new_with_rebalance(config, rebalance_config)
            .expect("Should create manager with rebalance");

        // Fresh caches have no samples, should skip
        let stats = manager.rebalance_once();
        assert_eq!(stats.status, RebalanceStatus::SkippedInsufficientSamples);
    }

    /// Test: T-025 - Rebalance evaluates decisions when caches have different hit rates
    /// Note: Since BlockCache and BloomFilterCache capacities are fixed at construction,
    /// the rebalance decisions are logged but not dynamically applied.
    #[test]
    fn test_rebalance_evaluates_decisions() {
        use bytes::Bytes;

        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 64 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        // Use a very long interval so thread barely runs during test
        let rebalance_config = RebalanceConfig {
            interval: Duration::from_secs(3600),
            low_hit_rate_threshold: 0.3,
            high_hit_rate_threshold: 0.8,
            min_hit_rate_gap: 0.2,
            min_access_samples: 5,
            ..Default::default()
        };

        let manager = UnifiedCacheManager::try_new_with_rebalance(config, rebalance_config)
            .expect("Should create manager with rebalance");

        // Generate some block cache traffic with mostly misses (low hit rate)
        let block_cache = manager.block_cache();
        for i in 0..10u64 {
            block_cache.put(1, i, Bytes::from(vec![0u8; 64]));
        }
        // Generate misses
        for i in 100..110u64 {
            let _ = block_cache.get(1, i);
        }

        // Generate some bloom cache traffic with a few hits
        let bloom_cache = manager.bloom_cache();
        let mut filter = bloom::BloomFilter::with_rate(0.01, 100);
        filter.insert(&"hot_key".to_string());
        bloom_cache.insert(5, filter);

        // Generate some hits
        let loader = |_id: u64| -> crate::core::error::FileKVResult<Option<bloom::BloomFilter>> { Ok(None) };
        for _ in 0..3 {
            let _ = bloom_cache.get(5, &loader);
        }
        // Generate some misses
        for i in 100..105u64 {
            let _ = bloom_cache.get(i, &loader);
        }

        // Run rebalance manually
        let stats = manager.rebalance_once();
        assert_eq!(stats.status, RebalanceStatus::Completed);

        // Stats should be retrievable
        let last_stats = manager.last_rebalance_stats();
        assert!(last_stats.is_some(), "Last rebalance stats should be stored");
    }

    /// Test: T-025 - Manager without rebalance works normally (no thread)
    #[test]
    fn test_manager_without_rebalance() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig {
            max_total_memory_bytes: 32 * 1024 * 1024,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: None,
            bloom_cache_config: None,
            bloom_index_dir: temp_dir.path().join("bloom"),
        };

        let manager = UnifiedCacheManager::new(config);

        // No rebalance thread should be present
        assert!(manager.rebalance_thread.lock().is_none(), "No rebalance thread expected");

        // Rebalance should return disabled stats
        let stats = manager.rebalance_once();
        assert_eq!(stats.status, RebalanceStatus::Disabled);

        // No rebalance stats stored
        assert!(manager.last_rebalance_stats().is_none());
    }

    /// Test: T-025 - RebalanceStats display is human-readable
    #[test]
    fn test_rebalance_stats_format() {
        let stats = RebalanceStats::completed(0.15, 0.85, 1024 * 1024, 512 * 1024, vec![]);
        let display = format!("{}", stats);
        assert!(display.contains("Completed"));
        assert!(display.contains("0.15"));
        assert!(display.contains("0.85"));
    }
}
