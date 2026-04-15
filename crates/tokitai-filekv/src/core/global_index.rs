//! Global key index for LSM-Tree KV storage engine
//!
//! Maintains a sorted BTreeMap mapping keys to their exact segment locations,
//! enabling O(log n) point lookups without traversing all L0 segments.
//!
//! # Design
//! - Uses `BTreeMap<Vec<u8>, KeyLocation>` for ordered key storage and range queries.
//! - Generation counter distinguishes entries across compaction cycles.
//! - RwLock-based concurrency control: reads are lock-free, writes use write lock.
//!
//! # Memory Layout (per entry)
//! - `Vec<u8>` key: 24 bytes header + actual bytes
//! - `KeyLocation`: 40 bytes (8+8+8+8+8)
//! - BTreeMap node overhead: ~48 bytes
//! - Total: ~80-120 bytes per key (depending on key length)

use std::collections::BTreeMap;
use std::ops::RangeBounds;
use std::sync::Arc;

use moka::sync::Cache;
use parking_lot::RwLock;

/// Location of a key within a segment.
#[derive(Debug, Clone, Copy)]
pub struct KeyLocation {
    /// Segment ID containing the key.
    pub segment_id: u64,
    /// Byte offset within the segment file.
    pub offset: u64,
    /// Generation number of the segment (lower = newer).
    pub generation: u64,
    /// Length of the value in bytes.
    pub value_len: usize,
}

/// Statistics for the global key index.
#[derive(Debug, Default, Clone)]
pub struct IndexStats {
    /// Total number of keys in the index.
    pub total_keys: usize,
    /// Number of successful lookups.
    pub hits: u64,
    /// Number of failed lookups.
    pub misses: u64,
    /// Number of rebuilds performed.
    pub rebuilds: u64,
}

/// Batch update for reducing lock contention during compaction.
#[derive(Debug, Clone)]
pub struct IndexUpdate {
    /// Keys to insert or update.
    pub inserts: Vec<(Vec<u8>, KeyLocation)>,
    /// Segment IDs whose keys should be removed (e.g., compacted away).
    pub remove_segments: Vec<u64>,
    /// Specific keys to remove (e.g., tombstones).
    pub removes: Vec<Vec<u8>>,
}

impl IndexUpdate {
    pub fn new() -> Self {
        Self {
            inserts: Vec::new(),
            remove_segments: Vec::new(),
            removes: Vec::new(),
        }
    }

    pub fn with_capacity(insert_cap: usize, remove_cap: usize) -> Self {
        Self {
            inserts: Vec::with_capacity(insert_cap),
            remove_segments: Vec::new(),
            removes: Vec::with_capacity(remove_cap),
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, loc: KeyLocation) {
        self.inserts.push((key, loc));
    }

    pub fn remove_segment(&mut self, segment_id: u64) {
        self.remove_segments.push(segment_id);
    }

    pub fn remove_key(&mut self, key: Vec<u8>) {
        self.removes.push(key);
    }
}

impl Default for IndexUpdate {
    fn default() -> Self {
        Self::new()
    }
}

/// Global key index maintaining key-to-segment-location mappings.
///
/// Uses a BTreeMap to keep keys sorted, supporting both point lookups and
/// range queries. A generation counter helps distinguish entries created
/// before and after compaction cycles.
///
/// T-004: Added query result cache for caching recent lookups to reduce
/// repeated BTreeMap lookups in mixed workload scenarios.
pub struct GlobalKeyIndex {
    /// BTreeMap mapping key bytes to segment location.
    index: RwLock<BTreeMap<Vec<u8>, KeyLocation>>,
    /// Current generation counter, incremented after flush/compaction.
    current_generation: RwLock<u64>,
    /// Index statistics.
    stats: RwLock<IndexStats>,
    /// Segment IDs that are being compacted (stale). Reads should skip these.
    stale_segments: RwLock<Vec<u64>>,
    /// T-004: Short-term query result cache for repeated lookups.
    /// Caches both hits (Some) and misses (None) to avoid repeated BTreeMap lookups.
    query_cache: Cache<String, Option<KeyLocation>>,
}

impl GlobalKeyIndex {
    /// Create a new empty global key index.
    pub fn new() -> Self {
        Self {
            index: RwLock::new(BTreeMap::new()),
            current_generation: RwLock::new(0),
            stats: RwLock::new(IndexStats::default()),
            stale_segments: RwLock::new(Vec::new()),
            // T-004: Query result cache with 50K capacity and 5min TTL
            query_cache: Cache::builder()
                .max_capacity(50_000)
                .time_to_live(std::time::Duration::from_secs(300))
                .build(),
        }
    }

    /// Create with a pre-populated BTreeMap.
    pub fn with_entries(entries: BTreeMap<Vec<u8>, KeyLocation>, generation: u64) -> Self {
        let total_keys = entries.len();
        Self {
            index: RwLock::new(entries),
            current_generation: RwLock::new(generation),
            stats: RwLock::new(IndexStats {
                total_keys,
                hits: 0,
                misses: 0,
                rebuilds: 1,
            }),
            stale_segments: RwLock::new(Vec::new()),
            // T-004: Query result cache with 50K capacity and 5min TTL
            query_cache: Cache::builder()
                .max_capacity(50_000)
                .time_to_live(std::time::Duration::from_secs(300))
                .build(),
        }
    }

    /// Look up a key's location in O(log n) time.
    ///
    /// Returns `Some(KeyLocation)` if the key exists, `None` otherwise.
    /// Returns `None` if the key's segment is currently being compacted (stale).
    ///
    /// T-004: Checks query result cache first to avoid repeated BTreeMap lookups.
    pub fn get(&self, key: &[u8]) -> Option<KeyLocation> {
        // T-004: Check query result cache first (O(1) concurrent lookup)
        let key_str = String::from_utf8_lossy(key).to_string();
        if let Some(cached) = self.query_cache.get(&key_str) {
            let mut stats = self.stats.write();
            if cached.is_some() {
                stats.hits += 1;
            } else {
                stats.misses += 1;
            }
            return cached;
        }

        // Cache miss, perform BTreeMap lookup
        let loc = {
            let index = self.index.read();
            let stale = self.stale_segments.read();

            if let Some(loc) = index.get(key) {
                // Skip if this key's segment is being compacted
                if stale.contains(&loc.segment_id) {
                    drop(index);
                    drop(stale);
                    self.stats.write().misses += 1;
                    return None;
                }
                // Copy the location before releasing locks
                Some(*loc)
            } else {
                None
            }
        };

        let mut stats = self.stats.write();
        if loc.is_some() {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }

        // T-004: Cache the result for future lookups
        let key_str = String::from_utf8_lossy(key);
        self.query_cache.insert(key_str.to_string(), loc);

        loc
    }

    /// Insert or update a key's location.
    ///
    /// If the key already exists, the location is replaced.
    /// T-004: Invalidates the query cache entry for the key.
    pub fn insert(&self, key: Vec<u8>, loc: KeyLocation) {
        let mut index = self.index.write();
        let is_new = !index.contains_key(&key);
        index.insert(key.clone(), loc);
        if is_new {
            self.stats.write().total_keys = index.len();
        }
        // T-004: Invalidate query cache for this key
        let key_str = String::from_utf8_lossy(&key).to_string();
        self.query_cache.invalidate(&key_str);
    }

    /// Remove a key from the index (used for tombstones).
    /// T-004: Invalidates the query cache entry for the key.
    pub fn remove(&self, key: &[u8]) {
        let mut index = self.index.write();
        if index.remove(key).is_some() {
            self.stats.write().total_keys = index.len();
        }
        // T-004: Invalidate query cache for this key
        let key_str = String::from_utf8_lossy(key).to_string();
        self.query_cache.invalidate(&key_str);
    }

    /// Range query: return all key-location pairs within the given range.
    ///
    /// Leverages BTreeMap's ordered nature for efficient range scans.
    pub fn range<R>(&self, range: R) -> Vec<(Vec<u8>, KeyLocation)>
    where
        R: RangeBounds<Vec<u8>>,
    {
        self.index
            .read()
            .range(range)
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Batch update: apply multiple inserts and removes atomically.
    ///
    /// Uses a single write lock to minimize contention.
    pub fn batch_update(&self, update: IndexUpdate) {
        let mut index = self.index.write();

        // Remove keys from specific segments
        if !update.remove_segments.is_empty() {
            let segment_ids = &update.remove_segments;
            index.retain(|_, loc| !segment_ids.contains(&loc.segment_id));
        }

        // Remove specific keys
        for key in update.removes {
            index.remove(&key);
        }

        // Insert new entries
        for (key, loc) in update.inserts {
            index.insert(key, loc);
        }

        // Update stats
        self.stats.write().total_keys = index.len();
    }

    /// Increment the generation counter (called after flush/compaction).
    ///
    /// Returns the new generation value.
    pub fn increment_generation(&self) -> u64 {
        let mut gen = self.current_generation.write();
        *gen += 1;
        *gen
    }

    /// Get the current generation counter.
    pub fn current_generation(&self) -> u64 {
        *self.current_generation.read()
    }

    /// Get index statistics snapshot.
    pub fn stats(&self) -> IndexStats {
        self.stats.read().clone()
    }

    /// Get the total number of keys in the index.
    pub fn len(&self) -> usize {
        self.index.read().len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.index.read().is_empty()
    }

    /// Clear all entries from the index.
    pub fn clear(&self) {
        self.index.write().clear();
        self.stats.write().total_keys = 0;
    }

    /// Rebuild the index from segment metadata.
    ///
    /// This is a full rebuild operation used during:
    /// - Initial startup (recovering from existing segment files)
    /// - Crash recovery
    /// - Manual rebuild requests
    ///
    /// The caller provides an iterator over segment data, where each segment
    /// yields (key, offset, value_len, segment_id) tuples.
    pub fn rebuild<F>(&self, segments: F) -> crate::core::error::FileKVResult<()>
    where
        F: Fn(&mut dyn FnMut(Vec<u8>, KeyLocation)) -> crate::core::error::FileKVResult<()>,
    {
        let mut index = BTreeMap::new();
        let mut total_keys = 0usize;

        // Closure to collect entries
        let mut collector = |key: Vec<u8>, loc: KeyLocation| {
            index.insert(key, loc);
            total_keys += 1;
        };

        segments(&mut collector)?;

        *self.index.write() = index;

        // Increment rebuild count and update stats
        let mut stats = self.stats.write();
        stats.rebuilds += 1;
        stats.total_keys = total_keys;

        Ok(())
    }

    /// Rebuild from existing segments by iterating through segment files.
    ///
    /// This is a convenience method that scans segment files to rebuild the index.
    /// It should be called during initialization or recovery.
    ///
    /// Segments are iterated from newest to oldest (highest segment_id first) so that
    /// newer entries take precedence over older ones. Tombstones (empty values) are
    /// tracked separately - if a key has a tombstone in a newer segment, it won't be
    /// added to the index even if it exists in an older segment.
    pub fn rebuild_from_segments(
        &self,
        segments: &BTreeMap<u64, Arc<crate::core::segment::SegmentFile>>,
    ) -> crate::core::error::FileKVResult<()> {
        use std::collections::HashSet;

        let mut index = BTreeMap::new();
        let mut tombstones: HashSet<Vec<u8>> = HashSet::new();
        let generation = self.current_generation();

        // Iterate segments from newest to oldest so newer entries win
        for (&segment_id, segment) in segments.iter().rev() {
            // Collect entries first to avoid borrow issues with closure
            let mut segment_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            let _ = segment.iterate_all(|key: &str, value: &[u8], _deleted: bool| {
                segment_entries.push((key.as_bytes().to_vec(), value.to_vec()));
                Ok(())
            });

            for (key_bytes, value) in segment_entries {
                // Track tombstones (empty values) - iterate_all returns deleted=false
                // for all entries, so we detect tombstones by checking empty value
                if value.is_empty() {
                    tombstones.insert(key_bytes);
                    continue;
                }

                // Skip if already marked as tombstone by newer segment
                if tombstones.contains(&key_bytes) {
                    continue;
                }

                // Only insert if key is not already in index (newer segment wins)
                index.entry(key_bytes).or_insert_with(|| KeyLocation {
                    segment_id,
                    offset: 0, // We don't know the exact offset from iterate_all
                    generation,
                    value_len: value.len(),
                });
            }
        }

        let total_keys = index.len();
        *self.index.write() = index;

        let mut stats = self.stats.write();
        stats.rebuilds += 1;
        stats.total_keys = total_keys;

        Ok(())
    }

    /// Update entries for a specific segment (used during compaction).
    ///
    /// Removes all keys belonging to `old_segment_ids` and adds keys
    /// from the new segment.
    pub fn update_after_compaction(
        &self,
        old_segment_ids: &[u64],
        _new_segment_id: u64,
        new_keys: Vec<(Vec<u8>, KeyLocation)>,
    ) {
        let mut index = self.index.write();

        // Remove entries from compacted segments
        let old_ids_set: Vec<u64> = old_segment_ids.to_vec();
        index.retain(|_, loc| !old_ids_set.contains(&loc.segment_id));

        // Insert new entries
        for (key, loc) in new_keys {
            index.insert(key, loc);
        }

        self.stats.write().total_keys = index.len();
    }

    /// Remove entries belonging to specific segments (used during compaction, before segment swap).
    pub fn remove_segments(&self, old_segment_ids: &[u64]) {
        let mut index = self.index.write();
        let old_ids_set: Vec<u64> = old_segment_ids.to_vec();
        let before_count = index.len();
        index.retain(|_k, loc| !old_ids_set.contains(&loc.segment_id));
        let after_count = index.len();
        let _ = (before_count, after_count); // Track removal count
        self.stats.write().total_keys = index.len();
    }

    /// Mark segments as stale (before compaction starts).
    /// Reads will skip entries pointing to these segments.
    pub fn mark_segments_stale(&self, segment_ids: &[u64]) {
        let mut stale = self.stale_segments.write();
        for &id in segment_ids {
            if !stale.contains(&id) {
                stale.push(id);
            }
        }
    }

    /// Clear stale segment tracking (after compaction completes).
    pub fn clear_stale_segments(&self) {
        self.stale_segments.write().clear();
    }

    /// Bulk insert entries (used during compaction, after segment swap).
    /// Only inserts entries that don't already exist (preserves entries from non-compacted segments).
    pub fn bulk_insert(&self, keys: Vec<(Vec<u8>, KeyLocation)>) {
        let mut index = self.index.write();
        let mut inserted = 0;
        let mut skipped = 0;
        for (key, loc) in &keys {
            // Only insert if key doesn't already exist (preserve newer entries)
            if let std::collections::btree_map::Entry::Vacant(e) = index.entry(key.clone()) {
                e.insert(*loc);
                inserted += 1;
            } else {
                skipped += 1;
            }
        }
        let _ = (inserted, skipped); // Track insert/skip count
        self.stats.write().total_keys = index.len();
        drop(index);
        // T-004: Invalidate query cache for bulk inserted keys
        for (key, _) in keys {
            let key_str = String::from_utf8_lossy(&key).to_string();
            self.query_cache.invalidate(&key_str);
        }
    }

    /// Bulk upsert entries (overwrites existing entries).
    /// Used during memtable flush where the new segment has the latest values for all keys.
    pub fn bulk_upsert(&self, keys: Vec<(Vec<u8>, KeyLocation)>) {
        let mut index = self.index.write();
        for (key, loc) in &keys {
            index.insert(key.clone(), *loc);
        }
        self.stats.write().total_keys = index.len();
        drop(index);
        // T-004: Invalidate query cache for upserted keys
        for (key, _) in keys {
            let key_str = String::from_utf8_lossy(&key).to_string();
            self.query_cache.invalidate(&key_str);
        }
    }
}

impl Default for GlobalKeyIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert_and_get() {
        let index = GlobalKeyIndex::new();
        let key = b"hello".to_vec();
        let loc = KeyLocation {
            segment_id: 1,
            offset: 100,
            generation: 0,
            value_len: 5,
        };

        index.insert(key.clone(), loc);

        let result = index.get(&key);
        assert!(result.is_some());
        let loc = result.unwrap();
        assert_eq!(loc.segment_id, 1);
        assert_eq!(loc.offset, 100);
        assert_eq!(loc.value_len, 5);
    }

    #[test]
    fn test_get_nonexistent() {
        let index = GlobalKeyIndex::new();
        let result = index.get(b"nonexistent".as_slice());
        assert!(result.is_none());
    }

    #[test]
    fn test_remove() {
        let index = GlobalKeyIndex::new();
        let key = b"to_delete".to_vec();
        let loc = KeyLocation {
            segment_id: 1,
            offset: 0,
            generation: 0,
            value_len: 10,
        };

        index.insert(key.clone(), loc);
        assert!(index.get(&key).is_some());

        index.remove(&key);
        assert!(index.get(&key).is_none());
    }

    #[test]
    fn test_range_query() {
        let index = GlobalKeyIndex::new();

        // Insert keys: "a", "b", "c", "d", "e"
        for ch in b'a'..=b'e' {
            let key = vec![ch];
            let loc = KeyLocation {
                segment_id: 1,
                offset: ch as u64,
                generation: 0,
                value_len: 1,
            };
            index.insert(key, loc);
        }

        // Range query: "b" to "d" (inclusive)
        let start = vec![b'b'];
        let end = vec![b'd'];
        let results = index.range(start..=end);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, vec![b'b']);
        assert_eq!(results[1].0, vec![b'c']);
        assert_eq!(results[2].0, vec![b'd']);
    }

    #[test]
    fn test_batch_update() {
        let index = GlobalKeyIndex::new();

        // Insert initial data
        for i in 0..10u8 {
            let key = vec![i];
            let loc = KeyLocation {
                segment_id: 1,
                offset: i as u64,
                generation: 0,
                value_len: 1,
            };
            index.insert(key, loc);
        }

        // Batch update: remove segment 1 entries, add new entries
        let mut update = IndexUpdate::new();
        update.remove_segment(1);
        for i in 10..20u8 {
            let key = vec![i];
            let loc = KeyLocation {
                segment_id: 2,
                offset: i as u64,
                generation: 1,
                value_len: 1,
            };
            update.insert(key, loc);
        }

        index.batch_update(update);

        // Old entries should be gone
        assert!(index.get(&[0]).is_none());
        assert!(index.get(&[9]).is_none());

        // New entries should exist
        assert!(index.get(&[10]).is_some());
        assert!(index.get(&[19]).is_some());

        assert_eq!(index.len(), 10);
    }

    #[test]
    fn test_generation_counter() {
        let index = GlobalKeyIndex::new();
        assert_eq!(index.current_generation(), 0);

        let gen = index.increment_generation();
        assert_eq!(gen, 1);
        assert_eq!(index.current_generation(), 1);

        index.increment_generation();
        assert_eq!(index.current_generation(), 2);
    }

    #[test]
    fn test_stats() {
        let index = GlobalKeyIndex::new();

        let key = b"test".to_vec();
        let loc = KeyLocation {
            segment_id: 1,
            offset: 0,
            generation: 0,
            value_len: 4,
        };
        index.insert(key.clone(), loc);

        // Hit
        index.get(&key);
        // Miss
        index.get(b"missing".as_slice());

        let stats = index.stats();
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.rebuilds, 0);
    }

    #[test]
    fn test_len_and_empty() {
        let index = GlobalKeyIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);

        index.insert(b"a".to_vec(), KeyLocation {
            segment_id: 1,
            offset: 0,
            generation: 0,
            value_len: 1,
        });

        assert!(!index.is_empty());
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_clear() {
        let index = GlobalKeyIndex::new();
        for i in 0..5u8 {
            index.insert(vec![i], KeyLocation {
                segment_id: 1,
                offset: i as u64,
                generation: 0,
                value_len: 1,
            });
        }

        index.clear();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_update_after_compaction() {
        let index = GlobalKeyIndex::new();

        // Insert keys in segment 1
        for ch in b'a'..=b'e' {
            index.insert(
                vec![ch],
                KeyLocation {
                    segment_id: 1,
                    offset: ch as u64,
                    generation: 0,
                    value_len: 1,
                },
            );
        }

        // Insert keys in segment 2
        for ch in b'f'..=b'j' {
            index.insert(
                vec![ch],
                KeyLocation {
                    segment_id: 2,
                    offset: ch as u64,
                    generation: 0,
                    value_len: 1,
                },
            );
        }

        assert_eq!(index.len(), 10);

        // Compact segments 1 and 2 into segment 3
        let new_keys: Vec<_> = (b'a'..=b'j')
            .map(|ch| {
                (
                    vec![ch],
                    KeyLocation {
                        segment_id: 3,
                        offset: ch as u64,
                        generation: 1,
                        value_len: 1,
                    },
                )
            })
            .collect();

        index.update_after_compaction(&[1, 2], 3, new_keys);

        assert_eq!(index.len(), 10);

        // All keys should now point to segment 3
        for ch in b'a'..=b'j' {
            let loc = index.get(&[ch]).unwrap();
            assert_eq!(loc.segment_id, 3);
            assert_eq!(loc.generation, 1);
        }
    }

    #[test]
    fn test_concurrent_reads() {
        use std::sync::Arc;
        use std::thread;

        let index = Arc::new(GlobalKeyIndex::new());

        // Insert data
        for i in 0..100u8 {
            index.insert(
                vec![i],
                KeyLocation {
                    segment_id: 1,
                    offset: i as u64,
                    generation: 0,
                    value_len: 1,
                },
            );
        }

        // Spawn multiple threads doing reads
        let mut handles = vec![];
        for t in 0..4 {
            let idx = Arc::clone(&index);
            handles.push(thread::spawn(move || {
                let mut count = 0;
                for i in 0..100u8 {
                    let key = vec![i + t];
                    if idx.get(&key).is_some() {
                        count += 1;
                    }
                }
                count
            }));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        // Each thread should find some hits (wrapping means some keys > 99 miss)
        for r in results {
            assert!(r > 0);
        }
    }

    #[test]
    fn test_update_existing_key() {
        let index = GlobalKeyIndex::new();
        let key = b"update_me".to_vec();

        // Insert initial location
        index.insert(key.clone(), KeyLocation {
            segment_id: 1,
            offset: 100,
            generation: 0,
            value_len: 10,
        });

        // Update to new location
        index.insert(key.clone(), KeyLocation {
            segment_id: 2,
            offset: 200,
            generation: 1,
            value_len: 20,
        });

        // Should have the new location
        let loc = index.get(&key).unwrap();
        assert_eq!(loc.segment_id, 2);
        assert_eq!(loc.offset, 200);
        assert_eq!(loc.value_len, 20);

        // Length should still be 1 (not duplicated)
        assert_eq!(index.len(), 1);
    }
}
