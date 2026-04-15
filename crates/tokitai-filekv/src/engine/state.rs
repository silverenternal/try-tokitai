//! Engine state containers split by functional concern
//!
//! Phase 2: Refactored from monolithic EngineState into focused state containers.
//! Each container holds only the data needed by specific engine components,
//! reducing implicit dependencies and enabling finer-grained locking.
//!
//! # State Containers
//!
//! ```text
//! EngineState (shell, holds Arc references to sub-containers)
//! ├── segment_state: Arc<SegmentState>
//! │   └── segments: RwLock<BTreeMap<...>>, next_segment_id: AtomicU64
//! ├── index_state: Arc<IndexState>
//! │   └── index_manager: RwLock<IndexManager>
//! ├── memtable_state: Arc<MemTableState>
//! │   └── memtable: Arc<MemTable>
//! ├── cache_state: Arc<CacheState>
//! │   ├── bloom_filter_cache, adaptive_bloom_cache, block_cache, unified_cache
//! └── stats_state: Arc<StatsState>
//!     └── stats: Arc<FileKVStats>
//! ```
//!
//! # Backward Compatibility
//!
//! EngineState provides `segments()`, `stats()`, `memtable()`, etc. as accessor
//! methods that forward to the appropriate sub-container fields.
//! Existing code like `state.segments.read()` must migrate to `state.segments().read()`.
//!
//! **Migration path**: Gradually replace `state.X` with `state.X_state.X` or `state.x()`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use arc_swap::ArcSwap;
use parking_lot::RwLock;

use crate::core::segment::SegmentFile;
use crate::core::sparse_index::IndexManager;
use crate::core::types::FileKVConfig;
use crate::core::types::FileKVStats;
use crate::core::memtable::MemTable;
use crate::core::global_index::GlobalKeyIndex;
use crate::bloom::filter_cache::BloomFilterCache;
use crate::bloom::adaptive::AdaptiveBloomCache;
use crate::cache::block_cache::BlockCache;
use crate::cache::UnifiedCacheManager;

// ============================================================================
// SegmentState - manages segment files and ID allocation
// ============================================================================

/// Segment state for write and compaction engines
///
/// Uses ArcSwap for lock-free segment snapshot reads.
/// Uses atomic counters for O(1) segment_count and total_size queries.
pub struct SegmentState {
    /// Segment files indexed by ID (ArcSwap for lock-free reads)
    pub segments: ArcSwap<BTreeMap<u64, Arc<SegmentFile>>>,
    /// Next segment ID allocator
    pub next_segment_id: AtomicU64,
    /// ENG-005: Atomic counter for segment count (avoids BTreeMap traversal)
    pub segment_count: AtomicUsize,
    /// ENG-005: Atomic counter for total segment size in bytes
    pub total_size_bytes: AtomicU64,
}

impl SegmentState {
    pub fn new(
        segments: BTreeMap<u64, Arc<SegmentFile>>,
        next_segment_id: AtomicU64,
    ) -> Self {
        // ENG-005: Initialize atomic counters from initial segments
        let segment_count = AtomicUsize::new(segments.len());
        let total_size_bytes: u64 = segments.values().map(|s| s.size()).sum();

        Self {
            segments: ArcSwap::new(Arc::new(segments)),
            next_segment_id,
            segment_count,
            total_size_bytes: AtomicU64::new(total_size_bytes),
        }
    }
}

// ============================================================================
// IndexState - manages sparse and dense indexes
// ============================================================================

/// Index state for read and write engines
pub struct IndexState {
    pub index_manager: RwLock<IndexManager>,
}

impl IndexState {
    pub fn new(index_manager: IndexManager) -> Self {
        Self {
            index_manager: RwLock::new(index_manager),
        }
    }
}

// ============================================================================
// MemTableState - manages in-memory buffer
// ============================================================================

/// MemTable state for write and read engines
pub struct MemTableState {
    pub memtable: Arc<MemTable>,
}

impl MemTableState {
    pub fn new(memtable: Arc<MemTable>) -> Self {
        Self { memtable }
    }
}

// ============================================================================
// CacheState - manages all cache layers
// ============================================================================

/// Cache state for read engine
pub struct CacheState {
    pub bloom_filter_cache: Arc<BloomFilterCache>,
    pub adaptive_bloom_cache: Option<Arc<AdaptiveBloomCache>>,
    pub block_cache: Arc<BlockCache>,
    pub unified_cache: Option<Arc<UnifiedCacheManager>>,
}

impl CacheState {
    pub fn new(
        bloom_filter_cache: Arc<BloomFilterCache>,
        adaptive_bloom_cache: Option<Arc<AdaptiveBloomCache>>,
        block_cache: Arc<BlockCache>,
        unified_cache: Option<Arc<UnifiedCacheManager>>,
    ) -> Self {
        Self {
            bloom_filter_cache,
            adaptive_bloom_cache,
            block_cache,
            unified_cache,
        }
    }
}

// ============================================================================
// StatsState - atomic counters for metrics
// ============================================================================

/// Statistics state (shared atomic counters)
pub struct StatsState {
    pub stats: Arc<FileKVStats>,
}

impl StatsState {
    pub fn new(stats: Arc<FileKVStats>) -> Self {
        Self { stats }
    }
}

// ============================================================================
// GlobalIndexState - manages the global sorted key index
// ============================================================================

/// Global key index state for read, write, and compaction engines
pub struct GlobalIndexState {
    pub global_index: Arc<GlobalKeyIndex>,
}

impl GlobalIndexState {
    pub fn new(global_index: Arc<GlobalKeyIndex>) -> Self {
        Self { global_index }
    }
}

// ============================================================================
// EngineState - shell that holds all sub-containers
// ============================================================================

/// Shared engine state - held by all engine components
///
/// Phase 2: Refactored to use focused state containers.
///
/// # Migration Guide
///
/// | Old code | New code |
/// |----------|----------|
/// | `state.segments.read()` | `state.segment_state.segments.read()` |
/// | `state.stats.read_count` | `state.stats_state.stats.read_count` |
/// | `state.memtable.get()` | `state.memtable_state.memtable.get()` |
/// | `state.block_cache.get()` | `state.cache_state.block_cache.get()` |
/// | `state.index_manager.read()` | `state.index_state.index_manager.read()` |
pub struct EngineState {
    pub config: FileKVConfig,

    // New state containers
    pub segment_state: Arc<SegmentState>,
    pub index_state: Arc<IndexState>,
    pub memtable_state: Arc<MemTableState>,
    pub cache_state: Arc<CacheState>,
    pub stats_state: Arc<StatsState>,
    pub global_index_state: Arc<GlobalIndexState>,
}

/// Builder for EngineState
///
/// ENG-007: Reduces constructor parameter count from 10 to a single builder object.
///
/// # Example
///
/// ```ignore
/// let state = EngineState::builder(config)
///     .segments(segments)
///     .next_segment_id(1)
///     .index_manager(index_manager)
///     .stats(stats)
///     .memtable(memtable)
///     .bloom_filter_cache(bloom_filter_cache)
///     .block_cache(block_cache)
///     .build();
/// ```
pub struct EngineStateBuilder {
    config: FileKVConfig,
    segments: BTreeMap<u64, Arc<SegmentFile>>,
    next_segment_id: u64,
    index_manager: IndexManager,
    stats: Arc<FileKVStats>,
    memtable: Arc<MemTable>,
    bloom_filter_cache: Arc<BloomFilterCache>,
    adaptive_bloom_cache: Option<Arc<AdaptiveBloomCache>>,
    block_cache: Arc<BlockCache>,
    unified_cache: Option<Arc<UnifiedCacheManager>>,
    global_index: Option<Arc<GlobalKeyIndex>>,
}

impl EngineStateBuilder {
    /// Create a new builder with the given config
    pub fn new(config: FileKVConfig) -> Self {
        Self {
            config,
            segments: BTreeMap::new(),
            next_segment_id: 1,
            index_manager: IndexManager::new(Path::new("")).expect("empty path for builder default"),
            stats: Arc::new(FileKVStats::default()),
            memtable: Arc::new(MemTable::new(crate::core::memtable::MemTableConfig::default())),
            bloom_filter_cache: Arc::new(BloomFilterCache::new(
                crate::bloom::filter_cache::BloomFilterCacheConfig::default(),
                PathBuf::new(),
            )),
            adaptive_bloom_cache: None,
            block_cache: Arc::new(BlockCache::new(crate::cache::block_cache::BlockCacheConfig::default())),
            unified_cache: None,
            global_index: None,
        }
    }

    /// Set the segments map
    pub fn segments(mut self, segments: BTreeMap<u64, Arc<SegmentFile>>) -> Self {
        self.segments = segments;
        self
    }

    /// Set the next segment ID
    pub fn next_segment_id(mut self, next_segment_id: u64) -> Self {
        self.next_segment_id = next_segment_id;
        self
    }

    /// Set the index manager
    pub fn index_manager(mut self, index_manager: IndexManager) -> Self {
        self.index_manager = index_manager;
        self
    }

    /// Set the stats
    pub fn stats(mut self, stats: Arc<FileKVStats>) -> Self {
        self.stats = stats;
        self
    }

    /// Set the memtable
    pub fn memtable(mut self, memtable: Arc<MemTable>) -> Self {
        self.memtable = memtable;
        self
    }

    /// Set the bloom filter cache
    pub fn bloom_filter_cache(mut self, bloom_filter_cache: Arc<BloomFilterCache>) -> Self {
        self.bloom_filter_cache = bloom_filter_cache;
        self
    }

    /// Set the adaptive bloom cache (optional)
    pub fn adaptive_bloom_cache(mut self, adaptive_bloom_cache: Option<Arc<AdaptiveBloomCache>>) -> Self {
        self.adaptive_bloom_cache = adaptive_bloom_cache;
        self
    }

    /// Set the block cache
    pub fn block_cache(mut self, block_cache: Arc<BlockCache>) -> Self {
        self.block_cache = block_cache;
        self
    }

    /// Set the unified cache manager (optional)
    pub fn unified_cache(mut self, unified_cache: Option<Arc<UnifiedCacheManager>>) -> Self {
        self.unified_cache = unified_cache;
        self
    }

    /// Set the global key index
    pub fn global_index(mut self, global_index: Arc<GlobalKeyIndex>) -> Self {
        self.global_index = Some(global_index);
        self
    }

    /// Build the EngineState
    pub fn build(self) -> EngineState {
        let global_index = self.global_index.unwrap_or_else(|| Arc::new(GlobalKeyIndex::new()));
        EngineState {
            config: self.config,
            segment_state: Arc::new(SegmentState::new(
                self.segments,
                AtomicU64::new(self.next_segment_id),
            )),
            index_state: Arc::new(IndexState::new(self.index_manager)),
            memtable_state: Arc::new(MemTableState::new(self.memtable)),
            cache_state: Arc::new(CacheState::new(
                self.bloom_filter_cache,
                self.adaptive_bloom_cache,
                self.block_cache,
                self.unified_cache,
            )),
            stats_state: Arc::new(StatsState::new(self.stats)),
            global_index_state: Arc::new(GlobalIndexState::new(global_index)),
        }
    }
}

impl EngineState {
    /// Create a new EngineState using the builder pattern (ENG-007)
    ///
    /// For the old direct-constructor API, see [`EngineState::new_raw`].
    pub fn builder(config: FileKVConfig) -> EngineStateBuilder {
        EngineStateBuilder::new(config)
    }

    /// Legacy constructor with all parameters inline (deprecated, use builder instead)
    #[deprecated(since = "0.2.0", note = "Use EngineState::builder() instead")]
    #[allow(clippy::too_many_arguments)]
    pub fn new_raw(
        config: FileKVConfig,
        segments: BTreeMap<u64, Arc<SegmentFile>>,
        next_segment_id: AtomicU64,
        index_manager: IndexManager,
        stats: Arc<FileKVStats>,
        memtable: Arc<MemTable>,
        bloom_filter_cache: Arc<BloomFilterCache>,
        adaptive_bloom_cache: Option<Arc<AdaptiveBloomCache>>,
        block_cache: Arc<BlockCache>,
        unified_cache: Option<Arc<UnifiedCacheManager>>,
    ) -> Self {
        Self {
            config,
            segment_state: Arc::new(SegmentState::new(segments, next_segment_id)),
            index_state: Arc::new(IndexState::new(index_manager)),
            memtable_state: Arc::new(MemTableState::new(memtable)),
            cache_state: Arc::new(CacheState::new(
                bloom_filter_cache,
                adaptive_bloom_cache,
                block_cache,
                unified_cache,
            )),
            stats_state: Arc::new(StatsState::new(stats)),
            global_index_state: Arc::new(GlobalIndexState::new(Arc::new(GlobalKeyIndex::new()))),
        }
    }

    /// Create a new EngineState (legacy name for backward compatibility)
    /// ENG-007: Delegates to builder internally, but accepts raw parameters for minimal migration friction.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: FileKVConfig,
        segments: BTreeMap<u64, Arc<SegmentFile>>,
        next_segment_id: AtomicU64,
        index_manager: IndexManager,
        stats: Arc<FileKVStats>,
        memtable: Arc<MemTable>,
        bloom_filter_cache: Arc<BloomFilterCache>,
        adaptive_bloom_cache: Option<Arc<AdaptiveBloomCache>>,
        block_cache: Arc<BlockCache>,
        unified_cache: Option<Arc<UnifiedCacheManager>>,
    ) -> Self {
        Self {
            config,
            segment_state: Arc::new(SegmentState::new(segments, next_segment_id)),
            index_state: Arc::new(IndexState::new(index_manager)),
            memtable_state: Arc::new(MemTableState::new(memtable)),
            cache_state: Arc::new(CacheState::new(
                bloom_filter_cache,
                adaptive_bloom_cache,
                block_cache,
                unified_cache,
            )),
            stats_state: Arc::new(StatsState::new(stats)),
            global_index_state: Arc::new(GlobalIndexState::new(Arc::new(GlobalKeyIndex::new()))),
        }
    }
}
