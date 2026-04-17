#![allow(clippy::type_complexity)]
//! Bloom filter manager for FileKV
//!
//! This module provides a standalone `BloomManager` that handles all bloom filter
//! operations without direct coupling to `FileKV`. The manager uses a narrow trait
//! interface (`BloomSegmentProvider`) to interact with segment storage.
//!
//! ## Design
//! - `BloomManager`: Standalone struct managing bloom lifecycle
//! - `BloomSegmentProvider`: Trait exposing minimal segment access needed for bloom
//! - FileKV (or EngineState) implements `BloomSegmentProvider`

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use ::bloom::BloomFilter;
use ::bloom::ASMS;
use tracing::debug;

use super::custom_bloom::CustomBloom;
use crate::core::error::FatalError;
use crate::BLOOM_MAGIC;
use crate::DEFAULT_BLOOM_FPR;

/// Result type for bloom operations
pub type Result<T> = std::result::Result<T, FatalError>;

/// Trait for providing segment access to BloomManager
///
/// This is a narrow interface exposing only the minimum information needed
/// for bloom filter operations. Implementors can be FileKV, EngineState,
/// or a mock for testing.
pub trait BloomSegmentProvider {
    /// Get list of all segment IDs
    fn get_segment_ids(&self) -> Vec<u64>;

    /// Rebuild bloom filter for a single segment by iterating its entries
    /// The callback is called for each (key, value, deleted) tuple
    fn iterate_segment_entries(
        &self,
        segment_id: u64,
        callback: &mut dyn FnMut(&str, &[u8], bool) -> std::io::Result<()>,
    ) -> std::io::Result<()>;

    /// Get the index directory path for bloom filter storage
    fn get_index_dir(&self) -> &Path;

    /// Check if bloom filter exists for a segment
    fn bloom_filter_exists(&self, segment_id: u64) -> bool;

    /// Load a bloom filter for a segment (returns bloom + keys)
    fn load_bloom_filter(&self, segment_id: u64) -> Result<Option<(BloomFilter, Vec<String>)>>;

    /// Save a bloom filter for a segment
    fn save_bloom_filter_atomic(&self, segment_id: u64, bloom: &BloomFilter, keys: &[String]) -> Result<()>;

    /// Insert bloom filter into cache
    fn insert_bloom_into_cache(&self, segment_id: u64, bloom: BloomFilter);
}

/// Bloom filter manager configuration
#[derive(Debug, Clone)]
pub struct BloomConfig {
    pub default_fpr: f32,
    pub default_capacity: u32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            default_fpr: DEFAULT_BLOOM_FPR,
            default_capacity: 10000,
        }
    }
}

/// Standalone bloom filter manager
pub struct BloomManager {
    config: BloomConfig,
}

impl BloomManager {
    /// Create a new BloomManager
    pub fn new(config: BloomConfig) -> Self {
        Self { config }
    }

    /// Rebuild bloom filters for all segments
    ///
    /// Uses the BloomSegmentProvider trait to access segments without
    /// direct coupling to FileKV.
    pub fn rebuild_all<P: BloomSegmentProvider>(&self, provider: &P) -> Result<usize> {
        let segment_ids = provider.get_segment_ids();
        let mut rebuilt_count = 0;
        let mut loaded_count = 0;
        let mut skipped_count = 0;

        for seg_id in segment_ids {
            // Try to load existing bloom filter
            match provider.load_bloom_filter(seg_id) {
                Ok(Some((bloom, _keys))) => {
                    provider.insert_bloom_into_cache(seg_id, bloom);
                    loaded_count += 1;
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "Bloom filter file for segment {} corrupted: {}. Will rebuild.",
                        seg_id,
                        e
                    );
                }
            }

            tracing::info!("Rebuilding bloom filter for segment {}", seg_id);

            // Rebuild bloom for segment
            let (bloom, keys) = match self.build_bloom_for_segment(provider, seg_id) {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!(
                        "Segment {} failed integrity check, skipping bloom rebuild: {}",
                        seg_id,
                        e
                    );
                    skipped_count += 1;
                    continue;
                }
            };

            // Save bloom filter
            if let Err(e) = provider.save_bloom_filter_atomic(seg_id, &bloom, &keys) {
                tracing::error!("Failed to save bloom filter for segment {}: {}", seg_id, e);
                skipped_count += 1;
                continue;
            }

            provider.insert_bloom_into_cache(seg_id, bloom);
            rebuilt_count += 1;
        }

        tracing::info!(
            "Bloom filter rebuild complete: loaded={}, rebuilt={}, skipped={}",
            loaded_count,
            rebuilt_count,
            skipped_count
        );
        Ok(rebuilt_count)
    }

    /// Rebuild bloom filter for a single segment using pre-collected keys
    ///
    /// This is optimized for compaction: uses keys already collected in memory
    /// during the compaction merge, avoiding segment file re-iteration.
    pub fn rebuild_for_segment<P: BloomSegmentProvider>(
        &self,
        provider: &P,
        segment_id: u64,
        keys: &[String],
    ) -> Result<()> {
        tracing::debug!(
            "Building bloom filter for segment {} using {} keys from memory",
            segment_id,
            keys.len()
        );

        let mut bloom = BloomFilter::with_rate(self.config.default_fpr, keys.len().max(10000) as u32);

        for key in keys {
            bloom.insert(key);
        }

        if let Err(e) = provider.save_bloom_filter_atomic(segment_id, &bloom, keys) {
            tracing::error!("Failed to save bloom filter for segment {}: {}", segment_id, e);
            return Err(FatalError::Corruption(format!("Failed to save bloom filter: {}", e)));
        }

        provider.insert_bloom_into_cache(segment_id, bloom);
        tracing::debug!("Bloom filter built and cached for segment {}", segment_id);
        Ok(())
    }

    /// Build bloom filter for a segment by iterating its entries
    fn build_bloom_for_segment<P: BloomSegmentProvider>(
        &self,
        provider: &P,
        segment_id: u64,
    ) -> Result<(BloomFilter, Vec<String>)> {
        let mut bloom = BloomFilter::with_rate(self.config.default_fpr, self.config.default_capacity);
        let mut keys = Vec::new();

        let mut callback = |key: &str, _value: &[u8], _deleted: bool| -> std::io::Result<()> {
            bloom.insert(&key);
            keys.push(key.to_string());
            Ok(())
        };

        provider.iterate_segment_entries(segment_id, &mut callback)?;

        Ok((bloom, keys))
    }
}

// Bloom filter file format versions:
// - v1: [magic 4B][version 4B][num_keys 8B][key1_len 4B][key1][key2_len 4B][key2]...
// - v2: [magic 4B][version 4B][num_bits 4B][num_hashes 4B][num_keys 8B][key1_len 4B][key1]...
//
// V2 adds num_bits and num_hashes metadata for faster reconstruction.
// Note: V3 (bitvector-only format) was attempted but abandoned because the bloom crate
// uses RandomState hash builders that cannot be serialized. V2 remains optimal.

/// Current bloom filter file version
pub const CURRENT_BLOOM_VERSION: u32 = 2;

/// Save bloom filter atomically using temp file + rename (v2 format)
///
/// V2 format includes num_bits and num_hashes metadata for faster reconstruction.
pub fn save_bloom_filter_atomic(index_dir: &Path, segment_id: u64, bloom: &BloomFilter, keys: &[String]) -> Result<()> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));
    let temp_path = index_dir.join(format!("bloom_{:06}.tmp", segment_id));

    let mut file = BufWriter::new(File::create(&temp_path).map_err(FatalError::Io)?);

    // Write header: magic + version (v2)
    file.write_all(&BLOOM_MAGIC.to_le_bytes())?;
    file.write_all(&CURRENT_BLOOM_VERSION.to_le_bytes())?;

    // Write bloom filter metadata (new in v2)
    let num_bits = bloom.num_bits() as u32;
    let num_hashes = bloom.num_hashes();
    file.write_all(&num_bits.to_le_bytes())?;
    file.write_all(&num_hashes.to_le_bytes())?;

    // Write keys
    let num_keys = keys.len() as u64;
    file.write_all(&num_keys.to_le_bytes())?;

    for key in keys {
        let key_bytes = key.as_bytes();
        let key_len = key_bytes.len() as u32;
        file.write_all(&key_len.to_le_bytes())?;
        file.write_all(key_bytes)?;
    }

    file.flush()?;
    file.get_ref().sync_all().map_err(FatalError::Io)?;
    drop(file);

    fs::rename(&temp_path, &bloom_path).map_err(FatalError::Io)?;

    if let Ok(dir) = File::open(index_dir) {
        let _ = dir.sync_all();
    }

    debug!(
        "Atomically saved bloom filter v2 with {} keys, {} bits, {} hashes for segment {} to {:?}",
        num_keys, num_bits, num_hashes, segment_id, bloom_path
    );
    Ok(())
}

/// Load bloom filter from disk (supports v1 and v2 formats)
///
/// V2 format is faster to load as it includes pre-computed num_bits and num_hashes.
pub fn load_bloom_filter(index_dir: &Path, segment_id: u64) -> Result<Option<(BloomFilter, Vec<String>)>> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));

    if !bloom_path.exists() {
        return Ok(None);
    }

    let mut file = File::open(&bloom_path).map_err(FatalError::Io)?;

    let mut magic_buf = [0u8; 4];
    file.read_exact(&mut magic_buf).map_err(FatalError::Io)?;
    let magic = u32::from_le_bytes(magic_buf);
    if magic != BLOOM_MAGIC {
        return Err(FatalError::Corruption(format!("Invalid bloom magic: {}", magic)));
    }

    let mut version_buf = [0u8; 4];
    file.read_exact(&mut version_buf).map_err(FatalError::Io)?;
    let version = u32::from_le_bytes(version_buf);

    // Parse based on version
    let (num_bits, num_hashes, num_keys) = if version == 1 {
        // V1 format: no metadata, just num_keys
        let mut num_keys_buf = [0u8; 8];
        file.read_exact(&mut num_keys_buf).map_err(FatalError::Io)?;
        let num_keys = u64::from_le_bytes(num_keys_buf);
        (None, None, num_keys)
    } else if version == 2 {
        // V2 format: has metadata
        let mut num_bits_buf = [0u8; 4];
        file.read_exact(&mut num_bits_buf).map_err(FatalError::Io)?;
        let num_bits = u32::from_le_bytes(num_bits_buf);

        let mut num_hashes_buf = [0u8; 4];
        file.read_exact(&mut num_hashes_buf).map_err(FatalError::Io)?;
        let num_hashes = u32::from_le_bytes(num_hashes_buf);

        let mut num_keys_buf = [0u8; 8];
        file.read_exact(&mut num_keys_buf).map_err(FatalError::Io)?;
        let num_keys = u64::from_le_bytes(num_keys_buf);

        (Some(num_bits), Some(num_hashes), num_keys)
    } else {
        return Err(FatalError::Corruption(format!(
            "Unsupported bloom version: {}",
            version
        )));
    };

    // Read keys
    let mut keys = Vec::with_capacity(num_keys as usize);
    for _ in 0..num_keys {
        let mut key_len_buf = [0u8; 4];
        file.read_exact(&mut key_len_buf).map_err(FatalError::Io)?;
        let key_len = u32::from_le_bytes(key_len_buf) as usize;

        let mut key_buf = vec![0u8; key_len];
        file.read_exact(&mut key_buf).map_err(FatalError::Io)?;
        keys.push(String::from_utf8_lossy(&key_buf).to_string());
    }

    // Build bloom filter from keys
    // V2 optimization: use pre-stored num_bits and num_hashes if available
    let bloom = if let (Some(nb), Some(nh)) = (num_bits, num_hashes) {
        // V2 fast path: use stored metadata for faster construction
        let mut bf = BloomFilter::with_size(nb as usize, nh);
        for key in &keys {
            bf.insert(key);
        }
        bf
    } else {
        // V1 fallback: estimate capacity from keys
        let mut bf = BloomFilter::with_rate(crate::DEFAULT_BLOOM_FPR, keys.len().max(10000) as u32);
        for key in &keys {
            bf.insert(key);
        }
        bf
    };

    Ok(Some((bloom, keys)))
}

/// Check if bloom filter exists for a segment
pub fn bloom_filter_exists(index_dir: &Path, segment_id: u64) -> bool {
    index_dir.join(format!("bloom_{:06}.bin", segment_id)).exists()
}

/// Save bloom filter in V3 format (CustomBloom with direct bitset serialization)
///
/// V3 format enables fast loading without reconstruction:
/// [magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitset_bytes]
pub fn save_bloom_filter_v3(index_dir: &Path, segment_id: u64, bloom: &BloomFilter, keys: &[String]) -> Result<()> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));
    let temp_path = index_dir.join(format!("bloom_{:06}.v3tmp", segment_id));

    // Convert from bloom crate to CustomBloom by rebuilding from keys
    // (bloom crate uses RandomState which is incompatible with our deterministic XXH3)
    let custom_bloom = CustomBloom::from_keys(keys, bloom.num_bits(), DEFAULT_BLOOM_FPR as f64);

    // Save to temp file first
    custom_bloom.save_to_file(&temp_path)?;

    // Atomic rename
    fs::rename(&temp_path, &bloom_path).map_err(FatalError::Io)?;

    if let Ok(dir) = File::open(index_dir) {
        let _ = dir.sync_all();
    }

    debug!(
        "Saved bloom filter v3 for segment {} with {} bits, {} hashes, {} bytes",
        segment_id,
        custom_bloom.num_bits(),
        custom_bloom.num_hashes(),
        custom_bloom.to_bytes().len()
    );

    Ok(())
}

/// Save CustomBloom directly in V3 format (no legacy BloomFilter needed)
pub fn save_custom_bloom_v3(index_dir: &Path, segment_id: u64, custom_bloom: &CustomBloom) -> Result<()> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));
    let temp_path = index_dir.join(format!("bloom_{:06}.v3tmp", segment_id));

    custom_bloom.save_to_file(&temp_path)?;

    fs::rename(&temp_path, &bloom_path).map_err(FatalError::Io)?;

    if let Ok(dir) = File::open(index_dir) {
        let _ = dir.sync_all();
    }

    debug!(
        "Saved custom bloom filter v3 for segment {} with {} bits, {} hashes, {} bytes",
        segment_id,
        custom_bloom.num_bits(),
        custom_bloom.num_hashes(),
        custom_bloom.to_bytes().len()
    );

    Ok(())
}

/// Load bloom filter from V3 format (fast, no reconstruction needed)
///
/// Returns None if file doesn't exist or is not V3 format.
/// Falls back to V1/V2 loading if magic number doesn't match.
pub fn load_bloom_filter_v3(index_dir: &Path, segment_id: u64) -> Result<Option<CustomBloom>> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));

    if !bloom_path.exists() {
        return Ok(None);
    }

    // Try loading as V3 format
    match CustomBloom::load_from_file(&bloom_path) {
        Ok(Some(custom_bloom)) => {
            debug!(
                "Loaded v3 bloom filter for segment {} in <100µs (direct bitset load)",
                segment_id
            );
            Ok(Some(custom_bloom))
        }
        Ok(None) => {
            // Not V3 format - caller should fall back to V1/V2 loading
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Migrate V1/V2 bloom filter to V3 format
///
/// Reads old format, converts to V3, saves back.
/// This is a one-time migration that eliminates keys-list storage.
pub fn migrate_to_v3(index_dir: &Path, segment_id: u64) -> Result<bool> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));

    if !bloom_path.exists() {
        return Ok(false);
    }

    // Check if already V3 format
    let mut file = File::open(&bloom_path).map_err(FatalError::Io)?;
    let mut magic_buf = [0u8; 4];
    file.read_exact(&mut magic_buf).map_err(FatalError::Io)?;
    let magic = u32::from_le_bytes(magic_buf);

    if magic == super::custom_bloom::CUSTOM_BLOOM_MAGIC {
        // Already V3, no migration needed
        return Ok(false);
    }

    // Load old format (V1/V2)
    let old_result = load_bloom_filter(index_dir, segment_id)?;
    if let Some((bloom, keys)) = old_result {
        // Save as V3
        save_bloom_filter_v3(index_dir, segment_id, &bloom, &keys)?;
        debug!("Migrated bloom filter for segment {} from V1/V2 to V3", segment_id);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Load CustomBloom with automatic V1/V2 to V3 migration
///
/// This is the preferred loader for CustomBloomCache:
/// 1. Tries to load V3 format first (fast, direct bitset load)
/// 2. If V3 not found, tries V1/V2 format
/// 3. If V1/V2 found, migrates to V3 automatically
/// 4. Returns the loaded CustomBloom (from V3)
pub fn load_custom_bloom_with_migration(index_dir: &Path, segment_id: u64) -> Result<Option<CustomBloom>> {
    let bloom_path = index_dir.join(format!("bloom_{:06}.bin", segment_id));

    if !bloom_path.exists() {
        return Ok(None);
    }

    // Step 1: Try loading V3 format first
    if let Ok(Some(custom_bloom)) = CustomBloom::load_from_file(&bloom_path) {
        debug!("Loaded v3 bloom filter for segment {} (direct bitset load)", segment_id);
        return Ok(Some(custom_bloom));
    }

    // Step 2: V3 not found, try migrating V1/V2 to V3
    let migrated = migrate_to_v3(index_dir, segment_id)?;
    if migrated {
        debug!(
            "Migrated bloom filter for segment {} from V1/V2 to V3 during load",
            segment_id
        );
    }

    // Step 3: Try loading V3 again (either original or newly migrated)
    match CustomBloom::load_from_file(&bloom_path) {
        Ok(Some(custom_bloom)) => {
            debug!("Loaded v3 bloom filter for segment {} after migration", segment_id);
            Ok(Some(custom_bloom))
        }
        Ok(None) => {
            // Not V3 format and migration failed - return None
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bloom::CustomBloom;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock BloomSegmentProvider for testing
    struct MockProvider {
        segments: HashMap<u64, Vec<(String, Vec<u8>)>>,
        index_dir: std::path::PathBuf,
        bloom_cache: Mutex<HashMap<u64, BloomFilter>>,
    }

    impl MockProvider {
        fn new(index_dir: std::path::PathBuf) -> Self {
            Self {
                segments: HashMap::new(),
                index_dir,
                bloom_cache: Mutex::new(HashMap::new()),
            }
        }

        fn add_segment(&mut self, id: u64, entries: Vec<(String, Vec<u8>)>) {
            self.segments.insert(id, entries);
        }
    }

    impl BloomSegmentProvider for MockProvider {
        fn get_segment_ids(&self) -> Vec<u64> {
            self.segments.keys().cloned().collect()
        }

        fn iterate_segment_entries(
            &self,
            segment_id: u64,
            callback: &mut dyn FnMut(&str, &[u8], bool) -> std::io::Result<()>,
        ) -> std::io::Result<()> {
            if let Some(entries) = self.segments.get(&segment_id) {
                for (key, value) in entries {
                    callback(key, value, false)?;
                }
            }
            Ok(())
        }

        fn get_index_dir(&self) -> &Path {
            &self.index_dir
        }

        fn bloom_filter_exists(&self, segment_id: u64) -> bool {
            super::bloom_filter_exists(&self.index_dir, segment_id)
        }

        fn load_bloom_filter(&self, segment_id: u64) -> Result<Option<(BloomFilter, Vec<String>)>> {
            super::load_bloom_filter(&self.index_dir, segment_id)
        }

        fn save_bloom_filter_atomic(&self, segment_id: u64, bloom: &BloomFilter, keys: &[String]) -> Result<()> {
            super::save_bloom_filter_atomic(&self.index_dir, segment_id, bloom, keys)
        }

        fn insert_bloom_into_cache(&self, segment_id: u64, bloom: BloomFilter) {
            self.bloom_cache.lock().unwrap().insert(segment_id, bloom);
        }
    }

    #[test]
    fn test_bloom_manager_rebuild_for_segment() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut provider = MockProvider::new(temp_dir.clone());
        provider.add_segment(
            1,
            vec![
                ("key1".to_string(), b"value1".to_vec()),
                ("key2".to_string(), b"value2".to_vec()),
            ],
        );

        let manager = BloomManager::new(BloomConfig::default());

        // Rebuild bloom filter for segment
        let keys = vec!["key1".to_string(), "key2".to_string()];

        let result = manager.rebuild_for_segment(&provider, 1, &keys);
        assert!(result.is_ok());

        // Verify bloom was saved
        assert!(provider.bloom_filter_exists(1));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_bloom_manager_rebuild_all() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_test_all");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut provider = MockProvider::new(temp_dir.clone());
        provider.add_segment(
            1,
            vec![
                ("key1".to_string(), b"value1".to_vec()),
                ("key2".to_string(), b"value2".to_vec()),
            ],
        );
        provider.add_segment(
            2,
            vec![
                ("key3".to_string(), b"value3".to_vec()),
                ("key4".to_string(), b"value4".to_vec()),
            ],
        );

        let manager = BloomManager::new(BloomConfig::default());
        let result = manager.rebuild_all(&provider);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        // Verify both blooms were saved
        assert!(provider.bloom_filter_exists(1));
        assert!(provider.bloom_filter_exists(2));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_v3_save_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_v3_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create a bloom filter
        let mut bloom = BloomFilter::with_rate(0.01, 100);
        bloom.insert(&"key1".to_string());
        bloom.insert(&"key2".to_string());

        let keys = vec!["key1".to_string(), "key2".to_string()];

        // Save as V3
        let result = save_bloom_filter_v3(&temp_dir, 1, &bloom, &keys);
        assert!(result.is_ok());

        // Load as V3
        let loaded = load_bloom_filter_v3(&temp_dir, 1).unwrap();
        assert!(loaded.is_some());

        let custom_bloom = loaded.unwrap();
        assert!(custom_bloom.contains(b"key1"));
        assert!(custom_bloom.contains(b"key2"));
        assert!(!custom_bloom.contains(b"key3"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_v3_migration_from_v2() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_v3_migration_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create and save as V2
        let mut bloom = BloomFilter::with_rate(0.01, 100);
        bloom.insert(&"test_key".to_string());
        let keys = vec!["test_key".to_string()];
        save_bloom_filter_atomic(&temp_dir, 1, &bloom, &keys).unwrap();

        // Migrate to V3
        let migrated = migrate_to_v3(&temp_dir, 1).unwrap();
        assert!(migrated);

        // Load as V3
        let loaded = load_bloom_filter_v3(&temp_dir, 1).unwrap();
        assert!(loaded.is_some());
        assert!(loaded.unwrap().contains(b"test_key"));

        // Try migrating again - should return false (already V3)
        let migrated_again = migrate_to_v3(&temp_dir, 1).unwrap();
        assert!(!migrated_again);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_custom_bloom_with_migration_v3_direct() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_custom_load_v3");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create and save as V3 directly
        let mut custom_bloom = CustomBloom::with_capacity(100, 0.01);
        custom_bloom.insert(b"key1");
        custom_bloom.insert(b"key2");
        custom_bloom.save_to_file(&temp_dir.join("bloom_000001.bin")).unwrap();

        // Load with automatic migration function
        let loaded = load_custom_bloom_with_migration(&temp_dir, 1).unwrap();
        assert!(loaded.is_some());

        let bloom = loaded.unwrap();
        assert!(bloom.contains(b"key1"));
        assert!(bloom.contains(b"key2"));
        assert!(!bloom.contains(b"key3"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_custom_bloom_with_migration_from_v2() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_custom_load_v2_migration");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create and save as V2
        let mut bloom = BloomFilter::with_rate(0.01, 100);
        bloom.insert(&"test_key".to_string());
        let keys = vec!["test_key".to_string()];
        save_bloom_filter_atomic(&temp_dir, 1, &bloom, &keys).unwrap();

        // Load with automatic migration function - should migrate to V3
        let loaded = load_custom_bloom_with_migration(&temp_dir, 1).unwrap();
        assert!(loaded.is_some());

        let custom_bloom = loaded.unwrap();
        assert!(custom_bloom.contains(b"test_key"));
        assert!(!custom_bloom.contains(b"nonexistent"));

        // Verify file is now V3 format
        let bloom_path = temp_dir.join("bloom_000001.bin");
        let mut file = File::open(&bloom_path).unwrap();
        let mut magic_buf = [0u8; 4];
        file.read_exact(&mut magic_buf).unwrap();
        let magic = u32::from_le_bytes(magic_buf);
        assert_eq!(
            magic,
            super::super::custom_bloom::CUSTOM_BLOOM_MAGIC,
            "Should be V3 format after migration"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_custom_bloom_nonexistent_file() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_custom_nonexistent");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Try to load non-existent bloom
        let loaded = load_custom_bloom_with_migration(&temp_dir, 999).unwrap();
        assert!(loaded.is_none());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_custom_bloom_fpr_accuracy() {
        // Test FPR accuracy with different configurations
        let test_cases = vec![
            (1000, 0.01),   // 1000 items, 1% target
            (10000, 0.01),  // 10000 items, 1% target
            (10000, 0.001), // 10000 items, 0.1% target
        ];

        for (num_items, target_fpr) in test_cases {
            let mut bloom = CustomBloom::with_capacity(num_items, target_fpr);

            // Insert items
            for i in 0..num_items {
                bloom.insert(format!("item_{}", i).as_bytes());
            }

            // Test false positive rate
            let mut false_positives = 0;
            let test_count = 10000;

            for i in num_items..(num_items + test_count) {
                if bloom.contains(format!("item_{}", i).as_bytes()) {
                    false_positives += 1;
                }
            }

            let actual_fpr = false_positives as f64 / test_count as f64;

            // Actual FPR should be reasonably close to target (within 5x for statistical variance)
            assert!(
                actual_fpr < target_fpr * 5.0,
                "FPR for {} items at target {:.4}: actual {:.4} exceeds target * 5",
                num_items,
                target_fpr,
                actual_fpr
            );

            println!(
                "FPR test: {} items, target={:.4}, actual={:.4} ({} false positives)",
                num_items, target_fpr, actual_fpr, false_positives
            );
        }
    }

    #[test]
    fn test_custom_bloom_save_load_roundtrip_large() {
        let temp_dir = std::env::temp_dir().join("filekv_bloom_custom_roundtrip_large");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let num_items = 50000;
        let path = temp_dir.join("large_bloom.bin");

        // Create and save large bloom filter
        let mut original = CustomBloom::with_capacity(num_items, 0.01);
        for i in 0..num_items {
            original.insert(format!("key_{}", i).as_bytes());
        }

        original.save_to_file(&path).unwrap();

        // Load
        let loaded = CustomBloom::load_from_file(&path).unwrap().expect("Should load");

        // Verify bitset identity
        assert_eq!(original, loaded, "Roundtrip should preserve bitset exactly");

        // Verify all keys still work
        for i in (0..num_items).step_by(100) {
            assert!(
                loaded.contains(format!("key_{}", i).as_bytes()),
                "Key {} should be found",
                i
            );
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
