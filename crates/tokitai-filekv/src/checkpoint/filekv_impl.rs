//! Checkpoint operations for FileKV

use crate::checkpoint::{CheckpointEntry, CheckpointStats, IncrementalCheckpoint, IncrementalCheckpointManager};
use crate::core::error::FatalError;
use crate::FileKV;
use std::collections::HashMap;

/// Result type for checkpoint operations
pub type Result<T> = std::result::Result<T, FatalError>;

impl FileKV {
    // ==================== P2-009: Incremental Checkpoint API ====================

    /// P2-009: Create a full checkpoint from current state
    ///
    /// This creates a complete snapshot of all key-value pairs in the store.
    /// Full checkpoints serve as the base for incremental checkpoints.
    ///
    /// # Arguments
    /// * `description` - Optional description for this checkpoint
    ///
    /// # Returns
    /// * `Ok(CheckpointId)` - The ID of the created checkpoint
    /// * `Err(ContextError)` - On checkpoint creation failure
    ///
    /// # Example
    /// ```rust,ignore
    /// let kv = FileKV::open(config)?;
    /// let checkpoint_id = kv.create_full_checkpoint(Some("Initial backup"))?;
    /// ```
    pub fn create_full_checkpoint(&self, description: Option<&str>) -> Result<String> {
        let mut state: HashMap<String, Vec<u8>> = HashMap::new();

        // 1. Collect all entries from MemTable (latest writes)
        for ref_multi in self.memtable_ref().iter() {
            let key: String = ref_multi.key().clone();
            let entry = ref_multi.value();

            if let Some(ref mem_entry) = entry.value {
                if !entry.deleted {
                    state.insert(key, mem_entry.as_ref().to_vec());
                }
            }
        }

        // 2. Collect all entries from Segments (older flushed data)
        // Note: This iterates all segments to capture data that has been flushed from MemTable
        let segments = self.segments().load();
        for segment in segments.values() {
            // Use segment's iterate_all method to collect all key-value pairs
            segment.iterate_all(|key: &str, value: &[u8], deleted: bool| {
                if !deleted {
                    // Only insert if not already in state (MemTable has newer data)
                    state.entry(key.to_string()).or_insert_with(|| value.to_vec());
                }
                Ok(())
            })?;
        }

        let mut manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager
            .create_full_checkpoint(&state, description)
            .map_err(|e| FatalError::Corruption(format!("Checkpoint creation failed: {}", e)))
    }

    /// P2-009: Create an incremental checkpoint with the given changes
    ///
    /// Incremental checkpoints only store changes (deltas) since the last checkpoint,
    /// making them much faster and smaller than full checkpoints.
    ///
    /// # Arguments
    /// * `changes` - List of changes (PUT/DELETE/MODIFY operations)
    /// * `description` - Optional description for this checkpoint
    ///
    /// # Returns
    /// * `Ok(CheckpointId)` - The ID of the created checkpoint
    /// * `Err(ContextError)` - On checkpoint creation failure
    pub fn create_incremental_checkpoint(
        &self,
        changes: Vec<CheckpointEntry>,
        description: Option<&str>,
    ) -> Result<String> {
        let mut manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager
            .create_incremental_checkpoint(changes, description)
            .map_err(|e| FatalError::Corruption(format!("Checkpoint creation failed: {}", e)))
    }

    /// P2-009: Create an incremental checkpoint with automatic full checkpoint trigger
    ///
    /// This method creates an incremental checkpoint, but when the full_checkpoint_interval
    /// is reached, it automatically creates a full checkpoint using the current state.
    /// This is the recommended method for production use.
    ///
    /// # Arguments
    /// * `changes` - List of changes (PUT/DELETE/MODIFY operations)
    /// * `description` - Optional description for this checkpoint
    ///
    /// # Returns
    /// * `Ok(CheckpointId)` - The ID of the created checkpoint (incremental or full)
    /// * `Err(ContextError)` - On checkpoint creation failure
    pub fn create_incremental_checkpoint_with_auto_full(
        &self,
        changes: Vec<CheckpointEntry>,
        description: Option<&str>,
    ) -> Result<String> {
        // Collect current full state for potential auto full checkpoint
        let mut state: HashMap<String, Vec<u8>> = HashMap::new();

        // 1. Collect all entries from MemTable (latest writes)
        for ref_multi in self.memtable_ref().iter() {
            let key: String = ref_multi.key().clone();
            let entry = ref_multi.value();

            if let Some(ref mem_entry) = entry.value {
                if !entry.deleted {
                    state.insert(key, mem_entry.as_ref().to_vec());
                }
            }
        }

        // 2. Collect all entries from Segments (older flushed data)
        let segments = self.segments().load();
        for segment in segments.values() {
            segment.iterate_all(|key: &str, value: &[u8], deleted: bool| {
                if !deleted {
                    state.entry(key.to_string()).or_insert_with(|| value.to_vec());
                }
                Ok(())
            })?;
        }

        let mut manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager
            .create_incremental_checkpoint_with_auto_full(changes, Some(&state), description)
            .map_err(|e| FatalError::Corruption(format!("Checkpoint creation failed: {}", e)))
    }

    /// P2-009: Compute the diff between two states for incremental checkpoint
    ///
    /// This utility function compares old and new state and returns the list of
    /// changes (PUT for new keys, DELETE for removed keys, MODIFY for changed values).
    ///
    /// # Arguments
    /// * `old_state` - Previous state
    /// * `new_state` - Current state
    ///
    /// # Returns
    /// * `Vec<CheckpointEntry>` - List of changes to apply
    pub fn compute_diff(
        old_state: &HashMap<String, Vec<u8>>,
        new_state: &HashMap<String, Vec<u8>>,
    ) -> Vec<CheckpointEntry> {
        IncrementalCheckpointManager::compute_diff(old_state, new_state)
    }

    /// P2-009: Restore state from a checkpoint
    ///
    /// Restores the key-value store to the state captured by the specified checkpoint.
    /// For incremental checkpoints, this will replay the chain from the base full checkpoint.
    ///
    /// # Arguments
    /// * `checkpoint_id` - The ID of the checkpoint to restore from
    ///
    /// # Returns
    /// * `Ok(HashMap<String, Vec<u8>>)` - The restored state
    /// * `Err(ContextError)` - On restoration failure
    ///
    /// # Example
    /// ```rust,ignore
    /// let state = kv.restore_from_checkpoint(&checkpoint_id)?;
    /// // Now you can use the restored state
    /// ```
    pub fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<HashMap<String, Vec<u8>>> {
        let manager = self.lifecycle_manager.checkpoint_manager().lock();
        let checkpoint_id_str = checkpoint_id.to_string();
        manager
            .restore(&checkpoint_id_str)
            .map_err(|e| FatalError::Corruption(format!("Checkpoint restore failed: {}", e)))
    }

    /// P2-009: Get the latest checkpoint
    ///
    /// # Returns
    /// * `Option<IncrementalCheckpoint>` - The latest checkpoint, if any exists
    pub fn get_latest_checkpoint(&self) -> Option<IncrementalCheckpoint> {
        let manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager.get_latest().cloned()
    }

    /// P2-009: Get a checkpoint by ID
    ///
    /// # Arguments
    /// * `checkpoint_id` - The ID of the checkpoint to retrieve
    ///
    /// # Returns
    /// * `Option<IncrementalCheckpoint>` - The checkpoint, if found
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<IncrementalCheckpoint> {
        let manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager.get_checkpoint(checkpoint_id).cloned()
    }

    /// P2-009: List all checkpoints
    ///
    /// # Returns
    /// * `Vec<IncrementalCheckpoint>` - All checkpoints sorted by sequence
    pub fn list_checkpoints(&self) -> Vec<IncrementalCheckpoint> {
        let manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager.list_checkpoints().into_iter().cloned().collect()
    }

    /// P2-009: Get checkpoint statistics
    ///
    /// # Returns
    /// * `CheckpointStats` - Statistics about checkpoints
    pub fn get_checkpoint_stats(&self) -> CheckpointStats {
        let manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager.get_stats()
    }

    /// P2-009: Compact old checkpoints to save space
    ///
    /// Deletes old checkpoints while preserving at least `keep_last_n` checkpoints
    /// and ensuring at least one full checkpoint remains.
    ///
    /// # Arguments
    /// * `keep_last_n` - Minimum number of checkpoints to keep
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of checkpoints deleted
    /// * `Err(ContextError)` - On compaction failure
    pub fn compact_checkpoints(&self, keep_last_n: usize) -> Result<usize> {
        let mut manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager
            .compact(keep_last_n)
            .map_err(|e| FatalError::Corruption(format!("Checkpoint compaction failed: {}", e)))
    }

    /// P2-009: Set the full checkpoint interval
    ///
    /// Configures how often a full checkpoint is created instead of incremental.
    ///
    /// # Arguments
    /// * `interval` - Create full checkpoint every N incremental checkpoints
    pub fn set_checkpoint_interval(&self, interval: u64) {
        let mut manager = self.lifecycle_manager.checkpoint_manager().lock();
        manager.set_full_checkpoint_interval(interval);
    }

    // ==================== End P2-009 ====================
}
