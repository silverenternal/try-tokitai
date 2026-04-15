//! L2 mmap-based Cache
//!
//! Implements a secondary cache layer backed by memory-mapped files.
//! L2 cache stores evicted hot entries from L1 (BlockCache) based on access frequency.
//!
//! # Architecture
//! - L1 (BlockCache): Moka-based, in-memory, fast access
//! - L2 (L2Cache): mmap-backed file-based, larger capacity, slower than L1
//!
//! # Cache Entry Format
//! [key_len: u16][key: key_len bytes][value_len: u32][value: value_len bytes][access_time: u64][access_count: u32][checksum: u32]
//!
//! # File Layout
//! [header: 64 bytes][entries...]
//! Header: [magic: u32][version: u32][max_bytes: u64][used_bytes: u64][entry_count: u64][eviction_cursor: u64][reserved: 32 bytes]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use memmap2::MmapMut;
use parking_lot::RwLock;

/// L2 cache magic number
const L2_CACHE_MAGIC: u32 = 0x4C324348; // "L2CH"
/// L2 cache version
const L2_CACHE_VERSION: u32 = 1;
/// Header size in bytes
const HEADER_SIZE: u64 = 64;
/// Minimum entry size (empty key + empty value + metadata)
const MIN_ENTRY_SIZE: u64 = 2 + 4 + 8 + 4 + 4; // 22 bytes

/// L2 cache configuration
#[derive(Debug, Clone)]
pub struct L2CacheConfig {
    /// Maximum L2 cache size in bytes
    pub max_bytes: u64,
    /// Directory to store L2 cache files
    pub cache_dir: PathBuf,
    /// Access count threshold to promote from L2 to L1
    pub l2_to_l1_threshold: u32,
}

impl Default for L2CacheConfig {
    fn default() -> Self {
        Self {
            max_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            cache_dir: PathBuf::from("cache_l2"),
            l2_to_l1_threshold: 5,
        }
    }
}

/// L2 cache statistics
#[derive(Debug, Clone, Default)]
pub struct L2CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub evictions: u64,
    pub promotions: u64, // Promoted to L1
    pub demotions: u64,  // Demoted from L1
    pub entry_count: u64,
    pub used_bytes: u64,
    pub max_bytes: u64,
}

impl L2CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Internal stats counters
#[derive(Debug, Default)]
struct L2CacheStatsInner {
    hits: AtomicU64,
    misses: AtomicU64,
    inserts: AtomicU64,
    evictions: AtomicU64,
    promotions: AtomicU64,
    demotions: AtomicU64,
    entry_count: AtomicU64,
}

/// L2 cache entry metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    value_len: u32,
    value_offset: u64, // Offset within mmap where value starts
    access_time: u64,
    access_count: u32,
}

/// L2 cache file manager
struct L2CacheFile {
    mmap: MmapMut,
}

impl L2CacheFile {
    /// Create or open an L2 cache file
    fn open_or_create(path: &Path, max_bytes: u64) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let exists = path.exists();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(!exists)
            .open(path)?;

        if !exists {
            // Initialize file with header + empty space
            let mut header = vec![0u8; HEADER_SIZE as usize];
            // Write magic
            header[0..4].copy_from_slice(&L2_CACHE_MAGIC.to_le_bytes());
            // Write version
            header[4..8].copy_from_slice(&L2_CACHE_VERSION.to_le_bytes());
            // Write max_bytes
            header[8..16].copy_from_slice(&max_bytes.to_le_bytes());
            // Write used_bytes (HEADER_SIZE, since header is part of the file)
            header[16..24].copy_from_slice(&HEADER_SIZE.to_le_bytes());
            // Write entry_count (0)
            header[24..32].copy_from_slice(&0u64.to_le_bytes());
            // Write eviction_cursor (HEADER_SIZE)
            header[32..40].copy_from_slice(&HEADER_SIZE.to_le_bytes());

            file.set_len(max_bytes)?;
            let mut written = 0;
            while written < header.len() {
                written += file.write(&header[written..])?;
            }
            file.sync_all()?;
        }

        // Safety: We have exclusive access to the file via the file handle.
        // The file handle is intentionally dropped after mmap creation; on Linux
        // the mapping remains valid through the page cache.
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        drop(file);

        // Verify magic if file existed
        if exists {
            let magic = u32::from_le_bytes(
                mmap[0..4]
                    .try_into()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid L2 cache header"))?,
            );
            if magic != L2_CACHE_MAGIC {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid L2 cache magic",
                ));
            }
        }

        Ok(Self { mmap })
    }

    /// Flush mmap changes to disk
    fn flush(&self) -> std::io::Result<()> {
        self.mmap.flush_async()
    }

    /// Get used bytes from header
    fn get_used_bytes(&self) -> u64 {
        u64::from_le_bytes(self.mmap[16..24].try_into().unwrap())
    }

    /// Set used bytes in header
    fn set_used_bytes(&mut self, bytes: u64) {
        self.mmap[16..24].copy_from_slice(&bytes.to_le_bytes());
    }

    /// Get entry count from header
    fn get_entry_count(&self) -> u64 {
        u64::from_le_bytes(self.mmap[24..32].try_into().unwrap())
    }

    /// Set entry count in header
    fn set_entry_count(&mut self, count: u64) {
        self.mmap[24..32].copy_from_slice(&count.to_le_bytes());
    }

    /// Get eviction cursor from header
    #[allow(dead_code)]
    fn get_eviction_cursor(&self) -> u64 {
        u64::from_le_bytes(self.mmap[32..40].try_into().unwrap())
    }

    /// Set eviction cursor in header
    fn set_eviction_cursor(&mut self, cursor: u64) {
        self.mmap[32..40].copy_from_slice(&cursor.to_le_bytes());
    }
}

/// L2 mmap-based cache manager
pub struct L2CacheManager {
    config: L2CacheConfig,
    cache_file: RwLock<L2CacheFile>,
    /// In-memory index: key -> CacheEntry (for fast lookup)
    index: RwLock<HashMap<String, CacheEntry>>,
    stats: Arc<L2CacheStatsInner>,
    /// Current timestamp for access_time (monotonically increasing)
    clock: AtomicU64,
    /// Tracks total size of live (non-evicted) entries in bytes.
    /// This diverges from the file header's used_bytes after evictions/removes
    /// (which don't compact the file). Used for capacity decisions.
    used_bytes: AtomicU64,
}

impl L2CacheManager {
    /// Create a new L2 cache manager
    pub fn new(config: L2CacheConfig) -> std::io::Result<Self> {
        let cache_dir = config.cache_dir.clone();
        let cache_path = cache_dir.join("cache_l2.dat");

        let cache_file = L2CacheFile::open_or_create(&cache_path, config.max_bytes)?;

        // Initialize clock from current time
        let clock = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let manager = Self {
            config,
            cache_file: RwLock::new(cache_file),
            index: RwLock::new(HashMap::new()),
            stats: Arc::default(),
            clock: AtomicU64::new(clock),
            used_bytes: AtomicU64::new(0),
        };

        // Load existing entries from disk
        manager.load_index()?;

        Ok(manager)
    }

    /// Load index from existing cache file
    fn load_index(&self) -> std::io::Result<()> {
        let cache_file = self.cache_file.read();
        let used_bytes = cache_file.get_used_bytes();

        tracing::debug!(
            "L2 cache load_index: used_bytes={}, header_size={}",
            used_bytes,
            HEADER_SIZE
        );

        if used_bytes <= HEADER_SIZE {
            return Ok(());
        }

        let mut pos = HEADER_SIZE;
        let mut index = self.index.write();
        let mut total_entries = 0u64;
        let mut live_bytes = 0u64;

        while pos + MIN_ENTRY_SIZE <= used_bytes {
            // Read key_len
            let key_len = u16::from_le_bytes(
                cache_file.mmap[pos as usize..pos as usize + 2]
                    .try_into()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key_len"))?,
            ) as u64;
            pos += 2;

            if pos + key_len > used_bytes {
                break;
            }

            // Read key
            let key_bytes = &cache_file.mmap[pos as usize..pos as usize + key_len as usize];
            let key = match String::from_utf8(key_bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => break,
            };
            pos += key_len;

            // Read value_len
            if pos + 4 > used_bytes {
                break;
            }
            let value_len = u32::from_le_bytes(
                cache_file.mmap[pos as usize..pos as usize + 4]
                    .try_into()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid value_len"))?,
            );
            pos += 4;

            let value_offset = pos;
            pos += value_len as u64;

            // Read access_time
            if pos + 8 > used_bytes {
                break;
            }
            let access_time = u64::from_le_bytes(
                cache_file.mmap[pos as usize..pos as usize + 8]
                    .try_into()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid access_time"))?,
            );
            pos += 8;

            // Read access_count
            if pos + 4 > used_bytes {
                break;
            }
            let access_count = u32::from_le_bytes(
                cache_file.mmap[pos as usize..pos as usize + 4]
                    .try_into()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid access_count"))?,
            );
            pos += 4;

            // Skip checksum
            if pos + 4 > used_bytes {
                break;
            }
            pos += 4;

            let esize = Self::entry_size(key_len, value_len as u64);
            live_bytes += esize;

            index.insert(
                key,
                CacheEntry {
                    value_len,
                    value_offset,
                    access_time,
                    access_count,
                },
            );
            total_entries += 1;
        }

        drop(index);
        self.stats.entry_count.store(total_entries, Ordering::Relaxed);
        self.used_bytes.store(live_bytes, Ordering::Relaxed);

        Ok(())
    }

    /// Calculate entry size in the file
    fn entry_size(key_len: u64, value_len: u64) -> u64 {
        2 + key_len + 4 + value_len + 8 + 4 + 4 // key_len + key + value_len + value + access_time + access_count + checksum
    }

    /// Calculate checksum for an entry
    fn calculate_checksum(key: &[u8], value: &[u8]) -> u32 {
        let mut combined = Vec::with_capacity(key.len() + value.len());
        combined.extend_from_slice(key);
        combined.extend_from_slice(value);
        crc32c::crc32c(&combined)
    }

    /// Get value by key
    pub fn get(&self, key: &str) -> Option<Bytes> {
        let entry = {
            let index = self.index.read();
            index.get(key).cloned()
        };

        let entry = match entry {
            Some(e) => e,
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        // Read value from mmap
        let cache_file = self.cache_file.read();
        let value_start = entry.value_offset as usize;
        let value_end = value_start + entry.value_len as usize;

        if value_end > cache_file.mmap.len() {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        // Verify checksum before copying
        let checksum_start = entry.value_offset as usize + entry.value_len as usize + 8 + 4;
        if checksum_start + 4 <= cache_file.mmap.len() {
            let stored_checksum = u32::from_le_bytes(
                cache_file.mmap[checksum_start..checksum_start + 4]
                    .try_into()
                    .unwrap_or([0; 4]),
            );
            let calculated_checksum =
                Self::calculate_checksum(key.as_bytes(), &cache_file.mmap[value_start..value_end]);
            if stored_checksum != calculated_checksum {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        }

        let value = Bytes::copy_from_slice(&cache_file.mmap[value_start..value_end]);
        drop(cache_file);

        // Update access count and time
        self.update_access(key, &entry);

        self.stats.hits.fetch_add(1, Ordering::Relaxed);
        Some(value)
    }

    /// Update access count and time for an entry
    fn update_access(&self, key: &str, entry: &CacheEntry) {
        let new_time = self.clock.fetch_add(1, Ordering::Relaxed);
        let new_count = entry.access_count + 1;

        // Update in-memory index
        {
            let mut index = self.index.write();
            if let Some(e) = index.get_mut(key) {
                e.access_time = new_time;
                e.access_count = new_count;
            }
        }

        // Update on-disk metadata
        {
            let mut cache_file = self.cache_file.write();
            // Find entry position and update access_time and access_count
            if let Some(pos) = self.find_entry_position(&cache_file, key) {
                // Update access_time
                let time_offset = pos + 2 + key.len() as u64 + 4 + entry.value_len as u64;
                cache_file.mmap[time_offset as usize..time_offset as usize + 8]
                    .copy_from_slice(&new_time.to_le_bytes());
                // Update access_count
                let count_offset = time_offset + 8;
                cache_file.mmap[count_offset as usize..count_offset as usize + 4]
                    .copy_from_slice(&new_count.to_le_bytes());
            }
        }
    }

    /// Find the position of an entry in the mmap file
    fn find_entry_position(&self, cache_file: &L2CacheFile, key: &str) -> Option<u64> {
        let used_bytes = cache_file.get_used_bytes();
        let mut pos = HEADER_SIZE;

        while pos + MIN_ENTRY_SIZE <= used_bytes {
            let key_len = u16::from_le_bytes(cache_file.mmap[pos as usize..pos as usize + 2].try_into().ok()?) as u64;
            pos += 2;

            if pos + key_len > used_bytes {
                return None;
            }

            let key_bytes = &cache_file.mmap[pos as usize..pos as usize + key_len as usize];
            if key_bytes == key.as_bytes() {
                return Some(pos - 2); // Return position including key_len field
            }
            pos += key_len;

            // Skip rest of entry
            if pos + 4 > used_bytes {
                return None;
            }
            let value_len = u32::from_le_bytes(cache_file.mmap[pos as usize..pos as usize + 4].try_into().ok()?) as u64;
            pos += 4 + value_len + 8 + 4 + 4;
        }

        None
    }

    /// Insert a key-value pair into L2 cache
    pub fn insert(&self, key: &str, value: Bytes) {
        let key_bytes = key.as_bytes();
        let key_len = key_bytes.len() as u64;
        let value_len = value.len() as u64;
        let entry_size = Self::entry_size(key_len, value_len);

        // If key already exists, subtract old entry size from live bytes
        let old_size = {
            let index = self.index.read();
            index.get(key).map(|e| Self::entry_size(key_len, e.value_len as u64))
        };
        if let Some(old) = old_size {
            self.used_bytes.fetch_sub(old, Ordering::Relaxed);
        }

        // Ensure we have enough space
        self.ensure_space(entry_size);

        let checksum = Self::calculate_checksum(key_bytes, &value);
        let current_time = self.clock.fetch_add(1, Ordering::Relaxed);

        // Write entry to mmap
        let value_offset;
        {
            let mut cache_file = self.cache_file.write();
            let file_used = cache_file.get_used_bytes();
            let write_pos = file_used;

            // Check if we can fit the entry
            if write_pos + entry_size > self.config.max_bytes {
                // Should not happen after ensure_space, but handle gracefully
                if let Some(old) = old_size {
                    self.used_bytes.fetch_add(old, Ordering::Relaxed);
                }
                return;
            }

            let pos = write_pos as usize;
            let mut offset = pos;

            // Write key_len
            cache_file.mmap[offset..offset + 2].copy_from_slice(&(key_len as u16).to_le_bytes());
            offset += 2;

            // Write key
            cache_file.mmap[offset..offset + key_bytes.len()].copy_from_slice(key_bytes);
            offset += key_bytes.len();

            // Write value_len
            let value_len_u32 = value_len.min(u32::MAX as u64) as u32;
            cache_file.mmap[offset..offset + 4].copy_from_slice(&value_len_u32.to_le_bytes());
            offset += 4;

            // Write value
            cache_file.mmap[offset..offset + value.len()].copy_from_slice(&value);
            value_offset = offset as u64;
            offset += value.len();

            // Write access_time
            cache_file.mmap[offset..offset + 8].copy_from_slice(&current_time.to_le_bytes());
            offset += 8;

            // Write access_count
            cache_file.mmap[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
            offset += 4;

            // Write checksum
            cache_file.mmap[offset..offset + 4].copy_from_slice(&checksum.to_le_bytes());

            // Update header
            let entry_count = cache_file.get_entry_count() + 1;
            cache_file.set_used_bytes(write_pos + entry_size);
            cache_file.set_entry_count(entry_count);

            // Update eviction cursor
            cache_file.set_eviction_cursor(write_pos + entry_size);
        }

        // Update in-memory index
        {
            let mut index = self.index.write();
            index.insert(
                key.to_string(),
                CacheEntry {
                    value_len: value.len() as u32,
                    value_offset,
                    access_time: current_time,
                    access_count: 1,
                },
            );
        }

        // Add new entry size to live bytes
        self.used_bytes.fetch_add(entry_size, Ordering::Relaxed);

        self.stats.inserts.fetch_add(1, Ordering::Relaxed);
        self.stats.entry_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Ensure there's enough space for a new entry
    fn ensure_space(&self, required_bytes: u64) {
        let live = self.used_bytes.load(Ordering::Relaxed);

        if live + required_bytes > self.config.max_bytes {
            // Need to evict - find LRU entry
            let to_evict = {
                let index = self.index.read();
                index
                    .iter()
                    .min_by_key(|(_, e)| e.access_time)
                    .map(|(k, e)| (k.clone(), Self::entry_size(k.len() as u64, e.value_len as u64)))
            };

            if let Some((key, entry_size)) = to_evict {
                // Remove from index
                {
                    let mut index = self.index.write();
                    index.remove(&key);
                }

                // Subtract from live used_bytes
                self.used_bytes.fetch_sub(entry_size, Ordering::Relaxed);

                // Update file header entry count
                {
                    let mut cache_file = self.cache_file.write();
                    let entry_count = cache_file.get_entry_count().saturating_sub(1);
                    cache_file.set_entry_count(entry_count);
                }

                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Check if a key exists in L2 cache
    pub fn contains(&self, key: &str) -> bool {
        self.index.read().contains_key(key)
    }

    /// Remove a key from L2 cache
    pub fn remove(&self, key: &str) {
        let removed = {
            let mut index = self.index.write();
            index.remove(key)
        };

        if let Some(entry) = removed {
            let entry_size = Self::entry_size(key.len() as u64, entry.value_len as u64);
            self.used_bytes.fetch_sub(entry_size, Ordering::Relaxed);

            // Update file header entry count
            {
                let mut cache_file = self.cache_file.write();
                let entry_count = cache_file.get_entry_count().saturating_sub(1);
                cache_file.set_entry_count(entry_count);
            }

            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get the access count for a key (for promotion decisions)
    pub fn get_access_count(&self, key: &str) -> Option<u32> {
        self.index.read().get(key).map(|e| e.access_count)
    }

    /// Check if a key should be promoted to L1
    pub fn should_promote(&self, key: &str) -> bool {
        self.get_access_count(key)
            .map(|count| count >= self.config.l2_to_l1_threshold)
            .unwrap_or(false)
    }

    /// Get L2 cache statistics
    pub fn stats(&self) -> L2CacheStats {
        L2CacheStats {
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            inserts: self.stats.inserts.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            promotions: self.stats.promotions.load(Ordering::Relaxed),
            demotions: self.stats.demotions.load(Ordering::Relaxed),
            entry_count: self.stats.entry_count.load(Ordering::Relaxed),
            used_bytes: self.get_used_bytes(),
            max_bytes: self.config.max_bytes,
        }
    }

    /// Record a promotion (L2 -> L1)
    pub fn record_promotion(&self) {
        self.stats.promotions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a demotion (L1 -> L2)
    pub fn record_demotion(&self) {
        self.stats.demotions.fetch_add(1, Ordering::Relaxed);
    }

    /// Flush L2 cache to disk
    pub fn flush(&self) -> std::io::Result<()> {
        self.cache_file.read().flush()
    }

    /// Get current live used bytes (tracks actual entry sizes, diverges from file
    /// header after evictions/removes since the file is not compacted).
    pub fn get_used_bytes(&self) -> u64 {
        self.used_bytes.load(Ordering::Relaxed)
    }

    /// Get current memory usage (index only, mmap is not counted as memory)
    pub fn index_memory_usage(&self) -> usize {
        let index = self.index.read();
        // Rough estimate: each HashMap entry ~64 bytes overhead + key + value struct
        index.len() * (64 + 32) // 32 bytes for String + CacheEntry
    }
}

impl Drop for L2CacheManager {
    fn drop(&mut self) {
        // Flush before dropping
        if let Err(e) = self.flush() {
            tracing::warn!("Failed to flush L2 cache on drop: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config() -> L2CacheConfig {
        let temp_dir = tempfile::tempdir().unwrap();
        L2CacheConfig {
            max_bytes: 1024 * 1024, // 1MB for testing
            cache_dir: temp_dir.path().to_path_buf(),
            l2_to_l1_threshold: 3,
        }
    }

    #[test]
    fn test_l2_cache_basic_insert_and_get() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        let value = Bytes::from("test_value");
        cache.insert("test_key", value.clone());

        let result = cache.get("test_key");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }

    #[test]
    fn test_l2_cache_miss() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        let result = cache.get("nonexistent");
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_l2_cache_multiple_entries() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        for i in 0..10 {
            let key = format!("key_{}", i);
            let value = Bytes::from(format!("value_{}", i));
            cache.insert(&key, value);
        }

        for i in 0..10 {
            let key = format!("key_{}", i);
            let expected = Bytes::from(format!("value_{}", i));
            assert_eq!(cache.get(&key), Some(expected));
        }
    }

    #[test]
    fn test_l2_cache_access_count() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        cache.insert("hot_key", Bytes::from("value"));

        // Access multiple times
        for _ in 0..5 {
            cache.get("hot_key");
        }

        let count = cache.get_access_count("hot_key");
        assert!(count.is_some());
        assert!(count.unwrap() >= 5);
    }

    #[test]
    fn test_l2_cache_should_promote() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        cache.insert("key", Bytes::from("value"));

        // Not promoted yet (threshold is 3)
        assert!(!cache.should_promote("key"));

        // Access 3 times
        for _ in 0..3 {
            cache.get("key");
        }

        // Now should promote
        assert!(cache.should_promote("key"));
    }

    #[test]
    fn test_l2_cache_stats() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        cache.insert("k1", Bytes::from("v1"));
        cache.insert("k2", Bytes::from("v2"));

        cache.get("k1");
        cache.get("k1");
        cache.get("k2");
        cache.get("nonexistent");

        let stats = cache.stats();
        assert_eq!(stats.inserts, 2);
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert!(stats.entry_count >= 2);
    }

    #[test]
    fn test_l2_cache_remove() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        cache.insert("key", Bytes::from("value"));
        assert!(cache.get("key").is_some());

        cache.remove("key");
        assert!(cache.get("key").is_none());
    }

    #[test]
    fn test_l2_cache_checksum_verification() {
        let config = make_test_config();
        let cache = L2CacheManager::new(config).unwrap();

        let value = Bytes::from("test_data_with_checksum");
        cache.insert("checksum_test", value.clone());

        // Get should verify checksum
        let result = cache.get("checksum_test");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }
}
