//! Deduplication and content-addressable storage
//!
//! Content deduplication and snapshot management for storage optimization.

pub mod cas;

pub use cas::{
    ChangeType, CompressionAlgorithm, CompressionConfig,
    ContentAddressableEntry, ContentAddressableStorage,
    GcResult, IncrementalSnapshot, SnapshotChange, SnapshotManager,
    SnapshotMetadata, StorageStats,
};
