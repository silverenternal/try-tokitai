//! FileKV configuration
//!
//! This module contains configuration structures for FileKV storage engine.

use std::path::PathBuf;
use crate::audit_log::AuditLogConfig;
use crate::compaction::CompactionConfig;
use super::{BlockCacheConfig, MemTableConfig};
use super::types::FileKVConfigValidation;

/// FileKV configuration
#[derive(Debug, Clone)]
pub struct FileKVConfig {
    /// Segment files directory
    pub segment_dir: PathBuf,
    /// Write-Ahead Log directory
    pub wal_dir: PathBuf,
    /// Index files directory
    pub index_dir: PathBuf,
    /// Enable WAL
    pub enable_wal: bool,
    /// Enable Bloom filter
    pub enable_bloom: bool,
    /// Enable background flush
    pub enable_background_flush: bool,
    /// Background flush interval in milliseconds
    pub background_flush_interval_ms: u64,
    /// Segment preallocate size in bytes
    pub segment_preallocate_size: u64,
    /// MemTable configuration
    pub memtable: MemTableConfig,
    /// Block cache configuration
    pub cache: BlockCacheConfig,
    /// Compaction configuration
    pub compaction: CompactionConfig,
    /// WAL max size in bytes
    pub wal_max_size_bytes: u64,
    /// WAL max files
    pub wal_max_files: u64,
    /// Enable write coalescing
    pub write_coalescing_enabled: bool,
    /// Enable cache warming
    pub cache_warming_enabled: bool,
    /// Dictionary compression configuration
    pub compression: super::DictionaryCompressionConfig,
    /// Enable async I/O
    pub async_io_enabled: bool,
    /// Async I/O max concurrent writes
    pub async_io_max_concurrent_writes: usize,
    /// Async I/O max queue depth
    pub async_io_max_queue_depth: usize,
    /// Async I/O write timeout in milliseconds
    pub async_io_write_timeout_ms: u64,
    /// Async I/O enable coalescing
    pub async_io_enable_coalescing: bool,
    /// Async I/O coalesce window in milliseconds
    pub async_io_coalesce_window_ms: u64,
    /// Checkpoint directory
    pub checkpoint_dir: PathBuf,
    /// Audit log configuration
    pub audit_log: AuditLogConfig,
}

impl Default for FileKVConfig {
    fn default() -> Self {
        Self {
            segment_dir: PathBuf::from("segments"),
            wal_dir: PathBuf::from("wal"),
            index_dir: PathBuf::from("index"),
            enable_wal: true,
            enable_bloom: true,
            enable_background_flush: true,
            background_flush_interval_ms: 100,
            segment_preallocate_size: 16 * 1024 * 1024, // 16MB
            memtable: MemTableConfig::default(),
            cache: BlockCacheConfig::default(),
            compaction: CompactionConfig::default(),
            wal_max_size_bytes: 100 * 1024 * 1024, // 100MB
            wal_max_files: 5,
            write_coalescing_enabled: true,
            cache_warming_enabled: true,
            compression: super::DictionaryCompressionConfig::default(),
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            async_io_enable_coalescing: true,
            async_io_coalesce_window_ms: 10,
            checkpoint_dir: PathBuf::from("checkpoints"),
            audit_log: AuditLogConfig::default(),
        }
    }
}

impl FileKVConfig {
    /// Validate configuration
    pub fn validate(&self) -> FileKVConfigValidation {
        let mut validation = FileKVConfigValidation::default();
        
        // Validate memtable config
        let memtable_validation = self.memtable.validate();
        validation.errors.extend(memtable_validation.errors);
        validation.warnings.extend(memtable_validation.warnings);
        
        // Validate cache config
        let cache_validation = self.cache.validate();
        validation.errors.extend(cache_validation.errors);
        validation.warnings.extend(cache_validation.warnings);
        
        // Validate compaction config
        let compaction_validation = self.compaction.validate();
        validation.errors.extend(compaction_validation.errors);
        validation.warnings.extend(compaction_validation.warnings);
        
        // Validate WAL config
        if self.enable_wal {
            if self.wal_max_size_bytes < 1024 * 1024 {
                validation.warnings.push("WAL max size is very small".to_string());
            }
            if self.wal_max_files < 1 {
                validation.errors.push("WAL max files must be at least 1".to_string());
            }
        }
        
        // Validate async I/O config
        if self.async_io_enabled {
            if self.async_io_max_concurrent_writes < 1 {
                validation.errors.push("Async I/O max concurrent writes must be at least 1".to_string());
            }
            if self.async_io_max_queue_depth < 1 {
                validation.errors.push("Async I/O max queue depth must be at least 1".to_string());
            }
        }
        
        validation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = FileKVConfig::default();
        assert!(config.enable_wal);
        assert!(config.enable_bloom);
        assert_eq!(config.wal_max_files, 5);
    }

    #[test]
    fn test_config_validate() {
        let config = FileKVConfig::default();
        let validation = config.validate();
        assert!(validation.errors.is_empty());
    }
}
