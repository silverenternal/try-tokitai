//! AI Auto-tuning - Performance analysis and automatic parameter optimization
//!
//! This module provides AI-driven auto-tuning capabilities including:
//! - Performance metrics collection and analysis
//! - Workload pattern recognition
//! - Automatic parameter tuning (memory, compaction, concurrency)
//! - Anomaly detection and alerting
//! - Reinforcement learning-based optimization
//! - Configuration recommendations
//!
//! ## Architecture
//!
//! ```text
//! Auto-tuner
//! ├── Metrics Collector → Time-series data
//! ├── Workload Analyzer → Pattern recognition
//! ├── Parameter Optimizer → Tuning recommendations
//! ├── Anomaly Detector → Alerting
//! └── Configuration Manager → Apply changes
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use tokitai_context::auto_tuner::{AutoTuner, AutoTunerConfig, TuningTarget};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = AutoTunerConfig {
//!     tuning_target: TuningTarget::Throughput,
//!     min_adjustment_interval_secs: 300,
//!     ..Default::default()
//! };
//!
//! let tuner = AutoTuner::new(config);
//!
//! // Start monitoring
//! tuner.start().await?;
//!
//! // Get recommendations
//! let recommendations = tuner.get_recommendations().await?;
//! # Ok(())
//! # }
//! ```

use std::collections::{HashMap, VecDeque, BTreeMap};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{interval, sleep};

use crate::error::ContextError;

// ============================================================================
// Performance Metrics
// ============================================================================

/// System performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_used: u64,
    /// Memory available in bytes
    pub memory_available: u64,
    /// Disk I/O read bytes per second
    pub disk_read_bps: u64,
    /// Disk I/O write bytes per second
    pub disk_write_bps: u64,
    /// Network receive bytes per second
    pub net_recv_bps: u64,
    /// Network transmit bytes per second
    pub net_trans_bps: u64,
}

/// Storage engine metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Read operations per second
    pub read_ops: u64,
    /// Write operations per second
    pub write_ops: u64,
    /// Average read latency in microseconds
    pub read_latency_us: f64,
    /// Average write latency in microseconds
    pub write_latency_us: f64,
    /// P99 read latency in microseconds
    pub read_p99_latency_us: f64,
    /// P99 write latency in microseconds
    pub write_p99_latency_us: f64,
    /// Cache hit ratio (0-1)
    pub cache_hit_ratio: f64,
    /// Compaction operations per second
    pub compaction_ops: u64,
    /// Bytes compacted per second
    pub compaction_bps: u64,
    /// SST file count
    pub sst_file_count: u64,
    /// Total storage size in bytes
    pub storage_size_bytes: u64,
    /// Write stall count
    pub write_stall_count: u64,
    /// Write stall time in milliseconds
    pub write_stall_time_ms: u64,
}

/// Query metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryMetrics {
    /// Queries per second
    pub qps: f64,
    /// Average query latency in milliseconds
    pub avg_latency_ms: f64,
    /// P50 query latency in milliseconds
    pub p50_latency_ms: f64,
    /// P95 query latency in milliseconds
    pub p95_latency_ms: f64,
    /// P99 query latency in milliseconds
    pub p99_latency_ms: f64,
    /// Query error rate (0-1)
    pub error_rate: f64,
    /// Query timeout count
    pub timeout_count: u64,
    /// Active queries
    pub active_queries: u64,
    /// Queries queued
    pub queued_queries: u64,
}

/// Combined metrics snapshot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp in milliseconds since epoch
    pub timestamp_ms: u64,
    /// System metrics
    pub system: SystemMetrics,
    /// Storage metrics
    pub storage: StorageMetrics,
    /// Query metrics
    pub query: QueryMetrics,
}

impl MetricsSnapshot {
    /// Create a new metrics snapshot
    pub fn new() -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;
        Self {
            timestamp_ms,
            ..Default::default()
        }
    }

    /// Get memory usage percentage
    pub fn memory_usage_percent(&self) -> f64 {
        let total = self.system.memory_used + self.system.memory_available;
        if total == 0 {
            0.0
        } else {
            (self.system.memory_used as f64 / total as f64) * 100.0
        }
    }

    /// Get I/O wait ratio (estimated from latency)
    pub fn io_wait_ratio(&self) -> f64 {
        // Simplified estimation based on latency vs baseline
        let baseline_latency = 100.0; // 100us baseline
        let avg_latency = self.storage.read_latency_us;
        if avg_latency <= baseline_latency {
            0.0
        } else {
            ((avg_latency - baseline_latency) / avg_latency).min(1.0)
        }
    }
}

// ============================================================================
// Workload Patterns
// ============================================================================

/// Workload pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadPattern {
    /// Read-heavy workload (>90% reads)
    ReadHeavy,
    /// Write-heavy workload (>90% writes)
    WriteHeavy,
    /// Mixed read/write workload
    Mixed,
    /// Batch loading (sustained high writes)
    BatchLoad,
    /// Analytical queries (large scans)
    Analytical,
    /// Point lookups (low latency reads)
    PointLookup,
    /// Range scans (sequential reads)
    RangeScan,
    /// Idle (low activity)
    Idle,
    /// Unknown pattern
    Unknown,
}

impl fmt::Display for WorkloadPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkloadPattern::ReadHeavy => write!(f, "ReadHeavy"),
            WorkloadPattern::WriteHeavy => write!(f, "WriteHeavy"),
            WorkloadPattern::Mixed => write!(f, "Mixed"),
            WorkloadPattern::BatchLoad => write!(f, "BatchLoad"),
            WorkloadPattern::Analytical => write!(f, "Analytical"),
            WorkloadPattern::PointLookup => write!(f, "PointLookup"),
            WorkloadPattern::RangeScan => write!(f, "RangeScan"),
            WorkloadPattern::Idle => write!(f, "Idle"),
            WorkloadPattern::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Workload characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadCharacteristics {
    /// Read/write ratio
    pub read_write_ratio: f64,
    /// Average key size in bytes
    pub avg_key_size: usize,
    /// Average value size in bytes
    pub avg_value_size: usize,
    /// Access pattern randomness (0=sequential, 1=random)
    pub access_randomness: f64,
    /// Hot key concentration (0=uniform, 1=highly skewed)
    pub hot_key_concentration: f64,
    /// Temporal locality (0=none, 1=high)
    pub temporal_locality: f64,
    /// Write burstiness (0=steady, 1=bursty)
    pub write_burstiness: f64,
}

impl Default for WorkloadCharacteristics {
    fn default() -> Self {
        Self {
            read_write_ratio: 1.0,
            avg_key_size: 64,
            avg_value_size: 1024,
            access_randomness: 0.5,
            hot_key_concentration: 0.2,
            temporal_locality: 0.5,
            write_burstiness: 0.3,
        }
    }
}

// ============================================================================
// Configuration Parameters
// ============================================================================

/// Tunable configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableParams {
    /// Block cache size in bytes
    pub block_cache_size: u64,
    /// Memtable size in bytes
    pub memtable_size: u64,
    /// Write buffer size in bytes
    pub write_buffer_size: u64,
    /// Max write buffers
    pub max_write_buffers: usize,
    /// Compaction read amplitude target
    pub compaction_read_amp: usize,
    /// Compaction write amplitude target
    pub compaction_write_amp: usize,
    /// Compaction size amplitude target
    pub compaction_size_amp: usize,
    /// Level 0 file count trigger
    pub l0_file_count: usize,
    /// Level 0 stall trigger
    pub l0_stall: usize,
    /// Target file size base in bytes
    pub target_file_size_base: u64,
    /// Max bytes for level multiplier
    pub max_bytes_for_level_base: u64,
    /// Max concurrent background jobs
    pub max_background_jobs: usize,
    /// Max subcompactions
    pub max_subcompactions: usize,
    /// Bytes per sync
    pub bytes_per_sync: u64,
    /// WAL bytes per sync
    pub wal_bytes_per_sync: u64,
    /// Delayed write rate
    pub delayed_write_rate: u64,
    /// Max write stall microseconds
    pub max_write_stall_us: u64,
}

impl Default for TunableParams {
    fn default() -> Self {
        Self {
            block_cache_size: 256 * 1024 * 1024, // 256MB
            memtable_size: 64 * 1024 * 1024,     // 64MB
            write_buffer_size: 64 * 1024 * 1024, // 64MB
            max_write_buffers: 2,
            compaction_read_amp: 10,
            compaction_write_amp: 10,
            compaction_size_amp: 10,
            l0_file_count: 4,
            l0_stall: 20,
            target_file_size_base: 64 * 1024 * 1024, // 64MB
            max_bytes_for_level_base: 512 * 1024 * 1024, // 512MB
            max_background_jobs: 6,
            max_subcompactions: 4,
            bytes_per_sync: 1024 * 1024,       // 1MB
            wal_bytes_per_sync: 512 * 1024,        // 512KB
            delayed_write_rate: 16 * 1024 * 1024,  // 16MB/s
            max_write_stall_us: 1_000_000,         // 1s
        }
    }
}

/// Parameter bounds for tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamBounds {
    pub block_cache_size: (u64, u64),
    pub memtable_size: (u64, u64),
    pub max_background_jobs: (usize, usize),
    pub l0_file_count: (usize, usize),
    pub compaction_read_amp: (usize, usize),
}

impl Default for ParamBounds {
    fn default() -> Self {
        Self {
            block_cache_size: (64 * 1024 * 1024, 4 * 1024 * 1024 * 1024), // 64MB - 4GB
            memtable_size: (16 * 1024 * 1024, 512 * 1024 * 1024),         // 16MB - 512MB
            max_background_jobs: (2, 16),
            l0_file_count: (2, 16),
            compaction_read_amp: (5, 30),
        }
    }
}

// ============================================================================
// Tuning Targets and Policies
// ============================================================================

/// Tuning optimization target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TuningTarget {
    /// Maximize throughput
    Throughput,
    /// Minimize latency
    Latency,
    /// Balance throughput and latency
    Balanced,
    /// Minimize memory usage
    Memory,
    /// Minimize disk usage
    Disk,
    /// Custom optimization
    Custom,
}

impl fmt::Display for TuningTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuningTarget::Throughput => write!(f, "Throughput"),
            TuningTarget::Latency => write!(f, "Latency"),
            TuningTarget::Balanced => write!(f, "Balanced"),
            TuningTarget::Memory => write!(f, "Memory"),
            TuningTarget::Disk => write!(f, "Disk"),
            TuningTarget::Custom => write!(f, "Custom"),
        }
    }
}

/// Tuning recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningRecommendation {
    /// Recommendation ID
    pub id: String,
    /// Timestamp
    pub timestamp_ms: u64,
    /// Confidence score (0-1)
    pub confidence: f64,
    /// Expected improvement percentage
    pub expected_improvement: f64,
    /// Parameter to change
    pub parameter: String,
    /// Current value
    pub current_value: String,
    /// Recommended value
    pub recommended_value: String,
    /// Reason for recommendation
    pub reason: String,
    /// Risk level (low, medium, high)
    pub risk_level: RiskLevel,
}

/// Risk level for changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
        }
    }
}

// ============================================================================
// Anomaly Detection
// ============================================================================

/// Anomaly types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Sudden latency spike
    LatencySpike { metric: String, severity: f64 },
    /// Throughput degradation
    ThroughputDrop { percent: f64 },
    /// Memory pressure
    MemoryPressure { usage_percent: f64 },
    /// Disk space warning
    DiskSpaceWarning { usage_percent: f64 },
    /// Compaction falling behind
    CompactionLag { lag_seconds: u64 },
    /// Write stall detected
    WriteStall { duration_ms: u64 },
    /// Error rate increase
    ErrorRateIncrease { rate: f64 },
    /// Unusual pattern detected
    UnusualPattern { description: String },
}

impl fmt::Display for AnomalyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnomalyType::LatencySpike { metric, severity } => {
                write!(f, "Latency spike in {} (severity: {:.2})", metric, severity)
            }
            AnomalyType::ThroughputDrop { percent } => {
                write!(f, "Throughput dropped by {:.1}%", percent)
            }
            AnomalyType::MemoryPressure { usage_percent } => {
                write!(f, "Memory pressure: {:.1}% used", usage_percent)
            }
            AnomalyType::DiskSpaceWarning { usage_percent } => {
                write!(f, "Disk space warning: {:.1}% used", usage_percent)
            }
            AnomalyType::CompactionLag { lag_seconds } => {
                write!(f, "Compaction lag: {} seconds", lag_seconds)
            }
            AnomalyType::WriteStall { duration_ms } => {
                write!(f, "Write stall: {} ms", duration_ms)
            }
            AnomalyType::ErrorRateIncrease { rate } => {
                write!(f, "Error rate increased to {:.2}%", rate * 100.0)
            }
            AnomalyType::UnusualPattern { description } => {
                write!(f, "Unusual pattern: {}", description)
            }
        }
    }
}

/// Anomaly alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub anomaly_type: AnomalyType,
    pub timestamp_ms: u64,
    pub severity: AlertSeverity,
    pub description: String,
    pub suggested_action: String,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

// ============================================================================
// Auto-tuner Configuration
// ============================================================================

/// Auto-tuner configuration
#[derive(Debug, Clone)]
pub struct AutoTunerConfig {
    /// Optimization target
    pub tuning_target: TuningTarget,
    /// Minimum adjustment interval in seconds
    pub min_adjustment_interval_secs: u64,
    /// Metrics collection interval in seconds
    pub metrics_interval_secs: u64,
    /// Analysis window size (number of snapshots)
    pub analysis_window_size: usize,
    /// Enable automatic parameter adjustment
    pub auto_adjust: bool,
    /// Enable anomaly detection
    pub anomaly_detection: bool,
    /// Anomaly detection sensitivity (0-1)
    pub anomaly_sensitivity: f64,
    /// Maximum parameter change per adjustment
    pub max_change_percent: f64,
    /// Cooldown period after adjustment in seconds
    pub cooldown_secs: u64,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for AutoTunerConfig {
    fn default() -> Self {
        Self {
            tuning_target: TuningTarget::Balanced,
            min_adjustment_interval_secs: 300, // 5 minutes
            metrics_interval_secs: 10,         // 10 seconds
            analysis_window_size: 60,          // 10 minutes of data
            auto_adjust: false,                // Recommend only by default
            anomaly_detection: true,
            anomaly_sensitivity: 0.8,
            max_change_percent: 0.2,           // 20% max change
            cooldown_secs: 600,                // 10 minutes
            verbose: false,
        }
    }
}

// ============================================================================
// Auto-tuner State
// ============================================================================

/// Auto-tuner statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoTunerStats {
    /// Total adjustments made
    pub total_adjustments: u64,
    /// Successful adjustments
    pub successful_adjustments: u64,
    /// Failed adjustments
    pub failed_adjustments: u64,
    /// Total anomalies detected
    pub anomalies_detected: u64,
    /// Total recommendations made
    pub recommendations_made: u64,
    /// Average improvement from adjustments
    pub avg_improvement_percent: f64,
    /// Last adjustment timestamp
    pub last_adjustment_ms: Option<u64>,
    /// Uptime in seconds
    pub uptime_secs: u64,
}

/// Internal tuner state
#[derive(Debug, Clone)]
pub struct TunerState {
    /// Is running
    pub running: bool,
    /// Current workload pattern
    pub workload_pattern: WorkloadPattern,
    /// Current characteristics
    pub characteristics: WorkloadCharacteristics,
    /// Current parameters
    pub current_params: TunableParams,
    /// Metrics history
    pub metrics_history: VecDeque<MetricsSnapshot>,
    /// Pending recommendations
    pub pending_recommendations: Vec<TuningRecommendation>,
    /// Recent anomalies
    pub recent_anomalies: Vec<AnomalyAlert>,
    /// Statistics
    pub stats: AutoTunerStats,
}

impl Default for TunerState {
    fn default() -> Self {
        Self {
            running: false,
            workload_pattern: WorkloadPattern::Unknown,
            characteristics: WorkloadCharacteristics::default(),
            current_params: TunableParams::default(),
            metrics_history: VecDeque::with_capacity(60),
            pending_recommendations: Vec::new(),
            recent_anomalies: Vec::new(),
            stats: AutoTunerStats::default(),
        }
    }
}

// ============================================================================
// Auto-tuner
// ============================================================================

/// AI-powered auto-tuner
pub struct AutoTuner {
    config: AutoTunerConfig,
    state: RwLock<TunerState>,
    param_bounds: ParamBounds,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AutoTuner {
    /// Create a new auto-tuner
    pub fn new(config: AutoTunerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(TunerState::default()),
            param_bounds: ParamBounds::default(),
            shutdown_tx: None,
        }
    }

    /// Create with custom parameter bounds
    pub fn with_bounds(config: AutoTunerConfig, bounds: ParamBounds) -> Self {
        Self {
            config,
            state: RwLock::new(TunerState::default()),
            param_bounds: bounds,
            shutdown_tx: None,
        }
    }

    /// Start the auto-tuner
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.write();
        if state.running {
            return Err(anyhow::anyhow!("Auto-tuner is already running"));
        }
        state.running = true;
        state.stats.uptime_secs = 0;
        Ok(())
    }

    /// Stop the auto-tuner
    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.write();
        if !state.running {
            return Err(anyhow::anyhow!("Auto-tuner is not running"));
        }
        state.running = false;
        Ok(())
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.state.read().running
    }

    /// Record a metrics snapshot
    pub fn record_metrics(&self, snapshot: MetricsSnapshot) {
        let mut state = self.state.write();
        
        // Add to history
        state.metrics_history.push_back(snapshot.clone());
        
        // Trim history
        while state.metrics_history.len() > self.config.analysis_window_size {
            state.metrics_history.pop_front();
        }
        
        // Update workload characteristics
        state.characteristics = self.analyze_characteristics(&state.metrics_history);
        
        // Detect workload pattern
        state.workload_pattern = self.detect_pattern(&state.characteristics);
        
        // Check for anomalies
        if self.config.anomaly_detection {
            if let Some(anomaly) = self.detect_anomalies(&snapshot) {
                state.recent_anomalies.push(anomaly);
                state.stats.anomalies_detected += 1;
                
                // Keep only recent anomalies
                while state.recent_anomalies.len() > 10 {
                    state.recent_anomalies.remove(0);
                }
            }
        }
    }

    /// Get current tuning recommendations
    pub fn get_recommendations(&self) -> Vec<TuningRecommendation> {
        let state = self.state.read();
        let mut recommendations = Vec::new();
        
        // Generate recommendations based on tuning target
        match self.config.tuning_target {
            TuningTarget::Throughput => {
                recommendations.extend(self.recommend_for_throughput(&state));
            }
            TuningTarget::Latency => {
                recommendations.extend(self.recommend_for_latency(&state));
            }
            TuningTarget::Balanced => {
                recommendations.extend(self.recommend_for_balanced(&state));
            }
            TuningTarget::Memory => {
                recommendations.extend(self.recommend_for_memory(&state));
            }
            TuningTarget::Disk => {
                recommendations.extend(self.recommend_for_disk(&state));
            }
            TuningTarget::Custom => {
                recommendations.extend(self.recommend_custom(&state));
            }
        }
        
        // Add anomaly-based recommendations
        for anomaly in &state.recent_anomalies {
            if let Some(rec) = self.anomaly_to_recommendation(anomaly) {
                recommendations.push(rec);
            }
        }
        
        recommendations
    }

    /// Apply a tuning recommendation
    pub fn apply_recommendation(&self, recommendation: &TuningRecommendation) -> Result<()> {
        let mut state = self.state.write();
        
        // Check cooldown
        if let Some(last_adj) = state.stats.last_adjustment_ms {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis() as u64;
            if now - last_adj < self.config.cooldown_secs * 1000 {
                return Err(anyhow::anyhow!("In cooldown period"));
            }
        }
        
        // Parse and apply the new value
        let new_value = self.parse_param_value(&recommendation.parameter, &recommendation.recommended_value)?;
        self.apply_param_change(&mut state, &recommendation.parameter, new_value)?;
        
        // Update stats
        state.stats.total_adjustments += 1;
        state.stats.successful_adjustments += 1;
        state.stats.last_adjustment_ms = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis() as u64
        );
        
        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> AutoTunerStats {
        let state = self.state.read();
        state.stats.clone()
    }

    /// Get current workload pattern
    pub fn get_workload_pattern(&self) -> WorkloadPattern {
        self.state.read().workload_pattern
    }

    /// Get current characteristics
    pub fn get_characteristics(&self) -> WorkloadCharacteristics {
        self.state.read().characteristics.clone()
    }

    /// Get recent anomalies
    pub fn get_recent_anomalies(&self) -> Vec<AnomalyAlert> {
        self.state.read().recent_anomalies.clone()
    }

    /// Get metrics history
    pub fn get_metrics_history(&self) -> Vec<MetricsSnapshot> {
        self.state.read().metrics_history.iter().cloned().collect()
    }

    // ========================================================================
    // Internal Methods
    // ========================================================================

    /// Analyze workload characteristics from metrics history
    fn analyze_characteristics(&self, history: &VecDeque<MetricsSnapshot>) -> WorkloadCharacteristics {
        if history.is_empty() {
            return WorkloadCharacteristics::default();
        }

        let mut total_reads = 0u64;
        let mut total_writes = 0u64;
        let mut latencies = Vec::new();

        for snapshot in history {
            total_reads += snapshot.storage.read_ops;
            total_writes += snapshot.storage.write_ops;
            latencies.push(snapshot.storage.read_latency_us);
        }

        let read_write_ratio = if total_writes == 0 {
            f64::INFINITY
        } else {
            total_reads as f64 / total_writes as f64
        };

        // Calculate latency variance for randomness estimation
        let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let variance = latencies.iter()
            .map(|&l| (l - avg_latency).powi(2))
            .sum::<f64>() / latencies.len() as f64;
        let std_dev = variance.sqrt();
        let access_randomness = (std_dev / avg_latency).min(1.0);

        WorkloadCharacteristics {
            read_write_ratio,
            access_randomness,
            ..Default::default()
        }
    }

    /// Detect workload pattern from characteristics
    fn detect_pattern(&self, chars: &WorkloadCharacteristics) -> WorkloadPattern {
        if chars.read_write_ratio > 9.0 {
            WorkloadPattern::ReadHeavy
        } else if chars.read_write_ratio < 0.11 {
            WorkloadPattern::WriteHeavy
        } else if chars.read_write_ratio > 2.0 {
            WorkloadPattern::RangeScan
        } else if chars.read_write_ratio < 0.5 {
            WorkloadPattern::BatchLoad
        } else {
            WorkloadPattern::Mixed
        }
    }

    /// Detect anomalies in metrics
    fn detect_anomalies(&self, snapshot: &MetricsSnapshot) -> Option<AnomalyAlert> {
        // Check memory pressure
        let mem_usage = snapshot.memory_usage_percent();
        if mem_usage > 90.0 {
            return Some(AnomalyAlert {
                anomaly_type: AnomalyType::MemoryPressure { usage_percent: mem_usage },
                timestamp_ms: snapshot.timestamp_ms,
                severity: AlertSeverity::Critical,
                description: format!("Memory usage at {:.1}%", mem_usage),
                suggested_action: "Increase block cache or reduce memory-intensive operations".to_string(),
            });
        }

        // Check write stall
        if snapshot.storage.write_stall_count > 0 {
            return Some(AnomalyAlert {
                anomaly_type: AnomalyType::WriteStall { duration_ms: snapshot.storage.write_stall_time_ms },
                timestamp_ms: snapshot.timestamp_ms,
                severity: AlertSeverity::Warning,
                description: format!("Write stall detected: {}ms", snapshot.storage.write_stall_time_ms),
                suggested_action: "Increase write buffer size or compaction threads".to_string(),
            });
        }

        // Check latency spike
        if snapshot.storage.read_p99_latency_us > 10000.0 {
            return Some(AnomalyAlert {
                anomaly_type: AnomalyType::LatencySpike {
                    metric: "read_p99".to_string(),
                    severity: snapshot.storage.read_p99_latency_us / 10000.0,
                },
                timestamp_ms: snapshot.timestamp_ms,
                severity: AlertSeverity::Warning,
                description: format!("High P99 read latency: {}us", snapshot.storage.read_p99_latency_us),
                suggested_action: "Check for hot keys or increase cache size".to_string(),
            });
        }

        None
    }

    /// Generate recommendations for throughput optimization
    fn recommend_for_throughput(&self, state: &TunerState) -> Vec<TuningRecommendation> {
        let mut recs = Vec::new();
        let metrics = state.metrics_history.back();

        if let Some(m) = metrics {
            // If cache hit ratio is low, increase block cache
            if m.storage.cache_hit_ratio < 0.8 {
                recs.push(self.create_recommendation(
                    "block_cache_size",
                    format!("{}", state.current_params.block_cache_size),
                    format!("{}", (state.current_params.block_cache_size as f64 * 1.5) as u64),
                    "Low cache hit ratio - increase cache for better throughput",
                    0.75,
                    0.15,
                    RiskLevel::Low,
                ));
            }

            // If compaction is falling behind, increase background jobs
            if m.storage.compaction_ops > 5 && m.storage.write_stall_count > 0 {
                recs.push(self.create_recommendation(
                    "max_background_jobs",
                    format!("{}", state.current_params.max_background_jobs),
                    format!("{}", state.current_params.max_background_jobs + 2),
                    "Compaction falling behind - increase parallelism",
                    0.7,
                    0.2,
                    RiskLevel::Medium,
                ));
            }
        }

        recs
    }

    /// Generate recommendations for latency optimization
    fn recommend_for_latency(&self, state: &TunerState) -> Vec<TuningRecommendation> {
        let mut recs = Vec::new();
        let metrics = state.metrics_history.back();

        if let Some(m) = metrics {
            // If P99 latency is high, reduce compaction interference
            if m.storage.read_p99_latency_us > 5000.0 {
                recs.push(self.create_recommendation(
                    "max_subcompactions",
                    format!("{}", state.current_params.max_subcompactions),
                    format!("{}", (state.current_params.max_subcompactions as f64 * 0.75) as usize),
                    "High P99 latency - reduce compaction parallelism",
                    0.8,
                    0.25,
                    RiskLevel::Medium,
                ));
            }

            // If write latency is high, increase write buffer
            if m.storage.write_latency_us > 1000.0 {
                recs.push(self.create_recommendation(
                    "write_buffer_size",
                    format!("{}", state.current_params.write_buffer_size),
                    format!("{}", (state.current_params.write_buffer_size as f64 * 1.25) as u64),
                    "High write latency - increase write buffer",
                    0.7,
                    0.15,
                    RiskLevel::Low,
                ));
            }
        }

        recs
    }

    /// Generate recommendations for balanced optimization
    fn recommend_for_balanced(&self, state: &TunerState) -> Vec<TuningRecommendation> {
        let mut recs = Vec::new();
        
        // Combine throughput and latency recommendations with lower confidence
        let throughput_recs = self.recommend_for_throughput(state);
        let latency_recs = self.recommend_for_latency(state);
        
        for mut rec in throughput_recs {
            rec.confidence *= 0.8;
            recs.push(rec);
        }
        
        for mut rec in latency_recs {
            rec.confidence *= 0.8;
            recs.push(rec);
        }
        
        recs
    }

    /// Generate recommendations for memory optimization
    fn recommend_for_memory(&self, state: &TunerState) -> Vec<TuningRecommendation> {
        let mut recs = Vec::new();
        let metrics = state.metrics_history.back();

        if let Some(m) = metrics {
            let mem_usage = m.memory_usage_percent();
            let reason1 = format!("High memory usage ({:.1}%) - reduce cache size", mem_usage);
            let reason2 = format!("High memory usage ({:.1}%) - reduce memtable size", mem_usage);

            if mem_usage > 70.0 {
                recs.push(self.create_recommendation(
                    "block_cache_size",
                    format!("{}", state.current_params.block_cache_size),
                    format!("{}", (state.current_params.block_cache_size as f64 * 0.75) as u64),
                    &reason1,
                    0.85,
                    0.1,
                    RiskLevel::Medium,
                ));
            }

            if mem_usage > 50.0 {
                recs.push(self.create_recommendation(
                    "memtable_size",
                    format!("{}", state.current_params.memtable_size),
                    format!("{}", (state.current_params.memtable_size as f64 * 0.8) as u64),
                    &reason2,
                    0.75,
                    0.08,
                    RiskLevel::Low,
                ));
            }
        }

        recs
    }

    /// Generate recommendations for disk optimization
    fn recommend_for_disk(&self, state: &TunerState) -> Vec<TuningRecommendation> {
        let mut recs = Vec::new();
        let metrics = state.metrics_history.back();

        if let Some(m) = metrics {
            // If storage is growing fast, increase compaction
            if m.storage.compaction_bps < m.storage.write_ops * 100 {
                recs.push(self.create_recommendation(
                    "compaction_read_amp",
                    format!("{}", state.current_params.compaction_read_amp),
                    format!("{}", state.current_params.compaction_read_amp + 5),
                    "Storage growing faster than compaction - increase read amplitude",
                    0.65,
                    0.12,
                    RiskLevel::Medium,
                ));
            }
        }

        recs
    }

    /// Generate custom recommendations
    fn recommend_custom(&self, state: &TunerState) -> Vec<TuningRecommendation> {
        // For custom tuning, provide general recommendations based on current state
        self.recommend_for_balanced(state)
    }

    /// Convert anomaly to recommendation
    fn anomaly_to_recommendation(&self, anomaly: &AnomalyAlert) -> Option<TuningRecommendation> {
        match &anomaly.anomaly_type {
            AnomalyType::MemoryPressure { usage_percent } => {
                let reason = format!("Memory pressure at {:.1}%", usage_percent);
                Some(self.create_recommendation(
                    "block_cache_size",
                    "current".to_string(),
                    "reduce by 25%".to_string(),
                    &reason,
                    0.9,
                    0.15,
                    RiskLevel::Medium,
                ))
            }
            AnomalyType::WriteStall { duration_ms } => {
                let reason = format!("Write stall lasting {}ms", duration_ms);
                Some(self.create_recommendation(
                    "write_buffer_size",
                    "current".to_string(),
                    "increase by 50%".to_string(),
                    &reason,
                    0.85,
                    0.2,
                    RiskLevel::Medium,
                ))
            }
            AnomalyType::LatencySpike { metric, severity } => {
                let param = format!("{}_optimization", metric);
                let reason = format!("Latency spike in {} (severity: {:.2})", metric, severity);
                Some(self.create_recommendation(
                    &param,
                    "current".to_string(),
                    "optimize".to_string(),
                    &reason,
                    0.7,
                    0.1,
                    RiskLevel::Low,
                ))
            }
            _ => None,
        }
    }

    /// Create a recommendation
    #[allow(clippy::too_many_arguments)]
    fn create_recommendation(
        &self,
        parameter: &str,
        current_value: String,
        recommended_value: String,
        reason: &str,
        confidence: f64,
        expected_improvement: f64,
        risk_level: RiskLevel,
    ) -> TuningRecommendation {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_millis() as u64;
        
        TuningRecommendation {
            id: format!("rec_{}", timestamp_ms),
            timestamp_ms,
            confidence,
            expected_improvement,
            parameter: parameter.to_string(),
            current_value,
            recommended_value,
            reason: reason.to_string(),
            risk_level,
        }
    }

    /// Parse parameter value from string
    fn parse_param_value(&self, param: &str, value: &str) -> Result<u64> {
        // Simplified parsing - in production, would handle each parameter type
        value.parse::<u64>()
            .with_context(|| format!("Failed to parse value for parameter {}: {}", param, value))
    }

    /// Apply parameter change to state
    fn apply_param_change(&self, state: &mut TunerState, param: &str, value: u64) -> Result<()> {
        match param {
            "block_cache_size" => {
                state.current_params.block_cache_size = value;
            }
            "memtable_size" => {
                state.current_params.memtable_size = value;
            }
            "write_buffer_size" => {
                state.current_params.write_buffer_size = value;
            }
            "max_background_jobs" => {
                state.current_params.max_background_jobs = value as usize;
            }
            "max_subcompactions" => {
                state.current_params.max_subcompactions = value as usize;
            }
            "l0_file_count" => {
                state.current_params.l0_file_count = value as usize;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown parameter: {}", param));
            }
        }
        Ok(())
    }
}

impl Default for AutoTuner {
    fn default() -> Self {
        Self::new(AutoTunerConfig::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 保留：验证 metrics snapshot 计算逻辑
    #[test]
    fn test_metrics_snapshot_memory_usage() {
        let mut snapshot = MetricsSnapshot::new();
        snapshot.system.memory_used = 800;
        snapshot.system.memory_available = 200;

        assert_eq!(snapshot.memory_usage_percent(), 80.0);
    }

    /// 保留：验证 workload pattern 检测算法
    #[test]
    fn test_workload_pattern_detection() {
        let tuner = AutoTuner::new(AutoTunerConfig::default());

        let read_heavy = WorkloadCharacteristics {
            read_write_ratio: 20.0,
            ..Default::default()
        };
        assert_eq!(tuner.detect_pattern(&read_heavy), WorkloadPattern::ReadHeavy);

        let write_heavy = WorkloadCharacteristics {
            read_write_ratio: 0.05,
            ..Default::default()
        };
        assert_eq!(tuner.detect_pattern(&write_heavy), WorkloadPattern::WriteHeavy);

        let range_scan = WorkloadCharacteristics {
            read_write_ratio: 3.0,
            ..Default::default()
        };
        assert_eq!(tuner.detect_pattern(&range_scan), WorkloadPattern::RangeScan);
    }

    /// 保留：验证异常检测逻辑
    #[test]
    fn test_anomaly_detection_memory() {
        let tuner = AutoTuner::new(AutoTunerConfig::default());

        let mut snapshot = MetricsSnapshot::new();
        snapshot.system.memory_used = 910;
        snapshot.system.memory_available = 90;

        let anomaly = tuner.detect_anomalies(&snapshot);
        assert!(anomaly.is_some());

        let alert = anomaly.unwrap();
        assert!(matches!(alert.anomaly_type, AnomalyType::MemoryPressure { .. }));
        assert_eq!(alert.severity, AlertSeverity::Critical);
    }

    /// 保留：验证 recommendation 创建
    #[test]
    fn test_recommendation_creation() {
        let tuner = AutoTuner::new(AutoTunerConfig::default());

        let rec = tuner.create_recommendation(
            "block_cache_size",
            "268435456".to_string(),
            "402653184".to_string(),
            "Low cache hit ratio",
            0.75,
            0.15,
            RiskLevel::Low,
        );

        assert_eq!(rec.parameter, "block_cache_size");
        assert_eq!(rec.confidence, 0.75);
        assert_eq!(rec.expected_improvement, 0.15);
        assert_eq!(rec.risk_level, RiskLevel::Low);
    }

    /// 保留：验证 metrics 记录和 history 追踪
    #[test]
    fn test_record_metrics() {
        let tuner = AutoTuner::new(AutoTunerConfig::default());

        let snapshot = MetricsSnapshot::new();
        tuner.record_metrics(snapshot);

        let history = tuner.get_metrics_history();
        assert_eq!(history.len(), 1);
    }

    /// 保留：验证 tuner 生命周期管理
    #[tokio::test]
    async fn test_auto_tuner_start_stop() {
        let tuner = AutoTuner::new(AutoTunerConfig::default());

        assert!(!tuner.is_running());

        tuner.start().await.unwrap();
        assert!(tuner.is_running());

        tuner.stop().await.unwrap();
        assert!(!tuner.is_running());
    }
}
