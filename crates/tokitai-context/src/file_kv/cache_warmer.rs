//! Cache Warmer module
//!
//! Pre-loads hot data into BlockCache on startup to improve initial read performance.
//!
//! # Features
//! - Analyzes segment files to identify hot data patterns
//! - Pre-loads frequently accessed data blocks into cache
//! - Configurable warming strategies (recent, frequent, size-based)
//! - Progress tracking and statistics
//!
//! # Warming Strategies
//! - **Recent**: Load most recently written entries (tail of segments)
//! - **Frequent**: Load entries from segments with highest entry density
//! - **SizeBased**: Load entries within optimal size range for cache efficiency
//! - **Hybrid**: Combination of all strategies with configurable weights

use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::error::{ContextResult, ContextError};
use crate::block_cache::BlockCache;
use super::segment::SegmentFile;

/// Cache warming strategy configuration
#[derive(Debug, Clone)]
pub struct CacheWarmingConfig {
    /// Enable cache warming on startup
    pub enabled: bool,
    /// Maximum number of entries to warm (prevents cache pollution)
    pub max_entries: usize,
    /// Maximum memory to use for warming (bytes)
    pub max_memory_bytes: usize,
    /// Minimum entry size to cache (filter out tiny entries)
    pub min_entry_size: usize,
    /// Maximum entry size to cache (avoid caching huge blobs)
    pub max_entry_size: usize,
    /// Warming strategy
    pub strategy: WarmingStrategy,
    /// Number of recent entries per segment to warm (for Recent strategy)
    pub recent_entries_per_segment: usize,
    /// Entry size weight in hybrid strategy (0.0-1.0)
    pub size_weight: f64,
    /// Recency weight in hybrid strategy (0.0-1.0)
    pub recency_weight: f64,
    /// Density weight in hybrid strategy (0.0-1.0)
    pub density_weight: f64,
}

impl Default for CacheWarmingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            max_memory_bytes: 16 * 1024 * 1024, // 16MB
            min_entry_size: 64,                  // Skip entries < 64 bytes
            max_entry_size: 64 * 1024,           // Skip entries > 64KB
            strategy: WarmingStrategy::Hybrid,
            recent_entries_per_segment: 50,
            size_weight: 0.3,
            recency_weight: 0.4,
            density_weight: 0.3,
        }
    }
}

/// Cache warming strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmingStrategy {
    /// Load most recently written entries
    Recent,
    /// Load entries from high-density segments
    Frequent,
    /// Load entries within optimal size range
    SizeBased,
    /// Combination of all strategies
    Hybrid,
}

/// Statistics from cache warming operation
#[derive(Debug, Clone, Default)]
pub struct CacheWarmingStats {
    /// Number of segments analyzed
    pub segments_analyzed: usize,
    /// Number of entries considered for warming
    pub entries_scanned: usize,
    /// Number of entries actually loaded into cache
    pub entries_loaded: usize,
    /// Number of entries skipped (too small/large)
    pub entries_skipped: usize,
    /// Total memory used by warmed entries (bytes)
    pub memory_used: usize,
    /// Time taken for warming (milliseconds)
    pub warming_time_ms: u64,
    /// Whether warming completed successfully
    pub completed: bool,
}

impl CacheWarmingStats {
    /// Get memory used in KB
    pub fn memory_used_kb(&self) -> f64 {
        self.memory_used as f64 / 1024.0
    }

    /// Get memory used in MB
    pub fn memory_used_mb(&self) -> f64 {
        self.memory_used as f64 / (1024.0 * 1024.0)
    }

    /// Get cache load efficiency (entries per MB)
    pub fn entries_per_mb(&self) -> f64 {
        if self.memory_used > 0 {
            self.entries_loaded as f64 / (self.memory_used as f64 / (1024.0 * 1024.0))
        } else {
            0.0
        }
    }

    /// Get skip rate
    pub fn skip_rate(&self) -> f64 {
        let total = self.entries_scanned + self.entries_skipped;
        if total > 0 {
            self.entries_skipped as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// Entry candidate for cache warming
#[derive(Debug, Clone)]
struct WarmingCandidate {
    segment_id: u64,
    offset: u64,
    key: String,
    value_len: usize,
    /// Score for ranking (higher = more important to cache)
    score: f64,
    /// Segment index (for recency calculation)
    segment_index: usize,
}

/// Cache Warmer - pre-loads hot data into BlockCache
pub struct CacheWarmer {
    config: CacheWarmingConfig,
    cache: Arc<BlockCache>,
}

impl CacheWarmer {
    /// Create a new CacheWarmer
    pub fn new(config: CacheWarmingConfig, cache: Arc<BlockCache>) -> Self {
        Self { config, cache }
    }

    /// Warm cache from segment files
    ///
    /// Analyzes segments and pre-loads hot data into cache
    pub fn warm(&self, segments: &[Arc<SegmentFile>]) -> ContextResult<CacheWarmingStats> {
        if !self.config.enabled {
            debug!("Cache warming is disabled");
            return Ok(CacheWarmingStats::default());
        }

        if segments.is_empty() {
            debug!("No segments to warm cache from");
            return Ok(CacheWarmingStats::default());
        }

        let start_time = std::time::Instant::now();
        let mut stats = CacheWarmingStats {
            segments_analyzed: segments.len(),
            ..Default::default()
        };

        info!(
            "Starting cache warming: {} segments, strategy: {:?}",
            segments.len(),
            self.config.strategy
        );

        // Collect candidates from all segments
        let mut candidates = Vec::new();
        
        for (seg_idx, segment) in segments.iter().enumerate() {
            match self.scan_segment_for_candidates(segment, seg_idx, segments.len()) {
                Ok(mut seg_candidates) => {
                    stats.entries_scanned += seg_candidates.len();
                    candidates.append(&mut seg_candidates);
                }
                Err(e) => {
                    warn!("Failed to scan segment {}: {}", segment.id, e);
                }
            }

            // Early exit if we have enough candidates
            if candidates.len() >= self.config.max_entries * 2 {
                break;
            }
        }

        if candidates.is_empty() {
            info!("No candidates found for cache warming");
            stats.completed = true;
            stats.warming_time_ms = start_time.elapsed().as_millis() as u64;
            return Ok(stats);
        }

        // Score and rank candidates based on strategy
        self.score_candidates(&mut candidates);

        // Sort by score (descending)
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Load top candidates into cache
        let mut memory_used = 0;
        let mut entries_loaded = 0;
        let mut entries_skipped = 0;

        for candidate in &candidates {
            // Check limits
            if entries_loaded >= self.config.max_entries {
                break;
            }

            if memory_used + candidate.value_len > self.config.max_memory_bytes {
                break;
            }

            // Skip entries outside size range
            if candidate.value_len < self.config.min_entry_size
                || candidate.value_len > self.config.max_entry_size
            {
                entries_skipped += 1;
                continue;
            }

            // Find the segment for this candidate
            let Some(segment) = segments.iter().find(|s| s.id == candidate.segment_id) else {
                warn!("Segment {} not found for candidate", candidate.segment_id);
                entries_skipped += 1;
                continue;
            };

            // Read entry from segment and load into cache
            match segment.read_entry(candidate.offset) {
                Ok((key, value, _checksum)) => {
                    // Verify key matches candidate
                    if key != candidate.key {
                        warn!("Key mismatch at offset {}: expected '{}', got '{}'",
                             candidate.offset, candidate.key, key);
                        entries_skipped += 1;
                        continue;
                    }

                    // Load into cache
                    let value_arc: Arc<[u8]> = Arc::from(value.into_boxed_slice());
                    self.cache.put(
                        candidate.segment_id,
                        candidate.offset,
                        value_arc,
                    );

                    memory_used += candidate.value_len;
                    entries_loaded += 1;
                }
                Err(e) => {
                    warn!("Failed to read entry at {}.{}: {}", candidate.segment_id, candidate.offset, e);
                    entries_skipped += 1;
                }
            }
        }

        stats.entries_loaded = entries_loaded;
        stats.entries_skipped = entries_skipped;
        stats.memory_used = memory_used;
        stats.warming_time_ms = start_time.elapsed().as_millis() as u64;
        stats.completed = true;

        info!(
            "Cache warming completed: {} entries loaded ({} KB), {} skipped, {} ms",
            entries_loaded,
            stats.memory_used_kb(),
            entries_skipped,
            stats.warming_time_ms
        );

        Ok(stats)
    }

    /// Scan a segment file for warming candidates
    fn scan_segment_for_candidates(
        &self,
        segment: &SegmentFile,
        segment_index: usize,
        total_segments: usize,
    ) -> ContextResult<Vec<WarmingCandidate>> {
        let mut candidates = Vec::new();

        // Get segment stats
        let entry_count = segment.entry_count();
        let size = segment.size();

        if entry_count == 0 || size <= 8 {
            // Empty segment or just header (magic + version = 8 bytes)
            return Ok(candidates);
        }

        // Scan segment entries from the beginning (skip header)
        let mut offset = 8u64; // Skip magic (4 bytes) and version (4 bytes)
        let mut entries_scanned = 0;
        let max_entries_to_scan = match self.config.strategy {
            WarmingStrategy::Recent => self.config.recent_entries_per_segment * 2,
            _ => entry_count as usize * 2, // Scan at most 2x target
        };

        // For Recent strategy, start from near the end
        if self.config.strategy == WarmingStrategy::Recent {
            // Estimate: start from last N entries
            // This is approximate - real impl would use index
            offset = size.saturating_sub(entry_count * 100); // Rough estimate
        }

        while entries_scanned < max_entries_to_scan && offset < size {
            match segment.read_entry(offset) {
                Ok((key, value, _checksum)) => {
                    let value_len = value.len();
                    let key_len = key.len();
                    
                    // Calculate score based on strategy
                    let score = self.calculate_candidate_score(
                        segment_index,
                        total_segments,
                        value_len,
                        entries_scanned,
                    );

                    candidates.push(WarmingCandidate {
                        segment_id: segment.id,
                        offset,
                        key,
                        value_len,
                        score,
                        segment_index,
                    });

                    // Move to next entry
                    offset += 8 + key_len as u64 + value_len as u64 + 4; // key_len + key + value_len + value + checksum
                    entries_scanned += 1;
                }
                Err(_) => {
                    // Reached end of valid entries or read error
                    break;
                }
            }
        }

        Ok(candidates)
    }

    /// Calculate score for a candidate based on warming strategy
    fn calculate_candidate_score(
        &self,
        segment_index: usize,
        total_segments: usize,
        value_len: usize,
        _position: usize,
    ) -> f64 {
        match self.config.strategy {
            WarmingStrategy::Recent => {
                // Higher score for more recent segments (higher index)
                if total_segments > 1 {
                    segment_index as f64 / (total_segments - 1) as f64
                } else {
                    0.5
                }
            }
            WarmingStrategy::Frequent => {
                // Score based on entry size (smaller = more can be cached)
                1.0 / (1.0 + (value_len as f64 / 1024.0))
            }
            WarmingStrategy::SizeBased => {
                // Score based on how close to optimal size (1KB)
                let optimal_size = 1024.0;
                let diff = (value_len as f64 - optimal_size).abs();
                1.0 / (1.0 + diff / optimal_size)
            }
            WarmingStrategy::Hybrid => {
                // Combine all factors
                let recency_score = if total_segments > 1 {
                    segment_index as f64 / (total_segments - 1) as f64
                } else {
                    0.5
                };

                let size_score = 1.0 / (1.0 + (value_len as f64 / 1024.0));
                
                let optimal_size = 1024.0;
                let diff = (value_len as f64 - optimal_size).abs();
                let density_score = 1.0 / (1.0 + diff / optimal_size);

                recency_score * self.config.recency_weight
                    + size_score * self.config.size_weight
                    + density_score * self.config.density_weight
            }
        }
    }

    /// Score candidates based on warming strategy
    fn score_candidates(&self, candidates: &mut [WarmingCandidate]) {
        if candidates.is_empty() {
            return;
        }

        let max_segment_index = candidates.iter().map(|c| c.segment_index).max().unwrap_or(0);

        for candidate in candidates.iter_mut() {
            let mut score = 0.0;

            match self.config.strategy {
                WarmingStrategy::Recent => {
                    // Higher score for more recent segments (higher index)
                    if max_segment_index > 0 {
                        score = candidate.segment_index as f64 / max_segment_index as f64;
                    }
                }
                WarmingStrategy::Frequent => {
                    // Score based on entry size (smaller = more can be cached)
                    score = 1.0 / (1.0 + (candidate.value_len as f64 / 1024.0));
                }
                WarmingStrategy::SizeBased => {
                    // Score based on how close to optimal size (1KB)
                    let optimal_size = 1024.0;
                    let diff = (candidate.value_len as f64 - optimal_size).abs();
                    score = 1.0 / (1.0 + diff / optimal_size);
                }
                WarmingStrategy::Hybrid => {
                    // Combine all factors
                    let recency_score = if max_segment_index > 0 {
                        candidate.segment_index as f64 / max_segment_index as f64
                    } else {
                        0.5
                    };

                    let size_score = 1.0 / (1.0 + (candidate.value_len as f64 / 1024.0));
                    
                    let optimal_size = 1024.0;
                    let diff = (candidate.value_len as f64 - optimal_size).abs();
                    let density_score = 1.0 / (1.0 + diff / optimal_size);

                    score = recency_score * self.config.recency_weight
                        + size_score * self.config.size_weight
                        + density_score * self.config.density_weight;
                }
            }

            candidate.score = score;
        }
    }

    /// Get cache warming statistics
    pub fn stats(&self) -> CacheWarmingStats {
        CacheWarmingStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_warming_config_default() {
        let config = CacheWarmingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.max_memory_bytes, 16 * 1024 * 1024);
        assert_eq!(config.strategy, WarmingStrategy::Hybrid);
    }

    #[test]
    fn test_cache_warming_disabled() {
        let cache = Arc::new(BlockCache::new(crate::block_cache::BlockCacheConfig::default()));
        let config = CacheWarmingConfig {
            enabled: false,
            ..Default::default()
        };

        let warmer = CacheWarmer::new(config, cache);
        let stats = warmer.warm(&[]).unwrap();

        assert!(!stats.completed);
        assert_eq!(stats.entries_loaded, 0);
    }

    #[test]
    fn test_warming_strategy_enum() {
        // Test all strategy variants can be created
        let _recent = WarmingStrategy::Recent;
        let _frequent = WarmingStrategy::Frequent;
        let _size_based = WarmingStrategy::SizeBased;
        let _hybrid = WarmingStrategy::Hybrid;
    }

    #[test]
    fn test_cache_warming_stats() {
        let stats = CacheWarmingStats {
            segments_analyzed: 5,
            entries_scanned: 1000,
            entries_loaded: 500,
            entries_skipped: 100,
            memory_used: 5 * 1024 * 1024,
            warming_time_ms: 150,
            completed: true,
        };

        assert!((stats.memory_used_mb() - 5.0).abs() < 0.01);
        assert!(stats.entries_per_mb() > 0.0);
        assert!((stats.skip_rate() - 0.09).abs() < 0.01);
    }
}
