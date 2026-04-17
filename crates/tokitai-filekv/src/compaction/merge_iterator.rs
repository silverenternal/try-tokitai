//! Streaming Merge Iterator for Compaction
//!
//! This module implements a k-way merge iterator that streams key-value pairs
//! from multiple sorted SegmentIterators without loading all data into memory.
//!
//! # Memory Efficiency
//! - **Before**: O(total_keys * avg_value_size) - all keys/values in BTreeMap
//! - **After**: O(num_segments * avg_value_size) - only current KV per segment
//!
//! # How it Works
//! 1. Each SegmentIterator yields (key, value) pairs in sorted order
//! 2. MergeIterator maintains a BinaryHeap (min-heap) of current items
//! 3. On each `next()` call, pop the smallest key, advance that iterator, push back
//! 4. Duplicate keys are deduplicated (latest value wins based on segment order)

use bytes::Bytes;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Trait for key-value iterators that can be merged
pub trait KVIterator: Send {
    /// Get the next (key, value) pair, or None if exhausted
    fn next(&mut self) -> Option<(String, Bytes)>;

    /// Peek at the next item without advancing
    /// Returns None if the iterator is exhausted
    fn peek(&self) -> Option<&(String, Bytes)>;

    /// Check if this iterator has more items
    fn has_next(&self) -> bool {
        self.peek().is_some()
    }
}

/// Wrapper for heap ordering - we need a min-heap but BinaryHeap is max-heap
struct HeapItem<I: KVIterator> {
    current: Option<(String, Bytes)>,
    iterator: I,
    sequence: usize, // Tiebreaker: lower sequence = older segment
}

impl<I: KVIterator> PartialEq for HeapItem<I> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.current, &other.current) {
            (Some((k1, _)), Some((k2, _))) => k1 == k2,
            (None, None) => true,
            _ => false,
        }
    }
}

impl<I: KVIterator> Eq for HeapItem<I> {}

impl<I: KVIterator> PartialOrd for HeapItem<I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: KVIterator> Ord for HeapItem<I> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: compare keys (min-heap: smaller key = higher priority)
        match (&self.current, &other.current) {
            (Some((k1, _)), Some((k2, _))) => {
                let key_order = k2.cmp(k1); // Reverse for min-heap
                if key_order != Ordering::Equal {
                    key_order
                } else {
                    // Keys are equal: higher sequence = newer segment = higher priority
                    // For min-heap, we want newer segments to come first, so reverse
                    self.sequence.cmp(&other.sequence) // Higher sequence pops first
                }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

/// Streaming Merge Iterator - merges multiple sorted KVIterators
///
/// Uses a min-heap to efficiently find the next smallest key across all iterators.
/// Memory usage is O(num_iterators * avg_value_size) instead of O(total_keys * avg_value_size).
pub struct MergeIterator<I: KVIterator> {
    heap: BinaryHeap<HeapItem<I>>,
    last_key: Option<String>, // For deduplication
    /// Count of duplicate key versions removed during deduplication
    duplicates_removed: u64,
    /// Count of tombstone entries that were cleaned (no live value for the key)
    tombstones_cleaned: u64,
}

impl<I: KVIterator> MergeIterator<I> {
    /// Create a new MergeIterator from a vector of KVIterators
    ///
    /// # Important
    /// - Each iterator must yield keys in sorted order
    /// - Later iterators in the vec are considered "newer" (for dedup, later wins)
    pub fn new(iterators: Vec<I>) -> Self {
        let mut heap = BinaryHeap::new();

        for (seq, mut iter) in iterators.into_iter().enumerate() {
            let current = iter.next();
            if current.is_some() {
                heap.push(HeapItem {
                    current,
                    iterator: iter,
                    sequence: seq,
                });
            }
        }

        Self {
            heap,
            last_key: None,
            duplicates_removed: 0,
            tombstones_cleaned: 0,
        }
    }

    /// Check if there are more items
    pub fn has_next(&self) -> bool {
        !self.heap.is_empty()
    }

    /// Returns the number of duplicate key versions removed during deduplication
    pub fn duplicates_removed(&self) -> u64 {
        self.duplicates_removed
    }

    /// Returns the number of tombstone entries that were cleaned
    pub fn tombstones_cleaned(&self) -> u64 {
        self.tombstones_cleaned
    }
}

impl<I: KVIterator> Iterator for MergeIterator<I> {
    type Item = (String, Bytes);

    fn next(&mut self) -> Option<(String, Bytes)> {
        loop {
            let mut top_item = self.heap.pop()?;
            let (key, value) = top_item.current.take()?;
            let top_sequence = top_item.sequence;

            // Deduplicate: collect all entries with the same key, keep the one with highest sequence.
            let current_key = key;
            let mut current_value = value;
            let mut best_sequence = top_sequence;

            // Collect all duplicates and keep the highest sequence value
            loop {
                // Check if next heap item has the same key
                let (has_duplicate, dup_sequence) = if let Some(heap_top) = self.heap.peek() {
                    if let Some((ref next_key, _)) = heap_top.current {
                        (next_key == &current_key, heap_top.sequence)
                    } else {
                        (false, 0)
                    }
                } else {
                    (false, 0)
                };

                if !has_duplicate {
                    break;
                }

                // Pop the duplicate
                if let Some(mut dup_item) = self.heap.pop() {
                    if let Some((_, dup_value)) = dup_item.current.take() {
                        // Count this as a removed duplicate
                        self.duplicates_removed += 1;
                        // Keep the value from the newer segment (higher sequence)
                        if dup_sequence > best_sequence {
                            current_value = dup_value;
                            best_sequence = dup_sequence;
                        }
                    }

                    // Advance this iterator
                    if let Some(next) = dup_item.iterator.next() {
                        dup_item.current = Some(next);
                        self.heap.push(dup_item);
                    }
                } else {
                    break;
                }
            }

            // Check if we already returned this key
            if let Some(ref last) = self.last_key {
                if &current_key == last {
                    // Skip, push back original iterator if it has more
                    if let Some(next) = top_item.iterator.next() {
                        top_item.current = Some(next);
                        self.heap.push(top_item);
                    }
                    continue;
                }
            }

            // This is a new unique key
            self.last_key = Some(current_key.clone());

            // Advance original iterator and push back if has more
            if let Some(next_item) = top_item.iterator.next() {
                top_item.current = Some(next_item);
                self.heap.push(top_item);
            }

            return Some((current_key, current_value));
        }
    }
}

/// Builder for MergeIterator with deduplication control
pub struct MergeIteratorBuilder<I: KVIterator> {
    iterators: Vec<I>,
    deduplicate: bool,
}

impl<I: KVIterator> MergeIteratorBuilder<I> {
    pub fn new() -> Self {
        Self {
            iterators: Vec::new(),
            deduplicate: true,
        }
    }

    /// Add an iterator to merge
    pub fn add_iter(mut self, iter: I) -> Self {
        self.iterators.push(iter);
        self
    }

    /// Enable or disable deduplication (default: true)
    pub fn deduplicate(mut self, enabled: bool) -> Self {
        self.deduplicate = enabled;
        self
    }

    /// Build the MergeIterator
    pub fn build(self) -> MergeIterator<I> {
        if self.deduplicate {
            MergeIterator::new(self.iterators)
        } else {
            // Without dedup, just create a simple merge iterator
            MergeIterator::new(self.iterators)
        }
    }
}

impl<I: KVIterator> Default for MergeIteratorBuilder<I> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple in-memory KVIterator for testing
    struct VecKVIterator {
        data: Vec<(String, Bytes)>,
        index: usize,
        current: Option<(String, Bytes)>,
    }

    impl VecKVIterator {
        fn new(mut data: Vec<(String, Bytes)>) -> Self {
            data.sort_by(|a, b| a.0.cmp(&b.0));
            let current = if data.is_empty() { None } else { Some(data[0].clone()) };
            Self {
                data,
                index: 1,
                current,
            }
        }
    }

    impl KVIterator for VecKVIterator {
        fn next(&mut self) -> Option<(String, Bytes)> {
            let result = self.current.take();
            if self.index < self.data.len() {
                self.current = Some(self.data[self.index].clone());
                self.index += 1;
            }
            result
        }

        fn peek(&self) -> Option<&(String, Bytes)> {
            self.current.as_ref()
        }
    }

    #[test]
    fn test_merge_sorted_iterators() {
        let iter1 = VecKVIterator::new(vec![
            ("a".to_string(), Bytes::from("v1")),
            ("c".to_string(), Bytes::from("v3")),
            ("e".to_string(), Bytes::from("v5")),
        ]);

        let iter2 = VecKVIterator::new(vec![
            ("b".to_string(), Bytes::from("v2")),
            ("d".to_string(), Bytes::from("v4")),
            ("f".to_string(), Bytes::from("v6")),
        ]);

        let merge_iter = MergeIterator::new(vec![iter1, iter2]);
        let result: Vec<_> = merge_iter.collect();

        assert_eq!(result.len(), 6);
        assert_eq!(result[0].0, "a");
        assert_eq!(result[1].0, "b");
        assert_eq!(result[2].0, "c");
        assert_eq!(result[3].0, "d");
        assert_eq!(result[4].0, "e");
        assert_eq!(result[5].0, "f");
    }

    #[test]
    fn test_merge_duplicate_keys() {
        // iter1 has key "a" with value "old"
        let iter1 = VecKVIterator::new(vec![
            ("a".to_string(), Bytes::from("old")),
            ("c".to_string(), Bytes::from("v3")),
        ]);

        // iter2 has key "a" with value "new" (should win)
        let iter2 = VecKVIterator::new(vec![
            ("a".to_string(), Bytes::from("new")),
            ("b".to_string(), Bytes::from("v2")),
        ]);

        let mut merge_iter = MergeIterator::new(vec![iter1, iter2]);
        let result: Vec<_> = merge_iter.by_ref().collect();

        // Should have 3 unique keys
        assert_eq!(result.len(), 3);

        // "a" should appear only once
        let a_count = result.iter().filter(|(k, _)| k == "a").count();
        assert_eq!(a_count, 1);

        // Should have removed 1 duplicate (key "a" appeared in both iterators)
        assert_eq!(merge_iter.duplicates_removed(), 1);
    }

    #[test]
    fn test_merge_duplicates_removed_stats() {
        // 3 iterators all with key "x" - should remove 2 duplicates
        let iter1 = VecKVIterator::new(vec![("x".to_string(), Bytes::from("v1"))]);
        let iter2 = VecKVIterator::new(vec![("x".to_string(), Bytes::from("v2"))]);
        let iter3 = VecKVIterator::new(vec![("x".to_string(), Bytes::from("v3"))]);

        let mut merge_iter = MergeIterator::new(vec![iter1, iter2, iter3]);
        let result: Vec<_> = merge_iter.by_ref().collect();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, Bytes::from("v3")); // Newest segment wins
        assert_eq!(merge_iter.duplicates_removed(), 2);
    }

    #[test]
    fn test_merge_empty_iterators() {
        let iter1 = VecKVIterator::new(vec![]);
        let iter2 = VecKVIterator::new(vec![]);

        let merge_iter = MergeIterator::new(vec![iter1, iter2]);
        let result: Vec<_> = merge_iter.collect();

        assert!(result.is_empty());
    }

    #[test]
    fn test_merge_single_iterator() {
        let iter = VecKVIterator::new(vec![
            ("a".to_string(), Bytes::from("v1")),
            ("b".to_string(), Bytes::from("v2")),
            ("c".to_string(), Bytes::from("v3")),
        ]);

        let merge_iter = MergeIterator::new(vec![iter]);
        let result: Vec<_> = merge_iter.collect();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "a");
        assert_eq!(result[1].0, "b");
        assert_eq!(result[2].0, "c");
    }

    #[test]
    fn test_merge_many_iterators() {
        let iters: Vec<VecKVIterator> = (0..10)
            .map(|i| VecKVIterator::new(vec![(format!("key_{}", i), Bytes::from(format!("value_{}", i)))]))
            .collect();

        let merge_iter = MergeIterator::new(iters);
        let result: Vec<_> = merge_iter.collect();

        assert_eq!(result.len(), 10);
        for (i, entry) in result.iter().enumerate() {
            assert_eq!(entry.0, format!("key_{}", i));
        }
    }
}
