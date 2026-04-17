//! Error types for FileKV
//!
//! # Layered Error Hierarchy (Phase 2)
//!
//! This module defines a hierarchy of error types so callers can distinguish
//! recoverable vs fatal vs expected errors at compile time:
//!
//! - **FatalError**: Cannot recover, must abort (data corruption, WAL corruption, I/O)
//! - **TransientError**: Retryable (resource exhausted, timeout, backpressure)
//! - **ExpectedError**: Not really errors, part of normal flow (key not found, segment not found)
//! - **DomainError**: Logic/domain failures (invalid config, compaction failed, index error)
//! - **FileKVError**: Unified error enum for internal use, wraps all categories

use thiserror::Error;

// =============================================================================
// Fatal errors (cannot recover, must abort)
// =============================================================================

/// Fatal errors indicate data corruption or unrecoverable I/O failures.
///
/// When a `FatalError` occurs, the store is in an inconsistent state and
/// must be aborted. Callers should not attempt retries.
#[derive(Debug, Error)]
pub enum FatalError {
    /// Data corruption detected in segment files or indexes
    #[error("Data corruption detected: {0}")]
    Corruption(String),

    /// Unrecoverable I/O error
    #[error("Unrecoverable I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// WAL file is corrupted
    #[error("WAL corrupted: {0}")]
    WalCorrupted(String),
}

// =============================================================================
// Transient errors (retryable)
// =============================================================================

/// Transient errors indicate temporary resource constraints.
///
/// These errors are expected to resolve on their own or after a retry
/// with backoff. Callers should implement retry logic for these.
#[derive(Debug, Error)]
pub enum TransientError {
    /// Resource exhausted (e.g., memory limit exceeded)
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Operation timed out
    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Backpressure applied (e.g., MemTable full)
    #[error("Backpressure: {0}")]
    Backpressure(String),
}

// =============================================================================
// Expected errors (not really errors, part of normal flow)
// =============================================================================

/// Expected errors represent normal control-flow outcomes, not actual failures.
///
/// These should typically be handled with `match` or `ok()` rather than
/// propagated as errors. For example, "key not found" is a normal result
/// of a lookup, not an error condition.
#[derive(Debug, Error)]
pub enum ExpectedError {
    /// Key was not found in the store
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Segment ID was not found
    #[error("Segment not found: {0}")]
    SegmentNotFound(u64),

    /// Bloom filter indicated key is definitely not in this segment
    #[error("Bloom filter negative: key not in segment {0}")]
    BloomNegative(u64),
}

// =============================================================================
// Domain errors (logic/domain failures)
// =============================================================================

/// Domain errors represent failures in business logic or configuration.
///
/// These are not retryable -- the caller must fix the underlying issue
/// (e.g., correct configuration, repair index) before retrying.
#[derive(Debug, Error)]
pub enum DomainError {
    /// Invalid configuration provided
    #[error("Invalid config: {0}")]
    Config(String),

    /// Compaction operation failed
    #[error("Compaction failed: {0}")]
    Compaction(String),

    /// Index operation failed
    #[error("Index error: {0}")]
    Index(String),

    /// Checkpoint operation failed
    #[error("Checkpoint error: {0}")]
    Checkpoint(String),
}

// =============================================================================
// Unified error for internal use
// =============================================================================

/// Unified FileKV error that wraps all error categories.
///
/// This is the primary error type used internally by FileKV modules.
/// Public API methods (`get`, `put`) still return `anyhow::Result<T>`
/// for backward compatibility, but internally they can be converted.
#[derive(Debug, Error)]
pub enum FileKVError {
    /// Fatal error -- cannot recover, must abort
    #[error(transparent)]
    Fatal(#[from] FatalError),

    /// Transient error -- retryable
    #[error(transparent)]
    Transient(#[from] TransientError),

    /// Expected error -- normal control-flow outcome
    #[error(transparent)]
    Expected(#[from] ExpectedError),

    /// Domain error -- logic/configuration failure
    #[error(transparent)]
    Domain(#[from] DomainError),
}

// =============================================================================
// Specialized Result types
// =============================================================================

/// General-purpose Result for FileKV internal operations.
pub type FileKVResult<T> = Result<T, FileKVError>;

/// Result type for read operations that may return "not found".
///
/// Use this for `get`-style operations where `ExpectedError::KeyNotFound`
/// is a normal outcome, not an error.
pub type ReadResult<T> = Result<T, ExpectedError>;

/// Result type for write operations.
///
/// Writes can fail fatally (corruption, I/O) or transiently (backpressure),
/// so this uses the full `FileKVError`.
pub type WriteResult<T> = Result<T, FileKVError>;

/// Legacy type alias for backward compatibility during migration.
/// New code should use `FileKVResult<T>` or specific result types.
#[deprecated(since = "0.2.0", note = "Use FileKVResult<T> instead")]
pub type ContextResult<T> = Result<T, FileKVError>;

// =============================================================================
// Error classification helpers
// =============================================================================

/// Error category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// I/O related errors
    Io,
    /// Configuration errors
    Config,
    /// Corruption errors
    Corruption,
    /// Resource exhaustion
    Resource,
    /// Timeout errors
    Timeout,
    /// Other errors
    Other,
}

impl FileKVError {
    /// Returns `true` if this error is retryable (Transient variant).
    pub fn is_retryable(&self) -> bool {
        matches!(self, FileKVError::Transient(_))
    }

    /// Returns `true` if this error is fatal (cannot recover).
    pub fn is_fatal(&self) -> bool {
        matches!(self, FileKVError::Fatal(_))
    }

    /// Returns `true` if this is an expected outcome (not a real error).
    pub fn is_expected(&self) -> bool {
        matches!(self, FileKVError::Expected(_))
    }

    /// Returns `true` if this is a domain/logic error.
    pub fn is_domain_error(&self) -> bool {
        matches!(self, FileKVError::Domain(_))
    }

    /// Map this error to a high-level category.
    pub fn category(&self) -> ErrorCategory {
        match self {
            FileKVError::Fatal(FatalError::Io(_)) => ErrorCategory::Io,
            FileKVError::Fatal(FatalError::Corruption(_) | FatalError::WalCorrupted(_)) => ErrorCategory::Corruption,
            FileKVError::Transient(TransientError::ResourceExhausted(_)) => ErrorCategory::Resource,
            FileKVError::Transient(TransientError::Timeout(_)) => ErrorCategory::Timeout,
            FileKVError::Domain(DomainError::Config(_)) => ErrorCategory::Config,
            _ => ErrorCategory::Other,
        }
    }
}

impl FatalError {
    /// Fatal errors are never retryable.
    pub fn is_retryable(&self) -> bool {
        false
    }
}

impl TransientError {
    /// Transient errors are always retryable.
    pub fn is_retryable(&self) -> bool {
        true
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- FatalError tests ----

    #[test]
    fn fatal_error_is_not_retryable() {
        let corruption = FatalError::Corruption("bad checksum".to_string());
        assert!(!corruption.is_retryable());

        let io_err = FatalError::Io(std::io::Error::other("disk full"));
        assert!(!io_err.is_retryable());

        let wal_corrupted = FatalError::WalCorrupted("truncated entry".to_string());
        assert!(!wal_corrupted.is_retryable());
    }

    #[test]
    fn fatal_error_is_fatal() {
        let corruption = FatalError::Corruption("bad checksum".to_string());
        let filekv_err = FileKVError::Fatal(corruption);
        assert!(filekv_err.is_fatal());
    }

    // ---- TransientError tests ----

    #[test]
    fn transient_error_is_retryable() {
        let exhausted = TransientError::ResourceExhausted("out of memory".to_string());
        assert!(exhausted.is_retryable());

        let timeout = TransientError::Timeout(std::time::Duration::from_secs(30));
        assert!(timeout.is_retryable());

        let backpressure = TransientError::Backpressure("memtable full".to_string());
        assert!(backpressure.is_retryable());
    }

    #[test]
    fn transient_error_is_not_fatal() {
        let backpressure = TransientError::Backpressure("memtable full".to_string());
        let filekv_err = FileKVError::Transient(backpressure);
        assert!(!filekv_err.is_fatal());
        assert!(filekv_err.is_retryable());
    }

    // ---- ExpectedError tests ----

    #[test]
    fn expected_error_is_not_real_error() {
        let not_found = ExpectedError::KeyNotFound("my_key".to_string());
        let filekv_err = FileKVError::Expected(not_found);
        assert!(filekv_err.is_expected());
        assert!(!filekv_err.is_fatal());
        assert!(!filekv_err.is_retryable());
    }

    #[test]
    fn expected_error_display_format() {
        let not_found = ExpectedError::KeyNotFound("abc".to_string());
        assert_eq!(format!("{}", not_found), "Key not found: abc");

        let seg_not_found = ExpectedError::SegmentNotFound(42);
        assert_eq!(format!("{}", seg_not_found), "Segment not found: 42");

        let bloom_negative = ExpectedError::BloomNegative(7);
        assert_eq!(
            format!("{}", bloom_negative),
            "Bloom filter negative: key not in segment 7"
        );
    }

    // ---- DomainError tests ----

    #[test]
    fn domain_error_is_not_retryable() {
        let config_err = DomainError::Config("invalid segment size".to_string());
        let filekv_err = FileKVError::Domain(config_err);
        assert!(!filekv_err.is_retryable());
        assert!(filekv_err.is_domain_error());
    }

    // ---- FileKVError classification tests ----

    #[test]
    fn filekv_error_category_mapping() {
        let io_err = FileKVError::Fatal(FatalError::Io(std::io::Error::other("disk full")));
        assert_eq!(io_err.category(), ErrorCategory::Io);

        let corruption = FileKVError::Fatal(FatalError::Corruption("bad data".to_string()));
        assert_eq!(corruption.category(), ErrorCategory::Corruption);

        let exhausted = FileKVError::Transient(TransientError::ResourceExhausted("oom".to_string()));
        assert_eq!(exhausted.category(), ErrorCategory::Resource);

        let timeout = FileKVError::Transient(TransientError::Timeout(std::time::Duration::from_secs(5)));
        assert_eq!(timeout.category(), ErrorCategory::Timeout);

        let config = FileKVError::Domain(DomainError::Config("bad config".to_string()));
        assert_eq!(config.category(), ErrorCategory::Config);
    }

    // ---- From implementations for FileKVError ----

    #[test]
    fn io_error_converts_to_fatal_filekv_error() {
        let io_err = std::io::Error::other("test");
        let fatal = FatalError::Io(io_err);
        let filekv_err = FileKVError::from(fatal);
        assert!(filekv_err.is_fatal());
    }

    #[test]
    fn fatal_error_converts_to_filekv_error() {
        let fatal = FatalError::Corruption("test".to_string());
        let filekv_err = FileKVError::from(fatal);
        assert!(matches!(filekv_err, FileKVError::Fatal(_)));
    }

    #[test]
    fn transient_error_converts_to_filekv_error() {
        let transient = TransientError::Backpressure("test".to_string());
        let filekv_err = FileKVError::from(transient);
        assert!(matches!(filekv_err, FileKVError::Transient(_)));
    }

    #[test]
    fn expected_error_converts_to_filekv_error() {
        let expected = ExpectedError::KeyNotFound("key".to_string());
        let filekv_err = FileKVError::from(expected);
        assert!(matches!(filekv_err, FileKVError::Expected(_)));
    }

    #[test]
    fn domain_error_converts_to_filekv_error() {
        let domain = DomainError::Config("test".to_string());
        let filekv_err = FileKVError::from(domain);
        assert!(matches!(filekv_err, FileKVError::Domain(_)));
    }
}
