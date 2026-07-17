//! Incremental Checkpoint Types
//!
//! Core types and enums for incremental checkpointing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Checkpoint ID type
pub type CheckpointId = String;

/// Checkpoint sequence number
pub type CheckpointSeq = u64;

/// Checkpoint types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckpointType {
    /// Full checkpoint - complete state snapshot
    Full,
    /// Incremental checkpoint - changes since base
    Incremental {
        /// Base checkpoint ID this incremental is based on
        base_checkpoint: CheckpointId,
    },
}

/// Checkpoint entry representing a single change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CheckpointEntry {
    /// New entry added
    Put {
        key: String,
        value: Vec<u8>,
        timestamp: u64,
    },
    /// Entry deleted
    Delete {
        key: String,
        timestamp: u64,
    },
    /// Entry modified (value changed)
    Modify {
        key: String,
        old_value_hash: String,
        new_value: Vec<u8>,
        timestamp: u64,
    },
}

impl CheckpointEntry {
    /// Get the key affected by this entry
    pub fn key(&self) -> &str {
        match self {
            CheckpointEntry::Put { key, .. } => key,
            CheckpointEntry::Delete { key, .. } => key,
            CheckpointEntry::Modify { key, .. } => key,
        }
    }

    /// Get the timestamp of this entry
    pub fn timestamp(&self) -> u64 {
        match self {
            CheckpointEntry::Put { timestamp, .. } => *timestamp,
            CheckpointEntry::Delete { timestamp, .. } => *timestamp,
            CheckpointEntry::Modify { timestamp, .. } => *timestamp,
        }
    }
}

/// Checkpoint metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Optional description
    pub description: Option<String>,
    /// Total entries in this checkpoint
    pub total_entries: usize,
    /// Number of PUT operations
    pub put_count: usize,
    /// Number of DELETE operations
    pub delete_count: usize,
    /// Number of MODIFY operations
    pub modify_count: usize,
    /// Size of checkpoint data in bytes
    pub size_bytes: u64,
    /// Time taken to create checkpoint (microseconds)
    pub creation_time_us: u64,
}

/// Incremental checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalCheckpoint {
    /// Unique checkpoint ID
    pub checkpoint_id: CheckpointId,
    /// Sequence number (monotonically increasing)
    pub sequence: CheckpointSeq,
    /// Checkpoint type
    pub checkpoint_type: CheckpointType,
    /// Creation timestamp
    pub created_at: u64,
    /// List of changes in this checkpoint
    pub entries: Vec<CheckpointEntry>,
    /// Checkpoint metadata
    pub metadata: CheckpointMetadata,
    /// Hash of checkpoint content for integrity verification
    pub content_hash: String,
}

/// Checkpoint chain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointChain {
    /// List of checkpoint IDs in order
    pub checkpoint_ids: Vec<CheckpointId>,
    /// Map from checkpoint ID to sequence number
    pub sequence_map: HashMap<CheckpointId, CheckpointSeq>,
    /// Map from checkpoint ID to checkpoint type
    pub type_map: HashMap<CheckpointId, CheckpointType>,
    /// Latest full checkpoint ID
    pub latest_full: Option<CheckpointId>,
}

/// Checkpoint statistics
#[derive(Debug, Clone)]
pub struct CheckpointStats {
    pub total_checkpoints: usize,
    pub full_checkpoints: usize,
    pub incremental_checkpoints: usize,
    pub total_size_bytes: u64,
    pub total_entries: usize,
    pub latest_sequence: CheckpointSeq,
}
