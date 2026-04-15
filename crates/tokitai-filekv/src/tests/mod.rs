//! Integration tests for FileKV
//!
//! This module contains comprehensive test suites:
//! - integration: Main integration tests
//! - batch_atomic: Batch and atomic operation tests
//! - write_buffer: Write buffer tests (Phase 6)
//! - range_query: Range query tests
//! - wal_recovery: WAL recovery tests
//! - stability: Long-running stability tests

pub mod integration;
pub mod batch_atomic;
pub mod write_buffer;
pub mod range_query;
pub mod wal_recovery;
pub mod stability;
pub mod property_tests;
