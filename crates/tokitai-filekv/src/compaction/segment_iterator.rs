//! Segment Iterator for Streaming Compaction
//!
//! Provides a streaming iterator over all key-value pairs in a segment file.
//! Used by the MergeIterator to read segments during compaction without
//! loading all data into memory.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use bytes::Bytes;
use crate::core::segment::SegmentFile;
use crate::core::error::FatalError;
use super::merge_iterator::KVIterator;

/// Streaming iterator over a single segment's key-value pairs
///
/// Instead of loading all keys into a BTreeMap, this iterator
/// streams entries directly from the segment's mmap, yielding
/// (key, value) pairs one at a time in file order (which is sorted by key).
pub struct SegmentIterator {
    #[allow(dead_code)] // Kept for potential future use (segment metadata access)
    segment: Arc<SegmentFile>,
    /// Current position in the segment file (starts after header)
    pos: usize,
    /// File size for bounds checking
    file_size: usize,
    /// Cached mmap reference for efficient reading
    mmap_data: Option<Vec<u8>>,
    /// Current peek buffer
    current: Option<(String, Bytes)>,
    /// Shared counter for tombstone entries skipped during iteration
    tombstones_skipped: Arc<AtomicU64>,
}

impl SegmentIterator {
    /// Create a new SegmentIterator
    ///
    /// # Arguments
    /// * `segment` - The segment to iterate over
    ///
    /// # Returns
    /// * `Ok(SegmentIterator)` - Iterator created successfully
    /// * `Err(FatalError)` - Failed to read segment
    pub fn new(segment: Arc<SegmentFile>) -> Result<Self, FatalError> {
        Self::with_tombstone_counter(segment, None)
    }

    /// Create a new SegmentIterator with an optional shared tombstone counter
    ///
    /// # Arguments
    /// * `segment` - The segment to iterate over
    /// * `tombstone_counter` - Optional shared counter to track tombstones.
    ///   If None, a new counter is created internally.
    ///
    /// # Returns
    /// * `Ok(SegmentIterator)` - Iterator created successfully
    /// * `Err(FatalError)` - Failed to read segment
    pub fn with_tombstone_counter(
        segment: Arc<SegmentFile>,
        tombstone_counter: Option<Arc<AtomicU64>>,
    ) -> Result<Self, FatalError> {
        // Flush any pending writes before iterating
        segment.flush()?;

        // Get file size
        let file_size = segment.size() as usize;

        // Read all data into memory for iteration
        let mmap_data = if file_size > 8 {
            segment.read_segment_data()?
        } else {
            Vec::new()
        };

        let counter = tombstone_counter.unwrap_or_else(|| Arc::new(AtomicU64::new(0)));

        let mut iter = Self {
            segment,
            pos: 8,
            file_size,
            mmap_data: Some(mmap_data),
            current: None,
            tombstones_skipped: counter,
        };

        iter.current = iter.read_next_entry();
        Ok(iter)
    }

    /// Returns the shared tombstone counter used by this iterator
    pub fn tombstone_counter(&self) -> &Arc<AtomicU64> {
        &self.tombstones_skipped
    }

    /// Read the next entry from the current position
    fn read_next_entry(&mut self) -> Option<(String, Bytes)> {
        let data = self.mmap_data.as_ref()?;

        while self.pos + 4 <= self.file_size {

            // Read key length
            if self.pos + 4 > self.file_size {
                break;
            }
            let key_len = u32::from_le_bytes(
                data[self.pos..self.pos + 4].try_into().ok()?
            ) as usize;
            self.pos += 4;

            // Read key
            if self.pos + key_len > self.file_size {
                break;
            }
            let key_bytes = &data[self.pos..self.pos + key_len];
            let key = match String::from_utf8(key_bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => {
                    // Invalid UTF-8, skip this entry
                    break;
                }
            };
            self.pos += key_len;

            // Read value length
            if self.pos + 4 > self.file_size {
                break;
            }
            let value_len = u32::from_le_bytes(
                data[self.pos..self.pos + 4].try_into().ok()?
            ) as usize;
            self.pos += 4;

            // Read value
            if self.pos + value_len > self.file_size {
                break;
            }
            let value = Bytes::copy_from_slice(&data[self.pos..self.pos + value_len]);
            self.pos += value_len;

            // Read checksum (skip it, already validated on segment open)
            if self.pos + 4 > self.file_size {
                break;
            }
            self.pos += 4;

            // Skip tombstones (empty values)
            if value.is_empty() {
                self.tombstones_skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            return Some((key, value));
        }

        None
    }
}

impl KVIterator for SegmentIterator {
    fn next(&mut self) -> Option<(String, Bytes)> {
        let result = self.current.take();
        self.current = self.read_next_entry();
        result
    }

    fn peek(&self) -> Option<&(String, Bytes)> {
        self.current.as_ref()
    }
}

impl Iterator for SegmentIterator {
    type Item = (String, Bytes);

    fn next(&mut self) -> Option<Self::Item> {
        KVIterator::next(self)
    }
}

/// Builder for creating multiple SegmentIterators from a list of segments
pub struct SegmentIteratorBuilder {
    segments: Vec<Arc<SegmentFile>>,
}

impl SegmentIteratorBuilder {
    pub fn new(segments: Vec<Arc<SegmentFile>>) -> Self {
        Self { segments }
    }

    /// Build all SegmentIterators, returning error if any fails
    pub fn build_all(self) -> Result<Vec<SegmentIterator>, FatalError> {
        let mut iterators = Vec::with_capacity(self.segments.len());
        for segment in self.segments {
            let iter = SegmentIterator::new(segment)?;
            iterators.push(iter);
        }
        Ok(iterators)
    }

    /// Build iterators, skipping any that fail
    pub fn build_all_skip_errors(self) -> Vec<SegmentIterator> {
        self.segments
            .into_iter()
            .filter_map(|seg| SegmentIterator::new(seg).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::io::memfs::MemFs;
    use crate::io::FileKVFileSystem;

    #[test]
    fn test_segment_iterator_basic() {
        // Create a temporary segment with some data
        let fs = Arc::new(MemFs::new());
        let path = std::path::Path::new("/test/segment_1.log");
        fs.create_dir_all(path.parent().unwrap()).unwrap();

        let segment = Arc::new(
            SegmentFile::create(
                fs.clone(),
                1,
                0,
                path,
                0,
                false,
                0,
                false,
            )
            .unwrap()
        );

        // Write some entries
        segment.append("key_a", b"value_a").unwrap();
        segment.append("key_b", b"value_b").unwrap();
        segment.append("key_c", b"value_c").unwrap();

        // Create iterator
        let iter = SegmentIterator::new(segment).unwrap();
        let entries: Vec<_> = {
            let it = iter;
            it.collect()
        };

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, "key_a");
        assert_eq!(entries[1].0, "key_b");
        assert_eq!(entries[2].0, "key_c");
    }

    #[test]
    fn test_segment_iterator_skips_tombstones() {
        let fs = Arc::new(MemFs::new());
        let path = std::path::Path::new("/test/segment_2.log");
        fs.create_dir_all(path.parent().unwrap()).unwrap();

        let segment = Arc::new(
            SegmentFile::create(
                fs.clone(),
                2,
                0,
                path,
                0,
                false,
                0,
                false,
            )
            .unwrap()
        );

        segment.append("key_a", b"value_a").unwrap();
        segment.append("key_b", b"").unwrap(); // Tombstone
        segment.append("key_c", b"value_c").unwrap();

        let iter = SegmentIterator::new(segment).unwrap();
        let entries: Vec<_> = {
            let it = iter;
            it.collect()
        };

        // Should skip the tombstone
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "key_a");
        assert_eq!(entries[1].0, "key_c");
    }

    #[test]
    fn test_segment_iterator_tombstone_counter() {
        let fs = Arc::new(MemFs::new());
        let path = std::path::Path::new("/test/segment_tomb.log");
        fs.create_dir_all(path.parent().unwrap()).unwrap();

        let segment = Arc::new(
            SegmentFile::create(
                fs.clone(),
                10,
                0,
                path,
                0,
                false,
                0,
                false,
            )
            .unwrap()
        );

        segment.append("key_a", b"value_a").unwrap();
        segment.append("key_b", b"").unwrap(); // Tombstone
        segment.append("key_c", b"value_c").unwrap();
        segment.append("key_d", b"").unwrap(); // Tombstone
        segment.append("key_e", b"").unwrap(); // Tombstone
        segment.append("key_f", b"value_f").unwrap();

        let iter = SegmentIterator::new(segment).unwrap();
        let entries: Vec<_> = {
            let it = iter;
            it.collect()
        };

        // Should have 3 non-tombstone entries
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_segment_iterator_shared_tombstone_counter() {
        use std::sync::atomic::Ordering;

        let fs = Arc::new(MemFs::new());
        let path = std::path::Path::new("/test/segment_shared.log");
        fs.create_dir_all(path.parent().unwrap()).unwrap();

        let segment = Arc::new(
            SegmentFile::create(
                fs.clone(),
                11,
                0,
                path,
                0,
                false,
                0,
                false,
            )
            .unwrap()
        );

        segment.append("a", b"1").unwrap();
        segment.append("b", b"").unwrap(); // Tombstone
        segment.append("c", b"3").unwrap();
        segment.append("d", b"").unwrap(); // Tombstone

        let counter = Arc::new(AtomicU64::new(0));
        let iter = SegmentIterator::with_tombstone_counter(segment, Some(counter.clone())).unwrap();
        let entries: Vec<_> = {
            let it = iter;
            it.collect()
        };

        assert_eq!(entries.len(), 2);
        // Counter should have counted 2 tombstones
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_segment_iterator_peek() {
        let fs = Arc::new(MemFs::new());
        let path = std::path::Path::new("/test/segment_3.log");
        fs.create_dir_all(path.parent().unwrap()).unwrap();

        let segment = Arc::new(
            SegmentFile::create(
                fs.clone(),
                3,
                0,
                path,
                0,
                false,
                0,
                false,
            )
            .unwrap()
        );

        segment.append("key_a", b"value_a").unwrap();
        segment.append("key_b", b"value_b").unwrap();

        let mut iter = SegmentIterator::new(segment).unwrap();

        // Peek should return first entry without consuming
        let peeked = iter.peek();
        assert!(peeked.is_some());
        assert_eq!(peeked.unwrap().0, "key_a");

        // Next should return the same entry
        let next = std::iter::Iterator::next(&mut iter);
        assert!(next.is_some());
        assert_eq!(next.unwrap().0, "key_a");

        // Peek should now return second entry
        let peeked = iter.peek();
        assert!(peeked.is_some());
        assert_eq!(peeked.unwrap().0, "key_b");
    }
}
