//! Caching algorithms
//!
//! LRU, ARC, and Cuckoo filter based caching for performance optimization.

pub mod arc;
pub mod cuckoo;
pub mod lru;

pub use arc::{
    ArcCache, ArcCacheConfig, ArcCacheStats, ArcEntry, BranchArcCache,
};
pub use cuckoo::{
    CuckooFilter, CuckooStats, CuckooConflictDetector,
};
pub use lru::{
    BranchLRUCache, BranchCacheConfig, CachedBranch, CacheStats,
    ThreadSafeBranchCache,
};
