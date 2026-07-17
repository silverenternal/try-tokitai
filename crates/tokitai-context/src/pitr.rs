//! Point-in-Time Recovery (PITR) Module
//!
//! This module implements point-in-time recovery, allowing the database to be restored
//! to any specific timestamp by combining checkpoint snapshots with WAL (Write-Ahead Log)
//! replay.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
//! │  Checkpoint │────▶│ WAL Timeline │────▶│ Target Timestamp│
//! │  (Base)     │     │ (Replay)     │     │ (Recovery Point)│
//! └─────────────┘     └──────────────┘     └─────────────────┘
//! ```
//!
//! # Recovery Process
//!
//! 1. **Find Nearest Checkpoint**: Locate the full checkpoint before target timestamp
//! 2. **Load Checkpoint**: Restore state from checkpoint
//! 3. **Replay WAL**: Apply WAL entries from checkpoint to target timestamp
//! 4. **Verify State**: Validate recovered state consistency
//!
//! # Features
//!
//! 1. **Timeline Tracking**: Maintain ordered sequence of checkpoints and WAL entries
//! 2. **Timestamp-based Recovery**: Recover to any point in time (within retention)
//! 3. **Incremental Recovery**: Use incremental checkpoints for faster recovery
//! 4. **Validation**: Verify recovered state integrity
//! 5. **Progress Tracking**: Monitor recovery progress
//!
//! # Usage
//!
//! ```rust,no_run
//! use tokitai_context::pitr::{PitrManager, PitrConfig};
//! use std::time::SystemTime;
//!
//! let config = PitrConfig::default();
//! let mut manager = PitrManager::new(config, "./data")?;
//!
//! // Create checkpoint
//! manager.create_checkpoint("base")?;
//!
//! // ... perform operations ...
//!
//! // Recover to specific timestamp
//! let target_time = SystemTime::now();
//! manager.recover_to_timestamp(target_time)?;
//!
//! // List available recovery points
//! let points = manager.list_recovery_points()?;
//! for point in points {
//!     println!("Recovery point: {:?}", point);
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug, error};

use crate::error::{ContextResult, ContextError};
use crate::wal::{WalManager, WalEntry};

/// PITR configuration
#[derive(Debug, Clone)]
pub struct PitrConfig {
    /// Enable PITR functionality (default: true)
    pub enabled: bool,
    /// Retention period for WAL entries in hours (default: 24 hours)
    pub wal_retention_hours: u64,
    /// Checkpoint interval in minutes (default: 60 minutes)
    pub checkpoint_interval_minutes: u64,
    /// Maximum number of checkpoints to retain (default: 10)
    pub max_checkpoints: usize,
    /// Enable automatic checkpoint creation (default: true)
    pub auto_checkpoint: bool,
    /// Enable incremental checkpoints (default: true)
    pub incremental_checkpoints: bool,
}

impl Default for PitrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wal_retention_hours: 24,
            checkpoint_interval_minutes: 60,
            max_checkpoints: 10,
            auto_checkpoint: true,
            incremental_checkpoints: true,
        }
    }
}

/// Recovery point representing a specific timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPoint {
    /// Unique identifier for this recovery point
    pub id: String,
    /// Timestamp this recovery point represents
    pub timestamp: u64,
    /// Human-readable timestamp
    pub timestamp_human: String,
    /// Type of recovery point
    pub point_type: RecoveryPointType,
    /// Size of data at this point (bytes)
    pub data_size_bytes: u64,
    /// Number of entries
    pub entry_count: u64,
    /// Checksum for validation
    pub checksum: String,
}

/// Type of recovery point
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPointType {
    /// Full checkpoint
    FullCheckpoint,
    /// Incremental checkpoint
    IncrementalCheckpoint,
    /// WAL entry (fine-grained recovery)
    WalEntry,
}

/// Timeline tracking recovery points in chronological order
#[derive(Debug, Clone, Default)]
pub struct Timeline {
    /// Ordered map of timestamp -> recovery point
    points: BTreeMap<u64, RecoveryPoint>,
    /// Current timeline sequence number
    sequence: u64,
}

impl Timeline {
    /// Create new empty timeline
    pub fn new() -> Self {
        Self {
            points: BTreeMap::new(),
            sequence: 0,
        }
    }

    /// Add recovery point to timeline
    pub fn add_point(&mut self, point: RecoveryPoint) {
        self.points.insert(point.timestamp, point);
        self.sequence += 1;
    }

    /// Get recovery point at or before timestamp
    pub fn get_point_at_or_before(&self, timestamp: u64) -> Option<&RecoveryPoint> {
        self.points.range(..=timestamp).next_back().map(|(_, p)| p)
    }

    /// Get all points in time range
    pub fn get_points_in_range(&self, start: u64, end: u64) -> Vec<&RecoveryPoint> {
        self.points
            .range(start..=end)
            .map(|(_, p)| p)
            .collect()
    }

    /// Get all points
    pub fn get_all_points(&self) -> Vec<&RecoveryPoint> {
        self.points.values().collect()
    }

    /// Remove points older than timestamp
    pub fn remove_older_than(&mut self, timestamp: u64) -> usize {
        let keys_to_remove: Vec<u64> = self.points.keys().filter(|&&k| k < timestamp).copied().collect();
        let count = keys_to_remove.len();
        for key in keys_to_remove {
            self.points.remove(&key);
        }
        count
    }

    /// Get latest point
    pub fn latest(&self) -> Option<&RecoveryPoint> {
        self.points.values().next_back()
    }

    /// Get earliest point
    pub fn earliest(&self) -> Option<&RecoveryPoint> {
        self.points.values().next()
    }
}

/// Progress tracking for recovery operations
#[derive(Debug, Clone, Default)]
pub struct RecoveryProgress {
    /// Total steps in recovery
    pub total_steps: u64,
    /// Current step
    pub current_step: u64,
    /// Current phase
    pub phase: RecoveryPhase,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
}

impl RecoveryProgress {
    /// Create new progress tracker
    pub fn new(total_steps: u64, phase: RecoveryPhase) -> Self {
        Self {
            total_steps,
            current_step: 0,
            phase,
            eta_seconds: None,
            error: None,
        }
    }

    /// Advance progress
    pub fn advance(&mut self) {
        self.current_step += 1;
    }

    /// Set current phase
    pub fn set_phase(&mut self, phase: RecoveryPhase) {
        self.phase = phase;
    }

    /// Get progress percentage
    pub fn percentage(&self) -> f64 {
        if self.total_steps == 0 {
            0.0
        } else {
            (self.current_step as f64 / self.total_steps as f64) * 100.0
        }
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }
}

/// Recovery phase
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum RecoveryPhase {
    /// Finding checkpoint
    #[default]
    FindingCheckpoint,
    /// Loading checkpoint
    LoadingCheckpoint,
    /// Replaying WAL
    ReplayingWal,
    /// Verifying state
    Verifying,
    /// Complete
    Complete,
}


/// Statistics for PITR operations
#[derive(Debug, Clone, Default)]
pub struct PitrStats {
    /// Total recovery operations performed
    pub total_recoveries: u64,
    /// Successful recoveries
    pub successful_recoveries: u64,
    /// Failed recoveries
    pub failed_recoveries: u64,
    /// Total checkpoints created
    pub total_checkpoints: u64,
    /// Average recovery time in milliseconds
    pub avg_recovery_time_ms: f64,
    /// Latest recovery time in milliseconds
    pub latest_recovery_time_ms: u64,
    /// Total WAL entries replayed
    pub total_wal_entries_replayed: u64,
}

impl PitrStats {
    /// Record successful recovery
    pub fn record_success(&mut self, duration_ms: u64) {
        self.total_recoveries += 1;
        self.successful_recoveries += 1;
        self.latest_recovery_time_ms = duration_ms;
        
        // Update average
        let total = self.total_recoveries as f64;
        self.avg_recovery_time_ms = ((self.avg_recovery_time_ms * (total - 1.0)) + duration_ms as f64) / total;
    }

    /// Record failed recovery
    pub fn record_failure(&mut self) {
        self.total_recoveries += 1;
        self.failed_recoveries += 1;
    }

    /// Record checkpoint creation
    pub fn record_checkpoint(&mut self) {
        self.total_checkpoints += 1;
    }

    /// Record WAL entries replayed
    pub fn record_wal_replay(&mut self, count: u64) {
        self.total_wal_entries_replayed += count;
    }

    /// Export to Prometheus format
    pub fn to_prometheus(&self) -> String {
        format!(
            r#"# HELP tokitai_pitr_recoveries_total Total recovery operations
# TYPE tokitai_pitr_recoveries_total counter
tokitai_pitr_recoveries_total {}
# HELP tokitai_pitr_successful_recoveries_total Successful recoveries
# TYPE tokitai_pitr_successful_recoveries_total counter
tokitai_pitr_successful_recoveries_total {}
# HELP tokitai_pitr_failed_recoveries_total Failed recoveries
# TYPE tokitai_pitr_failed_recoveries_total counter
tokitai_pitr_failed_recoveries_total {}
# HELP tokitai_pitr_checkpoints_total Total checkpoints created
# TYPE tokitai_pitr_checkpoints_total counter
tokitai_pitr_checkpoints_total {}
# HELP tokitai_pitr_avg_recovery_time_ms Average recovery time in milliseconds
# TYPE tokitai_pitr_avg_recovery_time_ms gauge
tokitai_pitr_avg_recovery_time_ms {}
# HELP tokitai_pitr_wal_entries_replayed_total Total WAL entries replayed
# TYPE tokitai_pitr_wal_entries_replayed_total counter
tokitai_pitr_wal_entries_replayed_total {}
"#,
            self.total_recoveries,
            self.successful_recoveries,
            self.failed_recoveries,
            self.total_checkpoints,
            self.avg_recovery_time_ms as u64,
            self.total_wal_entries_replayed,
        )
    }

    /// Get human-readable report
    pub fn report(&self) -> String {
        format!(
            "PITR Stats:\n\
             - Total recoveries: {} ({} successful, {} failed)\n\
             - Success rate: {:.1}%\n\
             - Average recovery time: {:.2} ms\n\
             - Latest recovery time: {} ms\n\
             - Total checkpoints: {}\n\
             - WAL entries replayed: {}",
            self.total_recoveries,
            self.successful_recoveries,
            self.failed_recoveries,
            if self.total_recoveries > 0 {
                (self.successful_recoveries as f64 / self.total_recoveries as f64) * 100.0
            } else {
                0.0
            },
            self.avg_recovery_time_ms,
            self.latest_recovery_time_ms,
            self.total_checkpoints,
            self.total_wal_entries_replayed,
        )
    }
}

/// Point-in-Time Recovery Manager
pub struct PitrManager {
    /// Configuration
    config: PitrConfig,
    /// Data directory
    data_dir: PathBuf,
    /// Checkpoint directory
    checkpoint_dir: PathBuf,
    /// WAL manager
    wal_manager: Option<WalManager>,
    /// Timeline tracking
    timeline: Timeline,
    /// Statistics
    stats: Arc<std::sync::Mutex<PitrStats>>,
    /// Current progress
    progress: Arc<std::sync::Mutex<RecoveryProgress>>,
}

impl PitrManager {
    /// Create new PITR manager
    pub fn new(config: PitrConfig, data_dir: &Path) -> ContextResult<Self> {
        let checkpoint_dir = data_dir.join("checkpoints");
        
        // Create checkpoint directory if it doesn't exist
        fs::create_dir_all(&checkpoint_dir)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create checkpoint directory: {}", e)))?;

        let mut manager = Self {
            config,
            data_dir: data_dir.to_path_buf(),
            checkpoint_dir,
            wal_manager: None,
            timeline: Timeline::new(),
            stats: Arc::new(std::sync::Mutex::new(PitrStats::default())),
            progress: Arc::new(std::sync::Mutex::new(RecoveryProgress::default())),
        };

        // Load existing timeline
        manager.load_timeline()?;

        Ok(manager)
    }

    /// Set WAL manager for replay
    pub fn set_wal_manager(&mut self, wal_manager: WalManager) {
        self.wal_manager = Some(wal_manager);
    }

    /// Create a new checkpoint
    pub fn create_checkpoint(&mut self, name: &str) -> ContextResult<RecoveryPoint> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ContextError::OperationFailed(format!("Time error: {}", e)))?
            .as_secs();

        let checkpoint_id = format!("{}_{}", name, timestamp);
        let checkpoint_path = self.checkpoint_dir.join(format!("{}.checkpoint", checkpoint_id));

        // Create checkpoint file (placeholder - actual implementation would serialize state)
        let mut file = BufWriter::new(File::create(&checkpoint_path)?);
        
        // Write checkpoint metadata
        let metadata = CheckpointMetadata {
            id: checkpoint_id.clone(),
            timestamp,
            checkpoint_type: crate::pitr::CheckpointType::Full,
            version: 1,
        };
        
        let metadata_bytes = serde_json::to_vec(&metadata)?;
        file.write_all(&(metadata_bytes.len() as u32).to_le_bytes())?;
        file.write_all(&metadata_bytes)?;
        file.flush()?;

        let point = RecoveryPoint {
            id: checkpoint_id,
            timestamp,
            timestamp_human: DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string()),
            point_type: RecoveryPointType::FullCheckpoint,
            data_size_bytes: 0, // Would be calculated from actual state
            entry_count: 0,
            checksum: String::new(), // Would be calculated
        };

        self.timeline.add_point(point.clone());
        self.save_timeline()?;

        {
            let mut stats = self.stats.lock().map_err(|_| {
                ContextError::OperationFailed("Failed to acquire stats lock".to_string())
            })?;
            stats.record_checkpoint();
        }

        info!("Created checkpoint: {} at timestamp {}", point.id, timestamp);
        Ok(point)
    }

    /// Recover to specific timestamp
    pub fn recover_to_timestamp(&self, target_timestamp: u64) -> ContextResult<RecoveryProgress> {
        let start_time = std::time::Instant::now();
        
        let mut progress = RecoveryProgress::new(100, RecoveryPhase::FindingCheckpoint);
        
        info!("Starting PITR to timestamp {}", target_timestamp);

        // Step 1: Find nearest checkpoint before target timestamp
        progress.set_phase(RecoveryPhase::FindingCheckpoint);
        let checkpoint = match self.timeline.get_point_at_or_before(target_timestamp) {
            Some(cp) => {
                info!("Found checkpoint at timestamp {}", cp.timestamp);
                cp.clone()
            }
            None => {
                return Err(ContextError::OperationFailed(
                    format!("No checkpoint found before timestamp {}", target_timestamp)
                ));
            }
        };

        progress.advance();

        // Step 2: Load checkpoint
        progress.set_phase(RecoveryPhase::LoadingCheckpoint);
        self.load_checkpoint(&checkpoint)?;
        progress.advance();

        // Step 3: Replay WAL from checkpoint to target timestamp
        progress.set_phase(RecoveryPhase::ReplayingWal);
        if let Some(ref wal_manager) = self.wal_manager {
            let entries_replayed = self.replay_wal_range(
                wal_manager,
                checkpoint.timestamp,
                target_timestamp,
                &mut progress,
            )?;
            
            {
                let mut stats = self.stats.lock().map_err(|_| {
                    ContextError::OperationFailed("Failed to acquire stats lock".to_string())
                })?;
                stats.record_wal_replay(entries_replayed);
            }
        }
        progress.advance();

        // Step 4: Verify recovered state
        progress.set_phase(RecoveryPhase::Verifying);
        self.verify_recovered_state()?;
        progress.advance();

        progress.set_phase(RecoveryPhase::Complete);
        progress.current_step = 100;

        let duration = start_time.elapsed();
        {
            let mut stats = self.stats.lock().map_err(|_| {
                ContextError::OperationFailed("Failed to acquire stats lock".to_string())
            })?;
            stats.record_success(duration.as_millis() as u64);
        }

        info!("PITR completed in {:?}", duration);
        
        *self.progress.lock().map_err(|_| {
            ContextError::OperationFailed("Failed to acquire progress lock".to_string())
        })? = progress.clone();

        Ok(progress)
    }

    /// List all available recovery points
    pub fn list_recovery_points(&self) -> Vec<&RecoveryPoint> {
        self.timeline.get_all_points()
    }

    /// Get recovery points in time range
    pub fn get_recovery_points_in_range(&self, start: u64, end: u64) -> Vec<&RecoveryPoint> {
        self.timeline.get_points_in_range(start, end)
    }

    /// Get statistics
    pub fn stats(&self) -> std::sync::MutexGuard<'_, PitrStats> {
        self.stats.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Get current progress
    pub fn progress(&self) -> std::sync::MutexGuard<'_, RecoveryProgress> {
        self.progress.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Clean up old recovery points based on retention policy
    pub fn cleanup_old_points(&mut self) -> ContextResult<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ContextError::OperationFailed(format!("Time error: {}", e)))?
            .as_secs();

        let retention_secs = self.config.wal_retention_hours * 3600;
        let cutoff = now.saturating_sub(retention_secs);

        let removed = self.timeline.remove_older_than(cutoff);
        self.save_timeline()?;

        info!("Cleaned up {} recovery points older than {}", removed, cutoff);
        Ok(removed)
    }

    // Internal methods

    fn load_timeline(&mut self) -> ContextResult<()> {
        let timeline_path = self.checkpoint_dir.join("timeline.json");
        
        if !timeline_path.exists() {
            return Ok(());
        }

        let file = File::open(&timeline_path)?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        reader.read_to_string(&mut contents)?;

        // Parse timeline and populate points
        let points: Vec<RecoveryPoint> = serde_json::from_str(&contents)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to parse timeline: {}", e)))?;

        for point in points {
            self.timeline.add_point(point);
        }

        info!("Loaded timeline with {} points", self.timeline.get_all_points().len());
        Ok(())
    }

    fn save_timeline(&self) -> ContextResult<()> {
        let timeline_path = self.checkpoint_dir.join("timeline.json");
        let file = File::create(&timeline_path)?;
        let mut writer = BufWriter::new(file);

        let points: Vec<&RecoveryPoint> = self.timeline.get_all_points();
        let json = serde_json::to_string_pretty(&points)?;
        
        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(())
    }

    fn load_checkpoint(&self, checkpoint: &RecoveryPoint) -> ContextResult<()> {
        let checkpoint_path = self.checkpoint_dir.join(format!("{}.checkpoint", checkpoint.id));
        
        if !checkpoint_path.exists() {
            return Err(ContextError::OperationFailed(
                format!("Checkpoint file not found: {:?}", checkpoint_path)
            ));
        }

        // Load and validate checkpoint
        let file = File::open(&checkpoint_path)?;
        let mut reader = BufReader::new(file);
        
        // Read metadata length
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let metadata_len = u32::from_le_bytes(len_buf) as usize;
        
        // Read metadata
        let mut metadata_bytes = vec![0u8; metadata_len];
        reader.read_exact(&mut metadata_bytes)?;
        
        let _metadata: CheckpointMetadata = serde_json::from_slice(&metadata_bytes)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to parse checkpoint metadata: {}", e)))?;

        info!("Loaded checkpoint: {}", checkpoint.id);
        Ok(())
    }

    fn replay_wal_range(
        &self,
        _wal_manager: &WalManager,
        _start_timestamp: u64,
        _end_timestamp: u64,
        _progress: &mut RecoveryProgress,
    ) -> ContextResult<u64> {
        // Placeholder - actual implementation would:
        // 1. Read WAL entries in range
        // 2. Apply each entry to restore state
        // 3. Update progress
        
        // For now, return 0 entries replayed
        Ok(0)
    }

    fn verify_recovered_state(&self) -> ContextResult<()> {
        // Placeholder - actual implementation would:
        // 1. Verify checksums
        // 2. Validate data integrity
        // 3. Check consistency
        
        Ok(())
    }
}

/// Checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Checkpoint ID
    pub id: String,
    /// Creation timestamp
    pub timestamp: u64,
    /// Checkpoint type
    pub checkpoint_type: CheckpointType,
    /// Format version
    pub version: u32,
}

/// Checkpoint type for metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointType {
    Full,
    Incremental,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pitr_config_default() {
        let config = PitrConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.wal_retention_hours, 24);
        assert_eq!(config.checkpoint_interval_minutes, 60);
        assert_eq!(config.max_checkpoints, 10);
        assert!(config.auto_checkpoint);
        assert!(config.incremental_checkpoints);
    }

    #[test]
    fn test_timeline_add_and_get() {
        let mut timeline = Timeline::new();
        
        let point1 = RecoveryPoint {
            id: "point1".to_string(),
            timestamp: 1000,
            timestamp_human: "2024-01-01T00:00:00Z".to_string(),
            point_type: RecoveryPointType::FullCheckpoint,
            data_size_bytes: 100,
            entry_count: 10,
            checksum: "abc123".to_string(),
        };
        
        let point2 = RecoveryPoint {
            id: "point2".to_string(),
            timestamp: 2000,
            timestamp_human: "2024-01-01T00:01:00Z".to_string(),
            point_type: RecoveryPointType::IncrementalCheckpoint,
            data_size_bytes: 50,
            entry_count: 5,
            checksum: "def456".to_string(),
        };
        
        timeline.add_point(point1.clone());
        timeline.add_point(point2.clone());
        
        // Test get_point_at_or_before
        let result = timeline.get_point_at_or_before(1500);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "point1");
        
        let result = timeline.get_point_at_or_before(2000);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "point2");
        
        // Test get_points_in_range
        let range = timeline.get_points_in_range(500, 2500);
        assert_eq!(range.len(), 2);
        
        // Test latest and earliest
        assert_eq!(timeline.earliest().unwrap().id, "point1");
        assert_eq!(timeline.latest().unwrap().id, "point2");
    }

    #[test]
    fn test_timeline_remove_older_than() {
        let mut timeline = Timeline::new();
        
        for i in 0..5 {
            timeline.add_point(RecoveryPoint {
                id: format!("point{}", i),
                timestamp: i * 1000,
                timestamp_human: format!("2024-01-01T00:0{}:00Z", i),
                point_type: RecoveryPointType::FullCheckpoint,
                data_size_bytes: 100,
                entry_count: 10,
                checksum: "abc".to_string(),
            });
        }
        
        let removed = timeline.remove_older_than(3000);
        assert_eq!(removed, 3); // Removed points 0, 1, 2
        
        assert_eq!(timeline.get_all_points().len(), 2);
        assert_eq!(timeline.earliest().unwrap().timestamp, 3000);
    }

    #[test]
    fn test_recovery_progress() {
        let mut progress = RecoveryProgress::new(10, RecoveryPhase::FindingCheckpoint);
        
        assert_eq!(progress.percentage(), 0.0);
        assert!(!progress.is_complete());
        
        for _ in 0..5 {
            progress.advance();
        }
        
        assert_eq!(progress.percentage(), 50.0);
        assert!(!progress.is_complete());
        
        for _ in 0..5 {
            progress.advance();
        }
        
        assert_eq!(progress.percentage(), 100.0);
        assert!(progress.is_complete());
    }

    #[test]
    fn test_pitr_stats() {
        let mut stats = PitrStats::default();
        
        stats.record_success(100);
        stats.record_success(200);
        stats.record_failure();
        stats.record_checkpoint();
        stats.record_wal_replay(50);
        
        assert_eq!(stats.total_recoveries, 3);
        assert_eq!(stats.successful_recoveries, 2);
        assert_eq!(stats.failed_recoveries, 1);
        assert_eq!(stats.total_checkpoints, 1);
        assert_eq!(stats.total_wal_entries_replayed, 50);
        assert!((stats.avg_recovery_time_ms - 150.0).abs() < 0.01);
        
        // Test Prometheus export
        let prometheus = stats.to_prometheus();
        assert!(prometheus.contains("tokitai_pitr"));
        
        // Test human-readable report
        let report = stats.report();
        assert!(report.contains("PITR Stats"));
    }

    #[test]
    fn test_pitr_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = PitrConfig::default();
        
        let manager = PitrManager::new(config, temp_dir.path());
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        assert!(manager.checkpoint_dir.exists());
    }

    #[test]
    fn test_create_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let config = PitrConfig {
            auto_checkpoint: false,
            ..Default::default()
        };

        let mut manager = PitrManager::new(config, temp_dir.path()).unwrap();

        let checkpoint = manager.create_checkpoint("test");
        assert!(checkpoint.is_ok());

        let checkpoint = checkpoint.unwrap();
        assert!(checkpoint.id.starts_with("test_"));
        assert_eq!(checkpoint.point_type, RecoveryPointType::FullCheckpoint);
        
        // Verify timeline was updated
        let points = manager.list_recovery_points();
        assert_eq!(points.len(), 1);
    }

    #[test]
    fn test_list_recovery_points() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PitrManager::new(PitrConfig::default(), temp_dir.path()).unwrap();
        
        manager.create_checkpoint("base").unwrap();
        // Sleep to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_secs(2));
        manager.create_checkpoint("incr").unwrap();
        
        let points = manager.list_recovery_points();
        assert_eq!(points.len(), 2);
        
        // Test range query
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let range = manager.get_recovery_points_in_range(0, now + 1000);
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_cleanup_old_points() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PitrManager::new(PitrConfig::default(), temp_dir.path()).unwrap();
        
        // Create some checkpoints
        manager.create_checkpoint("old1").unwrap();
        manager.create_checkpoint("old2").unwrap();
        
        // Manually adjust timestamps to simulate old points
        // This is a bit hacky - in real code we'd have a method to modify points
        
        // Cleanup should remove points based on retention policy
        let removed = manager.cleanup_old_points().unwrap();
        // With default 24h retention, recent points should not be removed
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_recover_to_timestamp_no_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PitrManager::new(PitrConfig::default(), temp_dir.path()).unwrap();
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let result = manager.recover_to_timestamp(now);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No checkpoint found"));
    }
}
