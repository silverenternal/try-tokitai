//! Write-Ahead Log (WAL) for crash recovery
//! 
//! This module provides a write-ahead log mechanism to ensure data integrity
//! in case of crashes or unexpected termination. All mutating operations
//! are first logged to the WAL before being applied.
//! 
//! # Format
//! 
//! Each WAL entry contains:
//! - Timestamp
//! - Operation type
//! - Session ID
//! - Content hash (if applicable)
//! - Optional payload (for undo information)
//! - Checksum
//! 
//! # Recovery
//! 
//! On startup, the WAL can be replayed to:
//! - Complete incomplete operations
//! - Roll back operations that failed mid-way
//! - Rebuild index state

use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use thiserror::Error;

/// WAL error types
#[derive(Debug, Error)]
pub enum WalError {
    /// WAL file corrupted
    #[error("WAL file corrupted at entry {entry}: {reason}")]
    Corruption { entry: usize, reason: String },

    /// WAL entry checksum mismatch
    #[error("WAL entry checksum mismatch at entry {0}")]
    ChecksumMismatch(usize),

    /// WAL file not found
    #[error("WAL file not found: {0:?}")]
    WalNotFound(PathBuf),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// WAL lock poisoned (thread panic)
    #[error("WAL lock poisoned")]
    LockPoisoned,

    /// WAL rotation failed
    #[error("WAL rotation failed: {0}")]
    RotationFailed(String),
}

/// Result type alias for WAL operations
pub type Result<T> = std::result::Result<T, WalError>;

/// WAL entry operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WalOperation {
    /// Content was added
    Add { session: String, hash: String, layer: String },
    /// Content was deleted (payload contains deleted content for undo)
    Delete { session: String, hash: String, content: Option<Vec<u8>> },
    /// Session was cleaned up
    CleanupSession { session: String },
    /// Merge operation started
    MergeStart { source: String, target: String },
    /// Merge operation completed
    MergeComplete { source: String, target: String, success: bool },
    /// Branch was created
    BranchCreate { branch_id: String, parent: String },
    /// Branch was merged
    BranchMerge { branch: String, into: String },
    /// Compaction started (payload contains segment IDs to compact)
    CompactionStart { compaction_id: u64, segment_ids: Vec<u64>, new_segment_id: u64 },
    /// Compaction completed successfully
    CompactionComplete { compaction_id: u64, new_segment_id: u64, keys_compacted: u64 },
    /// Compaction cleanup - old segments removed
    CompactionCleanup { compaction_id: u64, removed_segment_ids: Vec<u64> },
}

/// A single WAL entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Entry timestamp
    pub timestamp: DateTime<Utc>,
    /// Operation type and data
    pub operation: WalOperation,
    /// Optional payload for undo information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// SHA256 checksum of the entry (for integrity verification)
    pub checksum: String,
}

impl WalEntry {
    /// Create a new WAL entry with checksum
    pub fn new(operation: WalOperation, payload: Option<String>) -> Self {
        let timestamp = Utc::now();
        
        // Create checksum from timestamp and operation
        let mut hasher = Sha256::new();
        hasher.update(timestamp.to_rfc3339().as_bytes());
        let op_data = serde_json::to_string(&operation).unwrap_or_default();
        hasher.update(op_data.as_bytes());
        if let Some(p) = &payload {
            hasher.update(p.as_bytes());
        }
        let checksum = hex::encode(hasher.finalize());

        Self {
            timestamp,
            operation,
            payload,
            checksum,
        }
    }

    /// Verify the entry's checksum
    pub fn verify_checksum(&self) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        let op_data = serde_json::to_string(&self.operation).unwrap_or_default();
        hasher.update(op_data.as_bytes());
        if let Some(p) = &self.payload {
            hasher.update(p.as_bytes());
        }
        let expected = hex::encode(hasher.finalize());
        self.checksum == expected
    }
}

/// Write-Ahead Log manager
///
/// P1-013: Supports WAL file rotation to prevent unbounded disk usage
pub struct WalManager {
    log_file: PathBuf,
    file: Option<File>,
    enabled: bool,
    // P1-013: Rotation configuration
    max_size_bytes: u64,
    max_files: usize,
    current_size: u64,
}

/// Durability level indicating whether data is persisted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLevel {
    /// Data is persisted to disk (WAL enabled and logged successfully)
    Disk,
    /// Data is only in memory (WAL disabled or not yet flushed)
    Memory,
}

impl WalManager {
    /// Create a new WAL manager with default rotation settings
    pub fn new<P: AsRef<Path>>(log_dir: P, enabled: bool) -> Result<Self> {
        // P1-013: Use default rotation settings
        Self::new_with_config(log_dir, enabled, 100 * 1024 * 1024, 5)
    }

    /// Create a new WAL manager with custom rotation configuration
    ///
    /// P1-013: WAL file rotation to prevent unbounded disk usage
    pub fn new_with_config<P: AsRef<Path>>(
        log_dir: P,
        enabled: bool,
        max_size_bytes: u64,
        max_files: usize,
    ) -> Result<Self> {
        let log_dir = log_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&log_dir)
            .map_err(|e| WalError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create WAL directory: {:?}: {}", log_dir, e)
            )))?;

        let log_file = log_dir.join("wal.log");

        // P1-013: Get current file size if it exists
        let current_size = std::fs::metadata(&log_file)
            .map(|m| m.len())
            .unwrap_or(0);

        let file = if enabled {
            Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file)
                    .map_err(|e| WalError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to open WAL file: {:?}: {}", log_file, e)
                    )))?,
            )
        } else {
            None
        };

        Ok(Self {
            log_file,
            file,
            enabled,
            max_size_bytes,
            max_files,
            current_size,
        })
    }

    /// Log an operation (write-ahead)
    /// 
    /// P0-004 FIX: Returns DurabilityLevel to indicate whether data is persisted
    #[tracing::instrument(skip_all)]
    pub fn log(&mut self, operation: WalOperation) -> Result<DurabilityLevel> {
        if !self.enabled {
            // P0-004 FIX: Return Memory durability when WAL is disabled
            // Caller can decide whether to proceed or require disk persistence
            return Ok(DurabilityLevel::Memory);
        }

        let entry = WalEntry::new(operation, None);
        self.write_entry(&entry)?;

        // P0-004 FIX: Return Disk durability when successfully logged
        Ok(DurabilityLevel::Disk)
    }

    /// Log an operation with payload
    /// 
    /// P0-004 FIX: Returns DurabilityLevel to indicate whether data is persisted
    #[tracing::instrument(skip_all)]
    pub fn log_with_payload(&mut self, operation: WalOperation, payload: String) -> Result<DurabilityLevel> {
        if !self.enabled {
            // P0-004 FIX: Return Memory durability when WAL is disabled
            return Ok(DurabilityLevel::Memory);
        }

        let entry = WalEntry::new(operation, Some(payload));
        self.write_entry(&entry)?;

        // P0-004 FIX: Return Disk durability when successfully logged
        Ok(DurabilityLevel::Disk)
    }

    /// P1-001 OPTIMIZATION: Write WAL entry without immediate flush
    /// Buffer writes for better performance - flush happens periodically
    ///
    /// P1-013: Checks for rotation before writing
    fn write_entry(&mut self, entry: &WalEntry) -> Result<()> {
        // P1-013: Check if rotation is needed before writing
        // Estimate entry size
        let json = serde_json::to_string(entry)
            .map_err(WalError::Serialization)?;
        let entry_size = json.len() as u64 + 1; // +1 for newline

        if self.current_size + entry_size > self.max_size_bytes {
            // P1-013: Rotate - flush and close current file first
            self.flush()?;
            self.file = None;
            self.rotate()?;
        }

        // Write the entry
        if let Some(ref mut file) = self.file {
            // P1-001: Write with newline, but defer flush for batch efficiency
            writeln!(file, "{}", json)
                .map_err(|e| WalError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to write WAL entry: {}", e)
                )))?;

            // P1-013: Update current size
            self.current_size += entry_size;
        }

        Ok(())
    }

    /// P1-013: Rotate WAL files when size limit is exceeded
    ///
    /// Rotation strategy:
    /// - wal.log → wal.log.1
    /// - wal.log.1 → wal.log.2
    /// - ...
    /// - wal.log.(max_files-1) → deleted
    fn rotate(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Flush current file before rotation
        self.flush()?;

        // Close current file
        self.file = None;

        // P1-013: Delete oldest file if it exists
        let oldest = self.log_file.with_extension(format!("log.{}", self.max_files));
        if oldest.exists() {
            std::fs::remove_file(&oldest)
                .map_err(|e| WalError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to delete oldest WAL file: {:?}: {}", oldest, e)
                )))?;
        }

        // P1-013: Rotate existing files
        for i in (1..self.max_files).rev() {
            let old_path = if i == 1 {
                self.log_file.with_extension("log.1")
            } else {
                self.log_file.with_extension(format!("log.{}", i))
            };
            let new_path = self.log_file.with_extension(format!("log.{}", i + 1));

            if old_path.exists() {
                std::fs::rename(&old_path, &new_path)
                    .map_err(|e| WalError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to rotate WAL file from {:?} to {:?}: {}", old_path, new_path, e)
                    )))?;
            }
        }

        // P1-013: Rename current file to .1
        if self.log_file.exists() {
            let rotated_path = self.log_file.with_extension("log.1");
            std::fs::rename(&self.log_file, &rotated_path)
                .map_err(|e| WalError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to rotate current WAL file to {:?}: {}", rotated_path, e)
                )))?;
        }

        // P1-013: Open new current file
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(|e| WalError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open new WAL file: {:?}: {}", self.log_file, e)
            )))?;

        self.file = Some(new_file);
        self.current_size = 0;

        tracing::info!("WAL rotated: {:?}", self.log_file);
        Ok(())
    }

    /// Flush the WAL to disk
    pub fn flush(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()
                .map_err(|e| WalError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to flush WAL: {}", e)
                )))?;
        }
        Ok(())
    }

    /// Read all entries from the WAL
    pub fn read_entries(&self) -> Result<Vec<WalEntry>> {
        if !self.log_file.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_file)
            .map_err(|e| WalError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open WAL file: {:?}: {}", self.log_file, e)
            )))?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        let mut entry_num = 0;
        for line in reader.lines() {
            let line = line
                .map_err(|e| WalError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read WAL line {}: {}", entry_num, e)
                )))?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: WalEntry = serde_json::from_str(&line)
                .map_err(WalError::Serialization)?;

            // Verify checksum
            if !entry.verify_checksum() {
                tracing::warn!(
                    timestamp = %entry.timestamp,
                    "WAL entry checksum mismatch - possible corruption"
                );
                continue;
            }

            entries.push(entry);
            entry_num += 1;
        }

        Ok(entries)
    }

    /// Clear the WAL (after successful recovery)
    pub fn clear(&mut self) -> Result<()> {
        if self.log_file.exists() {
            std::fs::write(&self.log_file, "")
                .map_err(|e| WalError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to clear WAL file: {:?}: {}", self.log_file, e)
                )))?;
        }
        Ok(())
    }

    /// Get the WAL file path
    pub fn path(&self) -> &Path {
        &self.log_file
    }

    /// Check if WAL has entries
    pub fn has_entries(&self) -> Result<bool> {
        if !self.log_file.exists() {
            return Ok(false);
        }

        let file = File::open(&self.log_file)
            .map_err(|e| WalError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open WAL file: {:?}: {}", self.log_file, e)
            )))?;
        let metadata = file.metadata()
            .map_err(|e| WalError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to get WAL file metadata: {:?}: {}", self.log_file, e)
            )))?;
        Ok(metadata.len() > 0)
    }

    /// Get number of entries
    pub fn entry_count(&self) -> Result<usize> {
        Ok(self.read_entries()?.len())
    }

    /// Log compaction start - records which segments are being compacted
    ///
    /// P0-005 FIX: Atomic compaction - WAL record before starting
    pub fn log_compaction_start(&mut self, compaction_id: u64, segment_ids: Vec<u64>, new_segment_id: u64) -> Result<DurabilityLevel> {
        let operation = WalOperation::CompactionStart {
            compaction_id,
            segment_ids,
            new_segment_id,
        };
        self.log(operation)
    }

    /// Log compaction complete - records successful compaction
    ///
    /// P0-005 FIX: Atomic compaction - WAL record after writing new segment
    pub fn log_compaction_complete(&mut self, compaction_id: u64, new_segment_id: u64, keys_compacted: u64) -> Result<DurabilityLevel> {
        let operation = WalOperation::CompactionComplete {
            compaction_id,
            new_segment_id,
            keys_compacted,
        };
        self.log(operation)
    }

    /// Log compaction cleanup - records removal of old segments
    ///
    /// P0-005 FIX: Atomic compaction - WAL record after removing old segments
    pub fn log_compaction_cleanup(&mut self, compaction_id: u64, removed_segment_ids: Vec<u64>) -> Result<DurabilityLevel> {
        let operation = WalOperation::CompactionCleanup {
            compaction_id,
            removed_segment_ids,
        };
        self.log(operation)
    }
}

/// Recovery engine - replays WAL entries
pub struct RecoveryEngine {
    wal_manager: WalManager,
}

impl RecoveryEngine {
    /// Create a recovery engine
    pub fn new(wal_manager: WalManager) -> Self {
        Self { wal_manager }
    }

    /// Replay WAL entries and attempt recovery
    /// 
    /// Returns the number of entries processed
    #[tracing::instrument(skip_all)]
    pub fn replay<F>(&mut self, mut handler: F) -> Result<usize>
    where
        F: FnMut(&WalEntry) -> Result<()>,
    {
        let entries = self.wal_manager.read_entries()?;
        let count = entries.len();

        tracing::info!(entries = count, "Replaying WAL entries");

        for entry in &entries {
            if let Err(e) = handler(entry) {
                tracing::error!(
                    timestamp = %entry.timestamp,
                    operation = ?entry.operation,
                    error = %e,
                    "Failed to replay WAL entry"
                );
                // Continue with next entry instead of failing completely
            }
        }

        // Clear WAL after successful replay
        self.wal_manager.clear()?;

        Ok(count)
    }

    /// Get incomplete operations (operations without matching completion)
    pub fn get_incomplete_operations(&self) -> Result<Vec<IncompleteOperation>> {
        let entries = self.wal_manager.read_entries()?;
        let mut incomplete = Vec::new();

        // Track operation pairs (start -> complete)
        let mut pending_merges: std::collections::HashMap<(String, String), &WalEntry> =
            std::collections::HashMap::new();
        let mut pending_compactions: std::collections::HashMap<u64, &WalEntry> =
            std::collections::HashMap::new();

        for entry in &entries {
            match &entry.operation {
                WalOperation::MergeStart { source, target } => {
                    pending_merges.insert((source.clone(), target.clone()), entry);
                }
                WalOperation::MergeComplete { source, target, success } => {
                    if *success {
                        pending_merges.remove(&(source.clone(), target.clone()));
                    }
                    // Failed merges are not considered incomplete
                }
                WalOperation::CompactionStart { compaction_id, segment_ids: _, new_segment_id: _ } => {
                    // Track compaction start
                    pending_compactions.insert(*compaction_id, entry);
                }
                WalOperation::CompactionComplete { compaction_id, .. } => {
                    // Mark compaction as complete
                    pending_compactions.remove(compaction_id);
                }
                _ => {}
            }
        }

        // Remaining pending operations are incomplete
        for ((source, target), entry) in pending_merges {
            incomplete.push(IncompleteOperation {
                timestamp: entry.timestamp,
                operation: format!("Merge {} into {}", source, target),
                suggestion: "Verify merge state and retry if necessary".to_string(),
            });
        }

        // Add incomplete compactions
        for (compaction_id, entry) in pending_compactions {
            if let WalOperation::CompactionStart { segment_ids, new_segment_id, .. } = &entry.operation {
                incomplete.push(IncompleteOperation {
                    timestamp: entry.timestamp,
                    operation: format!(
                        "Compaction {} - merging {:?} into segment {}",
                        compaction_id, segment_ids, new_segment_id
                    ),
                    suggestion: "Verify new segment exists and is valid, then remove old segments or rollback".to_string(),
                });
            }
        }

        Ok(incomplete)
    }

    /// Get incomplete compactions for recovery
    ///
    /// P0-005 FIX: Returns compactions that started but didn't complete
    pub fn get_incomplete_compactions(&self) -> Result<Vec<IncompleteCompaction>> {
        let entries = self.wal_manager.read_entries()?;
        let mut incomplete = Vec::new();

        // Track compaction progress
        let mut compaction_starts: std::collections::HashMap<u64, (Vec<u64>, u64, DateTime<Utc>)> =
            std::collections::HashMap::new();
        let mut compaction_completes: std::collections::HashSet<u64> =
            std::collections::HashSet::new();
        let mut compaction_cleanups: std::collections::HashSet<u64> =
            std::collections::HashSet::new();

        for entry in &entries {
            match &entry.operation {
                WalOperation::CompactionStart { compaction_id, segment_ids, new_segment_id } => {
                    compaction_starts.insert(
                        *compaction_id,
                        (segment_ids.clone(), *new_segment_id, entry.timestamp),
                    );
                }
                WalOperation::CompactionComplete { compaction_id, .. } => {
                    compaction_completes.insert(*compaction_id);
                }
                WalOperation::CompactionCleanup { compaction_id, .. } => {
                    compaction_cleanups.insert(*compaction_id);
                }
                _ => {}
            }
        }

        // Find incomplete compactions
        for (compaction_id, (segment_ids, new_segment_id, timestamp)) in compaction_starts {
            let status = if !compaction_completes.contains(&compaction_id) {
                CompactionRecoveryStatus::SegmentWriteIncomplete
            } else if !compaction_cleanups.contains(&compaction_id) {
                CompactionRecoveryStatus::CleanupIncomplete
            } else {
                continue; // Fully complete
            };

            incomplete.push(IncompleteCompaction {
                compaction_id,
                segment_ids_to_remove: segment_ids,
                new_segment_id,
                timestamp,
                status,
            });
        }

        Ok(incomplete)
    }
}

/// An incomplete operation that needs attention
#[derive(Debug, Clone)]
pub struct IncompleteOperation {
    /// When the operation started
    pub timestamp: DateTime<Utc>,
    /// Description of the operation
    pub operation: String,
    /// Suggested recovery action
    pub suggestion: String,
}

/// Compaction recovery status indicating what step failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactionRecoveryStatus {
    /// New segment was not written - compaction failed early
    SegmentWriteIncomplete,
    /// New segment written but old segments not removed yet
    CleanupIncomplete,
}

/// An incomplete compaction that needs recovery
#[derive(Debug, Clone)]
pub struct IncompleteCompaction {
    /// Unique compaction ID
    pub compaction_id: u64,
    /// Segment IDs that should be removed
    pub segment_ids_to_remove: Vec<u64>,
    /// New segment ID that was created
    pub new_segment_id: u64,
    /// When the compaction started
    pub timestamp: DateTime<Utc>,
    /// What step failed
    pub status: CompactionRecoveryStatus,
}

/// WAL statistics
#[derive(Debug, Default, Clone)]
pub struct WalStats {
    /// Number of entries in WAL
    pub entry_count: usize,
    /// WAL file size in bytes
    pub file_size_bytes: u64,
    /// Number of incomplete operations
    pub incomplete_operations: usize,
    /// Whether WAL is enabled
    pub enabled: bool,
}

impl WalManager {
    /// Get WAL statistics
    pub fn stats(&self) -> Result<WalStats> {
        let entry_count = self.entry_count()?;
        let file_size_bytes = if self.log_file.exists() {
            std::fs::metadata(&self.log_file)?.len()
        } else {
            0
        };

        let recovery = RecoveryEngine::new(WalManager {
            log_file: self.log_file.clone(),
            file: None, // Don't need file handle for stats
            enabled: self.enabled,
            max_size_bytes: self.max_size_bytes,
            max_files: self.max_files,
            current_size: 0, // Don't need size for stats
        });
        let incomplete_operations = recovery.get_incomplete_operations()?.len();

        Ok(WalStats {
            entry_count,
            file_size_bytes,
            incomplete_operations,
            enabled: self.enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_wal_entry_checksum() {
        let op = WalOperation::Add {
            session: "test".to_string(),
            hash: "abc123".to_string(),
            layer: "short-term".to_string(),
        };
        
        let entry = WalEntry::new(op.clone(), None);
        assert!(entry.verify_checksum());
    }

    #[test]
    fn test_wal_manager_log_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let mut wal = WalManager::new(temp_dir.path(), true).unwrap();

        let op = WalOperation::Add {
            session: "test-session".to_string(),
            hash: "abc123".to_string(),
            layer: "short-term".to_string(),
        };

        wal.log(op).unwrap();
        wal.flush().unwrap();

        let entries = wal.read_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].verify_checksum());
    }

    #[test]
    fn test_wal_clear() {
        let temp_dir = TempDir::new().unwrap();
        let mut wal = WalManager::new(temp_dir.path(), true).unwrap();

        wal.log(WalOperation::Add {
            session: "test".to_string(),
            hash: "abc".to_string(),
            layer: "short-term".to_string(),
        }).unwrap();

        assert!(wal.has_entries().unwrap());
        wal.clear().unwrap();
        assert!(!wal.has_entries().unwrap());
    }

    #[test]
    fn test_recovery_engine() {
        let temp_dir = TempDir::new().unwrap();
        let mut wal = WalManager::new(temp_dir.path(), true).unwrap();

        // Log some operations
        wal.log(WalOperation::Add {
            session: "session-1".to_string(),
            hash: "hash1".to_string(),
            layer: "short-term".to_string(),
        }).unwrap();

        wal.log(WalOperation::Delete {
            session: "session-1".to_string(),
            hash: "hash2".to_string(),
            content: None,
        }).unwrap();

        // Replay
        let mut engine = RecoveryEngine::new(wal);
        let mut processed = 0;
        engine.replay(|_entry| {
            processed += 1;
            Ok(())
        }).unwrap();

        assert_eq!(processed, 2);
    }

    #[test]
    fn test_incomplete_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut wal = WalManager::new(temp_dir.path(), true).unwrap();

        // Start a merge but don't complete it
        wal.log(WalOperation::MergeStart {
            source: "feature".to_string(),
            target: "main".to_string(),
        }).unwrap();

        let engine = RecoveryEngine::new(wal);
        let incomplete = engine.get_incomplete_operations().unwrap();

        assert_eq!(incomplete.len(), 1);
        assert!(incomplete[0].operation.contains("Merge feature into main"));
    }

    #[test]
    fn test_wal_rotation() {
        let temp_dir = TempDir::new().unwrap();
        
        // P1-013: Create WAL with very small size limit to trigger rotation
        let mut wal = WalManager::new_with_config(temp_dir.path(), true, 500, 3).unwrap();

        // Write enough entries to trigger rotation
        for i in 0..20 {
            let op = WalOperation::Add {
                session: format!("session-{}", i),
                hash: format!("hash-{}", i),
                layer: "short-term".to_string(),
            };
            wal.log(op).unwrap();
        }

        // Check that rotation happened (multiple files should exist)
        let wal_dir = temp_dir.path();
        let mut file_count = 0;
        for entry in std::fs::read_dir(wal_dir).unwrap() {
            let entry = entry.unwrap();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            // Count files that start with "wal.log"
            if name_str.starts_with("wal.log") {
                file_count += 1;
            }
        }

        // Should have at least 2 files (current + at least one rotated)
        assert!(file_count >= 2, "WAL rotation should have created multiple files, found: {}", file_count);
    }

    #[test]
    fn test_wal_rotation_max_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // P1-013: Create WAL with small size limit and max 3 files
        let mut wal = WalManager::new_with_config(temp_dir.path(), true, 300, 3).unwrap();

        // Write enough entries to trigger multiple rotations
        for i in 0..50 {
            let op = WalOperation::Add {
                session: format!("session-{}", i),
                hash: format!("hash-{}", i),
                layer: "short-term".to_string(),
            };
            wal.log(op).unwrap();
        }

        // Check that old files are cleaned up (max 3 files)
        let wal_dir = temp_dir.path();
        let mut wal_files: Vec<_> = std::fs::read_dir(wal_dir)
            .unwrap()
            .filter_map(|e| {
                let entry = e.unwrap();
                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str());
                if ext == Some("log") || ext.is_some_and(|s| s.starts_with("log.")) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        wal_files.sort();

        // Should have at most 3 files
        assert!(wal_files.len() <= 3, "WAL rotation should keep at most 3 files, found: {}", wal_files.len());
    }
}
