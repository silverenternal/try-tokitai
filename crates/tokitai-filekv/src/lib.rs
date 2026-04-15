//! Tokitai FileKV - High-performance file-based KV storage engine
//!
//! A pure file KV storage engine based on LSM-Tree architecture with near in-memory database performance:
//! - **MemTable**: In-memory buffer for batch writes
//! - **Segment**: Sequential data segments for efficient appends
//! - **Sparse Index**: Memory index with binary search
//! - **WAL**: Write-Ahead Log for crash recovery
//! - **Adaptive Bloom Cache**: Multi-layer cache with adaptive FPR (INNO-001)
//! - **Zone Map**: Range query optimization with block pruning (INNO-002)
//!
//! ## mimalloc Allocator
//!
//! Enable the `mimalloc` feature to use the mimalloc allocator for improved
//! memory allocation performance in high-concurrency scenarios:
//!
//! ```toml
//! [dependencies]
//! tokitai-filekv = { version = "0.1", features = ["mimalloc"] }
//! ```
//!
//! ## Performance (Fair Comparison with RocksDB, 2026-04-08)
//!
//! | Operation | FileKV | RocksDB | Speedup |
//! |-----------|--------|---------|---------|
//! | **Bloom Filter Negative** | **62.37 µs** | **247.38 µs** | **3.97x** |
//! | **Full KV Get (Hot)** | **61.92 µs** | **600.07 µs** | **9.69x** |
//! | Write (64B, WAL) | 1.71 ms/entry | 1.88 ms/entry | FileKV 9% faster |
//! | Write (100B, WAL) | 1.86 ms/entry | 1.83 ms/entry | RocksDB 2% faster |
//!
//! **Note**: Previous reports claimed "90-187x" advantage, but those were unfair comparisons
//! (FileKV hot cache vs RocksDB cold query). Fair comparison shows **3-10x** advantage.
//! See `doc/rocksdb_fair_comparison_2026_04_08.md` for detailed methodology.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use tokitai_filekv::{FileKV, FileKVConfig};
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = FileKVConfig::default();
//! let kv = FileKV::open(config)?;
//!
//! // Write
//! kv.put("key1", b"value1")?;
//!
//! // Read
//! if let Some(value) = kv.get("key1")? {
//!     println!("Value: {:?}", value);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! FileKV Engine
//! ├── MemTable (DashMap, lock-free)
//! ├── Segment Files (sequential append)
//! ├── Sparse Index (key → position)
//! ├── BlockCache (LRU, hot data)
//! ├── BloomFilter Cache (negative lookup)
//! │   ├── L1: Hot (FPR 0.1-0.5%)
//! │   ├── L2: Warm (FPR 0.5-1.0%, compressed)
//! │   └── L3: Cold (FPR 1.0-10.0%)
//! ├── Zone Map (range pruning)
//! └── WAL (crash recovery)
//! ```
//!
//! ## Feature Flags
//!
//! - `wal`: Enable Write-Ahead Log (default)
//! - `mimalloc`: Use mimalloc allocator for improved concurrency
//! - `benchmarks`: Include performance benchmarking suite
//! - `rocksdb-compare`: RocksDB fair comparison benchmarks
//! - `metrics`: Prometheus metrics exporter
//! - `async-io`: Async I/O support
//! - `full`: Enable all features

// TEMPORARILY disabled for GAP-S7 audit: #![allow(dead_code)]

// mimalloc allocator for improved memory allocation performance
#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// ─── Phase 1: I/O Abstraction Layer ───
pub mod io;

// ─── Phase 3: Unified Cache + Memory Budget ───
pub mod cache;

// ─── Phase 4: Engine Decomposition ───
pub mod engine;

// Core storage modules
pub mod core;

// Bloom filter ecosystem
pub mod bloom;

// Query optimization
pub mod query;

// Compaction system
pub mod compaction;

// Checkpoint system
pub mod checkpoint;

// Operations and observability
pub mod ops;

// Compression algorithms
pub mod compression;

// Test modules
#[cfg(test)]
mod tests;

// Feature flag tests (only in test builds)
#[cfg(test)]
#[path = "ops/feature_flag_tests.rs"]
mod feature_flag_tests;

// Public API exports

// Phase 1: I/O abstraction
pub use crate::io::{FileKVFileSystem, FileKVFile, MmapView, MmapFileSystem, FileMetadata, StdFs, MemFs, FaultInjector, FaultRule, FaultStrategy};

// Phase 3: Unified cache
pub use crate::cache::{CacheBudget, UnifiedCacheManager, UnifiedCacheConfig, CacheUsageReport};
pub use crate::cache::{BlockCache, BlockCacheConfig, CacheStats, BlockCacheAsPrefetchCache};
pub use crate::cache::{CacheWarmer, CacheWarmingConfig, CacheWarmingStats, WarmingStrategy};
pub use crate::cache::{SequentialPrefetcher, SequentialPrefetcherConfig, SequentialPrefetcherStats, PrefetchCache};

// Phase 4: Engine decomposition
pub use crate::engine::EngineState;
pub use crate::engine::{ReadEngine, WriteEngine, CompactionEngine, LifecycleManager};

pub use crate::bloom::adaptive::{AdaptiveBloomCache, AdaptiveBloomCacheConfig, AdaptiveBloomCacheStats, CacheLayer};
pub use crate::ops::preallocator::{AdaptivePreallocator, AdaptivePreallocatorConfig, PreallocatorStats, SharedAdaptivePreallocator};
#[cfg(feature = "async-io")]
pub use crate::ops::async_io::{AsyncIoConfig, AsyncIoStats, AsyncWriter, AsyncWriteOp, AsyncWriteResult};
pub use crate::bloom::filter_cache::{BloomFilterCache, BloomFilterCacheConfig, BloomFilterCacheStats};
pub use crate::compression::dictionary::{DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats};
pub use crate::compression::{CompressionStrategy, CompressionAlgorithmId, create_compressor};
pub use core::types::{BlockCompressionMode, BlockCompressionConfig};
pub use core::config::FileKVConfig;
pub use core::types::FileKVConfigError;
pub use core::types::FileKVConfigValidation;
pub use core::types::{FileKVStats, FileKVStatsSnapshot, ValuePointer, AggressiveConfig, WalSyncMode, Durability};
pub use crate::bloom::fpr_controller::{FPRController, FPRControllerStats, AdaptationPolicy, FPRLevel, FPRAdjustedBloom};
pub use crate::checkpoint::{IncrementalCheckpoint, IncrementalCheckpointManager};
pub use crate::checkpoint::{
    CheckpointEntry, CheckpointId, CheckpointSeq, CheckpointStats, CheckpointType,
    CheckpointChain, CheckpointMetadata,
};
pub use core::memtable::{MemTable, MemTableConfig, MemTableEntry};
// Query module exports (re-exported from query module for backward compatibility)
pub use crate::query::{RangeQueryPruner, RangeQueryPrunerConfig, RangeQueryPrunerStats, PrunedBlockIterator};
pub use crate::query::{RangeScanIterator, RangeScanConfig, RangeScanStats, RangeEntry, QuerySegmentProvider};
pub use core::segment::{SegmentFile, SegmentStats};
pub use crate::ops::timeout_control::{TimeoutConfig, TimeoutStats};
pub use core::types::BLOOM_MAGIC;
pub use core::types::BLOOM_VERSION;
pub use core::types::DEFAULT_BLOOM_FPR;
pub use core::write_coalescer::{WriteCoalescer, WriteCoalescerConfig};
pub use crate::query::{ZoneMapEntry, ZoneMapBuilder, ZoneMapIndex, ZoneMapResult, ZoneMapError, RangeQueryStats, SequentialDetector};

// Compaction manager
pub use crate::compaction::CompactionConfig;
pub use crate::compaction::{CompactionManifest, CompactionStatus, CompactionExecutor, RecoveryAction, recover_incomplete};
pub use crate::compaction::{CompactionTrigger, TriggerType, TriggerState, TriggerResult, default_compaction_trigger};

// Internal modules - also export key types for advanced usage
pub use core::sparse_index::{SparseIndex, IndexManager as SparseIndexManager};
pub use core::wal::{WalManager, WalEntry};
pub use core::flush::FlushTrigger;
pub use ops::audit_log::{AuditLogConfig, AuditLogger, AuditEntry, AuditOperation, AuditLogStats};
#[cfg(feature = "metrics")]
pub use ops::metrics::PrometheusExporter;
pub use ops::memory_tracker::{MemoryTracker, MemoryUsage};
#[cfg(feature = "metrics")]
pub use ops::metrics::FileKVMetrics;
pub use ops::feature_flag::{FeatureFlag, FeatureFlagController, FeatureState, FeatureStateChange, FeatureFlagStats, FeatureReport};

use crate::core::error::FileKVResult;
use std::collections::BTreeMap;
use std::sync::Arc;
use bytes::Bytes;
use parking_lot::Mutex;
use tracing::{debug, info};
#[cfg(debug_assertions)]
use tracing::warn;


use crate::core::sparse_index::IndexManager;
use crate::compaction::CompactionManager;

pub use crate::bloom::{BloomManager, BloomConfig, BloomSegmentProvider};
pub use crate::bloom::{save_bloom_filter_atomic, load_bloom_filter, bloom_filter_exists};

// Re-export external bloom crate types
pub use ::bloom::ASMS;
pub use ::bloom::BloomFilter;

// Conditional compilation macros for tracing
#[cfg(not(debug_assertions))]
#[inline]
#[allow(dead_code)]
fn trace_debug(_: impl FnOnce() -> String) {}

#[cfg(debug_assertions)]
#[inline]
#[allow(dead_code)]
fn trace_debug(f: impl FnOnce() -> String) {
    debug!("{}", f());
}

#[cfg(not(debug_assertions))]
#[inline]
#[allow(dead_code)]
fn trace_info(_: impl FnOnce() -> String) {}

#[cfg(debug_assertions)]
#[inline]
#[allow(dead_code)]
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

/// File-based KV storage engine
/// Phase 4: FileKV is now a thin facade over specialized engines
pub struct FileKV {
    pub(crate) config: FileKVConfig,
    // Phase 4.7: Shared EngineState across all engines
    engine_state: Arc<crate::engine::EngineState>,
    // Phase 4: Engines
    read_engine: Arc<crate::engine::ReadEngine>,
    write_engine: Arc<crate::engine::WriteEngine>,
    compaction_engine: Arc<crate::engine::CompactionEngine>,
    lifecycle_manager: Arc<crate::engine::LifecycleManager>,
    /// Prometheus metrics (S2-3: auto-recording in production paths)
    #[cfg(feature = "metrics")]
    metrics: Arc<crate::ops::metrics::FileKVMetrics>,
}

impl FileKV {
    /// Create or open FileKV storage
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.checkpoint_dir = temp_dir.path().join("checkpoints");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// let stats = kv.get_stats();
    /// assert_eq!(stats.segment_count, 0);
    /// assert_eq!(stats.write_count, 0);
    /// ```
    pub fn open(config: FileKVConfig) -> anyhow::Result<Self> {
        let validation = config.validate();
        if !validation.errors.is_empty() {
            return Err(anyhow::anyhow!("Invalid config: {}", validation.errors[0]));
        }

        for warning in &validation.warnings {
            trace_warn(|| warning.clone());
        }

        config.fs.create_dir_all(&config.segment_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create segment dir: {}", e))?;

        if config.enable_wal {
            config.fs.create_dir_all(&config.wal_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create WAL dir: {}", e))?;
        }

        config.fs.create_dir_all(&config.index_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create index dir: {}", e))?;

        // DATA-002 FIX: Clean up any leftover temp files from previous crashes
        for path in config.fs.read_dir(&config.segment_dir)? {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with(".segment_") && name.ends_with(".log.tmp") {
                    tracing::warn!("Cleaning up leftover temp file: {}", path.display());
                    let _ = config.fs.remove_file(&path);
                }
            }
        }

        let mut segments = BTreeMap::new();
        let mut max_id = 0u64;

        for path in config.fs.read_dir(&config.segment_dir)? {

            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(id_str) = name.strip_prefix("segment_") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            // 1.2 OPTIMIZATION: Default to L0 for existing segments (backward compatible)
                            let level = 0u8;
                            let segment = SegmentFile::open(
                                config.fs.clone(),
                                id,
                                level,
                                &path,
                                config.aggressive.persistent_mmap_enabled,
                                config.aggressive.readahead_multiplier, // CFG-001
                                config.aggressive.dense_index_enabled,  // CFG-003
                            )?;
                            max_id = max_id.max(id);
                            segments.insert(id, Arc::new(segment));
                        }
                    }
                }
            }
        }

        let mut index_manager = IndexManager::new(&config.index_dir)?;
        index_manager.load_all_indexes()?;

        // CFG-004: Use aggressive.cache_max_memory_bytes to configure BlockCache
        let mut cache_config = config.cache.clone();
        if config.aggressive.cache_max_memory_bytes > 0 {
            cache_config.max_memory_bytes = config.aggressive.cache_max_memory_bytes as u64;
            // Also adjust max_items based on cache size (rough estimate: 4KB per item average)
            cache_config.max_items = std::cmp::max(
                cache_config.max_items,
                config.aggressive.cache_max_memory_bytes / 4096,
            );
        }

        // GAP-M5: Instantiate UnifiedCacheManager for coordinated cache management
        // The UnifiedCacheManager creates and manages BlockCache and BloomFilterCache
        // with memory limits derived from the total budget.
        let unified_cache_config = cache::UnifiedCacheConfig {
            max_total_memory_bytes: config.aggressive.cache_max_memory_bytes as u64,
            block_cache_ratio: 0.60,
            bloom_cache_ratio: 0.25,
            block_cache_config: Some(cache_config.clone()),
            bloom_cache_config: None,
            bloom_index_dir: config.index_dir.clone(),
        };
        let unified_cache = Arc::new(cache::UnifiedCacheManager::new(unified_cache_config));

        // Get the managed caches from UnifiedCacheManager
        let block_cache = unified_cache.block_cache().clone();
        let memtable = Arc::new(MemTable::new(config.memtable.clone()));

        let flush_trigger = if config.enable_background_flush {
            FlushTrigger::with_background_thread(config.background_flush_interval_ms, memtable.clone())
        } else {
            FlushTrigger::new()
        };

        // Phase 6: WriteCoalescer always instantiated (no longer optional)
        let write_coalescer = Arc::new(WriteCoalescer::new(WriteCoalescerConfig::default()));

        // Create a standalone BloomFilterCache for backward compatibility with existing APIs
        // (The UnifiedCacheManager manages its own bloom cache internally)
        let bloom_filter_cache = Arc::new(BloomFilterCache::new(
            BloomFilterCacheConfig::default(),
            config.index_dir.clone(),
        ));

        // INNO-001: 创建 AdaptiveBloomCache (三层自适应缓存)
        let adaptive_bloom_cache = if config.enable_adaptive_bloom_cache {
            let adaptive_config = crate::bloom::adaptive::AdaptiveBloomCacheConfig {
                l3_index_dir: config.index_dir.join("l3_bloom"),
                ..Default::default()
            };
            match crate::bloom::adaptive::AdaptiveBloomCache::try_new(adaptive_config) {
                Ok(cache) => Some(Arc::new(cache)),
                Err(e) => {
                    tracing::warn!("Failed to create AdaptiveBloomCache: {}, falling back to disabled", e);
                    None
                }
            }
        } else {
            None
        };

        let adaptive_preallocator = if config.segment_preallocate_size > 0 {
            let prealloc_config = AdaptivePreallocatorConfig {
                initial_preallocate_bytes: config.segment_preallocate_size,
                ..Default::default()
            };
            Some(Arc::new(AdaptivePreallocator::new(prealloc_config)))
        } else {
            None
        };

        // S2-1: Initialize dictionary compressor if enabled (shared between read and write engines)
        let compressor: Option<Arc<parking_lot::Mutex<crate::compression::dictionary::DictionaryCompressor>>> = if config.compression.enable_dictionary {
            Some(Arc::new(parking_lot::Mutex::new(crate::compression::dictionary::DictionaryCompressor::new(config.compression.clone()))))
        } else {
            None
        };

        // P3-001: Initialize async I/O writer if enabled
        #[cfg(feature = "async-io")]
        let async_writer = if config.async_io_enabled {
            let async_config = crate::ops::async_io::AsyncIoConfig {
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
                    tracing::warn!("Failed to initialize async writer: {}, falling back to sync I/O", e);
                    None
                }
            }
        } else {
            None
        };
        #[cfg(not(feature = "async-io"))]
        let async_writer = None;

        // V0.6.0: Create global key index for O(log n) key lookups
        let global_index = Arc::new(crate::core::global_index::GlobalKeyIndex::new());

        // Phase 4.7: Create SINGLE shared EngineState BEFORE FileKV struct
        // ENG-007: Use builder pattern instead of 10+ parameter constructor
        let stats = Arc::new(FileKVStats::default());
        let engine_state = Arc::new(crate::engine::EngineState::builder(config.clone())
            .segments(segments)  // Move (no clone) - ArcSwap wraps internally
            .next_segment_id(max_id + 1)
            .index_manager(index_manager)           // Move (no clone) - ArcSwap wraps internally
            .stats(stats.clone())
            .memtable(memtable.clone())
            .bloom_filter_cache(bloom_filter_cache.clone())
            .adaptive_bloom_cache(adaptive_bloom_cache.clone())  // INNO-001: AdaptiveBloomCache
            .block_cache(block_cache.clone())
            .unified_cache(Some(unified_cache.clone()))  // GAP-M5: UnifiedCacheManager for budget control
            .global_index(global_index.clone())  // V0.6.0: Global key index
            .build());

        // INNO-001: Create SINGLE shared bloom migration controller
        let bloom_migration_controller = Arc::new(bloom::migration::MigrationController::new(
            bloom::migration::MigrationThresholds::default(),
        ));

        // Create feature flags controller
        let feature_flags = {
            let controller = ops::feature_flag::FeatureFlagController::new();
            if !config.enable_adaptive_bloom_cache {
                controller.set_enabled(ops::feature_flag::FeatureFlag::Inno001AdaptiveBloomCache, false);
            }
            if !config.enable_zone_map_pruning {
                controller.set_enabled(ops::feature_flag::FeatureFlag::Inno002ZoneMapPruning, false);
                controller.set_enabled(ops::feature_flag::FeatureFlag::Inno002SequentialPrefetch, false);
            } else if !config.enable_sequential_prefetch {
                controller.set_enabled(ops::feature_flag::FeatureFlag::Inno002SequentialPrefetch, false);
            }
            Arc::new(controller)
        };

        // Create engines
        let read_engine = {
            let range_pruner = if config.enable_zone_map_pruning {
                Some(Arc::new(RangeQueryPruner::with_defaults()))
            } else {
                None
            };
            let prefetcher = if config.enable_sequential_prefetch {
                // GAP-C4: Create block reader callback for prefetch
                let engine_state_for_prefetch = engine_state.clone();
                let block_reader = move |segment_id: u64, block_id: u64, block_size: u64| -> Option<Bytes> {
                    // Read block from segment
                    let segments = engine_state_for_prefetch.segment_state.segments.load();
                    if let Some(segment) = segments.get(&segment_id) {
                        let offset = block_id * block_size;
                        // Read all entries in this block and cache them
                        // For simplicity, we read the entire block as raw data
                        if let Ok(data) = segment.read_at(offset, block_size as u32) {
                            return Some(Bytes::from(data));
                        }
                    }
                    None
                };
                let prefetch_cache = crate::cache::block_cache::BlockCacheAsPrefetchCache::new(
                    block_cache.clone(),
                    config.block_size,
                    block_reader,
                );
                let prefetcher = SequentialPrefetcher::with_defaults(Arc::new(prefetch_cache));
                Some(Arc::new(parking_lot::RwLock::new(prefetcher)))
            } else {
                None
            };
            Arc::new(crate::engine::ReadEngine::new(
                engine_state.clone(),
                feature_flags.clone(),
                range_pruner,
                prefetcher,
                Arc::new(ops::memory_tracker::MemoryTracker::new(0)),
                bloom_migration_controller.clone(),
                compressor.clone(),  // S2-1: Use shared compressor
            ))
        };

        let write_engine = {
            let wal_manager = if config.enable_wal {
                Some(Mutex::new(WalManager::new_with_config(
                    config.fs.clone(),
                    &config.wal_dir,
                    true,
                    config.wal_max_size_bytes,
                    config.wal_max_files,
                    config.aggressive.wal_sync_mode,
                )?))
            } else {
                None
            };
            let audit_logger = if config.audit_log.enabled {
                let logger = ops::audit_log::AuditLogger::open(config.audit_log.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to initialize audit logger: {}", e))?;
                Some(Arc::new(logger))
            } else {
                None
            };
            Arc::new(crate::engine::WriteEngine::new(
                engine_state.clone(),
                wal_manager,
                write_coalescer.clone(),
                compressor.clone(),  // S2-1: Use shared compressor
                async_writer,
                flush_trigger.clone(),
                Arc::new(CompactionManager::new(config.compaction.clone())),
                audit_logger,
                adaptive_preallocator.clone(),
            ))
        };

        let compaction_engine = Arc::new(crate::engine::CompactionEngine::new(
            engine_state.clone(),
            config.compaction.clone(),
            adaptive_preallocator.clone(),
        ));

        // S2-3: Create metrics for Prometheus export (shared between LifecycleManager and FileKV)
        #[cfg(feature = "metrics")]
        let metrics = Arc::new(crate::ops::metrics::FileKVMetrics::new());

        let lifecycle_manager = Arc::new(crate::engine::LifecycleManager::new(
            engine_state.clone(),
            parking_lot::Mutex::new(IncrementalCheckpointManager::new(
                &config.checkpoint_dir,
            )?),
            None,
            #[cfg(feature = "metrics")]
            metrics.clone(),
            ops::timeout_control::TimeoutConfig::default(),
            Some(write_coalescer.clone()),
            flush_trigger.clone(),
            Arc::new(CompactionManager::new(config.compaction.clone())),
        ));

        let kv = Self {
            config: config.clone(),
            engine_state: engine_state.clone(),
            read_engine,
            write_engine,
            compaction_engine,
            lifecycle_manager,
            #[cfg(feature = "metrics")]
            metrics: metrics.clone(),
        };

        {
            // Use atomic counters for segment stats
            kv.engine_state.stats_state.stats.segment_count.store(
                kv.engine_state.segment_state.segment_count.load(std::sync::atomic::Ordering::Relaxed),
                std::sync::atomic::Ordering::Relaxed,
            );
            kv.engine_state.stats_state.stats.total_size_bytes.store(
                kv.engine_state.segment_state.total_size_bytes.load(std::sync::atomic::Ordering::Relaxed),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        // V0.6.0: Rebuild global key index from existing segments (crash recovery)
        {
            let segments = kv.engine_state.segment_state.segments.load();
            if !segments.is_empty() {
                tracing::info!("Rebuilding global key index from {} existing segments", segments.len());
                if let Err(e) = kv.engine_state.global_index_state.global_index.rebuild_from_segments(&segments) {
                    tracing::warn!("Failed to rebuild global key index: {}", e);
                } else {
                    let idx_stats = kv.engine_state.global_index_state.global_index.stats();
                    tracing::info!(
                        "Global key index rebuilt: {} keys indexed",
                        idx_stats.total_keys
                    );
                }
            }
        }

        if config.enable_bloom {
            let _ = kv.rebuild_bloom_filters();
        }

        if config.cache_warming_enabled {
            // Load segments for cache warming
            let segments = kv.engine_state.segment_state.segments.load();
            let segments_vec: Vec<Arc<SegmentFile>> = segments.values().cloned().collect();
            if !segments_vec.is_empty() {
                let cache_warmer = CacheWarmer::new(
                    CacheWarmingConfig::default(),
                    kv.engine_state.cache_state.block_cache.clone(),
                );
                let _ = cache_warmer.warm(&segments_vec);
            }
        }

        // DATA-004 FIX: Replay WAL entries after opening to recover from crashes
        if config.enable_wal {
            match kv.recover() {
                Ok(recovered_count) => {
                    if recovered_count > 0 {
                        tracing::info!("WAL recovery completed: {} entries replayed", recovered_count);
                    }
                }
                Err(e) => {
                    tracing::error!("WAL recovery failed: {}", e);
                    return Err(anyhow::anyhow!("WAL recovery failed: {}", e));
                }
            }
        }

        // Phase 5: Recover from incomplete compactions (crash-safe compaction)
        let manifest_dir = config.index_dir.join("compaction_manifests");
        match compaction::manifest::recover_incomplete(
            config.fs.as_ref(),
            &manifest_dir,
            &config.segment_dir,
        ) {
            Ok(actions) => {
                if !actions.is_empty() {
                    for action in &actions {
                        match action {
                            compaction::manifest::RecoveryAction::CleanedUp {
                                compaction_id,
                                deleted_output_segments,
                                restored_input_segments,
                            } => {
                                tracing::warn!(
                                    "Compaction crash recovery: compaction {} cleaned up, deleted outputs: {:?}, restored inputs: {:?}",
                                    compaction_id,
                                    deleted_output_segments,
                                    restored_input_segments
                                );
                            }
                            compaction::manifest::RecoveryAction::None => {}
                        }
                    }
                } else {
                    tracing::debug!("No incomplete compactions found during recovery");
                }
            }
            Err(e) => {
                tracing::warn!("Compaction manifest recovery failed: {} (continuing anyway)", e);
                // Non-critical failure, continue opening
            }
        }

        Ok(kv)
    }

    /// Recover data from WAL after crash
    ///
    /// This delegates to LifecycleManager.recover_from_wal() for unified recovery.
    pub fn recover(&self) -> crate::core::error::FileKVResult<usize> {
        if let Some(wal) = self.wal_ref() {
            let result = self.lifecycle_manager.recover_from_wal(wal)
                .map_err(|e| crate::core::error::FileKVError::Fatal(crate::core::error::FatalError::Corruption(format!("WAL recovery failed: {}", e))));

            if let Ok(count) = &result {
                if *count > 0 {
                    tracing::info!("WAL recovery completed: {} entries replayed", count);
                }
            }

            result
        } else {
            Ok(0)
        }
    }

    /// 1.1 OPTIMIZATION: Start background compaction thread after FileKV is constructed
    ///
    /// This should be called immediately after `FileKV::open()` returns.
    /// It spawns a dedicated thread that handles compaction asynchronously,
    /// preventing compaction from blocking the write path.
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = FileKVConfig::default();
    /// let kv = Arc::new(FileKV::open(config)?);
    /// kv.start_background_compaction()?; // Start async compaction
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_background_compaction(self: &Arc<Self>) -> anyhow::Result<()> {
        // GAP-C2 FIX: Set kv_weak on CompactionEngine so it can execute compaction
        let kv_weak = Arc::downgrade(self);
        self.compaction_engine.set_filekv_ref(kv_weak);

        // Delegate to CompactionEngine for thread management
        self.compaction_engine.start_background_compaction(self.engine_state.clone())
    }

    /// Manually trigger a compaction cycle and return compaction statistics
    /// Manually trigger a compaction run
    ///
    /// Compaction merges multiple segments into fewer segments,
    /// resolving duplicates and removing deleted entries.
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig, CompactionConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.enable_wal = false;
    /// config.compaction = CompactionConfig {
    ///     min_segments: 2,
    ///     auto_compact: false,
    ///     ..Default::default()
    /// };
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// // Write and flush to create segments
    /// kv.put("k1", b"v1").unwrap();
    /// kv.flush_memtable().unwrap();
    /// kv.put("k2", b"v2").unwrap();
    /// kv.flush_memtable().unwrap();
    ///
    /// let segments_before = kv.segments().load().len();
    /// let stats = kv.run_compaction().unwrap();
    /// assert!(stats.segments_merged >= 0);
    /// ```
    pub fn run_compaction(&self) -> anyhow::Result<compaction::CompactionStats> {
        // Phase 4.5: Delegate to CompactionEngine for validation, then execute actual compaction
        let segment_count = self.engine_state.segment_state.segment_count.load(std::sync::atomic::Ordering::Relaxed);
        let total_size: u64 = self.engine_state.segment_state.total_size_bytes.load(std::sync::atomic::Ordering::Relaxed);

        self.compaction_engine.run_compaction(|_| {
            // Execute actual compaction using this FileKV reference
            let req = compaction::CompactionRequest {
                segment_count,
                total_size_bytes: total_size,
                target_level: None,
            };

            // Call the actual compaction logic
            compaction::execute_compaction(self, &req)
        })
    }

    /// Get a reference to the FileKV configuration
    pub fn get_config(&self) -> &FileKVConfig {
        &self.config
    }

    /// P1-015: Get timeout configuration
    pub fn get_timeout_config(&self) -> parking_lot::MutexGuard<'_, ops::timeout_control::TimeoutConfig> {
        self.lifecycle_manager.get_timeout_config()
    }

    /// P1-015: Set timeout configuration
    pub fn set_timeout_config(&self, config: ops::timeout_control::TimeoutConfig) {
        // LifecycleManager holds timeout_config in a Mutex, so we can modify through Arc
        self.lifecycle_manager.set_timeout_config(config);
    }

    /// P1-015: Get timeout statistics snapshot
    pub fn get_timeout_stats(&self) -> ops::timeout_control::TimeoutStats {
        self.lifecycle_manager.get_timeout_stats()
    }

    /// P1-015: Reset timeout statistics
    pub fn reset_timeout_stats(&self) {
        self.lifecycle_manager.reset_timeout_stats()
    }

    /// Get segments reference (Phase 4.7: returns shared EngineState segments)
    /// Returns ArcSwap - lock-free snapshot access
    pub fn segments(&self) -> &arc_swap::ArcSwap<BTreeMap<u64, Arc<SegmentFile>>> {
        &self.engine_state.segment_state.segments
    }

    /// Get index manager reference (Phase 4.7: returns shared EngineState index_manager)
    pub fn index_manager_ref(&self) -> &parking_lot::RwLock<crate::core::sparse_index::IndexManager> {
        &self.engine_state.index_state.index_manager
    }

    /// Get write coalescer reference (for testing)
    pub fn write_coalescer_ref(&self) -> &Arc<WriteCoalescer> {
        self.write_engine.write_coalescer()
    }

    /// Get reference to WAL manager (for recovery)
    pub fn wal_ref(&self) -> Option<&Mutex<WalManager>> {
        self.write_engine.wal_ref()
    }

    /// Get reference to memtable (for recovery and checkpoints)
    pub fn memtable_ref(&self) -> &Arc<MemTable> {
        &self.engine_state.memtable_state.memtable
    }

    /// Get reference to block cache (for range scan and compaction)
    pub fn block_cache_ref(&self) -> &Arc<BlockCache> {
        &self.engine_state.cache_state.block_cache
    }

    /// Get reference to bloom filter cache (for bloom operations)
    pub fn bloom_filter_cache_ref(&self) -> &Arc<BloomFilterCache> {
        &self.engine_state.cache_state.bloom_filter_cache
    }

    /// Get reference to unified cache manager for budget control (GAP-M5)
    pub fn unified_cache_ref(&self) -> Option<&Arc<cache::UnifiedCacheManager>> {
        self.engine_state.cache_state.unified_cache.as_ref()
    }

    /// Load bloom filter for a segment
    pub fn load_bloom_filter(&self, segment_id: u64) -> anyhow::Result<Option<(BloomFilter, Vec<String>)>> {
        self.read_engine.load_bloom_filter(segment_id)
    }

    /// Get feature flag controller
    pub fn get_feature_flag_controller(&self) -> Arc<ops::feature_flag::FeatureFlagController> {
        self.read_engine.get_feature_flag_controller()
    }

    /// Check if INNO-002 Zone Map pruning is enabled at runtime
    /// NOTE: Public API for runtime feature flag inspection
    #[allow(dead_code)]
    pub(crate) fn is_zone_map_pruning_enabled(&self) -> bool {
        self.read_engine.is_zone_map_pruning_enabled()
    }

    /// Check if INNO-002 Sequential Prefetch is enabled at runtime
    /// NOTE: Public API for runtime feature flag inspection
    #[allow(dead_code)]
    pub(crate) fn is_sequential_prefetch_enabled(&self) -> bool {
        self.read_engine.is_sequential_prefetch_enabled()
    }

    /// Check if INNO-001 Adaptive Bloom Cache is enabled at runtime
    /// NOTE: Public API for runtime feature flag inspection
    #[allow(dead_code)]
    pub(crate) fn is_adaptive_bloom_cache_enabled(&self) -> bool {
        self.read_engine.is_adaptive_bloom_cache_enabled()
    }

    /// Enable INNO-002 (both Zone Map pruning and Sequential prefetch)
    pub fn enable_inno002(&self) {
        self.read_engine.enable_inno002()
    }

    /// Disable INNO-002 (both Zone Map pruning and Sequential prefetch)
    pub fn disable_inno002(&self) {
        self.read_engine.disable_inno002()
    }

    /// Enable INNO-001 Adaptive Bloom Cache
    pub fn enable_inno001(&self) {
        self.read_engine.enable_inno001()
    }

    /// Disable INNO-001 Adaptive Bloom Cache
    pub fn disable_inno001(&self) {
        self.read_engine.disable_inno001()
    }

    /// Get feature flag statistics
    pub fn get_feature_flag_stats(&self) -> ops::feature_flag::FeatureFlagStats {
        self.read_engine.get_feature_flag_stats()
    }

    /// Generate feature flag report
    pub fn generate_feature_flag_report(&self) -> ops::feature_flag::FeatureReport {
        self.read_engine.generate_feature_flag_report()
    }

    /// Get next adaptive preallocate size
    pub fn get_next_preallocate_size(&self) -> u64 {
        self.write_engine.get_next_preallocate_size()
    }

    /// Get adaptive preallocator statistics
    pub fn get_preallocator_stats(&self) -> Option<PreallocatorStats> {
        self.write_engine.get_preallocator_stats()
    }

    /// Record segment closed with actual size
    /// NOTE: Called by segment closing code for preallocator feedback
    #[allow(dead_code)]
    pub(crate) fn record_segment_closed(&self, actual_size: u64) {
        self.write_engine.record_segment_closed(actual_size)
    }

    /// Get range query pruner reference
    /// NOTE: Public API for range query optimization
    #[allow(dead_code)]
    pub(crate) fn get_range_query_pruner(&self) -> Option<&RangeQueryPruner> {
        self.read_engine.get_range_query_pruner()
    }

    /// Get sequential prefetcher reference
    /// NOTE: Public API for sequential prefetch inspection
    #[allow(dead_code)]
    pub(crate) fn get_sequential_prefetcher(&self) -> Option<&Arc<parking_lot::RwLock<SequentialPrefetcher<crate::cache::block_cache::BlockCacheAsPrefetchCache>>>> {
        self.read_engine.get_sequential_prefetcher()
    }

    /// Write key-value pair
    ///
    /// Phase 6: Default durability is Buffered - inserts into memtable immediately,
    /// WAL write without fsync. Data is readable right after put() returns.
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    /// use std::path::PathBuf;
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.checkpoint_dir = temp_dir.path().join("checkpoints");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put("greeting", b"hello").unwrap();
    /// let val = kv.get("greeting").unwrap().unwrap();
    /// assert_eq!(val.as_ref(), b"hello");
    /// ```
    pub fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        // S2-3: Auto-record Prometheus metrics
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_write(&self.metrics);
        
        // Phase 4.4: Delegate to WriteEngine (now shares EngineState)
        let result = self.write_engine.put(key, value);
        
        // S2-3: Record result
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());
        
        result
    }

    /// Write key-value pair with specified durability
    ///
    /// Phase 6: Allows caller to choose between Buffered (default, high throughput)
    /// and Immediate (bypasses buffer, writes directly to WAL + MemTable)
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig, core::types::Durability};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.enable_wal = true;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put_with_durability("important_key", b"critical_data", Durability::Immediate).unwrap();
    /// let val = kv.get("important_key").unwrap().unwrap();
    /// assert_eq!(val.as_ref(), b"critical_data");
    /// ```
    pub fn put_with_durability(&self, key: &str, value: &[u8], durability: crate::core::types::Durability) -> anyhow::Result<()> {
        // Phase 4.4: Delegate to WriteEngine (now shares EngineState)
        self.write_engine.put_with_durability(key, value, durability)
    }

    // === Async I/O Methods (feature-gated) ===

    /// Async write key-value pair with full async I/O
    ///
    /// This method uses AsyncWriter for non-blocking WAL and segment writes.
    /// Prefer this over `put()` when running in an async runtime for better throughput.
    ///
    /// # Example
    /// ```ignore
    /// use tokitai_filekv::FileKV;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let kv = FileKV::open("/tmp/test")?;
    ///     kv.put_async("key", b"value").await?;
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "async-io")]
    pub async fn put_async(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
        // S2-3: Auto-record Prometheus metrics
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_write(&self.metrics);

        let result = self.write_engine.put_async(key, value).await;

        // S2-3: Record result
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());

        result
    }

    /// Async delete key with full async I/O
    ///
    /// This method uses AsyncWriter for non-blocking writes.
    /// Prefer this over `delete()` when running in an async runtime.
    #[cfg(feature = "async-io")]
    pub async fn delete_async(&self, key: &str) -> anyhow::Result<()> {
        // S4-2: Auto-record Prometheus metrics for delete
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_delete(&self.metrics);

        let result = self.write_engine.delete_async(key).await;

        // S4-2: Record result
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());

        result
    }

    /// Async flush memtable to segment file
    ///
    /// Forces all pending memtable entries to be flushed to a new segment file
    /// using async I/O when available.
    #[cfg(feature = "async-io")]
    pub async fn flush_async(&self) -> anyhow::Result<()> {
        // MAJ-007-PHASE2: Auto-record Prometheus metrics for flush
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_flush(&self.metrics);

        let result = self.write_engine.flush_memtable_async().await;

        // MAJ-007-PHASE2: Record flush latency
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());

        result
    }
    /// Read key-value pair
    ///
    /// PERF-001 FIX: Uses SegmentFile::get_by_key() for O(1) dense index lookup
    /// when dense_index is enabled, falling back to sparse index + read_at().
    /// PERF-004 FIX: Moved index_manager.read() outside the segments loop to
    /// Get value by key with optimized lock granularity
    ///
    /// PERF-004 FIX: Reduced lock acquisitions and optimized lookup order:
    /// 1. MemTable (fastest, in-memory)
    /// 2. Block Cache (O(1) DashMap lookup)
    /// 3. Segments with Bloom Filter + Zone Map pruning
    /// 4. Sparse Index O(1) lookup (now using HashMap)
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.checkpoint_dir = temp_dir.path().join("checkpoints");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put("key1", b"value1").unwrap();
    ///
    /// // Existing key
    /// let val = kv.get("key1").unwrap().unwrap();
    /// assert_eq!(val.as_ref(), b"value1");
    ///
    /// // Non-existent key returns None
    /// let missing = kv.get("missing").unwrap();
    /// assert!(missing.is_none());
    /// ```
    pub fn get(&self, key: &str) -> anyhow::Result<Option<Bytes>> {
        // S2-3: Auto-record Prometheus metrics
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_read(&self.metrics);

        // Phase 4.3: Delegate to ReadEngine (now shares EngineState)
        let (result, _cache_result) = self.read_engine.get(key)?;

        // S2-3: Record cache hit/miss based on actual lookup source
        // Only BlockCache hits count as cache hits, not MemTable or disk hits
        #[cfg(feature = "metrics")]
        {
            match _cache_result {
                crate::engine::CacheLookupResult::BlockCacheHit => {
                    self.metrics.record_cache_hit();
                }
                crate::engine::CacheLookupResult::CacheMiss => {
                    self.metrics.record_cache_miss();
                }
                // MemTableHit and DiskHit don't affect cache hit/miss stats
                _ => {}
            }
            timer.record(result.is_some() || _cache_result != crate::engine::CacheLookupResult::CacheMiss);
        }

        Ok(result)
    }

    /// Batch write key-value pairs atomically
    ///
    /// ARCH-003, DATA-001 FIX: Implements atomic batch write with:
    /// 1. Single WAL record containing all entries (atomic durability)
    /// 2. Batch memtable insert (atomic insertion)
    ///
    /// Either all entries are written successfully, or none are (on WAL failure).
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.checkpoint_dir = temp_dir.path().join("checkpoints");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// let entries: Vec<(&str, &[u8])> = vec![
    ///     ("k1", b"v1"),
    ///     ("k2", b"v2"),
    ///     ("k3", b"v3"),
    /// ];
    /// kv.put_batch(&entries).unwrap();
    ///
    /// assert_eq!(kv.get("k1").unwrap().unwrap().as_ref(), b"v1");
    /// assert_eq!(kv.get("k2").unwrap().unwrap().as_ref(), b"v2");
    /// assert_eq!(kv.get("k3").unwrap().unwrap().as_ref(), b"v3");
    /// ```
    pub fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()> {
        // Phase 4.4: Delegate to WriteEngine (now shares EngineState)
        self.write_engine.put_batch(entries)
    }
    /// Delete a key from the KV store
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.checkpoint_dir = temp_dir.path().join("checkpoints");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put("temp_key", b"temporary").unwrap();
    /// assert!(kv.get("temp_key").unwrap().is_some());
    ///
    /// kv.delete("temp_key").unwrap();
    /// // After delete, get returns empty value (not None)
    /// let val = kv.get("temp_key").unwrap();
    /// assert_eq!(val.as_ref().map(|b| b.as_ref()), Some(b"".as_ref()));
    /// ```
    pub fn delete(&self, key: &str) -> anyhow::Result<()> {
        // S4-2: Auto-record Prometheus metrics for delete
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_delete(&self.metrics);

        // Phase 4.4: Delegate to WriteEngine (now shares EngineState)
        let result = self.write_engine.delete(key);

        // S4-2: Record result
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());

        result
    }

    /// Delete a key with specified durability guarantee
    /// Delete a key with specified durability
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig, core::types::Durability};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.checkpoint_dir = temp_dir.path().join("checkpoints");
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put("temp", b"data").unwrap();
    /// kv.delete_with_durability("temp", Durability::Buffered).unwrap();
    /// // After delete, get returns empty value
    /// let val = kv.get("temp").unwrap();
    /// assert_eq!(val.as_ref().map(|b| b.as_ref()), Some(b"".as_ref()));
    /// ```
    pub fn delete_with_durability(&self, key: &str, durability: crate::core::types::Durability) -> anyhow::Result<()> {
        // S4-2: Auto-record Prometheus metrics for delete
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_delete(&self.metrics);

        // Phase 4.4: Delegate to WriteEngine (now shares EngineState)
        let result = self.write_engine.delete_with_durability(key, durability);

        // S4-2: Record result
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());

        result
    }

    /// Get a snapshot of the current FileKV statistics
    /// Get current statistics about the KV store
    ///
    /// Returns a snapshot of various internal stats including segment count,
    /// memtable size, write/read counts, etc.
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// let stats = kv.get_stats();
    /// assert_eq!(stats.write_count, 0);
    /// assert_eq!(stats.segment_count, 0);
    ///
    /// kv.put("key", b"value").unwrap();
    /// let stats = kv.get_stats();
    /// assert_eq!(stats.write_count, 1);
    /// ```
    pub fn get_stats(&self) -> FileKVStatsSnapshot {
        // Phase 4.4: Get stats from shared EngineState via WriteEngine
        self.write_engine.get_stats()
    }

    /// V0.6.0: Get global key index statistics
    pub fn get_global_index_stats(&self) -> crate::core::global_index::IndexStats {
        self.engine_state.global_index_state.global_index.stats()
    }

    /// 4.1 OPTIMIZATION: Get memory usage snapshot
    pub fn get_memory_usage(&self) -> ops::memory_tracker::MemoryUsage {
        self.read_engine.get_memory_usage()
    }

    /// INNO-001: Get Bloom filter layer migration statistics
    pub fn get_bloom_migration_stats(&self) -> bloom::migration::MigrationStats {
        self.read_engine.get_bloom_migration_stats()
    }

    /// Flush MemTable to segment
    /// Flush the current memtable to a segment file
    ///
    /// Forces all pending writes in the memtable to be flushed to disk
    /// as a new segment file. This is useful for ensuring durability
    /// before shutdown or for triggering segment creation.
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put("key1", b"value1").unwrap();
    /// assert_eq!(kv.get_stats().segment_count, 0);
    ///
    /// kv.flush_memtable().unwrap();
    /// assert!(kv.get_stats().segment_count >= 1);
    /// ```
    pub fn flush_memtable(&self) -> anyhow::Result<()> {
        // MAJ-007-PHASE2: Auto-record Prometheus metrics for flush
        #[cfg(feature = "metrics")]
        let timer = crate::ops::metrics::MetricsTimer::start_flush(&self.metrics);

        // Phase 4.4: Delegate to WriteEngine (now shares EngineState)
        let result = self.write_engine.flush_memtable();

        // MAJ-007-PHASE2: Record flush latency
        #[cfg(feature = "metrics")]
        timer.record(result.is_ok());

        result
    }

    /// Run compaction if needed
    /// NOTE: Public API for manual compaction trigger
    #[allow(dead_code)]
    fn maybe_run_compaction(&self) -> anyhow::Result<()> {
        self.compaction_engine.maybe_run_compaction()
    }

    /// Scan a range of keys
    ///
    /// # Arguments
    /// * `start_key` - Start of range (inclusive)
    /// * `end_key` - End of range (inclusive)
    ///
    /// # Returns
    /// Iterator over key-value pairs in the range
    ///
    /// # Example
    /// ```rust,no_run
    /// # use tokitai_filekv::{FileKV, FileKVConfig};
    /// # fn example() -> anyhow::Result<()> {
    /// let config = FileKVConfig::default();
    /// let kv = FileKV::open(config)?;
    ///
    /// // Scan range "key_000" to "key_999"
    /// let mut count = 0;
    /// for result in kv.range("key_000", "key_999")? {
    ///     let entry = result?;
    ///     println!("{}: {} bytes", entry.key, entry.value.len());
    ///     count += 1;
    /// }
    /// println!("Scanned {} entries", count);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # INNO-002 Optimizations
    /// - Zone Map pruning: Skip blocks that don't overlap with query range
    /// - Sequential prefetching: Prefetch adjacent blocks for sequential access patterns
    /// - Lazy evaluation: Entries are fetched on-demand, not all at once
    ///
    /// # Performance
    /// - With Zone Map pruning: 40-60% fewer I/O operations
    /// - With prefetching: 15%+ higher cache hit rate
    pub fn range(
        &self,
        start_key: &str,
        end_key: &str,
    ) -> FileKVResult<query::scan::RangeScanIterator<'_>> {
        self.range_with_config(start_key, end_key, query::scan::RangeScanConfig::default())
    }

    /// Scan a range of keys with custom configuration
    ///
    /// # Arguments
    /// * `start_key` - Start of range (inclusive)
    /// * `end_key` - End of range (inclusive)
    /// * `config` - Scan configuration
    ///
    /// # Returns
    /// Iterator over key-value pairs in the range
    pub fn range_with_config(
        &self,
        start_key: &str,
        end_key: &str,
        config: query::scan::RangeScanConfig,
    ) -> FileKVResult<query::scan::RangeScanIterator<'_>> {
        debug!(
            "Range scan: [{}, {}], pruning={}, prefetch={}",
            start_key,
            end_key,
            config.enable_pruning,
            config.enable_prefetch
        );

        query::scan::RangeScanIterator::new(self, start_key, end_key, config)
    }

    /// Scan a range of keys and collect results into a Vec
    ///
    /// Convenience method that consumes the iterator and collects results
    ///
    /// # Arguments
    /// * `start_key` - Start of range (inclusive)
    /// * `end_key` - End of range (inclusive)
    /// * `limit` - Maximum number of entries to return (0 = unlimited)
    ///
    /// # Returns
    /// Vector of (key, value) pairs
    ///
    /// # Example
    /// ```
    /// use tokitai_filekv::{FileKV, FileKVConfig};
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let mut config = FileKVConfig::default();
    /// config.segment_dir = temp_dir.path().join("segments");
    /// config.wal_dir = temp_dir.path().join("wal");
    /// config.index_dir = temp_dir.path().join("index");
    /// config.enable_wal = false;
    ///
    /// let kv = FileKV::open(config).unwrap();
    /// kv.put("alpha", b"1").unwrap();
    /// kv.put("beta", b"2").unwrap();
    /// kv.flush_memtable().unwrap();
    ///
    /// let results = kv.range_collect("alpha", "beta", 0).unwrap();
    /// assert!(results.len() >= 1, "Should return at least 1 result, got {}", results.len());
    /// ```
    pub fn range_collect(
        &self,
        start_key: &str,
        end_key: &str,
        limit: usize,
    ) -> FileKVResult<Vec<(String, Vec<u8>)>> {
        let config = query::scan::RangeScanConfig {
            limit,
            ..Default::default()
        };

        let mut results = Vec::new();
        for entry_result in self.range_with_config(start_key, end_key, config)? {
            let entry = entry_result?;
            results.push((entry.key, entry.value));
        }

        Ok(results)
    }
}

/// Implement QuerySegmentProvider for FileKV
impl query::scan::QuerySegmentProvider for FileKV {
    fn get_segments_ordered(&self) -> Vec<(u64, Arc<SegmentFile>)> {
        use std::collections::HashSet;
        let segments = self.segments().load();
        let index_manager = self.index_manager_ref().read();

        let mut seen_ids = HashSet::new();
        let mut all_segment_ids: Vec<u64> = Vec::new();

        for &id in index_manager.all_indexes().keys() {
            if seen_ids.insert(id) {
                all_segment_ids.push(id);
            }
        }
        for &id in index_manager.all_dense_indexes().keys() {
            if seen_ids.insert(id) {
                all_segment_ids.push(id);
            }
        }

        // Return in reverse order (newest first for LSM-Tree semantics)
        all_segment_ids
            .into_iter()
            .rev()
            .filter_map(|id| segments.get(&id).map(|s| (id, Arc::clone(s))))
            .collect()
    }

    fn get_zone_map(&self, segment_id: u64) -> Option<query::zone_map::ZoneMapIndex> {
        let index_manager = self.index_manager_ref().read();
        index_manager.get_zone_map(segment_id)
    }

    fn get_block_cache(&self) -> Arc<BlockCache> {
        self.block_cache_ref().clone()
    }
}
