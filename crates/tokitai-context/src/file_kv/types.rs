//! Type definitions for FileKV
//!
//! This module contains core data structures:
//! - ValuePointer: Points to values in segment files
//! - FileKVConfig: Configuration with validation
//! - FileKVStats: Statistics counters

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicU64, Ordering};

use crate::block_cache::BlockCacheConfig;
use crate::compaction::CompactionConfig;
use super::memtable::MemTableConfig;
use crate::optimization::compression::dictionary::DictionaryCompressionConfig;
use crate::audit_log::AuditLogConfig;

/// Bloom Filter 文件魔法数 (exported for bloom module)
pub const BLOOM_MAGIC: u32 = 0x424C4F4F; // "BLOO" = Bloom Filter
/// Bloom Filter 文件版本 (exported for bloom module)
pub const BLOOM_VERSION: u32 = 1;

/// 值指针（指向 segment 文件中的位置）
#[derive(Debug, Clone, Copy)]
pub struct ValuePointer {
    /// 段文件 ID
    pub segment_id: u64,
    /// 段内偏移
    pub offset: u64,
    /// 值长度
    pub len: u32,
    /// CRC32 校验和
    pub checksum: u32,
}

impl ValuePointer {
    pub fn new(segment_id: u64, offset: u64, len: u32, checksum: u32) -> Self {
        Self {
            segment_id,
            offset,
            len,
            checksum,
        }
    }

    /// 序列化为字节（用于 WAL）
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&self.segment_id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.offset.to_le_bytes());
        buf[16..20].copy_from_slice(&self.len.to_le_bytes());
        buf[20..24].copy_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    /// 从字节反序列化
    pub fn from_bytes(buf: &[u8; 24]) -> Self {
        Self {
            segment_id: u64::from_le_bytes(buf[0..8].try_into().expect("Invalid segment_id bytes")),
            offset: u64::from_le_bytes(buf[8..16].try_into().expect("Invalid offset bytes")),
            len: u32::from_le_bytes(buf[16..20].try_into().expect("Invalid len bytes")),
            checksum: u32::from_le_bytes(buf[20..24].try_into().expect("Invalid checksum bytes")),
        }
    }
}

/// FileKV 配置验证错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum FileKVConfigError {
    #[error("MemTable flush threshold too low: {0} bytes (minimum: {1})")]
    MemTableThresholdTooLow(usize, usize),
    #[error("MemTable max entries too low: {0} (minimum: {1})")]
    MemTableMaxEntriesTooLow(usize, usize),
    #[error("Block cache size too small: {0} bytes (minimum: {1})")]
    BlockCacheTooSmall(usize, usize),
    #[error("Block cache max items too low: {0} (minimum: {1})")]
    BlockCacheMaxItemsTooLow(usize, usize),
    #[error("Background flush interval too short: {0}ms (minimum: {1}ms)")]
    BackgroundFlushIntervalTooShort(u64, u64),
    #[error("Compaction min_segments too large: {0} (maximum: {1})")]
    CompactionMinSegmentsTooLarge(usize, usize),
    #[error("Segment max size smaller than target: max={max}, target={target}")]
    SegmentSizeMismatch { max: u64, target: u64 },
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Path is not writable: {0}")]
    PathNotWritable(String),
}

/// FileKV 配置验证结果
#[derive(Debug, Clone)]
pub struct FileKVConfigValidation {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<FileKVConfigError>,
}

impl FileKVConfigValidation {
    pub fn is_valid(&self) -> bool {
        self.is_valid && self.errors.is_empty()
    }

    pub fn all_issues(&self) -> Vec<String> {
        let mut issues = self.warnings.clone();
        for err in &self.errors {
            issues.push(err.to_string());
        }
        issues
    }
}

/// FileKV 配置
#[derive(Debug, Clone)]
pub struct FileKVConfig {
    pub memtable: MemTableConfig,
    pub segment_dir: PathBuf,
    pub enable_wal: bool,
    pub wal_dir: PathBuf,
    pub index_dir: PathBuf,
    pub cache: BlockCacheConfig,
    pub enable_bloom: bool,
    pub compaction: CompactionConfig,
    pub enable_background_flush: bool,
    pub background_flush_interval_ms: u64,
    pub segment_preallocate_size: u64,
    // P1-013: WAL rotation configuration
    pub wal_max_size_bytes: u64,
    pub wal_max_files: usize,
    // P2-012: Write coalescing configuration
    pub write_coalescing_enabled: bool,
    // P2-004: Cache warming configuration
    pub cache_warming_enabled: bool,
    // P2-014: Dictionary compression configuration
    pub compression: DictionaryCompressionConfig,
    // P3-001: Async I/O configuration
    pub async_io_enabled: bool,
    pub async_io_max_concurrent_writes: usize,
    pub async_io_max_queue_depth: usize,
    pub async_io_write_timeout_ms: u64,
    pub async_io_enable_coalescing: bool,
    pub async_io_coalesce_window_ms: u64,
    /// P2-009: Checkpoint directory for incremental checkpoints
    pub checkpoint_dir: PathBuf,
    /// P2-013: Audit log configuration
    pub audit_log: AuditLogConfig,
}

impl FileKVConfig {
    /// 验证配置
    pub fn validate(&self) -> FileKVConfigValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        const MIN_MEMTABLE_THRESHOLD: usize = 64 * 1024;
        const MIN_MEMTABLE_ENTRIES: usize = 100;

        if self.memtable.flush_threshold_bytes < MIN_MEMTABLE_THRESHOLD {
            errors.push(FileKVConfigError::MemTableThresholdTooLow(
                self.memtable.flush_threshold_bytes,
                MIN_MEMTABLE_THRESHOLD,
            ));
        }

        if self.memtable.max_entries < MIN_MEMTABLE_ENTRIES {
            errors.push(FileKVConfigError::MemTableMaxEntriesTooLow(
                self.memtable.max_entries,
                MIN_MEMTABLE_ENTRIES,
            ));
        }

        const MIN_BLOCK_CACHE_SIZE: usize = 1024 * 1024;
        const MIN_BLOCK_CACHE_ITEMS: usize = 100;

        if self.cache.max_memory_bytes < MIN_BLOCK_CACHE_SIZE {
            errors.push(FileKVConfigError::BlockCacheTooSmall(
                self.cache.max_memory_bytes,
                MIN_BLOCK_CACHE_SIZE,
            ));
        }

        if self.cache.max_items < MIN_BLOCK_CACHE_ITEMS {
            errors.push(FileKVConfigError::BlockCacheMaxItemsTooLow(
                self.cache.max_items,
                MIN_BLOCK_CACHE_ITEMS,
            ));
        }

        const MIN_FLUSH_INTERVAL_MS: u64 = 10;

        if self.enable_background_flush && self.background_flush_interval_ms < MIN_FLUSH_INTERVAL_MS {
            errors.push(FileKVConfigError::BackgroundFlushIntervalTooShort(
                self.background_flush_interval_ms,
                MIN_FLUSH_INTERVAL_MS,
            ));
        }

        const MAX_MIN_SEGMENTS: usize = 20;

        if self.compaction.min_segments > MAX_MIN_SEGMENTS {
            errors.push(FileKVConfigError::CompactionMinSegmentsTooLarge(
                self.compaction.min_segments,
                MAX_MIN_SEGMENTS,
            ));
        }

        if self.compaction.max_segment_size_bytes < self.compaction.target_segment_size_bytes {
            errors.push(FileKVConfigError::SegmentSizeMismatch {
                max: self.compaction.max_segment_size_bytes,
                target: self.compaction.target_segment_size_bytes,
            });
        }

        for (name, path) in [
            ("segment_dir", &self.segment_dir),
            ("wal_dir", &self.wal_dir),
            ("index_dir", &self.index_dir),
            ("checkpoint_dir", &self.checkpoint_dir),
        ] {
            if path.as_os_str().is_empty() {
                errors.push(FileKVConfigError::InvalidPath(format!("{} is empty", name)));
                continue;
            }

            if path.exists() {
                if !path.is_dir() {
                    errors.push(FileKVConfigError::InvalidPath(format!("{} is not a directory", name)));
                } else {
                    let test_file = path.join(".write_test");
                    match std::fs::File::create(&test_file) {
                        Ok(_) => {
                            let _ = std::fs::remove_file(test_file);
                        }
                        Err(_) => {
                            errors.push(FileKVConfigError::PathNotWritable(name.to_string()));
                        }
                    }
                }
            } else {
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        warnings.push(format!("{} parent directory does not exist: {:?}", name, parent));
                    } else if !parent.is_dir() {
                        errors.push(FileKVConfigError::InvalidPath(format!("{} parent is not a directory", name)));
                    } else {
                        let test_file = parent.join(".write_test");
                        match std::fs::File::create(&test_file) {
                            Ok(_) => {
                                let _ = std::fs::remove_file(test_file);
                            }
                            Err(_) => {
                                errors.push(FileKVConfigError::PathNotWritable(format!("{} parent", name)));
                            }
                        }
                    }
                }
            }
        }

        if self.memtable.flush_threshold_bytes > 64 * 1024 * 1024 {
            warnings.push(format!(
                "Large MemTable threshold ({} bytes) may cause long flush pauses",
                self.memtable.flush_threshold_bytes
            ));
        }

        if self.cache.max_memory_bytes > 512 * 1024 * 1024 {
            warnings.push(format!(
                "Large block cache size ({} bytes) may cause memory pressure",
                self.cache.max_memory_bytes
            ));
        }

        FileKVConfigValidation {
            is_valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    pub fn validate_strict(&self) -> Result<(), FileKVConfigError> {
        let validation = self.validate();
        if validation.errors.is_empty() {
            Ok(())
        } else {
            // P0-003 FIX: Use expect() with clear error message instead of unwrap()
            Err(validation.errors.into_iter().next().expect(
                "Validation reported errors but none were found - this is a bug in validate()"
            ))
        }
    }
}

impl Default for FileKVConfig {
    fn default() -> Self {
        Self {
            memtable: MemTableConfig::default(),
            segment_dir: PathBuf::from("./segments"),
            enable_wal: true,
            wal_dir: PathBuf::from("./wal"),
            index_dir: PathBuf::from("./index"),
            cache: BlockCacheConfig::default(),
            enable_bloom: true,
            compaction: CompactionConfig::default(),
            enable_background_flush: true,
            background_flush_interval_ms: 100,
            segment_preallocate_size: 16 * 1024 * 1024,
            // P1-013: WAL rotation defaults
            wal_max_size_bytes: 100 * 1024 * 1024, // 100MB
            wal_max_files: 5,
            // P2-012: Write coalescing enabled by default for better write performance
            write_coalescing_enabled: true,
            // P2-004: Cache warming enabled by default for better read performance
            cache_warming_enabled: true,
            // P2-014: Dictionary compression enabled by default for better storage efficiency
            compression: DictionaryCompressionConfig::default(),
            // P3-001: Async I/O disabled by default (opt-in for production use)
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            async_io_enable_coalescing: true,
            async_io_coalesce_window_ms: 10,
            // P2-009: Checkpoint directory default
            checkpoint_dir: PathBuf::from("./checkpoints"),
            // P2-013: Audit log disabled by default (opt-in for compliance)
            audit_log: AuditLogConfig::default(),
        }
    }
}

/// FileKV 统计信息快照（用于返回）
#[derive(Debug, Clone, Default)]
pub struct FileKVStatsSnapshot {
    pub memtable_size: usize,
    pub memtable_entries: usize,
    pub segment_count: usize,
    pub total_size_bytes: u64,
    pub total_entries: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub flush_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub bloom_filtered: u64,
    pub compaction_runs: u64,
    pub compaction_segments_merged: u64,
    pub compaction_tombstones_removed: u64,
    // P2-014: Compression statistics
    pub compression_dict_trained: bool,
    pub compression_dict_size: usize,
    pub compression_ratio: f64,
    pub compressed_writes: u64,
    pub uncompressed_bytes: u64,
    pub compressed_bytes: u64,
}

/// FileKV 统计信息（使用原子计数器，无锁）
#[derive(Debug, Default)]
pub struct FileKVStats {
    pub memtable_size: AtomicUsize,
    pub memtable_entries: AtomicUsize,
    pub segment_count: AtomicUsize,
    pub total_size_bytes: AtomicU64,
    pub total_entries: AtomicU64,
    pub write_count: AtomicU64,
    pub read_count: AtomicU64,
    pub flush_count: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub bloom_filtered: AtomicU64,
    pub compaction_runs: AtomicU64,
    pub compaction_segments_merged: AtomicU64,
    pub compaction_tombstones_removed: AtomicU64,
    // P2-014: Compression statistics
    pub compression_dict_trained: AtomicBool,
    pub compression_dict_size: AtomicUsize,
    pub compressed_writes: AtomicU64,
    pub uncompressed_bytes: AtomicU64,
    pub compressed_bytes: AtomicU64,
}

impl Clone for FileKVStats {
    fn clone(&self) -> Self {
        Self {
            memtable_size: AtomicUsize::new(self.memtable_size.load(Ordering::Relaxed)),
            memtable_entries: AtomicUsize::new(self.memtable_entries.load(Ordering::Relaxed)),
            segment_count: AtomicUsize::new(self.segment_count.load(Ordering::Relaxed)),
            total_size_bytes: AtomicU64::new(self.total_size_bytes.load(Ordering::Relaxed)),
            total_entries: AtomicU64::new(self.total_entries.load(Ordering::Relaxed)),
            write_count: AtomicU64::new(self.write_count.load(Ordering::Relaxed)),
            read_count: AtomicU64::new(self.read_count.load(Ordering::Relaxed)),
            flush_count: AtomicU64::new(self.flush_count.load(Ordering::Relaxed)),
            cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicU64::new(self.cache_misses.load(Ordering::Relaxed)),
            bloom_filtered: AtomicU64::new(self.bloom_filtered.load(Ordering::Relaxed)),
            compaction_runs: AtomicU64::new(self.compaction_runs.load(Ordering::Relaxed)),
            compaction_segments_merged: AtomicU64::new(self.compaction_segments_merged.load(Ordering::Relaxed)),
            compaction_tombstones_removed: AtomicU64::new(self.compaction_tombstones_removed.load(Ordering::Relaxed)),
            compression_dict_trained: AtomicBool::new(self.compression_dict_trained.load(Ordering::Relaxed)),
            compression_dict_size: AtomicUsize::new(self.compression_dict_size.load(Ordering::Relaxed)),
            compressed_writes: AtomicU64::new(self.compressed_writes.load(Ordering::Relaxed)),
            uncompressed_bytes: AtomicU64::new(self.uncompressed_bytes.load(Ordering::Relaxed)),
            compressed_bytes: AtomicU64::new(self.compressed_bytes.load(Ordering::Relaxed)),
        }
    }
}

/// Bloom Filter 文件魔法数 (exported for bloom module)
pub const BLOOM_MAGIC_PUB: u32 = BLOOM_MAGIC;
/// Bloom Filter 文件版本 (exported for bloom module)
pub const BLOOM_VERSION_PUB: u32 = BLOOM_VERSION;
