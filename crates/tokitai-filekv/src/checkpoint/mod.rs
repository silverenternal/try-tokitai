//! Checkpoint system for crash recovery
//!
//! This module contains the checkpoint functionality:
//! - Manager: Incremental checkpoint management
//! - Types: Checkpoint data structures
//! - Tests: Comprehensive checkpoint tests
//! - Checkpoint: FileKV checkpoint operations

pub mod manager;
pub mod types;
pub mod filekv_impl;
#[cfg(test)]
pub mod tests;

// Re-exports for convenience
pub use manager::IncrementalCheckpointManager;
pub use types::{
    CheckpointEntry, CheckpointId, CheckpointSeq, CheckpointStats, CheckpointType,
    CheckpointChain, CheckpointMetadata, IncrementalCheckpoint,
};
