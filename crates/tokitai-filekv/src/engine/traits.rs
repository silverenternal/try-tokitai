//! Engine trait interfaces for inter-engine communication
//!
//! This module defines explicit trait boundaries between the four engines,
//! enabling independent development, testing, and replacement of engine implementations.
//!
//! # Architecture
//!
//! ```text
//! FileKV (facade)
//! ├── ReadEngine: Arc<dyn ReadEngineAPI>
//! ├── WriteEngine: Arc<dyn WriteEngineAPI>
//! ├── CompactionEngine: Arc<dyn CompactionEngineAPI>
//! └── LifecycleManager: Arc<dyn LifecycleManagerAPI>
//! ```
//!
//! Each trait defines the minimum interface needed for other engines to interact
//! with it, without exposing internal implementation details.

use std::sync::Arc;

use bytes::Bytes;
use parking_lot::Mutex;

use crate::compaction::CompactionStats;

// Re-export for convenience
pub use crate::engine::types::CacheLookupResult;

// ============================================================================
// Read Engine API
// ============================================================================

/// Statistics from read operations
#[derive(Debug, Clone, Default)]
pub struct ReadStats {
    pub read_count: u64,
    pub read_io_operations: u64,
    pub cache_hit_rate: f64,
}

/// Read engine interface for KV lookups
///
/// This trait defines the minimum interface needed for other components
/// to perform read operations against the storage engine.
pub trait ReadEngineAPI: Send + Sync {
    /// Get value by key
    ///
    /// Returns (value, cache_hit_info) where cache_hit_info indicates
    /// where the data was found (MemTable, BlockCache, Disk, or Miss).
    fn get(&self, key: &str) -> anyhow::Result<(Option<Bytes>, CacheLookupResult)>;

    /// Get read operation statistics
    fn get_stats(&self) -> ReadStats;

    /// Get memory usage snapshot
    fn get_memory_usage(&self) -> crate::ops::memory_tracker::MemoryUsage;
}

// ============================================================================
// Write Engine API
// ============================================================================

/// Statistics from write operations
#[derive(Debug, Clone, Default)]
pub struct WriteStats {
    pub write_count: u64,
    pub memtable_size: u64,
    pub memtable_entries: u64,
    pub wal_bytes_written: u64,
    pub flush_count: u64,
}

/// Write engine interface for KV mutations
///
/// This trait defines the minimum interface needed for other components
/// to perform write operations and coordinate with the write path.
pub trait WriteEngineAPI: Send + Sync {
    /// Write key-value pair (default durability: Buffered)
    fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;

    /// Write key-value pair with specified durability
    fn put_with_durability(
        &self,
        key: &str,
        value: &[u8],
        durability: crate::core::types::Durability,
    ) -> anyhow::Result<()>;

    /// Batch write key-value pairs atomically
    fn put_batch(&self, entries: &[(&str, &[u8])]) -> anyhow::Result<()>;

    /// Delete key (tombstone)
    fn delete(&self, key: &str) -> anyhow::Result<()>;

    /// Flush memtable to segment file
    fn flush_memtable(&self) -> anyhow::Result<()>;

    /// Get reference to WAL manager (for recovery)
    fn wal_ref(&self) -> Option<&Mutex<crate::core::wal::WalManager>>;

    /// Get write operation statistics
    fn get_stats(&self) -> WriteStats;
}

// ============================================================================
// Compaction Engine API
// ============================================================================

/// Compaction engine interface for segment management
///
/// This trait defines the minimum interface needed for other components
/// to trigger and monitor compaction operations.
pub trait CompactionEngineAPI: Send + Sync {
    /// Run compaction if needed (based on internal thresholds)
    fn maybe_run_compaction(&self) -> anyhow::Result<()>;

    /// Force run compaction for a specific level (or all levels if None)
    fn run_compaction(&self, level: Option<u8>) -> anyhow::Result<CompactionStats>;

    /// Start background compaction thread (if async compaction is enabled)
    fn start_background_compaction(&self) -> anyhow::Result<()>;

    /// Record a write operation and potentially trigger compaction
    fn record_write(&self) -> bool;

    /// Get compaction statistics
    fn get_stats(&self) -> CompactionStats;

    /// Get reference to internal CompactionManager (for advanced usage)
    fn compaction_manager(&self) -> &Mutex<crate::compaction::CompactionManager>;
}

// ============================================================================
// Lifecycle Manager API
// ============================================================================

/// Recovery information from WAL replay
#[derive(Debug, Clone, Default)]
pub struct RecoveryInfo {
    pub entries_replayed: usize,
}

/// Lifecycle manager interface for initialization, recovery, and maintenance
///
/// This trait defines the minimum interface needed for managing the
/// lifecycle of the storage engine.
pub trait LifecycleManagerAPI: Send + Sync {
    /// Create or open FileKV storage
    ///
    /// This is the main initialization entry point.
    fn open(config: crate::core::types::FileKVConfig) -> anyhow::Result<Arc<crate::engine::EngineState>>;

    /// WAL recovery - replay entries from write-ahead log
    ///
    /// Returns the number of entries replayed.
    fn recover_from_wal(&self, wal: &Mutex<crate::core::wal::WalManager>) -> anyhow::Result<usize>;

    /// Rebuild bloom filters for all segments
    ///
    /// Returns the number of bloom filters rebuilt.
    fn rebuild_bloom_filters(&self) -> anyhow::Result<usize>;

    /// Warm block cache from segments
    fn warm_cache(&self) -> anyhow::Result<()>;

    /// Get configuration
    fn get_config(&self) -> &crate::core::types::FileKVConfig;

    /// Get timeout configuration
    fn get_timeout_config(&self) -> parking_lot::MutexGuard<'_, crate::ops::timeout_control::TimeoutConfig>;

    /// Set timeout configuration
    fn set_timeout_config(&self, config: crate::ops::timeout_control::TimeoutConfig);

    /// Get timeout statistics snapshot
    fn get_timeout_stats(&self) -> crate::ops::timeout_control::TimeoutStats;

    /// Reset timeout statistics
    fn reset_timeout_stats(&self);

    /// Get checkpoint manager reference
    fn checkpoint_manager(&self) -> &Mutex<crate::checkpoint::IncrementalCheckpointManager>;
}

// ============================================================================
// Blanket implementations for Arc-wrapped engine types
// ============================================================================

// These allow Arc<ConcreteEngine> to be used where Arc<dyn Trait> is expected
// without explicit trait object conversion.
