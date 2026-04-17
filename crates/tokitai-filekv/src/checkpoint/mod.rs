//! Checkpoint system for crash recovery
//!
//! This module contains the checkpoint functionality:
//! - Manager: Incremental checkpoint management
//! - Types: Checkpoint data structures
//! - Tests: Comprehensive checkpoint tests
//! - Checkpoint: FileKV checkpoint operations

pub mod filekv_impl;
pub mod manager;
#[cfg(test)]
pub mod tests;
pub mod types;

// Re-exports for convenience
pub use manager::IncrementalCheckpointManager;
pub use types::{
    CheckpointChain, CheckpointEntry, CheckpointId, CheckpointMetadata, CheckpointSeq, CheckpointStats, CheckpointType,
    IncrementalCheckpoint,
};
