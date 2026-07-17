//! Error types for tokitai-context
//!
//! This module provides fine-grained error types for better error handling
//! and recovery strategies.
//!
//! ## Error Handling Guidelines
//!
//! ### Public API (Facade, Managers)
//! - Return `FileKVError` or module-specific error types
//! - Never use `anyhow::bail!()` in public APIs
//! - Use `?` operator to propagate errors
//!
//! ### Internal Implementation
//! - May use `anyhow` for internal error context
//! - Convert to `FileKVError` at module boundaries
//! - Use `.with_context()` for adding context
//!
//! ### Error Conversion
//! - Implement `From<OtherError> for FileKVError` for seamless conversion
//! - Use `map_err()` for one-off conversions
//! - Preserve error context when converting

use std::path::PathBuf;
use thiserror::Error;

use crate::file_kv::FileKVConfigError;

/// Result type alias for tokitai-context (FileKV operations)
pub type Result<T> = std::result::Result<T, FileKVError>;

/// Result type alias for general context operations
pub type ContextResult<T> = std::result::Result<T, ContextError>;

/// FileKV error types
#[derive(Debug, Error)]
pub enum FileKVError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] FileKVConfigError),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Checksum verification failed
    #[error("Checksum verification failed: expected {expected:08X}, got {actual:08X}")]
    ChecksumMismatch { expected: u32, actual: u32 },

    /// Data corruption detected
    #[error("Data corruption detected at {location}: {reason}")]
    Corruption { location: String, reason: String },

    /// Key not found
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Write-ahead log error
    #[error("WAL error: {0}")]
    Wal(#[from] crate::wal::WalError),

    /// Index error
    #[error("Index error: {0}")]
    Index(#[from] IndexError),

    /// Compaction error
    #[error("Compaction error: {0}")]
    Compaction(#[from] CompactionError),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// Operation timeout
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Resource exhausted (disk space, memory, etc.)
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

/// Index error types
#[derive(Debug, Error)]
pub enum IndexError {
    /// Index entry not found
    #[error("Index entry not found for key: {0}")]
    IndexNotFound(String),

    /// Index file corrupted
    #[error("Index file corrupted: {0}")]
    Corruption(String),

    /// Index file not found
    #[error("Index file not found: {0}")]
    IndexFileNotFound(PathBuf),

    /// Invalid index format
    #[error("Invalid index format: {0}")]
    InvalidFormat(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Compaction error types
#[derive(Debug, Error)]
pub enum CompactionError {
    /// Compaction already in progress
    #[error("Compaction already in progress")]
    AlreadyCompacting,

    /// No segments to compact
    #[error("No segments to compact")]
    NoSegmentsToCompact,

    /// Compaction failed
    #[error("Compaction failed: {0}")]
    Failed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Cache error types
#[derive(Debug, Error)]
pub enum CacheError {
    /// Cache entry not found
    #[error("Cache entry not found")]
    NotFound,

    /// Cache size exceeded
    #[error("Cache size exceeded: requested {requested}, available {available}")]
    SizeExceeded { requested: usize, available: usize },

    /// Invalid cache configuration
    #[error("Invalid cache configuration: {0}")]
    InvalidConfig(String),
}

/// Context management error types (for graph, branch, merge operations)
#[derive(Debug, Error)]
pub enum ContextError {
    /// Branch not found
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Branch already exists
    #[error("Branch already exists: {0}")]
    BranchAlreadyExists(String),

    /// Invalid branch state transition
    #[error("Invalid branch state transition: {branch} (current state: {current_state})")]
    InvalidBranchState { branch: String, current_state: String },

    /// Parent branch does not exist
    #[error("Parent branch does not exist: {0}")]
    ParentBranchNotFound(String),

    /// Merge conflict detected
    #[error("Merge conflict detected: {0}")]
    MergeConflict(String),

    /// Merge failed
    #[error("Merge failed: {0}")]
    MergeFailed(String),

    /// Checkpoint not found
    #[error("Checkpoint not found: {0}")]
    CheckpointNotFound(String),

    /// Hash chain not found
    #[error("Hash chain not found for branch: {0}")]
    HashChainNotFound(String),

    /// Hash chain is not enabled
    #[error("Hash chain is not enabled")]
    HashChainNotEnabled,

    /// Item not found
    #[error("Item not found: {0}")]
    ItemNotFound(String),

    /// Content not found
    #[error("Content not found: {0}")]
    ContentNotFound(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    /// Internal error (wrapped anyhow error)
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),

    /// WAL error
    #[error("WAL error: {0}")]
    Wal(#[from] crate::wal::WalError),
}

/// Error categories for recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Recoverable error - retry may succeed
    Recoverable,
    /// Non-recoverable error - manual intervention required
    Fatal,
    /// Temporary error - wait and retry may succeed
    Temporary,
    /// Configuration error - fix configuration before retry
    Config,
}

impl FileKVError {
    /// Get the error category for recovery strategy
    pub fn category(&self) -> ErrorCategory {
        match self {
            FileKVError::Config(_) => ErrorCategory::Config,
            FileKVError::Io(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                ErrorCategory::Recoverable
            }
            FileKVError::Io(ref e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                ErrorCategory::Fatal
            }
            FileKVError::Io(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                ErrorCategory::Temporary
            }
            FileKVError::Io(_) => ErrorCategory::Recoverable, // Other I/O errors are recoverable
            FileKVError::ChecksumMismatch { .. } => ErrorCategory::Fatal,
            FileKVError::Corruption { .. } => ErrorCategory::Fatal,
            FileKVError::KeyNotFound(_) => ErrorCategory::Recoverable,
            FileKVError::Wal(_) => ErrorCategory::Recoverable,
            FileKVError::Index(_) => ErrorCategory::Recoverable,
            FileKVError::Compaction(_) => ErrorCategory::Temporary,
            FileKVError::Cache(_) => ErrorCategory::Recoverable,
            FileKVError::Timeout(_) => ErrorCategory::Temporary,
            FileKVError::ResourceExhausted(_) => ErrorCategory::Temporary,
            FileKVError::InvalidState(_) => ErrorCategory::Fatal,
            FileKVError::PermissionDenied(_) => ErrorCategory::Fatal,
            FileKVError::Unsupported(_) => ErrorCategory::Fatal,
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.category() == ErrorCategory::Recoverable
            || self.category() == ErrorCategory::Temporary
    }

    /// Check if the error is fatal (requires manual intervention)
    pub fn is_fatal(&self) -> bool {
        self.category() == ErrorCategory::Fatal
    }

    /// Get retry suggestion
    pub fn retry_suggestion(&self) -> Option<&'static str> {
        match self.category() {
            ErrorCategory::Recoverable => Some("Retry the operation"),
            ErrorCategory::Temporary => Some("Wait and retry with exponential backoff"),
            ErrorCategory::Config => Some("Fix configuration before retrying"),
            ErrorCategory::Fatal => None,
        }
    }
}

/// Recovery action suggestions
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    /// Action description
    pub description: String,
    /// Whether the action is automatic or requires manual intervention
    pub is_automatic: bool,
    /// Estimated success probability (0.0 - 1.0)
    pub success_probability: f32,
}

impl FileKVError {
    /// Get suggested recovery action
    pub fn recovery_action(&self) -> Option<RecoveryAction> {
        match self {
            FileKVError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Some(RecoveryAction {
                    description: "Create missing directory or file".to_string(),
                    is_automatic: true,
                    success_probability: 0.9,
                })
            }
            FileKVError::KeyNotFound(_) => {
                Some(RecoveryAction {
                    description: "Key does not exist, this is normal for delete operations".to_string(),
                    is_automatic: false,
                    success_probability: 1.0,
                })
            }
            FileKVError::ChecksumMismatch { .. } => {
                Some(RecoveryAction {
                    description: "Run data integrity check and rebuild affected segments".to_string(),
                    is_automatic: false,
                    success_probability: 0.7,
                })
            }
            FileKVError::Corruption { location, reason } => {
                Some(RecoveryAction {
                    description: format!("Manually inspect and repair corruption at {}: {}", location, reason),
                    is_automatic: false,
                    success_probability: 0.5,
                })
            }
            FileKVError::Timeout(_) => {
                Some(RecoveryAction {
                    description: "Increase timeout or retry with exponential backoff".to_string(),
                    is_automatic: true,
                    success_probability: 0.8,
                })
            }
            FileKVError::ResourceExhausted(resource) => {
                Some(RecoveryAction {
                    description: format!("Free up {} resources or increase limits", resource),
                    is_automatic: false,
                    success_probability: 0.9,
                })
            }
            FileKVError::Wal(_) => {
                Some(RecoveryAction {
                    description: "Replay WAL entries for recovery".to_string(),
                    is_automatic: true,
                    success_probability: 0.95,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category() {
        let io_not_found = FileKVError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert_eq!(io_not_found.category(), ErrorCategory::Recoverable);
        assert!(io_not_found.is_recoverable());
        assert!(!io_not_found.is_fatal());

        let checksum_error = FileKVError::ChecksumMismatch {
            expected: 0x12345678,
            actual: 0x87654321,
        };
        assert_eq!(checksum_error.category(), ErrorCategory::Fatal);
        assert!(!checksum_error.is_recoverable());
        assert!(checksum_error.is_fatal());

        let timeout_error = FileKVError::Timeout(std::time::Duration::from_secs(30));
        assert_eq!(timeout_error.category(), ErrorCategory::Temporary);
        assert!(timeout_error.is_recoverable());
    }

    #[test]
    fn test_recovery_action() {
        let io_not_found = FileKVError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        let action = io_not_found.recovery_action();
        assert!(action.is_some());
        assert!(action.unwrap().is_automatic);

        let checksum_error = FileKVError::ChecksumMismatch {
            expected: 0x12345678,
            actual: 0x87654321,
        };
        let action = checksum_error.recovery_action();
        assert!(action.is_some());
        assert!(!action.unwrap().is_automatic);
    }

    #[test]
    fn test_config_error_validation() {
        use crate::file_kv::{FileKVConfig, MemTableConfig};

        let bad_config = FileKVConfig {
            memtable: MemTableConfig {
                flush_threshold_bytes: 100, // Too small
                max_entries: 10, // Too small
                max_memory_bytes: 64 * 1024 * 1024, // 64MB - P2-007 backpressure limit
            },
            ..Default::default()
        };

        let validation = bad_config.validate();
        assert!(!validation.is_valid());
        assert!(!validation.errors.is_empty());

        // Convert to FileKVError
        let result: crate::error::Result<()> = Err(validation.errors.into_iter().next().unwrap().into());
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(matches!(err, FileKVError::Config(_)));
            // Config errors have their own category
            assert_eq!(err.category(), ErrorCategory::Config);
        }
    }
}
