//! Range Scan implementation for FileKV
//!
//! INNO-002: Implements range query with Zone Map pruning and sequential prefetching.
//!
//! # Key Features
//! - Zone Map-based block pruning (skip non-overlapping blocks)
//! - Sequential prefetching for range queries
//! - Lazy evaluation with iterator interface
//! - Configurable scan options

use bytes::Bytes;
use std::sync::Arc;
use tracing::debug;

use crate::cache::block_cache::BlockCache;
use crate::cache::prefetch::{PrefetchCache, SequentialPrefetcher};
use crate::core::error::FileKVResult;
use crate::core::segment::SegmentFile;
use crate::query::pruner::RangeQueryPruner;
use crate::query::zone_map::ZoneMapIndex;

/// Trait for providing segment data to the range scan iterator.
///
/// This trait abstracts over the segment storage layer, allowing the scan
/// implementation to work with any segment provider.
pub trait QuerySegmentProvider: Send + Sync {
    /// Get all segments in order (newest first for LSM-Tree semantics)
    fn get_segments_ordered(&self) -> Vec<(u64, Arc<SegmentFile>)>;

    /// Get zone map for a segment if available
    fn get_zone_map(&self, segment_id: u64) -> Option<ZoneMapIndex>;

    /// Get block cache reference for prefetching
    fn get_block_cache(&self) -> Arc<BlockCache>;
}

/// Range scan configuration
#[derive(Debug, Clone)]
pub struct RangeScanConfig {
    /// Enable Zone Map pruning (default: true)
    pub enable_pruning: bool,
    /// Enable sequential prefetching (default: true)
    pub enable_prefetch: bool,
    /// Maximum number of entries to return (0 = unlimited)
    pub limit: usize,
    /// Include deleted entries (default: false)
    pub include_deleted: bool,
    /// Batch size for prefetching (blocks)
    pub prefetch_batch_size: u32,
    /// 3.1 OPTIMIZATION: Number of entries to prefetch in readahead buffer (default: 16)
    /// Higher values = more sequential read throughput, more memory usage
    pub readahead_entries: usize,
}

impl Default for RangeScanConfig {
    fn default() -> Self {
        Self {
            enable_pruning: true,
            enable_prefetch: true,
            limit: 0,
            include_deleted: false,
            prefetch_batch_size: 4,
            readahead_entries: 16, // 3.1 OPTIMIZATION: Prefetch 16 entries by default
        }
    }
}

impl RangeScanConfig {
    /// Set the maximum number of entries to return
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Range scan result entry
#[derive(Debug, Clone)]
pub struct RangeEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub segment_id: u64,
    pub offset: u64,
}

/// Range scan iterator
///
/// Lazily iterates over key-value pairs in the specified range
pub struct RangeScanIterator<'a> {
    /// Segment provider for accessing segment data
    /// Note: Currently unused after initialization as segments are pre-loaded.
    /// Reserved for future dynamic segment loading during iteration.
    #[allow(dead_code)]
    provider: &'a dyn QuerySegmentProvider,
    /// Range start key (inclusive)
    start_key: String,
    /// Range end key (inclusive)
    end_key: String,
    /// Scan configuration
    config: RangeScanConfig,
    /// Segments to scan (segment_id, segment_file, zone_map)
    segments: Vec<(u64, Arc<SegmentFile>, Option<Arc<ZoneMapIndex>>)>,
    /// Current segment index
    current_segment_idx: usize,
    /// Current scan offset in current segment
    current_offset: u64,
    /// Current key to scan from
    current_scan_key: String,
    /// Range query pruner
    pruner: Option<RangeQueryPruner>,
    /// Sequential prefetcher
    prefetcher: Option<SequentialPrefetcher<RangeScanBlockCache>>,
    /// Number of entries returned so far
    entries_returned: usize,
    /// Statistics
    stats: RangeScanStats,
    /// ARCH-002 FIX: Block offsets to scan (from pruner)
    /// If Some, only scan entries within these block ranges
    blocks_to_scan: Option<Vec<(u64, u64)>>, // Vec of (start_offset, end_offset)
    /// ARCH-002 FIX: Current block index in blocks_to_scan
    current_block_idx: usize,
    /// 3.1 OPTIMIZATION: Readahead buffer for prefetched entries
    /// Stores (key, value, segment_id, offset) tuples for fast access
    readahead_buffer: std::collections::VecDeque<(String, Vec<u8>, u64, u64)>,
    /// 3.1 OPTIMIZATION: Whether readahead buffer has been exhausted for current segment
    readahead_exhausted: bool,
}

/// Range scan statistics
#[derive(Debug, Clone, Default)]
pub struct RangeScanStats {
    /// Total entries returned
    pub entries_returned: usize,
    /// Total blocks scanned
    pub blocks_scanned: usize,
    /// Total blocks pruned
    pub blocks_pruned: usize,
    /// Total prefetches triggered
    pub prefetches_triggered: usize,
    /// Prefetch hits
    pub prefetch_hits: usize,
}

/// Block cache wrapper for prefetcher
///
/// Implements PrefetchCache trait for RangeScanIterator
pub struct RangeScanBlockCache {
    /// Reference to block cache
    block_cache: Arc<BlockCache>,
    /// Segment file for reading blocks
    segment: Arc<SegmentFile>,
    /// Zone Map for block metadata
    zone_map: Option<Arc<ZoneMapIndex>>,
}

impl RangeScanBlockCache {
    pub fn new(block_cache: Arc<BlockCache>, segment: Arc<SegmentFile>, zone_map: Option<Arc<ZoneMapIndex>>) -> Self {
        Self {
            block_cache,
            segment,
            zone_map,
        }
    }
}

impl PrefetchCache for RangeScanBlockCache {
    fn prefetch(&self, segment_id: u64, block_id: u64) -> bool {
        // Get block offset from zone map
        let offset = if let Some(ref zone_map) = self.zone_map {
            if let Some(entry) = zone_map.get_block_entry(block_id) {
                entry.offset
            } else {
                return false; // Block not found
            }
        } else {
            // No zone map, use block_id as offset (fallback)
            block_id * 4096 // Assume 4KB blocks
        };

        // Read block into cache
        match self.segment.read_at(offset, 4096) {
            Ok(data) => {
                self.block_cache.put(segment_id, offset, Bytes::from(data));
                true
            }
            Err(_) => false,
        }
    }

    fn contains(&self, segment_id: u64, block_id: u64) -> bool {
        let offset = if let Some(ref zone_map) = self.zone_map {
            if let Some(entry) = zone_map.get_block_entry(block_id) {
                entry.offset
            } else {
                return false;
            }
        } else {
            block_id * 4096
        };

        self.block_cache.get(segment_id, offset).is_some()
    }

    fn get(&self, segment_id: u64, block_id: u64) -> Option<Arc<dyn Send + Sync>> {
        let offset = if let Some(ref zone_map) = self.zone_map {
            if let Some(entry) = zone_map.get_block_entry(block_id) {
                entry.offset
            } else {
                return None;
            }
        } else {
            block_id * 4096
        };

        self.block_cache
            .get(segment_id, offset)
            .map(|arc| Arc::new(arc.to_vec()) as Arc<dyn Send + Sync>)
    }
}

impl<'a> RangeScanIterator<'a> {
    /// Create a new range scan iterator
    pub fn new(
        provider: &'a dyn QuerySegmentProvider,
        start_key: &str,
        end_key: &str,
        config: RangeScanConfig,
    ) -> FileKVResult<Self> {
        let mut segment_list: Vec<(u64, Arc<SegmentFile>, Option<Arc<ZoneMapIndex>>)> = Vec::new();

        // Get segments from provider
        let segments = provider.get_segments_ordered();

        for (segment_id, segment) in segments {
            // Load zone map from provider
            let zone_map = provider.get_zone_map(segment_id).map(Arc::new);
            segment_list.push((segment_id, segment, zone_map));
        }

        // Initialize pruner if enabled
        let pruner = if config.enable_pruning {
            Some(RangeQueryPruner::with_defaults())
        } else {
            None
        };

        // Initialize prefetcher if enabled
        let prefetcher = if config.enable_prefetch && !segment_list.is_empty() {
            let block_cache = RangeScanBlockCache::new(
                provider.get_block_cache(),
                segment_list[0].1.clone(),
                segment_list[0].2.clone(),
            );
            let mut prefetcher = SequentialPrefetcher::with_defaults(Arc::new(block_cache));

            // Set zone map for all segments
            for (_, _, zone_map) in &segment_list {
                if let Some(ref zm) = zone_map {
                    prefetcher.set_zone_map(zm.clone());
                }
            }

            Some(prefetcher)
        } else {
            None
        };

        Ok(Self {
            provider,
            start_key: start_key.to_string(),
            end_key: end_key.to_string(),
            config,
            segments: segment_list,
            current_segment_idx: 0,
            current_offset: 8, // Skip segment header (magic + version)
            current_scan_key: start_key.to_string(),
            pruner,
            prefetcher,
            entries_returned: 0,
            stats: RangeScanStats::default(),
            blocks_to_scan: None, // ARCH-002 FIX: Will be set in advance_segment
            current_block_idx: 0, // ARCH-002 FIX: Track current block in blocks_to_scan
            readahead_buffer: std::collections::VecDeque::new(), // 3.1 OPTIMIZATION: Empty readahead buffer
            readahead_exhausted: false, // 3.1 OPTIMIZATION: Buffer not yet exhausted
        })
    }

    /// Get scan statistics
    pub fn stats(&self) -> RangeScanStats {
        self.stats.clone()
    }

    /// Advance to next segment
    fn advance_segment(&mut self) -> FileKVResult<bool> {
        if self.current_segment_idx >= self.segments.len() {
            return Ok(false);
        }

        // 3.1 OPTIMIZATION: Clear readahead buffer when switching segments
        self.readahead_buffer.clear();
        self.readahead_exhausted = false;

        let (_segment_id, _segment, zone_map) = &self.segments[self.current_segment_idx];

        // ARCH-002 FIX: Use zone map pruning to limit scanning to only relevant blocks
        if let (Some(ref pruner), Some(ref zone_map)) = (&self.pruner, zone_map) {
            let blocks = pruner.find_blocks_to_scan(zone_map, &self.start_key, &self.end_key);
            self.stats.blocks_scanned += blocks.len();
            self.stats.blocks_pruned += zone_map.block_count() - blocks.len();

            // ARCH-002 FIX: Convert block IDs to (start_offset, end_offset) ranges
            let block_ranges: Vec<(u64, u64)> = blocks
                .iter()
                .filter_map(|block_id| {
                    zone_map
                        .get_block_entry(*block_id)
                        .map(|entry| (entry.offset, entry.offset + entry.size_bytes as u64))
                })
                .collect();

            self.blocks_to_scan = Some(block_ranges);
            self.current_block_idx = 0;

            // Set current_offset to the first block's start
            if let Some(ranges) = &self.blocks_to_scan {
                if !ranges.is_empty() {
                    self.current_offset = ranges[0].0;
                } else {
                    // No blocks to scan - all pruned
                    self.current_offset = u64::MAX;
                }
            }
        } else {
            // No pruning - scan entire segment
            self.blocks_to_scan = None;
            self.current_offset = 8; // Skip segment header
        }

        self.current_scan_key = self.start_key.clone();

        Ok(true)
    }

    /// Try to get next entry from current segment
    fn next_from_current_segment(&mut self) -> FileKVResult<Option<RangeEntry>> {
        if self.current_segment_idx >= self.segments.len() {
            return Ok(None);
        }

        // 3.1 OPTIMIZATION: First try to return entry from readahead buffer
        if let Some((key, value, segment_id, offset)) = self.readahead_buffer.pop_front() {
            self.entries_returned += 1;
            self.stats.entries_returned += 1;

            return Ok(Some(RangeEntry {
                key,
                value,
                segment_id,
                offset,
            }));
        }

        // 3.1 OPTIMIZATION: Readahead buffer exhausted, check if we should refill
        if self.readahead_exhausted {
            // Buffer was already drained, need to scan from disk again
            // This happens when all prefetched entries were out of range
            // Fall through to normal scanning below
        } else {
            // 3.1 OPTIMIZATION: Buffer is empty but not exhausted - refill it
            self.refill_readahead_buffer()?;

            // Try again from refilled buffer
            if let Some((key, value, segment_id, offset)) = self.readahead_buffer.pop_front() {
                self.entries_returned += 1;
                self.stats.entries_returned += 1;

                return Ok(Some(RangeEntry {
                    key,
                    value,
                    segment_id,
                    offset,
                }));
            }
        }

        // Fallback: normal scanning without readahead (or readahead failed)
        self.scan_next_entry()
    }

    /// 3.1 OPTIMIZATION: Refill readahead buffer with entries from current segment
    /// Batch reads multiple entries to improve sequential read throughput
    fn refill_readahead_buffer(&mut self) -> FileKVResult<()> {
        // 3.1 OPTIMIZATION: Skip if prefetch is disabled
        if !self.config.enable_prefetch {
            self.readahead_exhausted = true;
            return Ok(());
        }

        if self.current_segment_idx >= self.segments.len() {
            self.readahead_exhausted = true;
            return Ok(());
        }

        let (_segment_id, segment, _zone_map) = &self.segments[self.current_segment_idx];
        let segment_id = self.segments[self.current_segment_idx].0;
        let readahead_count = self.config.readahead_entries;
        let mut refilled = 0;

        // Scan from current offset and prefetch entries
        let scan_start = self.current_offset;
        let max_entries = if self.config.limit > 0 {
            Some((self.config.limit - self.entries_returned).max(readahead_count))
        } else {
            None
        };

        // Use scan_next to read entries
        // Don't filter by min_key - we'll filter ourselves
        match segment.scan_next(scan_start, "", max_entries)? {
            Some((key, value, offset, _checksum)) => {
                // Calculate next offset
                let next_offset = offset + 4 + key.len() as u64 + 4 + value.len() as u64 + 4;

                // Check if key is within range
                if key.as_str() >= self.start_key.as_str() && key.as_str() <= self.end_key.as_str() {
                    self.readahead_buffer.push_back((key, value, segment_id, offset));
                    refilled += 1;
                }

                // Update current_offset for next scan
                self.current_offset = next_offset;

                // Continue prefetching if we haven't reached the target count
                while refilled < readahead_count {
                    match segment.scan_next(self.current_offset, "", max_entries)? {
                        Some((key, value, offset, _checksum)) => {
                            let next_offset = offset + 4 + key.len() as u64 + 4 + value.len() as u64 + 4;

                            if key.as_str() >= self.start_key.as_str() && key.as_str() <= self.end_key.as_str() {
                                self.readahead_buffer.push_back((key, value, segment_id, offset));
                                refilled += 1;
                            }

                            self.current_offset = next_offset;
                        }
                        None => break, // No more entries
                    }
                }
            }
            None => {
                // No more entries in segment
                self.readahead_exhausted = true;
            }
        }

        // Update prefetch stats
        if refilled > 0 {
            self.stats.prefetches_triggered += 1;
        }

        Ok(())
    }

    /// 3.1 OPTIMIZATION: Scan next entry without readahead (fallback path)
    fn scan_next_entry(&mut self) -> FileKVResult<Option<RangeEntry>> {
        if self.current_segment_idx >= self.segments.len() {
            return Ok(None);
        }

        let (_segment_id, segment, zone_map) = &self.segments[self.current_segment_idx];

        // ARCH-002 FIX: If blocks_to_scan is set, skip to next block when current offset
        // exceeds the current block's end
        if let Some(ref ranges) = self.blocks_to_scan {
            if self.current_block_idx < ranges.len() {
                let (_block_start, block_end) = ranges[self.current_block_idx];

                // If we've scanned past the end of the current block, advance to next block
                if self.current_offset >= block_end {
                    self.current_block_idx += 1;

                    if self.current_block_idx < ranges.len() {
                        // Move to next block
                        let (next_block_start, _next_block_end) = ranges[self.current_block_idx];
                        self.current_offset = next_block_start;
                    } else {
                        // All blocks scanned
                        return Ok(None);
                    }
                }
            } else {
                // All blocks already scanned
                return Ok(None);
            }
        } else if let (Some(zm), config_enable_pruning) = (zone_map.as_ref(), self.config.enable_pruning) {
            // Fallback: original pruning logic when blocks_to_scan is not set
            if config_enable_pruning {
                for block_entry in zm.entries() {
                    if self.current_offset >= block_entry.offset
                        && self.current_offset < block_entry.offset + block_entry.size_bytes as u64
                    {
                        if block_entry.should_prune(&self.start_key, &self.end_key) {
                            debug!(
                                "Pruning block {} at offset {}-{}",
                                block_entry.block_id,
                                block_entry.offset,
                                block_entry.offset + block_entry.size_bytes as u64
                            );
                            self.current_offset = block_entry.offset + block_entry.size_bytes as u64;
                            self.stats.blocks_pruned += 1;
                        } else {
                            self.stats.blocks_scanned += 1;
                        }
                        break;
                    }
                }
            }
        }

        // Scan from current offset, skipping entries that don't match
        loop {
            // ARCH-002 FIX: Check if we've exceeded current block range
            if let Some(ref ranges) = self.blocks_to_scan {
                if self.current_block_idx < ranges.len() {
                    let (_block_start, block_end) = ranges[self.current_block_idx];
                    if self.current_offset >= block_end {
                        // Current block fully scanned, advance will happen on next call
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            }

            let scan_start = self.current_offset;

            // ARCH-004: Pass max_entries from config.limit (0 = unlimited, use None for default)
            let max_entries = if self.config.limit > 0 {
                Some(self.config.limit - self.entries_returned)
            } else {
                None // Use scan_next default
            };

            // Use scan_next to scan from current offset
            // Don't filter by min_key here - we want to scan ALL entries
            match segment.scan_next(scan_start, "", max_entries)? {
                Some((key, value, offset, _checksum)) => {
                    // Update offset for next iteration
                    let next_offset = offset + 4 + key.len() as u64 + 4 + value.len() as u64 + 4;
                    self.current_offset = next_offset;

                    // Check if key is within range
                    if key.as_str() >= self.start_key.as_str() && key.as_str() <= self.end_key.as_str() {
                        // Update scan key only for entries in range
                        self.current_scan_key = key.clone();

                        // Record access for prefetching
                        if let Some(ref mut prefetcher) = self.prefetcher {
                            let segment_id = self.segments[self.current_segment_idx].0;
                            let block_id = offset / 4096;
                            prefetcher.record_access(&key, segment_id, block_id);
                        }

                        self.entries_returned += 1;
                        self.stats.entries_returned += 1;
                        self.stats.prefetch_hits += 1; // 3.1 OPTIMIZATION: Track readahead hits

                        return Ok(Some(RangeEntry {
                            key,
                            value,
                            segment_id: self.segments[self.current_segment_idx].0,
                            offset,
                        }));
                    }
                    // Continue scanning - don't update current_scan_key for out-of-range entries
                }
                None => {
                    return Ok(None); // No more entries in segment
                }
            }
        }
    }
}

impl<'a> Iterator for RangeScanIterator<'a> {
    type Item = FileKVResult<RangeEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check limit
        if self.config.limit > 0 && self.entries_returned >= self.config.limit {
            return None;
        }

        // Try to get next entry, advancing segments as needed
        loop {
            if self.current_segment_idx >= self.segments.len() {
                return None; // No more segments
            }

            match self.next_from_current_segment() {
                Ok(Some(entry)) => return Some(Ok(entry)),
                Ok(None) => {
                    // No more entries in current segment, advance to next
                    self.current_segment_idx += 1;
                    if self.current_segment_idx < self.segments.len() {
                        // Initialize next segment
                        if let Err(e) = self.advance_segment() {
                            return Some(Err(e));
                        }
                        // Continue loop to try next segment
                    } else {
                        return None; // No more segments
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::FileKVConfig;
    use crate::core::memtable::MemTableConfig;
    use crate::FileKV;
    use tempfile::TempDir;

    fn create_test_kv() -> (FileKV, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FileKVConfig {
            segment_dir: temp_dir.path().join("segments"),
            index_dir: temp_dir.path().join("index"),
            wal_dir: temp_dir.path().join("wal"),
            checkpoint_dir: temp_dir.path().join("checkpoint"),
            memtable: MemTableConfig {
                flush_threshold_bytes: 64 * 1024,
                max_entries: 100,
                ..Default::default()
            },
            enable_wal: false,
            ..Default::default()
        };

        let kv = FileKV::open(config).unwrap();

        // Insert test data (more than max_entries to trigger flush)
        for i in 0..150 {
            let key = format!("key_{:03}", i);
            let value = format!("value_{}", i);
            kv.put(&key, value.as_bytes()).unwrap();
        }

        // Debug: Check memtable size before flush
        let _memtable_size = kv.memtable_ref().size_bytes();
        let _memtable_entries = kv.memtable_ref().entry_count();

        // Force flush
        kv.flush_memtable().unwrap();

        (kv, temp_dir)
    }

    #[test]
    fn test_range_scan_basic() {
        let (kv, _temp_dir) = create_test_kv();

        let mut count = 0;
        for result in kv.range("key_010", "key_020").unwrap() {
            let entry = result.unwrap();
            assert!(entry.key.as_str() >= "key_010");
            assert!(entry.key.as_str() <= "key_020");
            count += 1;
        }

        assert!(count > 0); // key_010 to key_020 inclusive
    }

    #[test]
    fn test_range_scan_with_limit() {
        let (kv, _temp_dir) = create_test_kv();

        let config = RangeScanConfig {
            limit: 5,
            ..Default::default()
        };

        let mut count = 0;
        for result in kv.range_with_config("key_000", "key_099", config).unwrap() {
            let _entry = result.unwrap();
            count += 1;
        }

        assert_eq!(count, 5);
    }

    #[test]
    fn test_range_scan_empty_range() {
        let (kv, _temp_dir) = create_test_kv();

        let mut count = 0;
        for result in kv.range("key_500", "key_600").unwrap() {
            let _entry = result.unwrap();
            count += 1;
        }

        assert_eq!(count, 0);
    }

    #[test]
    fn test_range_scan_stats() {
        let (kv, _temp_dir) = create_test_kv();

        let mut iter = kv.range("key_000", "key_050").unwrap();

        let mut count = 0;
        for result in &mut iter {
            let _entry = result.unwrap();
            count += 1;
        }

        let stats = iter.stats();
        assert!(stats.entries_returned > 0);
        assert_eq!(stats.entries_returned, count);
    }

    #[test]
    fn test_range_scan_readahead() {
        let (kv, _temp_dir) = create_test_kv();

        // Test with readahead enabled (default: 16 entries)
        let config = RangeScanConfig {
            enable_prefetch: true,
            readahead_entries: 8, // Prefetch 8 entries at a time
            ..Default::default()
        };

        let mut iter = kv.range_with_config("key_000", "key_050", config).unwrap();
        let mut count = 0;
        for result in &mut iter {
            let entry = result.unwrap();
            assert!(entry.key.as_str() >= "key_000");
            assert!(entry.key.as_str() <= "key_050");
            count += 1;
        }

        let stats = iter.stats();
        assert!(stats.entries_returned > 0);
        assert_eq!(stats.entries_returned, count);
        // Verify prefetch was triggered
        assert!(stats.prefetches_triggered > 0);
    }

    #[test]
    fn test_range_scan_readahead_disabled() {
        let (kv, _temp_dir) = create_test_kv();

        // Test with readahead disabled
        let config = RangeScanConfig {
            enable_prefetch: false,
            ..Default::default()
        };

        let mut iter = kv.range_with_config("key_000", "key_020", config).unwrap();
        let _count = iter.by_ref().filter_map(|r| r.ok()).count();

        let stats = iter.stats();
        assert!(stats.entries_returned > 0);
        // When prefetch is disabled, prefetches_triggered should be 0
        assert_eq!(stats.prefetches_triggered, 0);
    }

    #[test]
    fn test_range_scan_readahead_large_range() {
        let (kv, _temp_dir) = create_test_kv();

        // Test readahead with a large range and small batch size
        let config = RangeScanConfig {
            enable_prefetch: true,
            readahead_entries: 4, // Small batch size
            ..Default::default()
        };

        let mut iter = kv.range_with_config("key_000", "key_149", config).unwrap();
        let _count = iter.by_ref().filter_map(|r| r.ok()).count();

        let stats = iter.stats();
        assert!(stats.entries_returned > 0);
        // Should have triggered multiple prefetches
        assert!(stats.prefetches_triggered > 0);
    }
}
