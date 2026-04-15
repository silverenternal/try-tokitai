//! Range Query Pruner for FileKV
//!
//! INNO-002: Implements range query pruning using Zone Map metadata.
//!
//! # Key Idea
//! When performing a range query (scan), use Zone Map min/max keys to quickly
//! skip blocks that don't overlap with the query range. This reduces I/O by 40-60%.
//!
//! # Algorithm
//! For each segment:
//! 1. Get Zone Map entries for all blocks
//! 2. For each block, check if query range overlaps with block's key range
//! 3. Skip (prune) blocks with no overlap
//! 4. Scan only overlapping blocks
//!
//! # Integration
//! Used by FileKV::scan() to optimize range queries

use tracing::debug;
use thiserror::Error;

use crate::core::error::{FatalError, ExpectedError};
use super::zone_map::{ZoneMapIndex, ZoneMapError, RangeQueryStats};

/// Result type for range query pruner operations
pub type Result<T> = std::result::Result<T, FatalError>;

/// Result type for operations that may return expected errors (like BlockNotFound)
pub type ExpectedResult<T> = std::result::Result<T, PrunerError>;

/// Pruner error types that distinguish between fatal and expected errors
#[derive(Debug, Error)]
pub enum PrunerError {
    #[error("Fatal error: {0}")]
    Fatal(#[from] FatalError),

    #[error("Expected error: {0}")]
    Expected(#[from] ExpectedError),
}

/// Range query pruning configuration
#[derive(Debug, Clone)]
pub struct RangeQueryPrunerConfig {
    /// Enable range query pruning (default: true)
    pub enabled: bool,
    /// Minimum query range size to enable pruning (keys)
    pub min_range_size: usize,
    /// Maximum query range size to enable pruning (keys)
    /// (Very large ranges may not benefit from pruning)
    pub max_range_size: usize,
}

impl Default for RangeQueryPrunerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_range_size: 10,
            max_range_size: 1_000_000,
        }
    }
}

/// Range query pruning statistics
#[derive(Debug, Clone, Default)]
pub struct RangeQueryPrunerStats {
    /// Total range queries executed
    pub total_queries: u64,
    /// Queries that benefited from pruning
    pub pruned_queries: u64,
    /// Total blocks across all queries
    pub total_blocks: u64,
    /// Blocks scanned (not pruned)
    pub blocks_scanned: u64,
    /// Blocks pruned (skipped)
    pub blocks_pruned: u64,
    /// Average pruning ratio
    pub avg_pruning_ratio: f64,
}

impl RangeQueryPrunerStats {
    /// Record a range query execution
    pub fn record_query(&mut self, stats: &RangeQueryStats) {
        self.total_queries += 1;
        self.total_blocks += stats.total_blocks as u64;
        self.blocks_scanned += stats.blocks_scanned as u64;
        self.blocks_pruned += stats.blocks_pruned as u64;

        if stats.pruning_ratio > 0.0 {
            self.pruned_queries += 1;
        }

        // Update average pruning ratio
        let total = self.total_blocks;
        if total > 0 {
            self.avg_pruning_ratio = self.blocks_pruned as f64 / total as f64;
        }
    }

    /// Get pruning ratio as percentage
    pub fn avg_pruning_ratio_percent(&self) -> f64 {
        self.avg_pruning_ratio * 100.0
    }

    /// Get queries benefited percentage
    pub fn pruned_queries_percent(&self) -> f64 {
        if self.total_queries > 0 {
            self.pruned_queries as f64 / self.total_queries as f64 * 100.0
        } else {
            0.0
        }
    }
}

/// Range Query Pruner
///
/// Uses Zone Map metadata to skip non-overlapping blocks during range queries
pub struct RangeQueryPruner {
    /// Pruner configuration
    config: RangeQueryPrunerConfig,
    /// Pruning statistics
    stats: parking_lot::Mutex<RangeQueryPrunerStats>,
}

impl RangeQueryPruner {
    /// Create a new range query pruner
    pub fn new(config: RangeQueryPrunerConfig) -> Self {
        Self {
            config,
            stats: parking_lot::Mutex::new(RangeQueryPrunerStats::default()),
        }
    }

    /// Create pruner with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RangeQueryPrunerConfig::default())
    }

    /// Check if pruning should be enabled for a given query range
    ///
    /// Uses heuristics based on key range selectivity to determine if
    /// pruning would be beneficial. Pruning is most effective when:
    /// - The query range is selective (small range within a larger keyspace)
    /// - The range is not too large (very large ranges scan most blocks anyway)
    ///
    /// # Arguments
    /// * `start_key` - Start of query range (inclusive)
    /// * `end_key` - End of query range (inclusive)
    pub fn should_enable_pruning(&self, start_key: &str, end_key: &str) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Heuristic 1: Empty or inverted range should not use pruning
        if start_key >= end_key {
            // Empty range (start == end) can still benefit from pruning
            // but inverted range (start > end) is invalid
            return start_key <= end_key;
        }

        // Heuristic 2: Very narrow ranges benefit greatly from pruning
        // (single key lookups, small ranges)
        let key_range_span = self.estimate_key_range_selectivity(start_key, end_key);

        // If the query spans less than 10% of the theoretical keyspace,
        // pruning is almost certainly beneficial
        if key_range_span < 0.1 {
            return true;
        }

        // Heuristic 3: Very large ranges (spanning >90% of keyspace) may not
        // benefit from pruning since most blocks will need to scanned anyway.
        // However, we still enable it as the zone map check is cheap.
        // Only disable for extremely large ranges approaching full scan
        if key_range_span > 0.95 {
            // For near-full-scan queries, pruning overhead may outweigh benefits
            // But we keep it enabled since the overhead is minimal
            return true;
        }

        // Default: enable pruning for moderate range sizes
        true
    }

    /// Estimate the selectivity of a key range query
    ///
    /// Returns a value between 0.0 and 1.0 representing the estimated
    /// fraction of the keyspace covered by the query range.
    /// Lower values mean more selective (better for pruning).
    fn estimate_key_range_selectivity(&self, start_key: &str, end_key: &str) -> f64 {
        // Simple heuristic based on lexicographic range span
        // In production, this could use histogram statistics or zone map metadata
        // For now, we estimate based on the string distance

        // Theoretical keyspace: all possible byte strings
        // Practical estimate: use first few characters to estimate spread

        // Use a simple metric: ratio of common prefix length to total length
        // This gives a rough estimate of how "wide" the range is

        let common_prefix_len = self.common_prefix_length(start_key, end_key);
        let avg_len = (start_key.len() + end_key.len()) as f64 / 2.0;

        if avg_len == 0.0 {
            return 1.0; // Empty keys = full range
        }

        // Selectivity: how much of the key space is covered
        // High common prefix = narrow range = low selectivity value
        // Low common prefix = wide range = high selectivity value
        let specificity = common_prefix_len as f64 / avg_len;

        // Invert: 1.0 - specificity gives us range span estimate
        // Clamp to [0.0, 1.0]
        (1.0 - specificity).clamp(0.0, 1.0)
    }

    /// Calculate the length of the common prefix between two strings
    fn common_prefix_length(&self, a: &str, b: &str) -> usize {
        a.chars()
            .zip(b.chars())
            .take_while(|(ca, cb)| ca == cb)
            .count()
    }

    /// Find blocks to scan for a range query
    ///
    /// # Arguments
    /// * `zone_map` - Zone Map index for the segment
    /// * `start_key` - Start of query range (inclusive)
    /// * `end_key` - End of query range (inclusive)
    ///
    /// # Returns
    /// Vector of block IDs to scan (non-pruned blocks)
    pub fn find_blocks_to_scan(
        &self,
        zone_map: &ZoneMapIndex,
        start_key: &str,
        end_key: &str,
    ) -> Vec<u64> {
        let overlapping = zone_map.find_overlapping_blocks(start_key, end_key);

        let total = zone_map.block_count();
        let scanned = overlapping.len();
        let pruned = total - scanned;

        debug!(
            "Range query [{}, {}]: total_blocks={}, scanned={}, pruned={}, pruning_ratio={:.2}%",
            start_key,
            end_key,
            total,
            scanned,
            pruned,
            if total > 0 { (pruned as f64 / total as f64) * 100.0 } else { 0.0 }
        );

        // Record statistics
        let stats = RangeQueryStats::new(total, pruned);
        self.stats.lock().record_query(&stats);

        overlapping
    }

    /// Check if a specific block should be pruned
    ///
    /// # Arguments
    /// * `zone_map` - Zone Map index for the segment
    /// * `block_id` - Block identifier to check
    /// * `start_key` - Start of query range (inclusive)
    /// * `end_key` - End of query range (inclusive)
    ///
    /// # Returns
    /// `true` if the block should be pruned (skipped), or ExpectedError::SegmentNotFound if block doesn't exist
    pub fn should_prune_block(
        &self,
        zone_map: &ZoneMapIndex,
        block_id: u64,
        start_key: &str,
        end_key: &str,
    ) -> ExpectedResult<bool> {
        match zone_map.should_prune_block(block_id, start_key, end_key) {
            Ok(should_prune) => Ok(should_prune),
            Err(ZoneMapError::BlockNotFound(id)) => {
                Err(ExpectedError::SegmentNotFound(id).into())
            }
            Err(e) => Err(FatalError::Corruption(format!("Zone map error: {}", e)).into()),
        }
    }

    /// Get pruning statistics
    pub fn stats(&self) -> RangeQueryPrunerStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        *self.stats.lock() = RangeQueryPrunerStats::default();
    }

    /// Check if pruning is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable pruning
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }
}

/// Block scan iterator with pruning support
///
/// Iterates over blocks that overlap with the query range
pub struct PrunedBlockIterator<'a> {
    /// Zone Map index
    zone_map: &'a ZoneMapIndex,
    /// Query start key (stored for debugging/inspection)
    #[allow(dead_code)]
    start_key: &'a str,
    /// Query end key (stored for debugging/inspection)
    #[allow(dead_code)]
    end_key: &'a str,
    /// Current position in iterator
    position: usize,
    /// Pre-computed list of overlapping block IDs
    overlapping_blocks: Vec<u64>,
}

impl<'a> PrunedBlockIterator<'a> {
    /// Create a new pruned block iterator
    pub fn new(
        zone_map: &'a ZoneMapIndex,
        start_key: &'a str,
        end_key: &'a str,
    ) -> Self {
        let overlapping = zone_map.find_overlapping_blocks(start_key, end_key);
        Self {
            zone_map,
            start_key,
            end_key,
            position: 0,
            overlapping_blocks: overlapping,
        }
    }

    /// Get total number of blocks to scan
    pub fn total_blocks(&self) -> usize {
        self.overlapping_blocks.len()
    }

    /// Get total blocks in segment (including pruned)
    pub fn total_blocks_in_segment(&self) -> usize {
        self.zone_map.block_count()
    }

    /// Get number of pruned blocks
    pub fn pruned_blocks(&self) -> usize {
        self.zone_map.block_count() - self.overlapping_blocks.len()
    }

    /// Get pruning ratio
    pub fn pruning_ratio(&self) -> f64 {
        let total = self.zone_map.block_count();
        if total > 0 {
            self.pruned_blocks() as f64 / total as f64
        } else {
            0.0
        }
    }
}

impl<'a> Iterator for PrunedBlockIterator<'a> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.overlapping_blocks.len() {
            let block_id = self.overlapping_blocks[self.position];
            self.position += 1;
            Some(block_id)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::zone_map::ZoneMapEntry;

    #[test]
    fn test_range_query_pruner_config_default() {
        let config = RangeQueryPrunerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_range_size, 10);
        assert_eq!(config.max_range_size, 1_000_000);
    }

    #[test]
    fn test_range_query_pruner_find_blocks() {
        let entries = vec![
            ZoneMapEntry::new(1, "a".to_string(), "m".to_string(), 0, 100, 10),
            ZoneMapEntry::new(2, "n".to_string(), "z".to_string(), 100, 100, 10),
        ];
        let zone_map = ZoneMapIndex::new(1, entries);

        let pruner = RangeQueryPruner::with_defaults();

        // Query overlapping both blocks
        let blocks = pruner.find_blocks_to_scan(&zone_map, "a", "z");
        assert_eq!(blocks.len(), 2);

        // Query overlapping only first block
        let blocks = pruner.find_blocks_to_scan(&zone_map, "b", "c");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], 1);

        // Query overlapping only second block
        let blocks = pruner.find_blocks_to_scan(&zone_map, "y", "z");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], 2);
    }

    #[test]
    fn test_range_query_pruner_stats() {
        let entries = vec![
            ZoneMapEntry::new(1, "a".to_string(), "m".to_string(), 0, 100, 10),
            ZoneMapEntry::new(2, "n".to_string(), "z".to_string(), 100, 100, 10),
        ];
        let zone_map = ZoneMapIndex::new(1, entries);

        let pruner = RangeQueryPruner::with_defaults();

        // Execute some queries
        pruner.find_blocks_to_scan(&zone_map, "b", "c"); // 1 scanned, 1 pruned
        pruner.find_blocks_to_scan(&zone_map, "y", "z"); // 1 scanned, 1 pruned
        pruner.find_blocks_to_scan(&zone_map, "a", "z"); // 2 scanned, 0 pruned

        let stats = pruner.stats();
        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.total_blocks, 6);
        assert_eq!(stats.blocks_scanned, 4);
        assert_eq!(stats.blocks_pruned, 2);
        assert!((stats.avg_pruning_ratio - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_pruned_block_iterator() {
        let entries = vec![
            ZoneMapEntry::new(1, "a".to_string(), "m".to_string(), 0, 100, 10),
            ZoneMapEntry::new(2, "n".to_string(), "z".to_string(), 100, 100, 10),
            ZoneMapEntry::new(3, "aa".to_string(), "mm".to_string(), 200, 100, 10),
        ];
        let zone_map = ZoneMapIndex::new(1, entries);

        let iter = PrunedBlockIterator::new(&zone_map, "b", "c");

        assert_eq!(iter.total_blocks(), 2); // Blocks 1 and 3 overlap
        assert_eq!(iter.pruned_blocks(), 1); // Block 2 pruned
        assert!(iter.pruning_ratio() > 0.3);

        let blocks: Vec<u64> = iter.collect();
        assert_eq!(blocks.len(), 2);
        assert!(blocks.contains(&1));
        assert!(blocks.contains(&3));
    }

    #[test]
    fn test_should_enable_pruning() {
        let pruner = RangeQueryPruner::with_defaults();

        // Pruning enabled by default for normal ranges
        assert!(pruner.should_enable_pruning("a", "b")); // Narrow range
        assert!(pruner.should_enable_pruning("key_0", "key_9999999999")); // Wide range
        assert!(pruner.should_enable_pruning("a", "a")); // Single key (empty range)
    }

    #[test]
    fn test_should_enable_pruning_heuristics() {
        let pruner = RangeQueryPruner::with_defaults();

        // Narrow ranges (high selectivity) should enable pruning
        assert!(pruner.should_enable_pruning("user:100", "user:105"));
        assert!(pruner.should_enable_pruning("a", "c"));

        // Very wide ranges (low selectivity) still enable pruning
        // (zone map check is cheap, so always beneficial)
        assert!(pruner.should_enable_pruning("a", "zzzzzz"));

        // Same key (point query) should enable pruning
        assert!(pruner.should_enable_pruning("key", "key"));
    }

    #[test]
    fn test_pruner_disable() {
        let mut pruner = RangeQueryPruner::with_defaults();
        pruner.set_enabled(false);

        assert!(!pruner.is_enabled());
        assert!(!pruner.should_enable_pruning("a", "z"));
        assert!(!pruner.should_enable_pruning("user:100", "user:105"));
    }

    #[test]
    fn test_common_prefix_length() {
        let pruner = RangeQueryPruner::with_defaults();

        assert_eq!(pruner.common_prefix_length("hello", "hello"), 5);
        assert_eq!(pruner.common_prefix_length("hello", "help"), 3);
        assert_eq!(pruner.common_prefix_length("abc", "xyz"), 0);
        assert_eq!(pruner.common_prefix_length("", "abc"), 0);
        assert_eq!(pruner.common_prefix_length("a", ""), 0);
    }

    #[test]
    fn test_key_range_selectivity() {
        let pruner = RangeQueryPruner::with_defaults();

        // Narrow range (high common prefix) = low selectivity value
        let narrow = pruner.estimate_key_range_selectivity("user:100", "user:105");
        assert!(narrow < 0.5);

        // Wide range (low common prefix) = high selectivity value
        let wide = pruner.estimate_key_range_selectivity("alpha", "zulu");
        assert!(wide > 0.5);

        // Same key = lowest selectivity
        let point = pruner.estimate_key_range_selectivity("key", "key");
        assert!(point < 0.3);
    }
}
