//! WAL (Write-Ahead Log) helpers for FileKV
//!
//! This module provides WAL operation helpers for the FileKV module.

use std::hash::Hasher;
use std::sync::Mutex;

use crate::error::{ContextResult, ContextError};
use crate::wal::{WalManager, WalOperation, DurabilityLevel};

/// WAL operation helper for batch operations
pub struct WalBatchWriter<'a> {
    wal_guard: std::sync::MutexGuard<'a, WalManager>,
    operations_count: usize,
}

impl<'a> WalBatchWriter<'a> {
    /// Create a new batch writer from a WAL mutex
    pub fn new(wal: &'a Mutex<WalManager>) -> Option<Self> {
        wal.lock().ok().map(|guard| Self {
            wal_guard: guard,
            operations_count: 0,
        })
    }

    /// Log an add operation
    ///
    /// Returns DurabilityLevel indicating whether data is persisted
    pub fn log_add(&mut self, key: &str, value: &[u8]) -> ContextResult<DurabilityLevel> {
        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(value);
        let hash = hasher.finish();
        let op = WalOperation::Add {
            session: key.to_string(),
            hash: format!("{:016X}", hash),
            layer: "segment".to_string(),
        };
        let durability = self.wal_guard.log_with_payload(op, format!("{}:{:016X}", value.len(), hash))
            .map_err(|e| ContextError::OperationFailed(format!("WAL operation failed: {}", e)))?;
        self.operations_count += 1;
        Ok(durability)
    }

    /// Log a delete operation
    ///
    /// Returns DurabilityLevel indicating whether data is persisted
    pub fn log_delete(&mut self, key: &str) -> ContextResult<DurabilityLevel> {
        let op = WalOperation::Delete {
            session: key.to_string(),
            hash: String::new(),
            content: None,
        };
        self.wal_guard.log(op).map_err(|e| ContextError::OperationFailed(format!("WAL operation failed: {}", e)))
    }

    /// Get the number of operations logged
    pub fn operations_count(&self) -> usize {
        self.operations_count
    }
}

/// Simple WAL writer for single operations
///
/// Returns DurabilityLevel indicating whether data is persisted
pub fn log_wal_add(wal: &Mutex<WalManager>, key: &str, value: &[u8]) -> ContextResult<DurabilityLevel> {
    let mut wal_guard = wal.lock().map_err(|e| ContextError::OperationFailed(format!("WAL lock poisoned: {}", e)))?;
    let mut hasher = crc32c::Crc32cHasher::default();
    hasher.write(value);
    let hash = hasher.finish();
    let op = WalOperation::Add {
        session: key.to_string(),
        hash: format!("{:016X}", hash),
        layer: "segment".to_string(),
    };
    wal_guard.log_with_payload(op, format!("{}:{:016X}", value.len(), hash))
        .map_err(|e| ContextError::OperationFailed(format!("WAL operation failed: {}", e)))
}

/// Simple WAL writer for delete operations
///
/// Returns DurabilityLevel indicating whether data is persisted
pub fn log_wal_delete(wal: &Mutex<WalManager>, key: &str) -> ContextResult<DurabilityLevel> {
    let mut wal_guard = wal.lock().map_err(|e| ContextError::OperationFailed(format!("WAL lock poisoned: {}", e)))?;
    let op = WalOperation::Delete {
        session: key.to_string(),
        hash: String::new(),
        content: None,
    };
    wal_guard.log(op).map_err(|e| ContextError::OperationFailed(format!("WAL operation failed: {}", e)))
}
