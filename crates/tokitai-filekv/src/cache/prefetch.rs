//! Sequential Prefetcher for FileKV
//!
//! INNO-002: Implements sequential access pattern detection and prefetching
//! to improve cache hit rate for range queries.
//!
//! # Key Idea
//! When sequential access patterns are detected (e.g., keys "1", "2", "3"...),
//! proactively prefetch adjacent blocks into cache before they are requested.
//! This can improve cache hit rate by 15%+ for sequential workloads.
//!
//! # Algorithm
//! 1. Track last accessed key and detect stride pattern
//! 2. When sequential pattern detected (N consecutive accesses), trigger prefetch
//! 3. Prefetch next K blocks based on detected stride
//! 4. Limit prefetch window to avoid cache pollution
//!
//! # Integration
//! Used by FileKV::get() and FileKV::scan() methods

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::debug;

use crate::query::zone_map::{SequentialDetector, ZoneMapIndex};

/// Prefetcher configuration
#[derive(Debug, Clone)]
pub struct SequentialPrefetcherConfig {
    /// Enable prefetching (default: true)
    pub enabled: bool,
    /// Number of sequential accesses before triggering prefetch
    pub sequential_threshold: u32,
    /// Number of blocks to prefetch ahead
    pub prefetch_distance: u32,
    /// Maximum prefetch window size (blocks)
    pub max_prefetch_window: u32,
    /// Enable adaptive prefetch distance (default: true)
    pub adaptive_distance: bool,
}

impl Default for SequentialPrefetcherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sequential_threshold: 3,
            prefetch_distance: 2,
            max_prefetch_window: 10,
            adaptive_distance: true,
        }
    }
}

/// Prefetcher statistics
#[derive(Debug, Clone, Default)]
pub struct SequentialPrefetcherStats {
    /// Total prefetches triggered
    pub total_prefetches: u64,
    /// Successful prefetches (prefetched block was accessed)
    pub successful_prefetches: u64,
    /// Wasted prefetches (prefetched block was not accessed)
    pub wasted_prefetches: u64,
    /// Prefetch accuracy (successful / total)
    pub accuracy: f64,
    /// Cache hits due to prefetching
    pub cache_hits_from_prefetch: u64,
}

impl SequentialPrefetcherStats {
    /// Get accuracy as percentage
    pub fn accuracy_percent(&self) -> f64 {
        self.accuracy * 100.0
    }

    /// Record a prefetch operation
    pub fn record_prefetch(&mut self, was_useful: bool) {
        self.total_prefetches += 1;
        if was_useful {
            self.successful_prefetches += 1;
        } else {
            self.wasted_prefetches += 1;
        }

        if self.total_prefetches > 0 {
            self.accuracy = self.successful_prefetches as f64 / self.total_prefetches as f64;
        }
    }
}

/// Block cache trait for prefetching
///
/// Abstracts the cache implementation for prefetching
pub trait PrefetchCache: Send + Sync {
    /// Prefetch a block into cache
    /// 
    /// # Arguments
    /// * `segment_id` - The segment containing the block
    /// * `block_id` - The block ID within the segment
    fn prefetch(&self, segment_id: u64, block_id: u64) -> bool;

    /// Check if block is in cache
    fn contains(&self, segment_id: u64, block_id: u64) -> bool;

    /// Get block from cache
    fn get(&self, segment_id: u64, block_id: u64) -> Option<Arc<dyn Send + Sync>>;
}

/// Sequential Prefetcher
///
/// Detects sequential access patterns and prefetches adjacent blocks
/// 
/// GAP-C4: Added per-segment tracking for correct block prefetch
pub struct SequentialPrefetcher<C: PrefetchCache> {
    /// Configuration
    config: SequentialPrefetcherConfig,
    /// Sequential access detector
    detector: SequentialDetector,
    /// Block cache for prefetching
    cache: Arc<C>,
    /// GAP-C4: Current segment ID being tracked
    current_segment_id: u64,
    /// Last prefetched block ID
    last_prefetched_block: AtomicU64,
    /// Prefetch statistics
    stats: SequentialPrefetcherStats,
    /// Current prefetch window (block IDs)
    prefetch_window: parking_lot::Mutex<VecDeque<u64>>,
    /// Zone Map index for current segment
    zone_map: Option<Arc<ZoneMapIndex>>,
}

impl<C: PrefetchCache> SequentialPrefetcher<C> {
    /// Create a new sequential prefetcher
    pub fn new(config: SequentialPrefetcherConfig, cache: Arc<C>) -> Self {
        let detector = SequentialDetector::new(config.sequential_threshold);
        Self {
            config,
            detector,
            cache,
            current_segment_id: 0,
            last_prefetched_block: AtomicU64::new(0),
            stats: SequentialPrefetcherStats::default(),
            prefetch_window: parking_lot::Mutex::new(VecDeque::new()),
            zone_map: None,
        }
    }

    /// Create prefetcher with default configuration
    pub fn with_defaults(cache: Arc<C>) -> Self {
        Self::new(SequentialPrefetcherConfig::default(), cache)
    }

    /// Set zone map index for the current segment
    pub fn set_zone_map(&mut self, zone_map: Arc<ZoneMapIndex>) {
        self.zone_map = Some(zone_map);
    }

    /// Record a key access and potentially trigger prefetching
    ///
    /// # Arguments
    /// * `key` - The accessed key
    /// * `segment_id` - The segment containing the key
    /// * `block_id` - The block containing the key within the segment
    ///
    /// # Returns
    /// `true` if prefetching was triggered
    pub fn record_access(&mut self, key: &str, segment_id: u64, block_id: u64) -> bool {
        if !self.config.enabled {
            return false;
        }

        // GAP-C4: Reset detector if segment changed
        if self.current_segment_id != segment_id {
            self.current_segment_id = segment_id;
            self.detector.reset();
        }

        // Record access and detect sequential pattern
        if let Some(stride) = self.detector.record_access(key) {
            // Sequential pattern detected, trigger prefetch
            debug!(
                "Sequential pattern detected: stride={}, segment_id={}, block_id={}",
                stride, segment_id, block_id
            );
            self.trigger_prefetch(segment_id, block_id, stride);
            return true;
        }

        false
    }

    /// Trigger prefetching for adjacent blocks
    ///
    /// # Arguments
    /// * `segment_id` - The segment to prefetch
    /// * `current_block` - Current block being accessed
    /// * `stride` - Detected access stride (1 for forward, -1 for backward)
    fn trigger_prefetch(&mut self, segment_id: u64, current_block: u64, stride: i64) {
        let mut prefetch_distance = self.config.prefetch_distance;

        // Adaptive prefetch distance based on accuracy
        if self.config.adaptive_distance && self.stats.total_prefetches > 10 {
            let accuracy = self.stats.accuracy;
            if accuracy > 0.8 {
                // High accuracy, increase prefetch distance
                prefetch_distance = (prefetch_distance * 2).min(self.config.max_prefetch_window);
            } else if accuracy < 0.5 {
                // Low accuracy, decrease prefetch distance
                prefetch_distance = (prefetch_distance / 2).max(1);
            }
        }

        // Prefetch blocks ahead
        let mut prefetched = 0;
        for i in 1..=prefetch_distance {
            let next_block = if stride > 0 {
                current_block + i as u64
            } else {
                current_block.saturating_sub(i as u64)
            };

            // Check if block exists in zone map (if available)
            if let Some(ref zone_map) = self.zone_map {
                if next_block > zone_map.block_count() as u64 {
                    break;
                }
            }

            // GAP-C4: Prefetch if not already in cache, using segment_id
            if !self.cache.contains(segment_id, next_block) && self.cache.prefetch(segment_id, next_block) {
                prefetched += 1;
                self.last_prefetched_block.store(next_block, Ordering::Relaxed);

                // Track in prefetch window
                let mut window = self.prefetch_window.lock();
                window.push_back(next_block);
                if window.len() > self.config.max_prefetch_window as usize {
                    window.pop_front();
                }
            }
        }

        if prefetched > 0 {
            debug!("Prefetched {} blocks starting from block {}", prefetched, current_block);
        }
    }

    /// Record that a prefetched block was accessed (useful prefetch)
    pub fn record_prefetch_hit(&mut self, block_id: u64) {
        // Check if this block was prefetched
        let window = self.prefetch_window.lock();
        if window.contains(&block_id) {
            drop(window);
            self.stats.record_prefetch(true);
            self.stats.cache_hits_from_prefetch += 1;
            debug!("Prefetch hit for block {}", block_id);
        }
    }

    /// Record that a non-prefetched block was accessed (missed prefetch opportunity)
    pub fn record_prefetch_miss(&mut self, block_id: u64) {
        // Check if we should have prefetched this
        let last_prefetched = self.last_prefetched_block.load(Ordering::Relaxed);
        if last_prefetched > 0 && block_id > last_prefetched {
            self.stats.record_prefetch(false);
            debug!("Prefetch miss: accessed block {} after prefetching up to {}", 
                   block_id, last_prefetched);
        }
    }

    /// Get prefetcher statistics
    pub fn stats(&self) -> SequentialPrefetcherStats {
        self.stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = SequentialPrefetcherStats::default();
    }

    /// Check if prefetching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable prefetching
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Reset the sequential detector (e.g., on query boundary)
    pub fn reset_detector(&mut self) {
        self.detector.reset();
    }

    /// Get current prefetch distance
    pub fn current_prefetch_distance(&self) -> u32 {
        if self.config.adaptive_distance && self.stats.total_prefetches > 10 {
            let accuracy = self.stats.accuracy;
            if accuracy > 0.8 {
                return (self.config.prefetch_distance * 2).min(self.config.max_prefetch_window);
            } else if accuracy < 0.5 {
                return (self.config.prefetch_distance / 2).max(1);
            }
        }
        self.config.prefetch_distance
    }
}

/// Simple in-memory block cache for testing
#[cfg(test)]
pub struct SimpleBlockCache {
    cache: parking_lot::Mutex<std::collections::HashMap<(u64, u64), Arc<dyn Send + Sync>>>,
}

#[cfg(test)]
impl SimpleBlockCache {
    pub fn new() -> Self {
        Self {
            cache: parking_lot::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(test)]
impl PrefetchCache for SimpleBlockCache {
    fn prefetch(&self, segment_id: u64, block_id: u64) -> bool {
        let mut cache = self.cache.lock();
        let key = (segment_id, block_id);
        if !cache.contains_key(&key) {
            // Simulate prefetch by storing a dummy value
            cache.insert(key, Arc::new(()));
            true
        } else {
            false
        }
    }

    fn contains(&self, segment_id: u64, block_id: u64) -> bool {
        self.cache.lock().contains_key(&(segment_id, block_id))
    }

    fn get(&self, segment_id: u64, block_id: u64) -> Option<Arc<dyn Send + Sync>> {
        self.cache.lock().get(&(segment_id, block_id)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefetcher_config_default() {
        let config = SequentialPrefetcherConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sequential_threshold, 3);
        assert_eq!(config.prefetch_distance, 2);
        assert!(config.adaptive_distance);
    }

    #[test]
    fn test_sequential_prefetching() {
        let cache = Arc::new(SimpleBlockCache::new());
        let mut prefetcher = SequentialPrefetcher::with_defaults(cache.clone());
        let segment_id = 1u64;

        // Simulate sequential access: "1", "2", "3", "4"
        assert!(!prefetcher.record_access("1", segment_id, 1)); // No pattern yet
        assert!(!prefetcher.record_access("2", segment_id, 2)); // Count = 1
        assert!(!prefetcher.record_access("3", segment_id, 3)); // Count = 2
        assert!(prefetcher.record_access("4", segment_id, 4)); // Count = 3, prefetch triggered!

        // Check that blocks were prefetched
        assert!(cache.contains(segment_id, 5)); // Prefetched block 5
        assert!(cache.contains(segment_id, 6)); // Prefetched block 6
    }

    #[test]
    fn test_prefetch_accuracy_tracking() {
        let cache = Arc::new(SimpleBlockCache::new());
        let mut prefetcher = SequentialPrefetcher::with_defaults(cache.clone());
        let segment_id = 1u64;

        // Simulate sequential access to trigger prefetch
        prefetcher.record_access("1", segment_id, 1);
        prefetcher.record_access("2", segment_id, 2);
        prefetcher.record_access("3", segment_id, 3);
        prefetcher.record_access("4", segment_id, 4);

        // Record that prefetched block was accessed
        prefetcher.record_prefetch_hit(5);

        let stats = prefetcher.stats();
        assert!(stats.total_prefetches > 0);
        assert!(stats.successful_prefetches > 0);
        assert!(stats.accuracy > 0.0);
    }

    #[test]
    fn test_prefetch_disable() {
        let cache = Arc::new(SimpleBlockCache::new());
        let mut prefetcher = SequentialPrefetcher::with_defaults(cache.clone());
        prefetcher.set_enabled(false);
        let segment_id = 1u64;

        // Should not trigger prefetch even with sequential access
        assert!(!prefetcher.record_access("1", segment_id, 1));
        assert!(!prefetcher.record_access("2", segment_id, 2));
        assert!(!prefetcher.record_access("3", segment_id, 3));
        assert!(!prefetcher.record_access("4", segment_id, 4));

        // Cache should be empty (no prefetching)
        assert!(!cache.contains(segment_id, 5));
    }

    #[test]
    fn test_detector_reset() {
        let cache = Arc::new(SimpleBlockCache::new());
        let mut prefetcher = SequentialPrefetcher::with_defaults(cache.clone());
        let segment_id = 1u64;

        // Trigger sequential detection
        prefetcher.record_access("1", segment_id, 1);
        prefetcher.record_access("2", segment_id, 2);
        prefetcher.record_access("3", segment_id, 3);

        // Reset detector
        prefetcher.reset_detector();

        // Access pattern should be reset
        assert!(!prefetcher.record_access("4", segment_id, 4)); // No longer sequential
    }

    #[test]
    fn test_adaptive_prefetch_distance() {
        let cache = Arc::new(SimpleBlockCache::new());
        let mut prefetcher = SequentialPrefetcher::with_defaults(cache.clone());

        // Initial prefetch distance
        assert_eq!(prefetcher.current_prefetch_distance(), 2);

        // Simulate high accuracy
        for _ in 0..15 {
            prefetcher.stats.record_prefetch(true);
        }

        // Prefetch distance should increase
        assert!(prefetcher.current_prefetch_distance() > 2);
    }

    /// GAP-C4: Test that detector resets when segment changes
    #[test]
    fn test_segment_change_resets_detector() {
        let cache = Arc::new(SimpleBlockCache::new());
        let mut prefetcher = SequentialPrefetcher::with_defaults(cache.clone());

        // Sequential access in segment 1
        prefetcher.record_access("1", 1, 1);
        prefetcher.record_access("2", 1, 2);
        prefetcher.record_access("3", 1, 3);

        // Switch to segment 2 - should reset detector
        assert!(!prefetcher.record_access("4", 2, 1)); // Reset, not sequential
        assert!(!prefetcher.record_access("5", 2, 2)); // Count = 1
        assert!(!prefetcher.record_access("6", 2, 3)); // Count = 2
        assert!(prefetcher.record_access("7", 2, 4)); // Count = 3, triggered!

        // Prefetch should have happened in segment 2
        assert!(cache.contains(2, 5));
        assert!(cache.contains(2, 6));
    }
}
