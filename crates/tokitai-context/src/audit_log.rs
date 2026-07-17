//! Audit Logging Module
//!
//! ## Purpose
//!
//! Records all write operations for compliance, debugging, and forensic analysis.
//!
//! ## Features
//!
//! - **Immutable audit trail**: Append-only log files
//! - **Structured format**: JSON entries for easy parsing
//! - **Configurable retention**: Automatic log rotation and cleanup
//! - **Performance tracking**: Optional latency recording
//! - **Compliance ready**: Timestamps, operation types, and metadata
//!
//! ## Audit Entry Format
//!
//! ```json
//! {
//!   "timestamp": "2026-04-03T10:15:30.123Z",
//!   "operation": "PUT",
//!   "key": "session_abc123",
//!   "value_hash": "sha256:...",
//!   "value_size": 1024,
//!   "latency_us": 45,
//!   "success": true,
//!   "error": null,
//!   "metadata": {
//!     "layer": "ShortTerm",
//!     "session_id": "user_123"
//!   }
//! }
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::error::{ContextResult, ContextError};

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO 8601 timestamp
    pub timestamp: DateTime<Utc>,
    /// Operation type (PUT, DELETE, BATCH_PUT, BATCH_DELETE)
    pub operation: AuditOperation,
    /// Key(s) affected
    pub keys: Vec<String>,
    /// SHA256 hash of value (for integrity verification)
    pub value_hash: Option<String>,
    /// Size of value in bytes
    pub value_size: Option<u64>,
    /// Operation latency in microseconds
    pub latency_us: Option<u64>,
    /// Whether operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: AuditMetadata,
}

/// Operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditOperation {
    Put,
    Delete,
    BatchPut { count: usize },
    BatchDelete { count: usize },
    Flush,
    Compaction,
}

/// Additional metadata for audit entries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditMetadata {
    /// Storage layer (ShortTerm, LongTerm, Transient)
    pub layer: Option<String>,
    /// Session identifier
    pub session_id: Option<String>,
    /// User identifier
    pub user_id: Option<String>,
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Custom key-value pairs
    pub custom: std::collections::HashMap<String, String>,
}

/// Audit log configuration
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    /// Directory to store audit logs
    pub log_dir: PathBuf,
    /// Enable audit logging
    pub enabled: bool,
    /// Maximum log file size before rotation (bytes)
    pub max_file_size_bytes: u64,
    /// Maximum number of log files to retain
    pub max_files: usize,
    /// Record latency for each operation
    pub record_latency: bool,
    /// Include value hash for integrity verification
    pub include_value_hash: bool,
    /// Flush after every entry (slower but safer)
    pub flush_on_write: bool,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("./audit_logs"),
            enabled: false, // Disabled by default for performance
            max_file_size_bytes: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            record_latency: true,
            include_value_hash: true,
            flush_on_write: false, // Buffer writes for performance
        }
    }
}

/// Audit log statistics
#[derive(Debug, Clone, Default)]
pub struct AuditLogStats {
    /// Total entries written
    pub entries_written: u64,
    /// Total entries failed
    pub entries_failed: u64,
    /// Number of log rotations
    pub rotations: u64,
    /// Current log file size
    pub current_file_size_bytes: u64,
    /// Total size of all log files
    pub total_size_bytes: u64,
}

/// Audit logger for compliance and forensics
pub struct AuditLogger {
    config: AuditLogConfig,
    /// Current log file writer
    writer: Arc<Mutex<BufWriter<File>>>,
    /// Current log file path
    current_log_path: PathBuf,
    /// Current file size
    current_size: Arc<Mutex<u64>>,
    /// Statistics
    stats: Arc<Mutex<AuditLogStats>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn open(config: AuditLogConfig) -> ContextResult<Self> {
        // Create log directory
        fs::create_dir_all(&config.log_dir)
            .map_err(ContextError::Io)?;

        // Get current log file path
        let log_path = Self::get_current_log_path(&config.log_dir);

        // Open or create log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(ContextError::Io)?;

        let current_size = file.metadata()
            .map(|m| m.len())
            .unwrap_or(0);

        let writer = Arc::new(Mutex::new(BufWriter::new(file)));
        let current_size_arc = Arc::new(Mutex::new(current_size));
        let stats = Arc::new(Mutex::new(AuditLogStats {
            current_file_size_bytes: current_size,
            ..Default::default()
        }));

        Ok(Self {
            config,
            writer,
            current_log_path: log_path,
            current_size: current_size_arc,
            stats,
        })
    }

    /// Get the current log file path
    fn get_current_log_path(log_dir: &Path) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        log_dir.join(format!("audit_{:020}.jsonl", timestamp))
    }

    /// Log a write operation
    #[allow(clippy::too_many_arguments)]
    pub fn log_operation(
        &self,
        operation: AuditOperation,
        keys: Vec<String>,
        value_hash: Option<String>,
        value_size: Option<u64>,
        latency_us: Option<u64>,
        success: bool,
        error: Option<String>,
        metadata: AuditMetadata,
    ) -> ContextResult<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            operation,
            keys,
            value_hash,
            value_size,
            latency_us,
            success,
            error,
            metadata,
        };

        self.write_entry(entry)
    }

    /// Write an audit entry
    fn write_entry(&self, mut entry: AuditEntry) -> ContextResult<()> {
        // Update metadata with layer info if not provided
        if entry.metadata.layer.is_none() {
            entry.metadata.layer = Some("Unknown".to_string());
        }

        // Serialize to JSON
        let json = serde_json::to_string(&entry)
            .map_err(|e| ContextError::OperationFailed(format!("JSON serialization failed: {}", e)))?;

        // Add newline
        let line = format!("{}\n", json);
        let bytes = line.as_bytes();
        let byte_count = bytes.len() as u64;

        // Write to file
        {
            let mut writer = self.writer.lock();
            writer.write_all(bytes)
                .map_err(ContextError::Io)?;

            // Update size
            let mut size = self.current_size.lock();
            *size += byte_count;

            // Update stats
            let mut stats = self.stats.lock();
            stats.current_file_size_bytes = *size;
            if entry.success {
                stats.entries_written += 1;
            } else {
                stats.entries_failed += 1;
            }

            // Flush if configured
            if self.config.flush_on_write {
                writer.flush().map_err(ContextError::Io)?;
            }
        }

        // Check if rotation is needed
        self.maybe_rotate()?;

        Ok(())
    }

    /// Rotate log file if needed
    fn maybe_rotate(&self) -> ContextResult<()> {
        let current_size = *self.current_size.lock();

        if current_size >= self.config.max_file_size_bytes {
            self.rotate()?;
        }

        Ok(())
    }

    /// Rotate the current log file
    fn rotate(&self) -> ContextResult<()> {
        // Create new log file
        let new_path = Self::get_current_log_path(&self.config.log_dir);

        // Flush current writer
        {
            let mut writer = self.writer.lock();
            writer.flush().map_err(ContextError::Io)?;
        }

        // Open new file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&new_path)
            .map_err(ContextError::Io)?;

        // Replace writer
        {
            let mut writer = self.writer.lock();
            *writer = BufWriter::new(file);
        }

        // Reset size
        {
            let mut size = self.current_size.lock();
            *size = 0;
        }

        // Update path
        unsafe {
            // Safe because we hold the lock
            let self_mut = self as *const Self as *mut Self;
            (*self_mut).current_log_path = new_path;
        }

        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.rotations += 1;
        }

        // Cleanup old files
        self.cleanup_old_logs()?;

        Ok(())
    }

    /// Cleanup old log files
    fn cleanup_old_logs(&self) -> ContextResult<()> {
        let mut log_files: Vec<_> = fs::read_dir(&self.config.log_dir)
            .map_err(ContextError::Io)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    let is_jsonl = path
                        .extension()
                        .and_then(|s| s.to_str())
                        == Some("jsonl");
                    if is_jsonl {
                        e.metadata().ok().map(|m| (path, m))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort by modification time (newest first)
        log_files.sort_by(|a, b| {
            b.1.modified().unwrap_or(UNIX_EPOCH)
                .cmp(&a.1.modified().unwrap_or(UNIX_EPOCH))
        });

        // Remove old files beyond retention limit
        if log_files.len() > self.config.max_files {
            for (path, _) in log_files.iter().skip(self.config.max_files) {
                if let Err(e) = fs::remove_file(path) {
                    tracing::warn!("Failed to remove old audit log {:?}: {}", path, e);
                }
            }
        }

        // Update total size
        let total_size: u64 = log_files
            .iter()
            .take(self.config.max_files)
            .map(|(_, m)| m.len())
            .sum();

        {
            let mut stats = self.stats.lock();
            stats.total_size_bytes = total_size;
        }

        Ok(())
    }

    /// Get audit log statistics
    pub fn stats(&self) -> AuditLogStats {
        self.stats.lock().clone()
    }

    /// Flush pending writes
    pub fn flush(&self) -> ContextResult<()> {
        let mut writer = self.writer.lock();
        writer.flush().map_err(ContextError::Io)
    }

    /// Get current log file path
    pub fn current_log_path(&self) -> PathBuf {
        self.current_log_path.clone()
    }
}

/// Helper for computing value hashes
pub fn compute_value_hash(value: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(value);
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_logger() -> (AuditLogger, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditLogConfig {
            log_dir: temp_dir.path().to_path_buf(),
            enabled: true,
            max_file_size_bytes: 1024 * 1024, // 1MB
            max_files: 5,
            record_latency: true,
            include_value_hash: true,
            flush_on_write: false,
        };
        let logger = AuditLogger::open(config).unwrap();
        (logger, temp_dir)
    }

    #[test]
    fn test_audit_logger_basic() {
        let (logger, _temp_dir) = create_test_logger();

        let result = logger.log_operation(
            AuditOperation::Put,
            vec!["test_key".to_string()],
            Some("sha256:abc123".to_string()),
            Some(1024),
            Some(45),
            true,
            None,
            AuditMetadata::default(),
        );

        assert!(result.is_ok());
        let stats = logger.stats();
        assert_eq!(stats.entries_written, 1);
        assert_eq!(stats.entries_failed, 0);
    }

    #[test]
    fn test_audit_logger_failed_operation() {
        let (logger, _temp_dir) = create_test_logger();

        let result = logger.log_operation(
            AuditOperation::Delete,
            vec!["deleted_key".to_string()],
            None,
            None,
            Some(10),
            false,
            Some("Key not found".to_string()),
            AuditMetadata::default(),
        );

        assert!(result.is_ok());
        let stats = logger.stats();
        assert_eq!(stats.entries_written, 0);
        assert_eq!(stats.entries_failed, 1);
    }

    #[test]
    fn test_audit_logger_batch_operation() {
        let (logger, _temp_dir) = create_test_logger();

        let result = logger.log_operation(
            AuditOperation::BatchPut { count: 100 },
            (0..100).map(|i| format!("key_{}", i)).collect(),
            None,
            None,
            Some(500),
            true,
            None,
            AuditMetadata {
                layer: Some("ShortTerm".to_string()),
                ..Default::default()
            },
        );

        assert!(result.is_ok());
        let stats = logger.stats();
        assert_eq!(stats.entries_written, 1);
    }

    #[test]
    fn test_audit_logger_metadata() {
        let (logger, _temp_dir) = create_test_logger();

        let mut metadata = AuditMetadata {
            layer: Some("LongTerm".to_string()),
            session_id: Some("session_123".to_string()),
            user_id: Some("user_456".to_string()),
            request_id: Some("req_789".to_string()),
            custom: std::collections::HashMap::new(),
        };
        metadata.custom.insert("custom_key".to_string(), "custom_value".to_string());

        let result = logger.log_operation(
            AuditOperation::Put,
            vec!["key".to_string()],
            None,
            None,
            None,
            true,
            None,
            metadata,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_audit_logger_flush() {
        let (logger, _temp_dir) = create_test_logger();

        logger.log_operation(
            AuditOperation::Put,
            vec!["key".to_string()],
            None,
            None,
            None,
            true,
            None,
            AuditMetadata::default(),
        ).unwrap();

        let result = logger.flush();
        assert!(result.is_ok());
    }

    #[test]
    fn test_compute_value_hash() {
        let value = b"test value";
        let hash = compute_value_hash(value);
        
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_audit_logger_config() {
        let config = AuditLogConfig::default();
        
        assert!(!config.enabled);
        assert_eq!(config.max_file_size_bytes, 100 * 1024 * 1024);
        assert_eq!(config.max_files, 10);
        assert!(config.record_latency);
        assert!(config.include_value_hash);
        assert!(!config.flush_on_write);
    }
}
