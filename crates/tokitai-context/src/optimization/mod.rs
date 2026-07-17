//! Optimization modules
//!
//! Performance optimizations including caching, compression,
//! deduplication, and efficient algorithms.

pub mod algorithms;
pub mod cache;
pub mod compression;
pub mod dedup;

// Re-exports for backward compatibility
pub use algorithms::lcs::{HirschbergLCS, OptimizedLcsResult};
pub use algorithms::lsh::{
    MinHashGenerator, MinHashSignature, LSHConfig, LSHIndex,
    LSHIndexStats, MinHashLSHIndex, DocumentMetadata,
};
pub use cache::arc::{
    ArcCache, ArcCacheConfig, ArcCacheStats, ArcEntry, BranchArcCache,
};
pub use cache::cuckoo::{
    CuckooFilter, CuckooStats, CuckooConflictDetector,
};
pub use cache::lru::{
    BranchLRUCache, BranchCacheConfig, CachedBranch, CacheStats,
    ThreadSafeBranchCache,
};
pub use compression::dictionary::{
    DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats,
    DictionaryMetadata, DictionaryContentAddressableStorage,
};
pub use dedup::cas::{
    ChangeType, CompressionAlgorithm, CompressionConfig,
    ContentAddressableEntry, ContentAddressableStorage,
    GcResult, IncrementalSnapshot, SnapshotChange, SnapshotManager,
    SnapshotMetadata, StorageStats,
};
pub use crate::three_way_merge::{
    FileMetadata, MergeOutcome, ThreeWayMerger, MergeComparison,
};
pub use crate::bloom_conflict::{
    BloomFilter, BloomConflictDetector, BloomStats, PerformanceComparison,
};
pub use crate::optimized_merge::{
    AdvancedMerger, ContentDeduplicator, DedupResult, DedupStats,
    Diff3Hunk, Diff3Result, LcsAlignment, SemanticBlock,
    SemanticMergeOutcome, SemanticMergeResult,
};
pub use crate::parallel_merge::{
    ParallelMerger, ParallelMergeConfig, ParallelMergeResult, ParallelMergeStats,
};
