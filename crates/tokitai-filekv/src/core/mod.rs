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

pub mod config;
pub mod error;
#[doc(hidden)]
pub mod flush;
pub mod global_index;
pub mod memtable;
#[doc(hidden)]
pub mod memtable_manager;
pub mod segment;
pub mod sparse_index;
pub mod types;
pub mod wal;
#[doc(hidden)]
pub mod wal_batcher;
#[doc(hidden)]
pub mod wal_channel;
#[doc(hidden)]
pub mod write_coalescer;

// Re-exports for convenience
pub use config::FileKVConfig;
pub use error::{
    DomainError, ErrorCategory, ExpectedError, FatalError, FileKVError, FileKVResult, ReadResult, TransientError,
    WriteResult,
};
pub use flush::FlushTrigger;
pub use global_index::{GlobalKeyIndex, IndexStats, IndexUpdate, KeyLocation};
pub use memtable::{MemTable, MemTableConfig, MemTableEntry};
pub use segment::{SegmentFile, SegmentStats};
pub use sparse_index::{IndexManager, SparseIndex};
pub use types::{AggressiveConfig, Durability, FileKVStats, FileKVStatsSnapshot, ValuePointer, WalSyncMode};
pub use wal::{WalEntry, WalManager};
pub use wal_channel::{WalChannel, WalChannelConfig, WalChannelStats};
pub use write_coalescer::{WriteCoalescer, WriteCoalescerConfig};
