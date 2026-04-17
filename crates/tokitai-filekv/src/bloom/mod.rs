//! Bloom filter operations for FileKV
//!
//! This module provides bloom filter management through two approaches:
//! 1. **BloomManager** (preferred): Standalone manager using `BloomSegmentProvider` trait
//! 2. **impl FileKV** (legacy, deprecated): Direct methods on FileKV for backward compatibility
//!
//! ## Migration Guide
//! - New code should use `BloomManager` with `BloomSegmentProvider` trait
//! - Existing code using `FileKV::rebuild_bloom_filters()` will continue to work
//! - The legacy `impl FileKV` methods are now thin wrappers around `BloomManager`

pub mod adaptive;
pub mod compressed;
pub mod custom_bloom;
pub mod filter_cache;
pub mod fpr_controller;
pub mod manager;
pub mod migration;

// Re-export main types
pub use adaptive::{AdaptiveBloomCache, AdaptiveBloomCacheConfig, AdaptiveBloomCacheStats, CacheLayer};
pub use adaptive::{BloomFilterWrapper, CustomBloomCache, CustomBloomCacheConfig, CustomBloomCacheStats};
pub use compressed::CompressedBloom;
pub use custom_bloom::{CustomBloom, CUSTOM_BLOOM_MAGIC, CUSTOM_BLOOM_VERSION};
pub use filter_cache::{BloomFilterCache, BloomFilterCacheConfig, BloomFilterCacheStats, FilterWrapper};
pub use fpr_controller::{AdaptationPolicy, FPRAdjustedBloom, FPRController, FPRControllerStats, FPRLevel};
pub use manager::{bloom_filter_exists, load_bloom_filter, save_bloom_filter_atomic};
pub use manager::{load_bloom_filter_v3, load_custom_bloom_with_migration, migrate_to_v3, save_bloom_filter_v3};
pub use manager::{save_custom_bloom_v3, BloomConfig, BloomManager, BloomSegmentProvider};
pub use migration::{classify_by_frequency, FrequencyTier, MigrationController, MigrationThresholds};

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::{FileKV, SegmentFile, DEFAULT_BLOOM_FPR};
use ::bloom::BloomFilter;

/// Result type for bloom operations
pub type Result<T> = std::result::Result<T, FatalError>;

use crate::core::error::FatalError;

// ============================================================
// Legacy impl FileKV - DEPRECATED, use BloomManager instead
// ============================================================
// These methods are kept for backward compatibility but internally
// delegate to the new BloomManager standalone functions.
// ============================================================

#[allow(dead_code)]
impl FileKV {
    /// Rebuild bloom filter for a single segment using pre-collected keys from memory
    ///
    /// This is optimized for compaction: instead of iterating the segment file again,
    /// it uses the keys already collected in memory during the compaction merge.
    /// This avoids the deadlock issue where rebuild_bloom_filters() acquires a read lock
    /// while being called from execute_compaction_inner() which holds a write lock.
    pub fn rebuild_bloom_filter_for_segment(&self, segment_id: u64, keys: &BTreeMap<String, Vec<u8>>) -> Result<()> {
        tracing::debug!(
            "Building bloom filter for segment {} using {} keys from memory",
            segment_id,
            keys.len()
        );

        let key_strings: Vec<String> = keys.keys().cloned().collect();

        let custom_bloom = CustomBloom::from_keys(&key_strings, key_strings.len(), DEFAULT_BLOOM_FPR as f64);

        if let Err(e) = self.save_custom_bloom_v3(segment_id, &custom_bloom) {
            tracing::error!("Failed to save bloom filter for segment {}: {}", segment_id, e);
            return Err(FatalError::Corruption(format!("Failed to save bloom filter: {}", e)));
        }

        self.bloom_filter_cache_ref()
            .insert(segment_id, filter_cache::FilterWrapper::Custom(custom_bloom));
        tracing::debug!("Bloom filter built and cached for segment {}", segment_id);
        Ok(())
    }

    /// Rebuild bloom filters for all segments with validation and atomic writes
    ///
    /// P0-008 FIX:
    /// - Validates segment integrity before rebuilding (checksum verification)
    /// - Uses atomic rename (temp file → final file) to prevent corruption
    /// - Preserves old filter as backup during rebuild
    /// - Only rebuilds if segment passes validation
    ///
    /// P2-011: Updated to use bloom_filter_cache with on-demand loading
    pub fn rebuild_bloom_filters(&self) -> Result<usize> {
        let segments_guard = self.segments().load();
        let mut rebuilt_count = 0;
        let mut loaded_count = 0;
        let mut skipped_count = 0;

        for (seg_id, _) in segments_guard.iter() {
            // Try loading V3 CustomBloom first (preferred path)
            match self.load_custom_bloom_v3(*seg_id) {
                Ok(Some(custom_bloom)) => {
                    self.bloom_filter_cache_ref()
                        .insert(*seg_id, filter_cache::FilterWrapper::Custom(custom_bloom));
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

            if let Some(segment) = segments_guard.get(seg_id) {
                if let Err(e) = self.validate_segment_integrity(segment) {
                    tracing::error!(
                        "Segment {} failed integrity check, skipping bloom rebuild: {}",
                        seg_id,
                        e
                    );
                    skipped_count += 1;
                    continue;
                }

                let mut keys = Vec::new();

                segment.iterate_entries(|key, _value, _deleted| {
                    keys.push(key.to_string());
                    Ok(())
                })?;

                let custom_bloom = CustomBloom::from_keys(&keys, keys.len(), DEFAULT_BLOOM_FPR as f64);

                if let Err(e) = self.save_custom_bloom_v3(*seg_id, &custom_bloom) {
                    tracing::error!("Failed to save bloom filter for segment {}: {}", seg_id, e);
                    skipped_count += 1;
                    continue;
                }

                self.bloom_filter_cache_ref()
                    .insert(*seg_id, filter_cache::FilterWrapper::Custom(custom_bloom));
                rebuilt_count += 1;
            } else {
                tracing::warn!("Segment {} not found in segments map", seg_id);
                skipped_count += 1;
            }
        }

        tracing::info!(
            "Bloom filter rebuild complete: loaded={}, rebuilt={}, skipped={}",
            loaded_count,
            rebuilt_count,
            skipped_count
        );
        Ok(rebuilt_count)
    }

    /// Validate segment file integrity by checking magic bytes and sampling checksums
    pub(super) fn validate_segment_integrity(&self, segment: &SegmentFile) -> Result<()> {
        const SEGMENT_MAGIC: u32 = 0x54435347; // "TCSG" = Tokitai Context SeGment
        const SEGMENT_VERSION: u32 = 1;

        let mut file = File::open(&segment.path).map_err(FatalError::Io)?;

        let mut header = [0u8; 8];
        file.read_exact(&mut header).map_err(FatalError::Io)?;

        let magic = u32::from_le_bytes(
            header[0..4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid magic bytes: {}", e)))?,
        );
        if magic != SEGMENT_MAGIC {
            return Err(FatalError::Corruption(format!(
                "Invalid segment magic: expected {:08X}, got {:08X}",
                SEGMENT_MAGIC, magic
            )));
        }

        let version = u32::from_le_bytes(
            header[4..8]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid version bytes: {}", e)))?,
        );
        if version != SEGMENT_VERSION {
            return Err(FatalError::Corruption(format!(
                "Unsupported segment version: expected {}, got {}",
                SEGMENT_VERSION, version
            )));
        }

        let mut verified_entries = 0;
        let max_verify_entries = 3;

        drop(file);
        let mut file = File::open(&segment.path)?;
        file.seek(SeekFrom::Start(8))?;

        while verified_entries < max_verify_entries {
            let mut len_buf = [0u8; 4];
            match file.read_exact(&mut len_buf) {
                Ok(_) => {
                    let key_len = u32::from_le_bytes(len_buf) as usize;
                    file.seek(SeekFrom::Current(key_len as i64))?;
                    file.read_exact(&mut len_buf)?;
                    let value_len = u32::from_le_bytes(len_buf) as usize;
                    file.seek(SeekFrom::Current(value_len as i64))?;

                    let mut checksum_buf = [0u8; 4];
                    file.read_exact(&mut checksum_buf)?;
                    let stored_checksum = u32::from_le_bytes(checksum_buf);

                    if stored_checksum == 0 {
                        return Err(FatalError::Corruption(format!(
                            "Entry {} has invalid checksum (0)",
                            verified_entries
                        )));
                    }

                    verified_entries += 1;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => {
                    return Err(FatalError::Io(e));
                }
            }
        }

        if verified_entries == 0 {
            tracing::warn!("Segment {} has no entries to verify", segment.id);
        }

        Ok(())
    }

    /// Save bloom filter atomically using temp file + rename (v3 format)
    ///
    /// P0-008 FIX: Prevents corruption from crashes during write
    /// POL-003: V3 format stores bit vector directly, eliminating rebuild overhead.
    /// Converts to CustomBloom from keys for V3 persistence.
    pub(super) fn save_bloom_filter_atomic(
        &self,
        segment_id: u64,
        _bloom: &BloomFilter,
        keys: &[String],
    ) -> Result<()> {
        // Convert BloomFilter to CustomBloom via key reconstruction
        let custom_bloom = CustomBloom::from_keys(keys, keys.len(), DEFAULT_BLOOM_FPR as f64);
        save_custom_bloom_v3(&self.config.index_dir, segment_id, &custom_bloom)
    }

    /// Save CustomBloom directly in V3 format (uses deterministic XXH3 hashing)
    pub(super) fn save_custom_bloom_v3(&self, segment_id: u64, custom_bloom: &CustomBloom) -> Result<()> {
        save_custom_bloom_v3(&self.config.index_dir, segment_id, custom_bloom)
    }

    /// Load CustomBloom from V3 format (fast, direct bitset load)
    pub(super) fn load_custom_bloom_v3(&self, segment_id: u64) -> Result<Option<CustomBloom>> {
        load_custom_bloom_with_migration(&self.config.index_dir, segment_id)
    }
}
