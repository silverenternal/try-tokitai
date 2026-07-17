//! Compaction wrapper for FileKV
//!
//! This module provides a thin wrapper around the main CompactionManager
//! for use within the file_kv module.

pub use crate::compaction::{CompactionManager, CompactionConfig, CompactionStats};
