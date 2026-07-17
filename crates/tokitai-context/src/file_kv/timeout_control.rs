//! Operation Timeout Control for FileKV
//!
//! This module provides configurable timeout controls for all FileKV operations
//! to prevent indefinite blocking on I/O operations.
//!
//! # Features
//! - Per-operation timeout configuration
//! - Global default timeout settings
//! - Timeout retry logic with exponential backoff
//! - Timeout statistics and monitoring

use std::time::Duration;
use tracing::{warn, debug};
use crate::error::{ContextResult, ContextError};

/// Default timeout values in milliseconds
pub const DEFAULT_READ_TIMEOUT_MS: u64 = 5000;
pub const DEFAULT_WRITE_TIMEOUT_MS: u64 = 10000;
pub const DEFAULT_DELETE_TIMEOUT_MS: u64 = 10000;
pub const DEFAULT_COMPACTION_TIMEOUT_MS: u64 = 300000; // 5 minutes
pub const DEFAULT_FLUSH_TIMEOUT_MS: u64 = 60000; // 1 minute
pub const DEFAULT_CHECKPOINT_TIMEOUT_MS: u64 = 120000; // 2 minutes

/// Maximum retry attempts for timed-out operations
pub const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Exponential backoff base in milliseconds
pub const BACKOFF_BASE_MS: u64 = 100;

/// Timeout configuration for FileKV operations
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for read operations (get, retrieve)
    pub read_timeout_ms: u64,
    /// Timeout for write operations (put, store)
    pub write_timeout_ms: u64,
    /// Timeout for delete operations
    pub delete_timeout_ms: u64,
    /// Timeout for compaction operations
    pub compaction_timeout_ms: u64,
    /// Timeout for flush operations
    pub flush_timeout_ms: u64,
    /// Timeout for checkpoint operations
    pub checkpoint_timeout_ms: u64,
    /// Enable automatic retry on timeout
    pub enable_retry: bool,
    /// Maximum retry attempts (default: 3)
    pub max_retry_attempts: u32,
    /// Enable exponential backoff on retry
    pub enable_backoff: bool,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            read_timeout_ms: DEFAULT_READ_TIMEOUT_MS,
            write_timeout_ms: DEFAULT_WRITE_TIMEOUT_MS,
            delete_timeout_ms: DEFAULT_DELETE_TIMEOUT_MS,
            compaction_timeout_ms: DEFAULT_COMPACTION_TIMEOUT_MS,
            flush_timeout_ms: DEFAULT_FLUSH_TIMEOUT_MS,
            checkpoint_timeout_ms: DEFAULT_CHECKPOINT_TIMEOUT_MS,
            enable_retry: true,
            max_retry_attempts: MAX_RETRY_ATTEMPTS,
            enable_backoff: true,
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new timeout config with custom read timeout
    pub fn with_read_timeout(mut self, timeout_ms: u64) -> Self {
        self.read_timeout_ms = timeout_ms;
        self
    }

    /// Create a new timeout config with custom write timeout
    pub fn with_write_timeout(mut self, timeout_ms: u64) -> Self {
        self.write_timeout_ms = timeout_ms;
        self
    }

    /// Create a new timeout config with custom delete timeout
    pub fn with_delete_timeout(mut self, timeout_ms: u64) -> Self {
        self.delete_timeout_ms = timeout_ms;
        self
    }

    /// Get timeout for a specific operation type
    pub fn get_timeout(&self, op: OperationType) -> Duration {
        let ms = match op {
            OperationType::Read => self.read_timeout_ms,
            OperationType::Write => self.write_timeout_ms,
            OperationType::Delete => self.delete_timeout_ms,
            OperationType::Compaction => self.compaction_timeout_ms,
            OperationType::Flush => self.flush_timeout_ms,
            OperationType::Checkpoint => self.checkpoint_timeout_ms,
        };
        Duration::from_millis(ms)
    }

    /// Calculate backoff duration for a retry attempt
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        if !self.enable_backoff {
            return Duration::from_millis(0);
        }

        // Exponential backoff: base * 2^attempt
        let backoff_ms = BACKOFF_BASE_MS * (1 << attempt.min(10)); // Cap at 2^10
        Duration::from_millis(backoff_ms)
    }
}

/// Types of operations that can have timeouts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Read,
    Write,
    Delete,
    Compaction,
    Flush,
    Checkpoint,
}

/// Statistics for timeout operations
#[derive(Debug, Default, Clone)]
pub struct TimeoutStats {
    /// Total number of timeout events
    pub timeout_count: u64,
    /// Total number of retry attempts
    pub retry_count: u64,
    /// Number of successful retries
    pub successful_retries: u64,
    /// Number of failed retries (all attempts exhausted)
    pub failed_retries: u64,
    /// Total time spent in retries (microseconds)
    pub total_retry_time_us: u64,
}

impl TimeoutStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a timeout event
    pub fn record_timeout(&mut self) {
        self.timeout_count += 1;
    }

    /// Record a retry attempt
    pub fn record_retry(&mut self, success: bool, duration_us: u64) {
        self.retry_count += 1;
        self.total_retry_time_us += duration_us;
        if success {
            self.successful_retries += 1;
        } else {
            self.failed_retries += 1;
        }
    }
}

/// Execute an operation with timeout and optional retry
///
/// # Arguments
/// * `op` - The operation type (for timeout selection)
/// * `config` - Timeout configuration
/// * `stats` - Optional stats tracker
/// * `f` - The operation to execute (receives timeout duration)
///
/// # Returns
/// Result of the operation or timeout error
pub fn execute_with_timeout<T, F>(
    op: OperationType,
    config: &TimeoutConfig,
    mut stats: Option<&mut TimeoutStats>,
    mut f: F,
) -> ContextResult<T>
where
    F: FnMut(Duration) -> ContextResult<T>,
{
    let timeout = config.get_timeout(op);
    let mut attempts = 0u32;

    loop {
        attempts += 1;

        // Execute the operation with the specified timeout
        let result = f(timeout);

        match result {
            Ok(value) => {
                if attempts > 1 {
                    if let Some(ref mut stats) = stats {
                        stats.record_retry(true, 0);
                    }
                }
                return Ok(value);
            }
            Err(e) => {
                if is_timeout_error(&e) {
                    if let Some(ref mut stats) = stats {
                        stats.record_timeout();
                    }

                    // Check if we should retry
                    if config.enable_retry && attempts < config.max_retry_attempts {
                        if let Some(ref mut stats) = stats {
                            stats.record_retry(false, 0);
                        }

                        // Apply backoff before retry
                        if config.enable_backoff {
                            let backoff = config.calculate_backoff(attempts);
                            debug!(
                                "Operation {:?} timed out, retrying in {:?} (attempt {}/{})",
                                op, backoff, attempts + 1, config.max_retry_attempts
                            );
                            std::thread::sleep(backoff);
                        } else {
                            debug!(
                                "Operation {:?} timed out, retrying (attempt {}/{})",
                                op, attempts + 1, config.max_retry_attempts
                            );
                        }
                        continue;
                    }
                }

                // No retry or not a timeout error
                if attempts > 1 {
                    if let Some(ref mut stats) = stats {
                        stats.record_retry(false, 0);
                    }
                }

                return Err(e);
            }
        }
    }
}

/// Check if an error is a timeout error
fn is_timeout_error(err: &ContextError) -> bool {
    match err {
        ContextError::OperationFailed(msg) => {
            msg.to_lowercase().contains("timeout")
        }
        ContextError::Io(io_err) => {
            io_err.kind() == std::io::ErrorKind::TimedOut
        }
        _ => false,
    }
}

/// Helper macro to wrap operations with timeout
#[macro_export]
macro_rules! with_timeout {
    ($op:expr, $config:expr, $stats:expr, $op_type:expr) => {
        $crate::file_kv::timeout_control::execute_with_timeout(
            $op_type,
            &$config,
            $stats.as_ref().map(|s| unsafe { &mut *(s as *const _ as *mut _) }),
            |timeout| {
                // Pass timeout to operation if it accepts it
                $op
            }
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.read_timeout_ms, DEFAULT_READ_TIMEOUT_MS);
        assert_eq!(config.write_timeout_ms, DEFAULT_WRITE_TIMEOUT_MS);
        assert!(config.enable_retry);
        assert!(config.enable_backoff);
    }

    #[test]
    fn test_timeout_config_builder() {
        let config = TimeoutConfig::new()
            .with_read_timeout(1000)
            .with_write_timeout(2000)
            .with_delete_timeout(3000);

        assert_eq!(config.read_timeout_ms, 1000);
        assert_eq!(config.write_timeout_ms, 2000);
        assert_eq!(config.delete_timeout_ms, 3000);
    }

    #[test]
    fn test_get_timeout() {
        let config = TimeoutConfig::default();
        
        assert_eq!(config.get_timeout(OperationType::Read), Duration::from_millis(DEFAULT_READ_TIMEOUT_MS));
        assert_eq!(config.get_timeout(OperationType::Write), Duration::from_millis(DEFAULT_WRITE_TIMEOUT_MS));
        assert_eq!(config.get_timeout(OperationType::Compaction), Duration::from_millis(DEFAULT_COMPACTION_TIMEOUT_MS));
    }

    #[test]
    fn test_calculate_backoff() {
        let config = TimeoutConfig::default();
        
        // Without backoff enabled
        let config_no_backoff = TimeoutConfig {
            enable_backoff: false,
            ..Default::default()
        };
        assert_eq!(config_no_backoff.calculate_backoff(0), Duration::from_millis(0));
        
        // With backoff enabled - exponential growth
        assert_eq!(config.calculate_backoff(0), Duration::from_millis(BACKOFF_BASE_MS));
        assert_eq!(config.calculate_backoff(1), Duration::from_millis(BACKOFF_BASE_MS * 2));
        assert_eq!(config.calculate_backoff(2), Duration::from_millis(BACKOFF_BASE_MS * 4));
    }

    #[test]
    fn test_timeout_stats() {
        let mut stats = TimeoutStats::new();
        
        stats.record_timeout();
        stats.record_timeout();
        stats.record_retry(true, 100);
        stats.record_retry(false, 200);
        
        assert_eq!(stats.timeout_count, 2);
        assert_eq!(stats.retry_count, 2);
        assert_eq!(stats.successful_retries, 1);
        assert_eq!(stats.failed_retries, 1);
        assert_eq!(stats.total_retry_time_us, 300);
    }

    #[test]
    fn test_execute_with_timeout_success() {
        let config = TimeoutConfig::default();
        let mut stats = TimeoutStats::new();
        
        let result = execute_with_timeout(
            OperationType::Read,
            &config,
            Some(&mut stats),
            |_timeout| {
                Ok("success".to_string())
            }
        );
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(stats.timeout_count, 0);
        assert_eq!(stats.retry_count, 0);
    }

    #[test]
    fn test_execute_with_timeout_error() {
        let config = TimeoutConfig {
            enable_retry: false, // Disable retry for this test
            ..Default::default()
        };
        
        let result = execute_with_timeout::<(), _>(
            OperationType::Read,
            &config,
            None,
            |_timeout| {
                Err(ContextError::OperationFailed("test error".to_string()))
            }
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_is_timeout_error() {
        let timeout_err = ContextError::OperationFailed("operation timeout".to_string());
        assert!(is_timeout_error(&timeout_err));
        
        let io_timeout = ContextError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "timed out"
        ));
        assert!(is_timeout_error(&io_timeout));
        
        let other_err = ContextError::OperationFailed("some other error".to_string());
        assert!(!is_timeout_error(&other_err));
    }
}
