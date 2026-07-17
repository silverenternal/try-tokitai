//! Atlas Context - Git-style parallel context management for AI agents
//!
//! This crate provides a sophisticated context storage and management system
//! inspired by Git branching semantics. It enables AI agents to maintain
//! multiple parallel conversation contexts with efficient forking, merging,
//! and conflict resolution.
//!
//! ## Core Features
//!
//! - **Parallel Context Branches**: Create O(1) context forks using copy-on-write semantics
//! - **Git-style Merging**: 6 merge strategies (FastForward, SelectiveMerge, AIAssisted, etc.)
//! - **Conflict Resolution**: AI-powered conflict detection and resolution
//! - **Layered Storage**: Transient, short-term, and long-term context layers
//! - **Semantic Indexing**: SimHash-based semantic retrieval
//! - **Incremental Hash Chains**: Snapshot and rollback support
//!
//! ## Architecture
//!
//! ```text
//! .context/
//! ├── branches/        # Branch metadata and content
//! ├── graph.json       # Context graph (DAG of branches)
//! ├── merge_logs/      # Merge history
//! ├── checkpoints/     # Saved checkpoints
//! └── cow_store/       # Copy-on-write storage
//! ```
//!
//! ## Quick Start
//!
//! ### Simple API (Facade)
//!
//! ```rust,no_run
//! use tokitai_context::facade::{Context, Layer};
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut ctx = Context::open("./.context")?;
//!
//! // Store content
//! let hash = ctx.store("session-1", b"Hello, World!", Layer::ShortTerm)?;
//!
//! // Retrieve content
//! let item = ctx.retrieve("session-1", &hash)?;
//! println!("Content: {:?}", String::from_utf8_lossy(&item.content));
//! # Ok(())
//! # }
//! ```
//!
//! ### Advanced API (Parallel Context Management)
//!
//! ```rust,no_run
//! use tokitai_context::{ParallelContextManager, ParallelContextManagerConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ParallelContextManagerConfig {
//!     context_root: std::path::PathBuf::from(".context"),
//!     ..Default::default()
//! };
//!
//! let mut manager = ParallelContextManager::new(config)?;
//!
//! // Create a new branch
//! let branch = manager.create_branch("feature", "main")?;
//!
//! // Checkout branch
//! manager.checkout(&branch.branch_id)?;
//!
//! // Merge back to main
//! manager.merge("feature", "main", None)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Performance
//!
//! - Fork: ~6ms (O(1) via symlinks)
//! - Merge: ~45ms (average)
//! - Checkout: ~2ms
//! - Storage overhead: ~18%
//!
//! ## Module Structure
//!
//! The crate is organized into four layers:
//!
//! - **core**: Fundamental storage (file service, layers, hash index, logging)
//! - **parallel**: Git-style branch management (branch, merge, COW)
//! - **optimization**: Performance enhancements (caching, compression, deduplication)
//! - **ai**: AI-powered features (conflict resolution, semantic search) - requires `ai` feature
//!
//! ## Feature Flags
//!
//! - `ai`: Enable AI-powered features (conflict resolution, purpose inference)
//! - `benchmarks`: Include performance benchmarking suite
//! - `full`: Enable all features

#![allow(dead_code)]
#![allow(unused_imports)]

// ============================================================================
// Module Organization
// ============================================================================

// Core storage modules
mod file_service;
mod hash_index;
mod layers;
mod logger;
mod hash_chain;
mod distiller;
mod semantic_index;
mod knowledge_index;
mod knowledge_watcher;
mod path_resolver;

// Legacy modules (now organized into subdirectories)
// These are re-exported from the new modular structure:
// - parallel/: branch, graph, merge, manager, cow, parallel_cache
// - optimization/: cache, compression, dedup, algorithms
// - ai/: resolver, purpose, smart_merge, summarizer, enhanced_manager

// Merge algorithms (still in root - TODO: move to parallel/merge/)
mod three_way_merge;
mod bloom_conflict;
pub mod optimized_merge;
mod parallel_merge;

// Additional utilities
mod window_manager;
mod unified_manager;

// New modules for improved architecture
pub mod core;
pub mod parallel;
pub mod optimization;
#[cfg(feature = "ai")]
pub mod ai;
pub mod facade;
pub mod wal;
pub mod file_kv;
pub mod sparse_index;
pub mod block_cache;
pub mod compaction;
pub mod error;
pub mod tracing_config;
pub mod consistency_check;
pub mod crash_recovery;
pub mod metrics;
pub mod metrics_prometheus;
pub mod audit_log;
pub mod mvcc;
pub mod simd_checksum;
pub mod pitr;
#[cfg(feature = "distributed")]
pub mod distributed_coordination;
pub mod column_family;
#[cfg(feature = "fuse")]
pub mod fuse_fs;
pub mod query_optimizer;
pub mod auto_tuner;

// Benchmarks (optional)
#[cfg(feature = "benchmarks")]
pub mod benchmarks;

// ============================================================================
// Public API Exports
// ============================================================================

// Error types
pub use error::{
    FileKVError, IndexError,
    CompactionError, CacheError, ErrorCategory, RecoveryAction,
    Result,
};
pub use file_kv::FileKVConfigError;

// Facade API (simplified interface)
pub use facade::{
    Context, ContextConfig, ContextItem, ContextStats,
    Layer, RecoveryReport, SearchHit,
};

#[cfg(feature = "ai")]
pub use facade::AIContext;

// WAL (Write-Ahead Log)
pub use wal::{
    WalManager, WalOperation, WalEntry, WalStats,
    RecoveryEngine, IncompleteOperation,
};

// Core exports (re-exported from core module)
pub use crate::core::*;

// Parallel context exports (re-exported from parallel module)
pub use crate::parallel::*;

// Optimization exports (re-exported from optimization module)
pub use crate::optimization::*;

// AI exports (re-exported from ai module, feature-gated)
#[cfg(feature = "ai")]
pub use crate::ai::*;

// Tracing configuration
pub use tracing_config::{TracingTarget, init_tracing, init_tracing_minimal, init_tracing_json};

// Consistency check
pub use consistency_check::{
    ConsistencyChecker, ConsistencyCheckerConfig,
    CheckReport, ConsistencyIssue, RepairReport,
};

// MVCC
pub use mvcc::{
    MvccManager, MvccConfig, MvccStats, MvccStatsSnapshot,
    Transaction, TransactionId, TransactionState,
    Snapshot, SnapshotId,
    VersionChain, Version, VersionRef,
};

// Distributed coordination (feature-gated)
#[cfg(feature = "distributed")]
pub use distributed_coordination::{
    CoordinationConfig, CoordinationManager, CoordinationStats, CoordinationError,
    CoordinationResult, DistributedLock, LeaderElection, LeaderState,
};

// Column family
pub use column_family::{
    ColumnFamily, ColumnFamilyConfig, ColumnFamilyManager, ColumnFamilyStats,
    ColumnFamilyError, ColumnFamilyResult, BatchOperation, CompressionType,
};

// FUSE filesystem (feature-gated)
#[cfg(feature = "fuse")]
pub use fuse_fs::{
    FuseFS, FuseConfig, FuseError, FuseResult, FileHandle, Inode, InodeAttr,
};

// Query optimizer
pub use query_optimizer::{
    QueryOptimizer, OptimizerConfig, Query, QueryOp, QueryValue, QueryPredicate,
    QueryExecutor, QueryResult, QueryRow, LogicalPlan, PhysicalPlan, PlanNode,
    SortOrder, AggregateFunction, JoinType, JoinCondition, CostModel,
    TableStatistics, ColumnStatistics, IndexStatistics, Histogram, HistogramBucket,
    PlanStatistics, SortAlgorithm, DistinctMethod, ExecutionStats,
};

// Auto tuner
pub use auto_tuner::{
    AutoTuner, AutoTunerConfig, AutoTunerStats, TuningTarget, TuningRecommendation,
    TunableParams, ParamBounds, WorkloadPattern, WorkloadCharacteristics,
    MetricsSnapshot, SystemMetrics, StorageMetrics, QueryMetrics,
    AnomalyType, AnomalyAlert, AlertSeverity, RiskLevel,
};

// PITR (Point-in-Time Recovery)
pub use pitr::{
    PitrManager, PitrConfig, PitrStats, RecoveryPoint, RecoveryPointType,
    RecoveryProgress, RecoveryPhase, Timeline,
};

// Async I/O
pub use file_kv::async_io::{
    AsyncWriter, AsyncIoConfig, AsyncIoStats, AsyncWriteOp, AsyncWriteResult,
};

// Root context manager
use std::path::{Path, PathBuf};
use std::sync::Arc;
use error::{ContextResult, ContextError};

/// Context storage root directory manager
pub struct ContextRoot {
    root: PathBuf,
    sessions_dir: PathBuf,
    hashes_dir: PathBuf,
    logs_dir: PathBuf,
}

impl ContextRoot {
    /// Create or open context root directory
    pub fn new<P: AsRef<Path>>(root: P) -> ContextResult<Self> {
        let root = root.as_ref().to_path_buf();
        let sessions_dir = root.join("sessions");
        let hashes_dir = root.join("hashes");
        let logs_dir = root.join("logs");

        // Create directory structure
        std::fs::create_dir_all(&sessions_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create sessions directory: {:?}: {}", sessions_dir, e)))?;
        std::fs::create_dir_all(&hashes_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create hashes directory: {:?}: {}", hashes_dir, e)))?;
        std::fs::create_dir_all(&logs_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create logs directory: {:?}: {}", logs_dir, e)))?;

        Ok(Self {
            root,
            sessions_dir,
            hashes_dir,
            logs_dir,
        })
    }

    /// Get session directory path
    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
    }

    /// Get hashes directory
    pub fn hashes_dir(&self) -> &Path {
        &self.hashes_dir
    }

    /// Get logs directory
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Create session directory structure
    pub fn create_session(&self, session_id: &str) -> ContextResult<SessionDirs> {
        let session_dir = self.session_dir(session_id);
        let transient_dir = session_dir.join("transient");
        let short_term_dir = session_dir.join("short-term");
        let long_term_dir = session_dir.join("long-term");

        std::fs::create_dir_all(&transient_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create transient directory: {:?}: {}", transient_dir, e)))?;
        std::fs::create_dir_all(&short_term_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create short-term directory: {:?}: {}", short_term_dir, e)))?;
        std::fs::create_dir_all(&long_term_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create long-term directory: {:?}: {}", long_term_dir, e)))?;

        // Create subdirectories for long-term layer
        std::fs::create_dir_all(long_term_dir.join("git_rules"))
            .map_err(ContextError::Io)?;
        std::fs::create_dir_all(long_term_dir.join("tool_configs"))
            .map_err(ContextError::Io)?;
        std::fs::create_dir_all(long_term_dir.join("task_patterns"))
            .map_err(ContextError::Io)?;

        Ok(SessionDirs {
            session_dir,
            transient_dir,
            short_term_dir,
            long_term_dir,
        })
    }

    /// Remove session (delete entire session directory)
    pub fn remove_session(&self, session_id: &str) -> ContextResult<()> {
        let session_dir = self.session_dir(session_id);
        if session_dir.exists() {
            std::fs::remove_dir_all(&session_dir)
                .map_err(ContextError::Io)
                .map_err(|e| ContextError::OperationFailed(format!("Failed to remove session directory: {:?}: {}", session_dir, e)))?;
        }
        Ok(())
    }

    /// Get root directory
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Session directory structure
pub struct SessionDirs {
    pub session_dir: PathBuf,
    pub transient_dir: PathBuf,
    pub short_term_dir: PathBuf,
    pub long_term_dir: PathBuf,
}

/// Knowledge manager - integrates indexing, watching, and recommendation
pub struct KnowledgeManager {
    index: Option<KnowledgeIndex>,
    #[allow(dead_code)]
    watcher: Option<KnowledgeWatcher>,
    auto_recommend: bool,
    recommend_threshold: f32,
    recommend_limit: usize,
}

impl KnowledgeManager {
    /// Create knowledge manager
    pub fn new(
        knowledge_root: Option<&str>,
        auto_recommend: bool,
        recommend_threshold: f32,
        recommend_limit: usize,
    ) -> ContextResult<Self> {
        let (index, watcher) = if let Some(root) = knowledge_root {
            let path = std::path::PathBuf::from(root);
            if path.exists() {
                let idx = KnowledgeIndex::from_directory(&path)?;
                let arc_idx = std::sync::Arc::new(std::sync::RwLock::new(idx.clone()));
                let watcher = match KnowledgeWatcher::new(&path, Arc::clone(&arc_idx)) {
                    Ok(w) => Some(w),
                    Err(e) => {
                        tracing::warn!("Failed to create knowledge watcher: {}", e);
                        None
                    }
                };
                (Some(idx), watcher)
            } else {
                tracing::warn!("Knowledge directory does not exist: {}", root);
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(Self {
            index,
            watcher,
            auto_recommend,
            recommend_threshold,
            recommend_limit,
        })
    }

    /// Recommend knowledge based on query
    pub fn recommend(&self, query: &str) -> Vec<&KnowledgeNode> {
        if !self.auto_recommend {
            return Vec::new();
        }

        if let Some(ref idx) = self.index {
            idx.recommend(query, self.recommend_limit)
        } else {
            Vec::new()
        }
    }

    /// Get knowledge index
    pub fn index(&self) -> Option<&KnowledgeIndex> {
        self.index.as_ref()
    }

    /// Get statistics
    pub fn stats(&self) -> Option<KnowledgeStats> {
        self.index.as_ref().map(|idx| idx.stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_context_root_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = ContextRoot::new(temp_dir.path()).unwrap();

        assert!(context_root.root().exists());
        assert!(context_root.sessions_dir.exists());
        assert!(context_root.hashes_dir.exists());
        assert!(context_root.logs_dir.exists());
    }

    #[test]
    fn test_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = ContextRoot::new(temp_dir.path()).unwrap();

        let session_dirs = context_root.create_session("test_session").unwrap();

        assert!(session_dirs.session_dir.exists());
        assert!(session_dirs.transient_dir.exists());
        assert!(session_dirs.short_term_dir.exists());
        assert!(session_dirs.long_term_dir.exists());
        assert!(session_dirs.long_term_dir.join("git_rules").exists());
        assert!(session_dirs.long_term_dir.join("tool_configs").exists());
        assert!(session_dirs.long_term_dir.join("task_patterns").exists());
    }

    #[test]
    fn test_remove_session() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = ContextRoot::new(temp_dir.path()).unwrap();

        context_root.create_session("test_session").unwrap();
        context_root.remove_session("test_session").unwrap();

        assert!(!context_root.session_dir("test_session").exists());
    }
}
