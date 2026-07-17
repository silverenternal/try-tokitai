//! Incremental Checkpoint Module
//!
//! This module implements incremental checkpointing for efficient state persistence.
//! Instead of creating full snapshots, incremental checkpoints only store changes
//! (deltas) since the last checkpoint, significantly reducing storage overhead
//! and checkpoint creation time.
//!
//! # Design
//!
//! ## Checkpoint Types
//! - **Full Checkpoint**: Complete state snapshot (base checkpoint)
//! - **Incremental Checkpoint**: Only contains changes since base checkpoint
//!
//! ## Checkpoint Chain
//! ```text
//! Full_0 → Incr_1 → Incr_2 → Incr_3 → Full_4 → Incr_5 → ...
//! ```
//!
//! ## Recovery
//! To restore state:
//! 1. Load the nearest full checkpoint
//! 2. Replay all incremental checkpoints in order
//! 3. Apply changes to reconstruct current state
//!
//! # Example
//!
//! ```rust,ignore
//! let mut manager = IncrementalCheckpointManager::new(checkpoint_dir);
//!
//! // Create initial full checkpoint
//! let base_id = manager.create_full_checkpoint(&state)?;
//!
//! // Create incremental checkpoints
//! let changes = compute_changes(&old_state, &new_state);
//! let incr_id = manager.create_incremental_checkpoint(&base_id, changes)?;
//!
//! // Restore from checkpoint chain
//! let restored_state = manager.restore(&incr_id)?;
//! ```

// Re-export all incremental checkpoint functionality
pub use super::incremental_types::*;
pub use super::incremental_manager::IncrementalCheckpointManager;
// Recovery is implemented as an impl on IncrementalCheckpointManager, no need to re-export
