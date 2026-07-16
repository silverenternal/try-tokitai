//! Audit Logging Module
//!
//! Records all write operations for compliance and debugging.

use crate::core::error::{FatalError, FileKVError};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

fn io_to_filekv(e: std::io::Error) -> FileKVError {
    FileKVError::Fatal(FatalError::Io(e))
}

fn msg_to_filekv(msg: String) -> FileKVError {
    FileKVError::Fatal(FatalError::Corruption(msg))
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub operation: AuditOperation,
    pub keys: Vec<String>,
    pub value_hash: Option<String>,
    pub value_size: Option<u64>,
    pub latency_us: Option<u64>,
    pub success: bool,
    pub error: Option<String>,
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

/// Additional metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditMetadata {
    pub layer: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub request_id: Option<String>,
    pub custom: std::collections::HashMap<String, String>,
}

/// Audit log configuration
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    pub log_dir: PathBuf,
    pub enabled: bool,
    pub rotation_interval_hours: u64,
    pub retention_days: u32,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("audit_logs"),
            enabled: false,
            rotation_interval_hours: 24,
            retention_days: 30,
        }
    }
}

/// Audit log statistics
#[derive(Debug, Clone, Default)]
pub struct AuditLogStats {
    pub entries_written: u64,
    pub errors: u64,
}

/// Audit logger
pub struct AuditLogger {
    config: AuditLogConfig,
    log_file: Mutex<Option<std::fs::File>>,
    current_log_path: Mutex<Option<PathBuf>>,
    stats: Arc<Mutex<AuditLogStats>>,
}

impl AuditLogger {
    pub fn open(config: AuditLogConfig) -> crate::core::error::FileKVResult<Self> {
        std::fs::create_dir_all(&config.log_dir).map_err(io_to_filekv)?;

        let logger = Self {
            config,
            log_file: Mutex::new(None),
            current_log_path: Mutex::new(None),
            stats: Arc::default(),
        };

        // Initialize the log file if auditing is enabled
        logger.open_log_file()?;

        Ok(logger)
    }

    /// Open a new log file with timestamp-based rotation
    fn open_log_file(&self) -> crate::core::error::FileKVResult<()> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let log_path = self.config.log_dir.join(format!("audit_{}.log", timestamp));

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(io_to_filekv)?;

        let mut log_file = self.log_file.lock();
        *log_file = Some(file);

        let mut current_path = self.current_log_path.lock();
        *current_path = Some(log_path);

        Ok(())
    }

    /// Check if log file needs rotation based on rotation_interval_hours
    fn should_rotate(&self) -> bool {
        if let Some(ref log_path) = *self.current_log_path.lock() {
            if let Ok(metadata) = std::fs::metadata(log_path) {
                if let Ok(created) = metadata.created() {
                    let elapsed = Utc::now().signed_duration_since(DateTime::<Utc>::from(created));
                    let hours = self.config.rotation_interval_hours as i64;
                    return elapsed.num_hours() >= hours;
                }
            }
        }
        false
    }

    /// Rotate log file if needed
    fn rotate_if_needed(&self) -> crate::core::error::FileKVResult<()> {
        if self.should_rotate() {
            self.open_log_file()?;
        }
        Ok(())
    }

    /// Log an audit operation
    ///
    /// # Arguments
    /// * `operation` - The type of operation being logged
    /// * `keys` - List of keys affected by the operation
    /// * `value_hash` - Optional hash of the value
    /// * `value_size` - Optional size of the value in bytes
    /// * `latency_us` - Optional operation latency in microseconds
    /// * `success` - Whether the operation succeeded
    /// * `error` - Optional error message if failed
    /// * `metadata` - Additional metadata
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
    ) -> crate::core::error::FileKVResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

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

        let json = serde_json::to_string(&entry)
            .map_err(|e| io_to_filekv(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

        // Check if we need to rotate the log file
        self.rotate_if_needed()?;

        // Append to log file
        let mut log_file = self.log_file.lock();
        if let Some(ref mut file) = *log_file {
            use std::io::Write;
            writeln!(file, "{}", json).map_err(io_to_filekv)?;
            file.flush().map_err(io_to_filekv)?;
        } else {
            // This should not happen if open() was called successfully
            let mut stats = self.stats.lock();
            stats.errors += 1;
            return Err(msg_to_filekv("Audit log file not opened".to_string()));
        }

        self.stats.lock().entries_written += 1;
        Ok(())
    }

    pub fn stats(&self) -> AuditLogStats {
        self.stats.lock().clone()
    }
}

/// Compute value hash for audit
pub fn compute_value_hash(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
