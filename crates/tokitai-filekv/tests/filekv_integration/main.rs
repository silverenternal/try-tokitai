//! Integration tests for tokitai-filekv
//!
//! These tests exercise the FileKV public API from an external consumer perspective,
//! testing complete workflows, concurrency, compaction, checkpoints, and batch operations.

mod batch_and_range;
mod checkpoint;
mod compaction_consistency;
mod concurrency;
mod high_concurrency;
mod lifecycle;
