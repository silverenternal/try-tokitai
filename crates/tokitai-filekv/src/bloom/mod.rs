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

pub mod manager;
pub mod filter_cache;
pub mod adaptive;
pub mod compressed;
pub mod migration;
pub mod fpr_controller;
pub mod custom_bloom;

// Re-export main types
pub use manager::{BloomManager, BloomConfig, BloomSegmentProvider};
pub use manager::{save_bloom_filter_atomic, load_bloom_filter, bloom_filter_exists};
pub use manager::{save_bloom_filter_v3, load_bloom_filter_v3, migrate_to_v3};
pub use filter_cache::{BloomFilterCache, BloomFilterCacheConfig, BloomFilterCacheStats};
pub use adaptive::{AdaptiveBloomCache, AdaptiveBloomCacheConfig, AdaptiveBloomCacheStats, CacheLayer};
pub use compressed::CompressedBloom;
pub use migration::{MigrationController, MigrationThresholds, FrequencyTier, classify_by_frequency};
pub use fpr_controller::{FPRController, FPRControllerStats, AdaptationPolicy, FPRLevel, FPRAdjustedBloom};
pub use custom_bloom::{CustomBloom, CUSTOM_BLOOM_MAGIC, CUSTOM_BLOOM_VERSION};

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use tracing::debug;

use crate::{FileKV, SegmentFile, BLOOM_MAGIC, BLOOM_VERSION, DEFAULT_BLOOM_FPR};
use ::bloom::BloomFilter;
use ::bloom::ASMS;

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
    pub fn rebuild_bloom_filter_for_segment(
        &self,
        segment_id: u64,
        keys: &BTreeMap<String, Vec<u8>>,
    ) -> Result<()> {
        tracing::debug!("Building bloom filter for segment {} using {} keys from memory", segment_id, keys.len());

        let mut bloom = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, keys.len().max(10000) as u32);
        let key_strings: Vec<String> = keys.keys().cloned().collect();

        for key in &key_strings {
            bloom.insert(key);
        }

        if let Err(e) = self.save_bloom_filter_atomic(segment_id, &bloom, &key_strings) {
            tracing::error!("Failed to save bloom filter for segment {}: {}", segment_id, e);
            return Err(FatalError::Corruption(format!("Failed to save bloom filter: {}", e)));
        }

        self.bloom_filter_cache_ref().insert(segment_id, bloom);
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
            match self.load_bloom_filter(*seg_id) {
                Ok(Some((bloom, _keys))) => {
                    self.bloom_filter_cache_ref().insert(*seg_id, bloom);
                    loaded_count += 1;
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Bloom filter file for segment {} corrupted: {}. Will rebuild.", seg_id, e);
                }
            }

            tracing::info!("Rebuilding bloom filter for segment {}", seg_id);

            if let Some(segment) = segments_guard.get(seg_id) {
                if let Err(e) = self.validate_segment_integrity(segment) {
                    tracing::error!("Segment {} failed integrity check, skipping bloom rebuild: {}", seg_id, e);
                    skipped_count += 1;
                    continue;
                }

                let mut bloom = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 10000);
                let mut keys = Vec::new();

                segment.iterate_entries(|key, _value, _deleted| {
                    bloom.insert(&key);
                    keys.push(key.to_string());
                    Ok(())
                })?;

                if let Err(e) = self.save_bloom_filter_atomic(*seg_id, &bloom, &keys) {
                    tracing::error!("Failed to save bloom filter for segment {}: {}", seg_id, e);
                    skipped_count += 1;
                    continue;
                }

                self.bloom_filter_cache_ref().insert(*seg_id, bloom);
                rebuilt_count += 1;
            } else {
                tracing::warn!("Segment {} not found in segments map", seg_id);
                skipped_count += 1;
            }
        }

        tracing::info!(
            "Bloom filter rebuild complete: loaded={}, rebuilt={}, skipped={}",
            loaded_count, rebuilt_count, skipped_count
        );
        Ok(rebuilt_count)
    }

    /// Validate segment file integrity by checking magic bytes and sampling checksums
    pub(super) fn validate_segment_integrity(&self, segment: &SegmentFile) -> Result<()> {
        const SEGMENT_MAGIC: u32 = 0x54435347; // "TCSG" = Tokitai Context SeGment
        const SEGMENT_VERSION: u32 = 1;

        let mut file = File::open(&segment.path)
            .map_err(FatalError::Io)?;

        let mut header = [0u8; 8];
        file.read_exact(&mut header)
            .map_err(FatalError::Io)?;

        let magic = u32::from_le_bytes(header[0..4].try_into().map_err(|e| FatalError::Corruption(format!("Invalid magic bytes: {}", e)))?);
        if magic != SEGMENT_MAGIC {
            return Err(FatalError::Corruption(format!("Invalid segment magic: expected {:08X}, got {:08X}",
                         SEGMENT_MAGIC, magic)));
        }

        let version = u32::from_le_bytes(header[4..8].try_into().map_err(|e| FatalError::Corruption(format!("Invalid version bytes: {}", e)))?);
        if version != SEGMENT_VERSION {
            return Err(FatalError::Corruption(format!("Unsupported segment version: expected {}, got {}",
                         SEGMENT_VERSION, version)));
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
                        return Err(FatalError::Corruption(format!("Entry {} has invalid checksum (0)", verified_entries)));
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
    /// Format: [magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitvec_len 4B][bitvec_bytes...]
    pub(super) fn save_bloom_filter_atomic(&self, segment_id: u64, bloom: &BloomFilter, keys: &[String]) -> Result<()> {
        let _ = keys; // unused in v3, but kept for API compatibility
        
        let bloom_path = self.config.index_dir.join(format!("bloom_{:06}.bin", segment_id));
        let temp_path = self.config.index_dir.join(format!("bloom_{:06}.tmp", segment_id));

        let mut file = BufWriter::new(
            File::create(&temp_path)
                .map_err(FatalError::Io)?
        );

        file.write_all(&BLOOM_MAGIC.to_le_bytes())?;
        file.write_all(&BLOOM_VERSION.to_le_bytes())?;

        // Write bloom filter metadata
        let num_bits = bloom.num_bits() as u32;
        let num_hashes = bloom.num_hashes();
        file.write_all(&num_bits.to_le_bytes())?;
        file.write_all(&num_hashes.to_le_bytes())?;

        // POL-003: Write bit vector directly
        let bitvec_bytes = bloom.to_bytes();
        let bitvec_len = bitvec_bytes.len() as u32;
        file.write_all(&bitvec_len.to_le_bytes())?;
        file.write_all(&bitvec_bytes)?;

        file.flush()?;
        file.get_ref().sync_all()
            .map_err(FatalError::Io)?;
        drop(file);

        fs::rename(&temp_path, &bloom_path)
            .map_err(FatalError::Io)?;

        if let Ok(dir) = File::open(&self.config.index_dir) {
            let _ = dir.sync_all();
        }

        debug!("Atomically saved bloom filter v3 with {} bits, {} hashes, {} byte bitvec for segment {} to {:?}",
                             num_bits, num_hashes, bitvec_len, segment_id, bloom_path);
        Ok(())
    }
}
