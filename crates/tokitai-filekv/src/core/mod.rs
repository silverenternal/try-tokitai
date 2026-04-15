//! Core storage types and data structures
//!
//! This module contains the fundamental storage components:
//! - Types: Configuration, statistics, value pointers
//! - Config: FileKV configuration validation
//! - Error: Four-layer error hierarchy
//! - MemTable: In-memory write buffer
//! - Segment: Sequential data segment files
//! - SparseIndex: Memory-resident sparse index
//! - WAL: Write-ahead log for crash recovery
//! - Flush: Background flush trigger
//! - GlobalKeyIndex: Global sorted key-to-segment-location index

pub mod types;
pub mod config;
pub mod error;
pub mod memtable;
pub mod segment;
pub mod sparse_index;
pub mod wal;
#[doc(hidden)]
pub mod flush;
#[doc(hidden)]
pub mod write_coalescer;
pub mod global_index;

// Re-exports for convenience
pub use types::{FileKVStats, FileKVStatsSnapshot, ValuePointer, AggressiveConfig, WalSyncMode, Durability};
pub use config::FileKVConfig;
pub use error::{
    FileKVError, FatalError, TransientError, ExpectedError, DomainError,
    FileKVResult, ReadResult, WriteResult, ErrorCategory,
};
pub use memtable::{MemTable, MemTableConfig, MemTableEntry};
pub use segment::{SegmentFile, SegmentStats};
pub use sparse_index::{SparseIndex, IndexManager};
pub use wal::{WalManager, WalEntry};
pub use flush::FlushTrigger;
pub use write_coalescer::{WriteCoalescer, WriteCoalescerConfig};
pub use global_index::{GlobalKeyIndex, KeyLocation, IndexStats, IndexUpdate};
