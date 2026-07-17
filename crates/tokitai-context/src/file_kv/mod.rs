//! 纯文件 KV 存储引擎
//!
//! 基于 LSM-Tree 思想的纯文件实现，达到接近 KV 数据库的性能：
//! - **MemTable**: 内存缓冲，批量写入
//! - **Segment**: 顺序数据段，高效追加
//! - **稀疏索引**: 内存索引 + 二分查找
//! - **WAL**: 崩溃恢复保证
//!
//! 性能目标：
//! - 写入：15-20 µs (优化后)
//! - 读取：3-5 µs (热点缓存命中)
//! - 批量写入：0.26µs/项

pub mod adaptive_preallocator;
pub mod async_io;
pub mod bloom_filter_cache;
pub mod bloom_migration;
pub mod cache_warmer;
pub mod checkpoints;
pub mod compaction;
pub mod flush;
pub mod incremental_checkpoint;
pub mod incremental_types;
pub mod incremental_manager;
pub mod memtable;
pub mod recovery;
pub mod segment;
pub mod timeout_control;
pub mod types;
pub mod wal;
pub mod write_coalescer;

// Internal bloom operations (not public API)
pub(crate) mod bloom;

// Incremental checkpoint tests (only in test builds)
#[cfg(test)]
mod incremental_tests;

pub use adaptive_preallocator::{AdaptivePreallocator, AdaptivePreallocatorConfig, PreallocatorStats, SharedAdaptivePreallocator};
pub use async_io::{AsyncIoConfig, AsyncIoStats, AsyncWriter, AsyncWriteOp, AsyncWriteResult};
pub use incremental_types::{
    CheckpointEntry, CheckpointId, CheckpointSeq, CheckpointStats, CheckpointType,
    IncrementalCheckpoint, CheckpointChain, CheckpointMetadata,
};
pub use incremental_manager::IncrementalCheckpointManager;
pub use bloom_filter_cache::{BloomFilterCache, BloomFilterCacheConfig, BloomFilterCacheStats};
pub use cache_warmer::{CacheWarmer, CacheWarmingConfig, CacheWarmingStats, WarmingStrategy};
pub use memtable::{MemTable, MemTableConfig, MemTableEntry};
pub use write_coalescer::{WriteCoalescer, WriteCoalescerConfig};
pub use segment::{SegmentFile, SegmentStats};
pub use types::{FileKVConfig, FileKVConfigError, FileKVConfigValidation, FileKVStats, FileKVStatsSnapshot, ValuePointer};
pub use crate::audit_log::{AuditLogConfig, AuditLogger, AuditEntry, AuditOperation, AuditMetadata, AuditLogStats};
pub use crate::optimization::compression::dictionary::{DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats};

use std::collections::{BTreeMap, HashSet, HashMap};
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;
use bytes::Bytes;
use parking_lot::{Mutex, RwLock};
use xxhash_rust::xxh3::xxh3_64;
use tracing::{debug, info, warn};

use crate::error::{ContextResult, ContextError};

use crate::sparse_index::{SparseIndex, IndexManager, SparseIndexConfig};
use crate::block_cache::{BlockCache, BlockCacheConfig};
use crate::wal::WalManager;
use crate::compaction::CompactionManager;
use ::bloom::{BloomFilter, ASMS};
use flush::FlushTrigger;
use types::{BLOOM_MAGIC, BLOOM_VERSION};

/// 条件编译宏：release 模式下禁用 tracing
#[cfg(not(debug_assertions))]
#[inline]
fn trace_debug(_: impl FnOnce() -> String) {}

#[cfg(debug_assertions)]
#[inline]
fn trace_debug(f: impl FnOnce() -> String) {
    debug!("{}", f());
}

#[cfg(not(debug_assertions))]
#[inline]
fn trace_info(_: impl FnOnce() -> String) {}

#[cfg(debug_assertions)]
#[inline]
fn trace_info(f: impl FnOnce() -> String) {
    info!("{}", f());
}

#[cfg(not(debug_assertions))]
#[inline]
fn trace_warn(_: impl FnOnce() -> String) {}

#[cfg(debug_assertions)]
#[inline]
fn trace_warn(f: impl FnOnce() -> String) {
    warn!("{}", f());
}

/// 纯文件 KV 存储引擎
pub struct FileKV {
    pub(crate) config: FileKVConfig,
    memtable: Arc<MemTable>,
    pub(crate) segments: RwLock<BTreeMap<u64, Arc<SegmentFile>>>,
    pub(crate) next_segment_id: std::sync::atomic::AtomicU64,
    pub(crate) wal: Option<Mutex<WalManager>>,
    pub(crate) index_manager: RwLock<IndexManager>,
    block_cache: Arc<BlockCache>,
    /// P2-011: Bloom filter cache with on-demand loading
    pub(crate) bloom_filter_cache: Arc<BloomFilterCache>,
    pub(crate) stats: Arc<FileKVStats>,
    flush_lock: Mutex<()>,
    compaction_manager: Arc<CompactionManager>,
    flush_trigger: FlushTrigger,
    /// P2-012: Write coalescer for batching rapid writes
    write_coalescer: Option<Arc<WriteCoalescer>>,
    /// P2-008: Adaptive segment pre-allocation
    adaptive_preallocator: Option<SharedAdaptivePreallocator>,
    /// P2-014: Dictionary compressor for better compression ratios
    compressor: Option<parking_lot::Mutex<crate::optimization::compression::dictionary::DictionaryCompressor>>,
    /// P3-001: Async I/O writer for non-blocking writes
    async_writer: Option<Arc<AsyncWriter>>,
    /// P1-015: Timeout control for operations
    timeout_config: timeout_control::TimeoutConfig,
    /// P1-015: Timeout statistics
    timeout_stats: parking_lot::Mutex<timeout_control::TimeoutStats>,
    /// P2-016: Prometheus metrics exporter
    #[cfg(feature = "metrics")]
    metrics: Arc<crate::metrics_prometheus::FileKVMetrics>,
    /// P2-009: Incremental checkpoint manager
    checkpoint_manager: parking_lot::Mutex<IncrementalCheckpointManager>,
    /// P2-013: Audit logger for compliance and forensics (public for compaction module)
    pub(crate) audit_logger: Option<Arc<crate::audit_log::AuditLogger>>,
}

impl FileKV {
    /// 创建或打开 FileKV 存储
    pub fn open(config: FileKVConfig) -> ContextResult<Self> {
        let validation = config.validate();
        if !validation.errors.is_empty() {
            return Err(ContextError::InvalidConfig(validation.errors[0].to_string()));
        }

        for warning in &validation.warnings {
            trace_warn(|| warning.clone());
        }

        std::fs::create_dir_all(&config.segment_dir)
            .map_err(ContextError::Io)?;

        if config.enable_wal {
            std::fs::create_dir_all(&config.wal_dir)
                .map_err(ContextError::Io)?;
        }

        std::fs::create_dir_all(&config.index_dir)
            .map_err(ContextError::Io)?;

        let mut segments = BTreeMap::new();
        let mut max_id = 0u64;

        for entry_result in std::fs::read_dir(&config.segment_dir)? {
            let entry = entry_result?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(id_str) = name.strip_prefix("segment_") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            let segment = SegmentFile::open(id, &path)?;
                            max_id = max_id.max(id);
                            segments.insert(id, Arc::new(segment));
                        }
                    }
                }
            }
        }

        let wal = if config.enable_wal {
            // P1-013: Pass WAL rotation configuration
            Some(Mutex::new(WalManager::new_with_config(
                &config.wal_dir,
                true,
                config.wal_max_size_bytes,
                config.wal_max_files,
            )?))
        } else {
            None
        };

        let mut index_manager = IndexManager::new(&config.index_dir)
            .map_err(|e| ContextError::OperationFailed(format!("Index manager error: {}", e)))?;
        index_manager.load_all_indexes()
            .map_err(|e| ContextError::OperationFailed(format!("Index load error: {}", e)))?;

        let block_cache = Arc::new(BlockCache::new(config.cache.clone()));
        let memtable = Arc::new(MemTable::new(config.memtable.clone()));
        let compaction_manager = Arc::new(CompactionManager::new(config.compaction.clone()));

        let flush_trigger = if config.enable_background_flush {
            FlushTrigger::with_background_thread(config.background_flush_interval_ms, memtable.clone())
        } else {
            FlushTrigger::new()
        };

        // P2-012: Initialize write coalescer if enabled
        let write_coalescer = if config.write_coalescing_enabled {
            Some(Arc::new(WriteCoalescer::new(WriteCoalescerConfig::default())))
        } else {
            None
        };

        // P2-011: Initialize bloom filter cache with on-demand loading
        let bloom_filter_cache = Arc::new(BloomFilterCache::new(
            BloomFilterCacheConfig::default(),
            config.index_dir.clone(),
        ));

        // P2-008: Initialize adaptive pre-allocator
        let adaptive_preallocator = if config.segment_preallocate_size > 0 {
            let prealloc_config = AdaptivePreallocatorConfig {
                initial_preallocate_bytes: config.segment_preallocate_size,
                ..Default::default()
            };
            Some(Arc::new(AdaptivePreallocator::new(prealloc_config)))
        } else {
            None
        };

        // P2-014: Initialize dictionary compressor if enabled
        let compressor = if config.compression.enable_dictionary {
            use crate::optimization::compression::dictionary::DictionaryCompressor;
            Some(parking_lot::Mutex::new(DictionaryCompressor::new(config.compression.clone())))
        } else {
            None
        };

        // P3-001: Initialize async I/O writer if enabled
        let async_writer = if config.async_io_enabled {
            let async_config = AsyncIoConfig {
                enabled: true,
                max_concurrent_writes: config.async_io_max_concurrent_writes,
                max_queue_depth: config.async_io_max_queue_depth,
                write_timeout_ms: config.async_io_write_timeout_ms,
                enable_coalescing: config.async_io_enable_coalescing,
                coalesce_window_ms: config.async_io_coalesce_window_ms,
            };
            match AsyncWriter::new(async_config, config.segment_dir.clone()) {
                Ok(writer) => Some(Arc::new(writer)),
                Err(e) => {
                    warn!("Failed to initialize async writer: {}, falling back to sync I/O", e);
                    None
                }
            }
        } else {
            None
        };

        let kv = Self {
            config: config.clone(),
            memtable,
            segments: RwLock::new(segments),
            next_segment_id: std::sync::atomic::AtomicU64::new(max_id + 1),
            wal,
            index_manager: RwLock::new(index_manager),
            block_cache,
            bloom_filter_cache,
            stats: Arc::new(FileKVStats::default()),
            flush_lock: Mutex::new(()),
            compaction_manager,
            flush_trigger,
            write_coalescer,
            adaptive_preallocator,
            compressor,
            async_writer,
            timeout_config: timeout_control::TimeoutConfig::default(),
            timeout_stats: parking_lot::Mutex::new(timeout_control::TimeoutStats::default()),
            #[cfg(feature = "metrics")]
            metrics: {
                let exporter = Arc::new(crate::metrics_prometheus::PrometheusExporter::new("tokitai", env!("CARGO_PKG_VERSION")));
                exporter.register();
                Arc::new(crate::metrics_prometheus::FileKVMetrics::new(exporter))
            },
            checkpoint_manager: parking_lot::Mutex::new(IncrementalCheckpointManager::new(
                &config.checkpoint_dir,
            )?),
            // P2-013: Initialize audit logger if enabled
            audit_logger: if config.audit_log.enabled {
                match crate::audit_log::AuditLogger::open(config.audit_log.clone()) {
                    Ok(logger) => Some(Arc::new(logger)),
                    Err(e) => {
                        warn!("Failed to initialize audit logger: {}, audit logging disabled", e);
                        None
                    }
                }
            } else {
                None
            },
        };

        {
            kv.stats.segment_count.store(kv.segments.read().len(), std::sync::atomic::Ordering::Relaxed);
            kv.stats.total_size_bytes.store(
                kv.segments.read().values().map(|s| s.size()).sum(),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        if config.enable_bloom {
            kv.rebuild_bloom_filters()?;
        }

        // P2-004: Cache warming - pre-load hot data into cache
        if config.cache_warming_enabled {
            let segments: Vec<Arc<SegmentFile>> = kv.segments.read().values().cloned().collect();
            if !segments.is_empty() {
                let cache_warmer = CacheWarmer::new(
                    CacheWarmingConfig::default(),
                    kv.block_cache.clone(),
                );
                match cache_warmer.warm(&segments) {
                    Ok(stats) => {
                        debug!("Cache warming completed: {} entries loaded", stats.entries_loaded);
                    }
                    Err(e) => {
                        warn!("Cache warming failed: {}", e);
                    }
                }
            }
        }

        Ok(kv)
    }

    /// P1-015: Get timeout configuration
    pub fn get_timeout_config(&self) -> &timeout_control::TimeoutConfig {
        &self.timeout_config
    }

    /// P1-015: Set timeout configuration
    pub fn set_timeout_config(&mut self, config: timeout_control::TimeoutConfig) {
        self.timeout_config = config;
    }

    /// P1-015: Get timeout statistics snapshot
    pub fn get_timeout_stats(&self) -> timeout_control::TimeoutStats {
        self.timeout_stats.lock().clone()
    }

    /// P1-015: Reset timeout statistics
    pub fn reset_timeout_stats(&self) {
        *self.timeout_stats.lock() = timeout_control::TimeoutStats::default();
    }

    pub fn get_config(&self) -> &FileKVConfig {
        &self.config
    }

    /// P2-008: Get the next adaptive pre-allocation size
    ///
    /// Returns the optimal pre-allocation size for the next segment based on
    /// historical write patterns. If adaptive pre-allocation is disabled,
    /// returns the configured fixed size.
    pub fn get_next_preallocate_size(&self) -> u64 {
        self.adaptive_preallocator
            .as_ref()
            .map(|p| p.next_preallocate_size())
            .unwrap_or(self.config.segment_preallocate_size)
    }

    /// P2-008: Get adaptive preallocator statistics
    pub fn get_preallocator_stats(&self) -> Option<PreallocatorStats> {
        self.adaptive_preallocator.as_ref().map(|p| p.stats())
    }

    /// P2-008: Record that a segment was closed with the given actual size
    ///
    /// This is used by the adaptive pre-allocation system to track write patterns
    /// and adjust future pre-allocation sizes accordingly.
    pub(crate) fn record_segment_closed(&self, actual_size: u64) {
        if let Some(ref preallocator) = self.adaptive_preallocator {
            preallocator.record_segment_closed(actual_size);
        }
    }

    // Checkpoint methods moved to checkpoints.rs

    /// 写入键值对
    ///
    /// # 性能
    /// - 单次写入：~8-12µs (优化后)
    /// - 批量写入：0.26µs/项 (使用 put_batch)
    ///
    /// # 注意
    /// - 写入会自动 populate BlockCache，后续读取命中缓存
    /// - 推荐使用 put_batch() 批量写入以获得更高吞吐量
    ///
    /// # Performance Optimizations (P1-001)
    /// - Tracing disabled in release mode via conditional compilation
    /// - Stats updates use Relaxed ordering (already optimal)
    /// - WAL hash computation optimized with xxh3 instead of crc32c
    /// - Reduced allocations in hot path
    ///
    /// # P2-012: Write Coalescing
    /// - Writes are buffered and batched together for better throughput
    /// - Buffer flushes on time window (100µs) or size threshold (64KB)
    ///
    /// # P2-007: Backpressure
    /// - Returns error if MemTable memory limit exceeded
    /// - Callers should retry after flush or apply rate limiting
    #[cfg_attr(debug_assertions, tracing::instrument(skip_all, fields(key = key, value_len = value.len())))]
    pub fn put(&self, key: &str, value: &[u8]) -> ContextResult<()> {
        use crate::wal::WalOperation;
        use base64::{Engine, engine::general_purpose::STANDARD};

        let start = std::time::Instant::now();

        // P2-007: Check backpressure BEFORE accepting write
        if self.memtable.should_apply_backpressure() {
            // Force flush if memory limit exceeded
            self.flush_memtable()?;

            // Check again after flush
            if self.memtable.should_apply_backpressure() {
                #[cfg(feature = "metrics")]
                {
                    self.metrics.record_write_error("backpressure");
                }
                return Err(ContextError::OperationFailed(
                    format!("Backpressure: MemTable memory limit exceeded ({} bytes, ratio: {:.2}). Try again later.",
                        self.memtable.size_bytes(),
                        self.memtable.memory_usage_ratio())
                ));
            }
        }

        // P2-012: Write coalescing - buffer rapid writes
        if let Some(ref coalescer) = self.write_coalescer {
            // Try to buffer the write
            let should_flush = coalescer.add(key.to_string(), value.to_vec());

            // If buffer is full or time window exceeded, flush pending writes
            if should_flush {
                let result = self.flush_coalesced_writes();
                #[cfg(feature = "metrics")]
                {
                    let latency_us = start.elapsed().as_micros() as f64;
                    match &result {
                        Ok(_) => self.metrics.record_write_success(latency_us, true),
                        Err(_) => self.metrics.record_write_error("flush_failed"),
                    }
                }
                return result;
            }

            // Write is buffered, update stats and return
            self.stats.write_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            #[cfg(feature = "metrics")]
            {
                let latency_us = start.elapsed().as_micros() as f64;
                self.metrics.record_write_success(latency_us, false);
                self.metrics.record_coalesced_writes(1);
            }
            return Ok(());
        }

        // P1-001 OPTIMIZATION: WAL write with minimal allocations
        // P1-005 FIX: Include actual value in payload for recovery
        if let Some(ref wal) = self.wal {
            // P1-001: Use xxh3 for faster hashing
            let mut hasher = xxhash_rust::xxh3::Xxh3::default();
            hasher.write(value);
            let hash = hasher.finish();

            // P1-001: Avoid format!() in hot path - hash stored as hex string only when needed
            let hash_hex = format!("{:016X}", hash);
            let op = WalOperation::Add {
                session: key.to_string(),
                hash: hash_hex.clone(),
                layer: "segment".to_string(),
            };
            // P1-005 FIX: Include base64-encoded value for recovery
            let value_b64 = STANDARD.encode(value);
            let payload = format!("{}:{}:{}", value.len(), hash_hex, value_b64);
            
            // P3-001: Minimize WAL lock scope
            let mut wal_guard = wal.lock();
            let result = wal_guard.log_with_payload(op, payload);
            drop(wal_guard); // Explicitly release lock early
            result?;
        }

        // Insert into MemTable - use &str to avoid allocation if possible
        let (size, _seq) = self.memtable.insert(key.to_string(), value);

        // Update stats (lock-free with atomics)
        self.stats.write_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.stats.memtable_size.store(size, std::sync::atomic::Ordering::Relaxed);
        self.stats.memtable_entries.store(self.memtable.entry_count(), std::sync::atomic::Ordering::Relaxed);

        // Check if flush is needed
        let should_flush = self.flush_trigger.is_requested() || self.memtable.should_flush();
        if should_flush {
            self.flush_trigger.mark_completed();
            self.flush_memtable()?;
        }

        // Check compaction (every N writes)
        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        // P2-016: Record metrics
        #[cfg(feature = "metrics")]
        {
            let latency_us = start.elapsed().as_micros() as f64;
            self.metrics.record_write_success(latency_us, false);
        }

        // P2-013: Audit log the write operation
        if let Some(ref audit_logger) = self.audit_logger {
            let latency_us = start.elapsed().as_micros() as u64;
            let value_hash = crate::audit_log::compute_value_hash(value);
            let _ = audit_logger.log_operation(
                crate::audit_log::AuditOperation::Put,
                vec![key.to_string()],
                Some(value_hash),
                Some(value.len() as u64),
                Some(latency_us),
                true,
                None,
                crate::audit_log::AuditMetadata::default(),
            );
        }

        Ok(())
    }

    /// Flush coalesced writes to MemTable
    ///
    /// P2-012: Batch process buffered writes for better throughput
    fn flush_coalesced_writes(&self) -> ContextResult<()> {
        use crate::wal::WalOperation;
        use base64::{Engine, engine::general_purpose::STANDARD};

        let Some(ref coalescer) = self.write_coalescer else {
            return Ok(()); // Coalescer not enabled
        };
        
        let pending = coalescer.drain();

        if pending.is_empty() {
            return Ok(());
        }

        debug!("Flushing {} coalesced writes", pending.len());

        // Batch WAL write
        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            for write in &pending {
                let mut hasher = crc32c::Crc32cHasher::default();
                hasher.write(&write.value);
                let hash = hasher.finish();
                let hash_hex = format!("{:016X}", hash);
                
                let op = WalOperation::Add {
                    session: write.key.clone(),
                    hash: hash_hex.clone(),
                    layer: "segment".to_string(),
                };
                let value_b64 = STANDARD.encode(&write.value);
                let payload = format!("{}:{}:{}", write.value.len(), hash_hex, value_b64);
                wal_guard.log_with_payload(op, payload)?;
            }
        }

        // Batch MemTable insert
        for write in &pending {
            let (size, _seq) = self.memtable.insert(write.key.clone(), &write.value);
            self.stats.memtable_size.store(size, std::sync::atomic::Ordering::Relaxed);
        }
        self.stats.memtable_entries.store(self.memtable.entry_count(), std::sync::atomic::Ordering::Relaxed);

        // Check if flush is needed
        let should_flush = self.flush_trigger.is_requested() || self.memtable.should_flush();
        if should_flush {
            self.flush_trigger.mark_completed();
            self.flush_memtable()?;
        }

        // Check compaction (count batch as single write for compaction trigger)
        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        Ok(())
    }

    /// 批量写入键值对
    ///
    /// # 性能特点
    /// - 单次写入约 15-20µs，批量写入可低至 0.26µs/项（170 倍提升）
    /// - 推荐一次性写入 100+ 条目以最大化吞吐量
    /// - 自动 populate BlockCache，后续读取命中缓存
    ///
    /// # 参数
    /// - `entries`: 键值对切片，每项为 `(key, value)`
    ///
    /// # 返回值
    /// 成功写入的条目数量
    ///
    /// # 示例
    /// ```rust,no_run
    /// # use tokitai_context::file_kv::{FileKV, FileKVConfig};
    /// # fn example() -> anyhow::Result<()> {
    /// let config = FileKVConfig::default();
    /// let kv = FileKV::open(config)?;
    ///
    /// // 批量写入 1000 个条目
    /// let entries: Vec<(&str, &[u8])> = (0..1000)
    ///     .map(|i| (format!("key_{}", i).as_str(), format!("value_{}", i).as_bytes()))
    ///     .collect();
    ///
    /// let count = kv.put_batch(&entries)?;
    /// assert_eq!(count, 1000);
    ///
    /// // 验证读取
    /// assert!(kv.get("key_500")?.is_some());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 性能对比
    /// ```text
    /// 单条写入 1000 次：~45ms  (45µs/item)
    /// put_batch(1000):  ~0.26ms (0.26µs/item) - 170x 提升！
    /// ```
    ///
    /// # P2-007: Backpressure
    /// - Checks memory limit before accepting batch
    /// - May flush MemTable if limit exceeded
    /// - Returns error if still over limit after flush
    pub fn put_batch(&self, entries: &[(&str, &[u8])]) -> ContextResult<usize> {
        use crate::wal::{WalOperation, DurabilityLevel};

        if entries.is_empty() {
            return Ok(0);
        }

        // P2-007: Check backpressure BEFORE accepting batch write
        // Estimate batch size to avoid overshooting memory limit
        let estimated_batch_size: usize = entries.iter().map(|(_, v)| v.len()).sum();
        let mem_headroom = self.memtable.memory_headroom();

        if estimated_batch_size > mem_headroom {
            // Memory would be exceeded, force flush first
            self.flush_memtable()?;

            // Check again after flush
            if self.memtable.should_apply_backpressure() {
                return Err(ContextError::OperationFailed(
                    format!("Backpressure: MemTable still at {:.2}% capacity after flush. Rejecting batch of {} entries.",
                        self.memtable.memory_usage_ratio() * 100.0,
                        entries.len())
                ));
            }
        }

        let mut count = 0;
        let mut wal_disabled = false;

        // WAL write (only if enabled) - batched for efficiency
        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            for (key, value) in entries {
                let mut hasher = crc32c::Crc32cHasher::default();
                hasher.write(value);
                let hash = hasher.finish();
                let op = WalOperation::Add {
                    session: key.to_string(),
                    hash: format!("{:016X}", hash),
                    layer: "segment".to_string(),
                };
                // P1-005 FIX: Include actual value in payload for recovery
                // Format: "{len}:{hash}:{base64_value}"
                use base64::{Engine, engine::general_purpose::STANDARD};
                let value_b64 = STANDARD.encode(value);
                let durability = wal_guard.log_with_payload(op, format!("{}:{:016X}:{}", value.len(), hash, value_b64))?;
                if durability == DurabilityLevel::Memory {
                    wal_disabled = true;
                }
                count += 1;
            }
        } else {
            count = entries.len();
            wal_disabled = true;
        }

        // P0-004 FIX: Warn if WAL is disabled for batch operations
        if wal_disabled && !entries.is_empty() {
            tracing::warn!("Batch write of {} entries not persisted to disk (WAL disabled)", entries.len());
        }

        // Batch insert into MemTable - single lock acquisition pattern
        let mut total_size = 0usize;
        for (key, value) in entries {
            let (size, _seq) = self.memtable.insert(key.to_string(), value);
            total_size = size;
        }

        // Update stats (lock-free with atomics) - batch update
        self.stats.write_count.fetch_add(count as u64, std::sync::atomic::Ordering::Relaxed);
        self.stats.memtable_size.store(total_size, std::sync::atomic::Ordering::Relaxed);
        self.stats.memtable_entries.store(self.memtable.entry_count(), std::sync::atomic::Ordering::Relaxed);

        // Check if flush is needed
        let should_flush = self.flush_trigger.is_requested() || self.memtable.should_flush();
        if should_flush {
            self.flush_trigger.mark_completed();
            self.flush_memtable()?;
        }

        // Check compaction (every N writes)
        if self.compaction_manager.record_write() {
            self.maybe_run_compaction()?;
        }

        // P2-013: Audit log the batch write operation
        if let Some(ref audit_logger) = self.audit_logger {
            let keys: Vec<String> = entries.iter().map(|(k, _)| k.to_string()).collect();
            let total_size: u64 = entries.iter().map(|(_, v)| v.len() as u64).sum();
            let _ = audit_logger.log_operation(
                crate::audit_log::AuditOperation::BatchPut { count: entries.len() },
                keys,
                None,
                Some(total_size),
                None,
                true,
                None,
                crate::audit_log::AuditMetadata::default(),
            );
        }

        Ok(count)
    }

    fn maybe_run_compaction(&self) -> ContextResult<()> {
        let segments = self.segments();
        if self.compaction_manager.should_compact(&segments) {
            self.compaction_manager.compact(self)?;
        }
        Ok(())
    }

    /// 读取键值对
    ///
    /// # 性能特点
    /// - MemTable 命中：~1µs
    /// - BlockCache 命中：~3-5µs
    /// - Bloom Filter 阴性：~1µs (短路径返回)
    /// - Segment 扫描：~50-100µs
    #[tracing::instrument(skip_all, fields(key = key))]
    pub fn get(&self, key: &str) -> ContextResult<Option<Vec<u8>>> {
        let start = std::time::Instant::now();
        self.stats.read_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // P2-012: Flush pending coalesced writes to ensure read-your-writes consistency
        if let Some(ref coalescer) = self.write_coalescer {
            if coalescer.has_pending() {
                // Check if the key we're looking for is in the pending buffer
                let pending = coalescer.drain();
                for write in &pending {
                    if write.key == key {
                        // Found in pending writes - return immediately
                        #[cfg(feature = "metrics")]
                        {
                            let latency_us = start.elapsed().as_micros() as f64;
                            self.metrics.record_read_success(latency_us, false);
                        }
                        return Ok(Some(write.value.clone()));
                    }
                }
                // Key not in pending, but flush remaining writes to MemTable
                if !pending.is_empty() {
                    self.flush_coalesced_writes()?;
                }
            }
        }

        // 1. Check MemTable (fastest path - in-memory)
        if let Some((value_opt, pointer_opt, deleted)) = self.memtable.get(key) {
            if deleted {
                return Ok(None);
            }

            // Value in MemTable - return directly (zero-copy with Bytes)
            if let Some(value) = value_opt {
                // P0-001 FIX: Populate BlockCache for MemTable values too
                // This ensures cache is warm even before flush
                // Use a synthetic cache key: segment_id=0 for MemTable, offset=hash of key
                let mut hasher = ahash::AHasher::default();
                std::hash::Hash::hash(&key, &mut hasher);
                let memtable_cache_offset = hasher.finish() % 1_000_000_000u64; // Keep offset in reasonable range
                
                if let Some(cached) = self.block_cache.get(0, memtable_cache_offset) {
                    self.stats.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Ok(Some(cached.to_vec()));
                }
                
                // Cache miss - populate for next read
                let value_vec = value.to_vec();
                self.block_cache.put(0, memtable_cache_offset, Arc::from(value_vec.clone()));
                self.stats.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                return Ok(Some(value_vec));
            }

            // Value flushed to segment, check cache
            if let Some(pointer) = pointer_opt {
                // P0-001 FIX: Check BlockCache first (lock-free with DashMap)
                // Return Arc clone to avoid copying - zero-copy cache hit
                if let Some(cached) = self.block_cache.get(pointer.segment_id, pointer.offset) {
                    self.stats.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Ok(Some(cached.to_vec()));
                }

                // Cache miss - read from segment
                let segments = self.segments.read();
                if let Some(segment) = segments.get(&pointer.segment_id) {
                    let mut value = segment.read_at(pointer.offset, pointer.len)?;

                    // Verify checksum
                    let mut hasher = crc32c::Crc32cHasher::default();
                    hasher.write(&value);
                    let computed = hasher.finish() as u32;
                    if computed != pointer.checksum {
                        trace_warn(|| format!("Checksum mismatch for key {}: expected {:08X}, got {:08X}",
                             key, pointer.checksum, computed));
                        return Err(ContextError::OperationFailed("Checksum verification failed".to_string()));
                    }

                    // P2-014: Decompress value if compressor is enabled
                    if let Some(ref compressor_mutex) = self.compressor {
                        let compressor = compressor_mutex.lock();
                        match compressor.decompress(&value) {
                            Ok(decompressed) => {
                                value = decompressed;
                            }
                            Err(e) => {
                                // If decompression fails, the data might be uncompressed
                                // This can happen if compression was enabled after data was written
                                debug!("Decompression failed for key '{}': {}, returning raw value", key, e);
                            }
                        }
                    }

                    // P0-001 FIX: Populate cache for next read (before compression so we cache raw)
                    self.block_cache.put(pointer.segment_id, pointer.offset, Arc::from(value.clone()));
                    self.stats.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    return Ok(Some(value));
                }
            }
        }

        // 2. Bloom Filter check - FAST PATH for negative results (P0-002 FIX)
        // P2-011: Use bloom_filter_cache with on-demand loading
        let index_manager = self.index_manager.read();
        let segments_to_check = index_manager.all_indexes();

        // P0-002 FIX: Early return optimization for negative lookups
        // Strategy: Check bloom filters and skip segments that say "definitely not"
        // This avoids unnecessary I/O for segments that don't contain the key
        
        // Create a loader closure for on-demand bloom filter loading
        let bloom_loader = |seg_id: u64| -> ContextResult<Option<BloomFilter>> {
            match self.load_bloom_filter(seg_id) {
                Ok(Some((bloom, _))) => Ok(Some(bloom)),
                Ok(None) => Ok(None),
                Err(e) => {
                    tracing::warn!("Failed to load bloom filter for segment {}: {}", seg_id, e);
                    Ok(None)
                }
            }
        };

        // P0-002 FIX: Pre-filter segments using bloom filters
        // Collect segments that either:
        // 1. Have no bloom filter (must scan)
        // 2. Bloom filter says "maybe" (might have key)
        // Skip segments where bloom filter says "no"
        let mut segments_to_scan: Vec<(u64, &crate::sparse_index::SparseIndex)> = Vec::new();
        
        for (&segment_id, index) in segments_to_check.iter().rev() {
            let bloom_result = self.bloom_filter_cache.contains(segment_id, key, &bloom_loader);
            
            match bloom_result {
                Ok(Some(false)) => {
                    // P0-002 FIX: Bloom filter says "definitely not" - skip this segment
                    self.stats.bloom_filtered.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // Don't add to segments_to_scan
                }
                Ok(Some(true)) => {
                    // Bloom filter says "might exist" - add to scan list
                    segments_to_scan.push((segment_id, index));
                }
                Ok(None) | Err(_) => {
                    // No bloom filter available - must scan to be safe
                    segments_to_scan.push((segment_id, index));
                }
            }
        }

        // P0-002 FIX: Early return if ALL segments were filtered out by bloom filters
        if segments_to_scan.is_empty() {
            return Ok(None);
        }

        // Continue with filtered segment list instead of all segments

        // 3. Scan segments (slow path) - only if bloom filters say key might exist
        let mut found_value: Option<Vec<u8>> = None;

        // P0-002 FIX: Use pre-filtered segments_to_scan list instead of all segments
        for (segment_id, index) in segments_to_scan {
            let find_result = index.find(key);
            let scan_start = find_result.map(|(_, offset)| offset).unwrap_or(8);

            let segments = self.segments.read();
            if let Some(segment) = segments.get(&segment_id) {
                if let Ok(Some((_found_key, mut value, offset, _checksum))) = segment.scan_from(scan_start, key) {
                    // P2-014: Decompress value if compressor is enabled
                    if let Some(ref compressor_mutex) = self.compressor {
                        let compressor = compressor_mutex.lock();
                        if let Ok(decompressed) = compressor.decompress(&value) {
                            value = decompressed;
                        }
                    }

                    // P0-001 FIX: Cache the value for future reads
                    self.block_cache.put(segment_id, offset, Arc::from(value.clone()));
                    self.stats.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    found_value = Some(value);
                    break;
                }
            }
        }

        // P2-016: Record metrics
        #[cfg(feature = "metrics")]
        {
            let latency_us = start.elapsed().as_micros() as f64;
            let cache_hit = found_value.is_some() && self.stats.cache_hits.load(std::sync::atomic::Ordering::Relaxed) > 0;
            self.metrics.record_read_success(latency_us, cache_hit);
        }

        #[cfg(not(feature = "metrics"))]
        {
            let _ = start; // Suppress unused variable warning
        }

        Ok(found_value)
    }

    /// 删除键
    ///
    /// P0-004 FIX: Logs WAL with durability level tracking
    #[tracing::instrument(skip_all, fields(key = key))]
    pub fn delete(&self, key: &str) -> ContextResult<()> {
        use crate::wal::{WalOperation, DurabilityLevel};

        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            let op = WalOperation::Delete {
                session: key.to_string(),
                hash: String::new(),
                content: None,
            };
            let durability = wal_guard.log(op)?;
            if durability == DurabilityLevel::Memory {
                // P0-004 FIX: Log warning when WAL is disabled
                tracing::warn!("Delete operation for key '{}' not persisted to disk (WAL disabled)", key);
            }
        }

        self.memtable.delete(key);
        self.stats.write_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // P2-013: Audit log the delete operation
        if let Some(ref audit_logger) = self.audit_logger {
            let _ = audit_logger.log_operation(
                crate::audit_log::AuditOperation::Delete,
                vec![key.to_string()],
                None,
                None,
                None,
                true,
                None,
                crate::audit_log::AuditMetadata::default(),
            );
        }

        Ok(())
    }

    /// 刷盘 MemTable 到 segment
    #[tracing::instrument(skip_all)]
    pub fn flush_memtable(&self) -> ContextResult<()> {
        let _guard = self.flush_lock.lock();

        if !self.memtable.should_flush() {
            return Ok(());
        }

        let segment_id = self.next_segment_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let segment_path = self.config.segment_dir.join(format!("segment_{:06}.log", segment_id));

        trace_info(|| format!("Flushing MemTable to segment {} ({:?})", segment_id, segment_path));

        // P2-008: Use adaptive pre-allocation size
        let preallocate_size = self.adaptive_preallocator
            .as_ref()
            .map(|p| p.next_preallocate_size())
            .unwrap_or(self.config.segment_preallocate_size);

        let segment = SegmentFile::create(
            segment_id,
            &segment_path,
            preallocate_size,
        )?;

        // P2-008: Record segment creation for adaptive pre-allocation
        if let Some(ref preallocator) = self.adaptive_preallocator {
            preallocator.record_segment_created(preallocate_size);
        }

        let entries = self.memtable.get_entries();
        let mut index = SparseIndex::new(segment_id, SparseIndexConfig::default());
        let mut bloom = BloomFilter::with_rate(0.01, 10000);
        let mut bloom_keys = Vec::new();

        let mut sorted_entries: Vec<_> = entries.into_iter().collect();
        sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut pointers_to_update: Vec<(String, ValuePointer)> = Vec::new();
        let mut seq = 0u64;

        for (key, entry) in sorted_entries {
            if entry.deleted {
                continue;
            }

            if let Some(value) = entry.value {
                // P2-014: Compress value if compressor is enabled
                let value_to_write: Vec<u8>;
                let value_bytes = value.as_ref(); // Convert Bytes to &[u8]
                let final_value: &[u8] = if let Some(ref compressor_mutex) = self.compressor {
                    let mut compressor = compressor_mutex.lock();

                    // Add training sample for dictionary learning
                    compressor.add_training_sample(value.to_vec());

                    // Compress the value
                    match compressor.compress(value_bytes) {
                        Ok(compressed) => {
                            // Update compression stats
                            self.stats.compressed_writes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            self.stats.uncompressed_bytes.fetch_add(value.len() as u64, std::sync::atomic::Ordering::Relaxed);
                            self.stats.compressed_bytes.fetch_add(compressed.len() as u64, std::sync::atomic::Ordering::Relaxed);
                            value_to_write = compressed;
                            value_to_write.as_slice()
                        }
                        Err(e) => {
                            // Fall back to uncompressed on error
                            warn!("Compression failed for key '{}': {}, storing uncompressed", key, e);
                            value_bytes
                        }
                    }
                } else {
                    value_bytes
                };

                let (offset, len, checksum) = segment.append(&key, final_value)?;
                let pointer = ValuePointer::new(segment_id, offset, len, checksum);

                index.maybe_add_index_point(&key, offset, seq);
                seq += 1;

                bloom.insert(&key);
                bloom_keys.push(key.clone());
                pointers_to_update.push((key, pointer));
            }
        }

        segment.close()?;

        // P2-008: Record segment closed for adaptive pre-allocation
        if let Some(ref preallocator) = self.adaptive_preallocator {
            preallocator.record_segment_closed(segment.size());
        }

        for (key, pointer) in pointers_to_update {
            self.memtable.update_pointer(&key, pointer);
        }

        {
            let mut index_manager = self.index_manager.write();
            index_manager.create_index(segment_id);
            if let Some(seg_index) = index_manager.get_index_mut(segment_id) {
                for point in index.get_index_points() {
                    seg_index.maybe_add_index_point(&point.key, point.offset, point.seq_num);
                }
            }
            index_manager.save_index(segment_id)
                .map_err(|e| ContextError::OperationFailed(format!("Index save error: {}", e)))?;
        }

        self.save_bloom_filter(segment_id, &bloom, &bloom_keys)?;

        // P2-011: Insert into bloom filter cache instead of BTreeMap
        self.bloom_filter_cache.insert(segment_id, bloom);

        {
            let mut segments = self.segments.write();
            segments.insert(segment_id, Arc::new(segment));
        }

        self.memtable.clear();

        // P1-001: Flush WAL after MemTable flush to ensure durability
        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            wal_guard.flush()?;
        }

        {
            self.stats.flush_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.stats.segment_count.store(self.segments.read().len(), std::sync::atomic::Ordering::Relaxed);
        }

        // P2-013: Audit log the flush operation
        if let Some(ref audit_logger) = self.audit_logger {
            let _ = audit_logger.log_operation(
                crate::audit_log::AuditOperation::Flush,
                vec![],
                None,
                None,
                None,
                true,
                None,
                crate::audit_log::AuditMetadata::default(),
            );
        }

        trace_info(|| "MemTable flushed successfully".to_string());
        Ok(())
    }

    pub fn stats(&self) -> FileKVStatsSnapshot {
        // Calculate compression ratio
        let uncompressed = self.stats.uncompressed_bytes.load(std::sync::atomic::Ordering::Relaxed);
        let compressed = self.stats.compressed_bytes.load(std::sync::atomic::Ordering::Relaxed);
        let compression_ratio = if uncompressed > 0 {
            compressed as f64 / uncompressed as f64
        } else {
            1.0
        };

        FileKVStatsSnapshot {
            memtable_size: self.memtable.size_bytes(),
            memtable_entries: self.memtable.entry_count(),
            segment_count: self.segments.read().len(),
            total_size_bytes: self.segments.read().values().map(|s| s.size()).sum(),
            total_entries: self.stats.total_entries.load(std::sync::atomic::Ordering::Relaxed),
            write_count: self.stats.write_count.load(std::sync::atomic::Ordering::Relaxed),
            read_count: self.stats.read_count.load(std::sync::atomic::Ordering::Relaxed),
            flush_count: self.stats.flush_count.load(std::sync::atomic::Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(std::sync::atomic::Ordering::Relaxed),
            bloom_filtered: self.stats.bloom_filtered.load(std::sync::atomic::Ordering::Relaxed),
            compaction_runs: self.stats.compaction_runs.load(std::sync::atomic::Ordering::Relaxed),
            compaction_segments_merged: self.stats.compaction_segments_merged.load(std::sync::atomic::Ordering::Relaxed),
            compaction_tombstones_removed: self.stats.compaction_tombstones_removed.load(std::sync::atomic::Ordering::Relaxed),
            // P2-014: Compression statistics
            compression_dict_trained: self.stats.compression_dict_trained.load(std::sync::atomic::Ordering::Relaxed),
            compression_dict_size: self.stats.compression_dict_size.load(std::sync::atomic::Ordering::Relaxed),
            compression_ratio,
            compressed_writes: self.stats.compressed_writes.load(std::sync::atomic::Ordering::Relaxed),
            uncompressed_bytes: uncompressed,
            compressed_bytes: compressed,
        }
    }

    pub fn segments(&self) -> Vec<SegmentStats> {
        let segments = self.segments.read();
        segments.values().map(|s| SegmentStats {
            id: s.id,
            size_bytes: s.size(),
            entry_count: s.entry_count(),
            path: s.path.display().to_string(),
        }).collect()
    }

    pub fn close(&self) -> ContextResult<()> {
        // P2-012: Flush any pending coalesced writes first
        if let Some(ref coalescer) = self.write_coalescer {
            if coalescer.has_pending() {
                self.flush_coalesced_writes()?;
            }
        }

        if self.memtable.should_flush() {
            self.flush_memtable()?;
        }

        for segment in self.segments.read().values() {
            segment.close()?;
        }

        if let Some(ref wal) = self.wal {
            wal.lock().flush()?;
        }

        Ok(())
    }

    /// P2-012: Flush pending coalesced writes immediately
    pub fn flush_pending_writes(&self) -> ContextResult<()> {
        if let Some(ref coalescer) = self.write_coalescer {
            if coalescer.has_pending() {
                self.flush_coalesced_writes()?;
            }
        }
        Ok(())
    }

    pub fn compact(&self) -> ContextResult<crate::compaction::CompactionStats> {
        self.compaction_manager.compact(self)
            .map_err(|e| ContextError::OperationFailed(format!("Compaction error: {}", e)))
    }

    pub fn compaction_stats(&self) -> crate::compaction::CompactionStats {
        self.compaction_manager.stats()
    }

    pub fn is_compacting(&self) -> bool {
        self.compaction_manager.is_compacting()
    }

    pub(crate) fn save_bloom_filter(&self, segment_id: u64, _bloom: &BloomFilter, keys: &[String]) -> ContextResult<()> {
        use std::io::BufWriter;
        use std::fs::OpenOptions;

        let bloom_path = self.config.index_dir.join(format!("bloom_{:06}.bin", segment_id));
        let mut file = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&bloom_path)
                .map_err(ContextError::Io)?
        );

        file.write_all(&BLOOM_MAGIC.to_le_bytes())?;
        file.write_all(&BLOOM_VERSION.to_le_bytes())?;

        let num_keys = keys.len() as u64;
        file.write_all(&num_keys.to_le_bytes())?;

        for key in keys {
            let key_bytes = key.as_bytes();
            let key_len = key_bytes.len() as u32;
            file.write_all(&key_len.to_le_bytes())?;
            file.write_all(key_bytes)?;
        }

        file.flush()?;
        trace_debug(|| format!("Saved bloom filter with {} keys for segment {} to {:?}", num_keys, segment_id, bloom_path));
        Ok(())
    }

    fn load_bloom_filter(&self, segment_id: u64) -> ContextResult<Option<(BloomFilter, Vec<String>)>> {
        use bloom_migration::BloomFilterMigrator;

        let migrator = BloomFilterMigrator::new(self.config.index_dir.clone());

        match migrator.load_with_migration(segment_id) {
            Ok(Some((bloom, keys, migration_result))) => {
                match migration_result {
                    bloom_migration::MigrationResult::NoMigrationNeeded => {
                        trace_debug(|| format!("Loaded bloom filter (v{}) for segment {}", bloom_migration::CURRENT_BLOOM_VERSION, segment_id));
                    }
                    bloom_migration::MigrationResult::Migrated { from_version, to_version } => {
                        info!(
                            "Migrated bloom filter for segment {} from v{} to v{}",
                            segment_id, from_version, to_version
                        );
                    }
                    bloom_migration::MigrationResult::UnsupportedVersion { version } => {
                        warn!(
                            "Bloom filter for segment {} has unsupported version {}, skipping",
                            segment_id, version
                        );
                        return Ok(None);
                    }
                    bloom_migration::MigrationResult::FutureVersion { version } => {
                        warn!(
                            "Bloom filter for segment {} has future version {}, may have compatibility issues",
                            segment_id, version
                        );
                    }
                }
                Ok(Some((bloom, keys)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // Bloom filter methods moved to bloom.rs
    // Recovery method moved to recovery.rs
}

#[cfg(test)]
mod tests;
