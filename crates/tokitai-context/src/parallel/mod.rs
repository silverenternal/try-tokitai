//! Parallel context management modules
//!
//! Git-style branching for AI agent contexts, supporting:
//! - O(1) fork operations via copy-on-write
//! - 6 merge strategies
//! - Conflict detection and resolution

pub mod branch;
pub mod cow;
pub mod graph;
pub mod manager;
pub mod merge;
pub mod parallel_cache;

// Re-exports for backward compatibility
pub use branch::{
    BranchMetadata, BranchState, ConflictType, ContextBranch,
    MergeDecision, MergeStrategy,
};
pub use parallel_cache::{
    AncestorCache, AncestorCacheStats, BranchCache,
    CacheStats as CacheStatsV1, CacheWarmup, CacheWarmupConfig,
    CachedBranch as CachedBranchV1,
};
pub use cow::{
    BranchCloner, CowConfig, CowManager, CowStats, ForkResult,
};
pub use graph::{
    BranchPoint, Conflict, ConflictResolution, ConflictVersion,
    ContextGraph, ContextGraphManager, ContextGraphStats,
    MergeDecision as GraphMergeDecision, MergeRecord, MergedItem,
};
pub use manager::{
    ParallelContextManager, ParallelContextManagerConfig,
};
pub use merge::{
    compute_diff, BranchDiff, ContextItem as MergeContextItem,
    Merger, MergeResult, ModifiedItem,
};
