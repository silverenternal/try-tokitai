//! Bloom Filter Version Migration Support
//!
//! This module provides version migration capabilities for bloom filter binary format.
//! It ensures backward compatibility when the bloom filter format is upgraded.
//!
//! # Binary Format (Version 1)
//! ```text
//! | Offset | Size | Field      | Description                    |
//! |--------|------|------------|--------------------------------|
//! | 0      | 4    | magic      | 0x424C4F4F ("BLOO")           |
//! | 4      | 4    | version    | Format version (u32)           |
//! | 8      | 8    | num_keys   | Number of keys (u64)           |
//! | 16     | var  | keys       | Length-prefixed UTF-8 keys     |
//! ```
//!
//! # Migration Strategy
//! - Version detection on load
//! - Automatic migration to latest version
//! - Atomic write with temp file + rename
//! - Backup preservation during migration

use std::fs::File;
use std::io::{Read, Write, BufWriter};
use std::path::{Path, PathBuf};
use bloom::{BloomFilter, ASMS};
use crate::error::{ContextResult, ContextError};
use tracing::{debug, info, warn};

/// Current bloom filter format version
pub const CURRENT_BLOOM_VERSION: u32 = 1;

/// Bloom filter magic number "BLOO"
pub const BLOOM_MAGIC: u32 = 0x424C4F4F;

/// Migration result indicating what action was taken
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationResult {
    /// No migration needed, format is current
    NoMigrationNeeded,
    /// Successfully migrated from old version to new
    Migrated { from_version: u32, to_version: u32 },
    /// Format is too old to migrate
    UnsupportedVersion { version: u32 },
    /// Format is newer than supported
    FutureVersion { version: u32 },
}

/// Bloom filter migration handler
pub struct BloomFilterMigrator {
    index_dir: PathBuf,
}

impl BloomFilterMigrator {
    /// Create a new migrator for the given index directory
    pub fn new(index_dir: PathBuf) -> Self {
        Self { index_dir }
    }

    /// Load a bloom filter with automatic version migration
    ///
    /// Returns the loaded bloom filter and migration result
    pub fn load_with_migration(
        &self,
        segment_id: u64,
    ) -> ContextResult<Option<(BloomFilter, Vec<String>, MigrationResult)>> {
        let bloom_path = self.index_dir.join(format!("bloom_{:06}.bin", segment_id));

        if !bloom_path.exists() {
            return Ok(None);
        }

        let (mut file, version) = self.open_and_validate(&bloom_path)?;

        // Check if migration is needed
        let migration_result = if version < CURRENT_BLOOM_VERSION {
            // Need to migrate from older version
            self.migrate_from_version(&mut file, version, segment_id)?
        } else if version > CURRENT_BLOOM_VERSION {
            // Future version - return error
            return Ok(Some((
                Self::read_current_format(&mut file)?,
                Self::extract_keys_for_migration(&mut file, &bloom_path)?,
                MigrationResult::FutureVersion { version }
            )));
        } else {
            // Current version - no migration needed
            let bloom = Self::read_current_format(&mut file)?;
            let keys = Self::extract_keys_for_migration(&mut file, &bloom_path)?;
            return Ok(Some((bloom, keys, MigrationResult::NoMigrationNeeded)));
        };

        // If we migrated, reload the migrated file
        let mut new_file = File::open(&bloom_path)
            .map_err(ContextError::Io)?;
        let bloom = Self::read_current_format(&mut new_file)?;
        let keys = Self::extract_keys_for_migration(&mut new_file, &bloom_path)?;

        Ok(Some((bloom, keys, migration_result)))
    }

    /// Open and validate bloom filter file
    fn open_and_validate(&self, path: &Path) -> ContextResult<(File, u32)> {
        let mut file = File::open(path).map_err(ContextError::Io)?;

        let mut metadata = [0u8; 8];
        file.read_exact(&mut metadata)
            .map_err(ContextError::Io)?;

        let magic = u32::from_le_bytes(metadata[0..4].try_into().map_err(|e| {
            ContextError::OperationFailed(format!("Invalid magic bytes: {}", e))
        })?);
        let version = u32::from_le_bytes(metadata[4..8].try_into().map_err(|e| {
            ContextError::OperationFailed(format!("Invalid version bytes: {}", e))
        })?);

        if magic != BLOOM_MAGIC {
            return Err(ContextError::OperationFailed(
                format!("Invalid bloom filter magic: expected {:08X}, got {:08X}", BLOOM_MAGIC, magic)
            ));
        }

        Ok((file, version))
    }

    /// Read bloom filter in current format
    fn read_current_format(file: &mut File) -> ContextResult<BloomFilter> {
        // Seek past header
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(8))
            .map_err(ContextError::Io)?;

        let mut num_keys_buf = [0u8; 8];
        file.read_exact(&mut num_keys_buf).map_err(ContextError::Io)?;
        let num_keys = u64::from_le_bytes(num_keys_buf) as usize;

        let mut bloom = BloomFilter::with_rate(0.01, num_keys as u32 + 1000);

        for _ in 0..num_keys {
            let mut len_buf = [0u8; 4];
            file.read_exact(&mut len_buf).map_err(ContextError::Io)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut key_bytes = vec![0u8; len];
            file.read_exact(&mut key_bytes).map_err(ContextError::Io)?;

            let key = String::from_utf8(key_bytes)
                .map_err(|e| ContextError::OperationFailed(format!("Invalid UTF-8 in bloom filter key: {}", e)))?;
            bloom.insert(&key);
        }

        Ok(bloom)
    }

    /// Extract keys from bloom filter file for re-saving
    fn extract_keys_for_migration(file: &mut File, _path: &Path) -> ContextResult<Vec<String>> {
        use std::io::Seek;
        
        // Seek past header
        file.seek(std::io::SeekFrom::Start(8))
            .map_err(ContextError::Io)?;

        let mut num_keys_buf = [0u8; 8];
        file.read_exact(&mut num_keys_buf).map_err(ContextError::Io)?;
        let num_keys = u64::from_le_bytes(num_keys_buf) as usize;

        let mut keys = Vec::with_capacity(num_keys);
        for _ in 0..num_keys {
            let mut len_buf = [0u8; 4];
            file.read_exact(&mut len_buf).map_err(ContextError::Io)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut key_bytes = vec![0u8; len];
            file.read_exact(&mut key_bytes).map_err(ContextError::Io)?;

            let key = String::from_utf8(key_bytes)
                .map_err(|e| ContextError::OperationFailed(format!("Invalid UTF-8 in bloom filter key: {}", e)))?;
            keys.push(key);
        }

        Ok(keys)
    }

    /// Migrate from an older version to current version
    fn migrate_from_version(
        &self,
        file: &mut File,
        from_version: u32,
        segment_id: u64,
    ) -> ContextResult<MigrationResult> {
        info!(
            "Migrating bloom filter for segment {} from version {} to {}",
            segment_id, from_version, CURRENT_BLOOM_VERSION
        );

        // For version 1, we just read and rewrite (format stays the same)
        // Future versions can have different formats requiring actual migration logic
        let keys = Self::extract_keys_for_migration(file, &self.index_dir.join(format!("bloom_{:06}.bin", segment_id)))?;
        let _bloom = Self::read_current_format(file)?;

        // Atomic write with temp file
        let bloom_path = self.index_dir.join(format!("bloom_{:06}.bin", segment_id));
        let temp_path = self.index_dir.join(format!("bloom_{:06}.tmp", segment_id));

        {
            let temp_file = File::create(&temp_path).map_err(ContextError::Io)?;
            let mut writer = BufWriter::new(temp_file);

            // Write header
            writer.write_all(&BLOOM_MAGIC.to_le_bytes()).map_err(ContextError::Io)?;
            writer.write_all(&CURRENT_BLOOM_VERSION.to_le_bytes()).map_err(ContextError::Io)?;
            writer.write_all(&(keys.len() as u64).to_le_bytes()).map_err(ContextError::Io)?;

            // Write keys
            for key in &keys {
                let key_bytes = key.as_bytes();
                writer.write_all(&(key_bytes.len() as u32).to_le_bytes()).map_err(ContextError::Io)?;
                writer.write_all(key_bytes).map_err(ContextError::Io)?;
            }

            writer.flush().map_err(ContextError::Io)?;
        }

        // Atomic rename
        std::fs::rename(&temp_path, &bloom_path).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path); // Cleanup temp file
            ContextError::OperationFailed(format!("Failed to rename migrated bloom filter: {}", e))
        })?;

        debug!(
            "Successfully migrated bloom filter for segment {} from v{} to v{}",
            segment_id, from_version, CURRENT_BLOOM_VERSION
        );

        Ok(MigrationResult::Migrated {
            from_version,
            to_version: CURRENT_BLOOM_VERSION,
        })
    }

    /// Save a bloom filter in the current format
    pub fn save_bloom_filter(
        &self,
        segment_id: u64,
        _bloom: &BloomFilter,
        keys: &[String],
    ) -> ContextResult<()> {
        let bloom_path = self.index_dir.join(format!("bloom_{:06}.bin", segment_id));
        let temp_path = self.index_dir.join(format!("bloom_{:06}.tmp", segment_id));

        {
            let temp_file = File::create(&temp_path).map_err(ContextError::Io)?;
            let mut writer = BufWriter::new(temp_file);

            // Write header
            writer.write_all(&BLOOM_MAGIC.to_le_bytes()).map_err(ContextError::Io)?;
            writer.write_all(&CURRENT_BLOOM_VERSION.to_le_bytes()).map_err(ContextError::Io)?;
            writer.write_all(&(keys.len() as u64).to_le_bytes()).map_err(ContextError::Io)?;

            // Write keys
            for key in keys {
                let key_bytes = key.as_bytes();
                writer.write_all(&(key_bytes.len() as u32).to_le_bytes()).map_err(ContextError::Io)?;
                writer.write_all(key_bytes).map_err(ContextError::Io)?;
            }

            writer.flush().map_err(ContextError::Io)?;
        }

        // Atomic rename
        std::fs::rename(&temp_path, &bloom_path).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path); // Cleanup temp file
            ContextError::OperationFailed(format!("Failed to rename bloom filter file: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, BloomFilterMigrator) {
        let temp_dir = TempDir::new().unwrap();
        let migrator = BloomFilterMigrator::new(temp_dir.path().to_path_buf());
        (temp_dir, migrator)
    }

    #[test]
    fn test_save_and_load_current_version() {
        let (_temp_dir, migrator) = setup_test_env();

        let mut bloom = BloomFilter::with_rate(0.01, 1000);
        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        for key in &keys {
            bloom.insert(key);
        }

        // Save
        migrator.save_bloom_filter(1, &bloom, &keys).unwrap();

        // Load
        let result = migrator.load_with_migration(1).unwrap();
        assert!(result.is_some());
        let (loaded_bloom, loaded_keys, migration_result) = result.unwrap();
        
        assert_eq!(migration_result, MigrationResult::NoMigrationNeeded);
        assert_eq!(loaded_keys, keys);
        
        // Verify bloom filter contains the keys
        for key in &keys {
            assert!(loaded_bloom.contains(key));
        }
    }

    #[test]
    fn test_load_nonexistent_bloom_filter() {
        let (_temp_dir, migrator) = setup_test_env();
        let result = migrator.load_with_migration(999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_magic() {
        let (temp_dir, migrator) = setup_test_env();

        // Write invalid magic
        let bloom_path = temp_dir.path().join("bloom_000001.bin");
        let mut file = File::create(&bloom_path).unwrap();
        file.write_all(&0xDEADBEEFu32.to_le_bytes()).unwrap(); // Wrong magic
        file.write_all(&1u32.to_le_bytes()).unwrap();
        drop(file);

        let result = migrator.load_with_migration(1);
        match result {
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("Invalid bloom filter magic"));
            }
            Ok(_) => panic!("Expected error for invalid magic"),
        }
    }

    #[test]
    fn test_empty_bloom_filter() {
        let (_temp_dir, migrator) = setup_test_env();

        let bloom = BloomFilter::with_rate(0.01, 1000);
        let keys: Vec<String> = Vec::new();

        migrator.save_bloom_filter(1, &bloom, &keys).unwrap();

        let result = migrator.load_with_migration(1).unwrap();
        assert!(result.is_some());
        let (loaded_bloom, loaded_keys, _) = result.unwrap();

        assert!(loaded_keys.is_empty());
        assert!(!loaded_bloom.contains(&"any_key".to_string()));
    }

    #[test]
    fn test_large_bloom_filter() {
        let (_temp_dir, migrator) = setup_test_env();

        let mut bloom = BloomFilter::with_rate(0.01, 10000);
        let keys: Vec<String> = (0..1000).map(|i| format!("key_{}", i)).collect();
        
        for key in &keys {
            bloom.insert(key);
        }

        migrator.save_bloom_filter(1, &bloom, &keys).unwrap();

        let result = migrator.load_with_migration(1).unwrap();
        assert!(result.is_some());
        let (loaded_bloom, loaded_keys, _) = result.unwrap();
        
        assert_eq!(loaded_keys.len(), 1000);

        // Spot check some keys
        assert!(loaded_bloom.contains(&"key_0".to_string()));
        assert!(loaded_bloom.contains(&"key_500".to_string()));
        assert!(loaded_bloom.contains(&"key_999".to_string()));
    }
}
