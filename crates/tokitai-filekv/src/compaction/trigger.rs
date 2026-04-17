//! Compaction Trigger Strategies
//!
//! This module defines various strategies for triggering compaction:
//! - WriteCount: Trigger after N writes
//! - SizeThreshold: Trigger when total size exceeds threshold
//! - LevelBased: Trigger when a level exceeds its size budget
//! - TimeBased: Trigger after a time interval
//! - Composite: Trigger when ANY sub-trigger fires
//! - WriteAmplificationAware: WA-aware trigger with I/O bandwidth awareness (OPT-003)

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Compaction priority levels based on WA and I/O pressure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionPriority {
    /// No compaction needed
    None,
    /// Low priority: WA < 2.0x, can compact aggressively
    Low,
    /// Normal priority: WA 2.0x ~ 3.0x, conservative compaction
    Normal,
    /// High priority: WA > 3.0x or L0 segments exceed emergency threshold
    High,
    /// Urgent: L0 segments critical, must compact immediately
    Urgent,
}

/// I/O pressure indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPressure {
    /// Low I/O pressure, compaction can run freely
    Low,
    /// Medium I/O pressure, compaction should throttle
    Medium,
    /// High I/O pressure, compaction should pause
    High,
}

/// Write amplification awareness state
#[derive(Debug, Clone)]
pub struct WaAwareState {
    /// Current write amplification factor
    pub write_amplification: f64,
    /// Current I/O pressure level
    pub io_pressure: IoPressure,
    /// Current write queue depth (pending writes)
    pub write_queue_depth: usize,
    /// Recent write latency (microseconds, P99)
    pub write_latency_p99_us: u64,
    /// L0 segment count
    pub l0_segment_count: usize,
    /// L0 total size in bytes
    pub l0_total_size_bytes: u64,
}

impl Default for WaAwareState {
    fn default() -> Self {
        Self {
            write_amplification: 1.0,
            io_pressure: IoPressure::Low,
            write_queue_depth: 0,
            write_latency_p99_us: 0,
            l0_segment_count: 0,
            l0_total_size_bytes: 0,
        }
    }
}

/// I/O bandwidth tracker for monitoring write pressure
pub struct IoPressureTracker {
    /// Current write queue depth (pending operations)
    write_queue_depth: AtomicU64,
    /// Recent write latency samples (microseconds) - ring buffer index
    latency_samples: Mutex<Vec<u64>>,
    /// Maximum samples to keep for P99 calculation
    max_latency_samples: usize,
    /// Whether compaction should pause due to high I/O pressure
    compaction_paused: AtomicBool,
    /// Last time I/O pressure was evaluated
    last_evaluation: Mutex<Instant>,
}

impl IoPressureTracker {
    /// Create a new I/O pressure tracker
    pub fn new(max_latency_samples: usize) -> Self {
        Self {
            write_queue_depth: AtomicU64::new(0),
            latency_samples: Mutex::new(Vec::with_capacity(max_latency_samples)),
            max_latency_samples,
            compaction_paused: AtomicBool::new(false),
            last_evaluation: Mutex::new(Instant::now()),
        }
    }

    /// Record a write operation starting
    pub fn record_write_start(&self) {
        self.write_queue_depth.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a write operation completing with latency
    pub fn record_write_complete(&self, latency_us: u64) {
        self.write_queue_depth.fetch_sub(1, Ordering::Relaxed);

        let mut samples = self.latency_samples.lock();
        if samples.len() >= self.max_latency_samples {
            // Remove oldest sample
            samples.remove(0);
        }
        samples.push(latency_us);
    }

    /// Get current write queue depth
    pub fn queue_depth(&self) -> usize {
        self.write_queue_depth.load(Ordering::Relaxed) as usize
    }

    /// Calculate P99 write latency from recent samples
    pub fn p99_latency_us(&self) -> u64 {
        let samples = self.latency_samples.lock();
        if samples.is_empty() {
            return 0;
        }

        let mut sorted = samples.clone();
        sorted.sort_unstable();

        let p99_index = (sorted.len() as f64 * 0.99).ceil() as usize - 1;
        sorted[p99_index.min(sorted.len() - 1)]
    }

    /// Evaluate current I/O pressure level
    pub fn evaluate_pressure(&self, queue_depth_threshold: usize, latency_threshold_us: u64) -> IoPressure {
        let queue_depth = self.queue_depth();
        let p99_latency = self.p99_latency_us();

        // Update evaluation time
        *self.last_evaluation.lock() = Instant::now();

        // Determine pressure level
        let pressure = if queue_depth >= queue_depth_threshold * 2 || p99_latency >= latency_threshold_us * 2 {
            IoPressure::High
        } else if queue_depth >= queue_depth_threshold || p99_latency >= latency_threshold_us {
            IoPressure::Medium
        } else {
            IoPressure::Low
        };

        // Update pause flag
        let should_pause = matches!(pressure, IoPressure::High);
        self.compaction_paused.store(should_pause, Ordering::Release);

        pressure
    }

    /// Check if compaction should pause due to I/O pressure
    pub fn should_compaction_pause(&self) -> bool {
        self.compaction_paused.load(Ordering::Acquire)
    }

    /// Force pause/resume compaction
    pub fn set_compaction_paused(&self, paused: bool) {
        self.compaction_paused.store(paused, Ordering::Release);
    }
}

impl Default for IoPressureTracker {
    fn default() -> Self {
        Self::new(1000) // Keep last 1000 samples
    }
}

/// WA-aware compaction trigger configuration
#[derive(Debug, Clone)]
pub struct WaAwareTriggerConfig {
    /// WA threshold below which compaction is aggressive (default: 2.0)
    pub wa_aggressive_threshold: f64,
    /// WA threshold above which compaction is conservative (default: 3.0)
    pub wa_conservative_threshold: f64,
    /// WA threshold above which compaction is delayed unless emergency (default: 4.0)
    pub wa_delay_threshold: f64,
    /// L0 segment count emergency threshold (default: 8)
    pub l0_emergency_threshold: usize,
    /// L0 segment count warning threshold (default: 5)
    pub l0_warning_threshold: usize,
    /// Write queue depth threshold for I/O pressure (default: 64)
    pub io_queue_depth_threshold: usize,
    /// Write P99 latency threshold in microseconds for I/O pressure (default: 100µs)
    pub io_latency_threshold_us: u64,
    /// Maximum compaction priority boost from L0 pressure (default: 2 levels)
    pub max_l0_priority_boost: u8,
}

impl Default for WaAwareTriggerConfig {
    fn default() -> Self {
        Self {
            wa_aggressive_threshold: 2.0,
            wa_conservative_threshold: 3.0,
            wa_delay_threshold: 4.0,
            l0_emergency_threshold: 8,
            l0_warning_threshold: 5,
            io_queue_depth_threshold: 64,
            io_latency_threshold_us: 100, // OPT-003 target: P99 < 100µs
            max_l0_priority_boost: 2,
        }
    }
}

/// WA-aware compaction trigger
pub struct WriteAmplificationAwareTrigger {
    config: WaAwareTriggerConfig,
    io_tracker: Arc<IoPressureTracker>,
}

impl WriteAmplificationAwareTrigger {
    /// Create a new WA-aware trigger
    pub fn new(config: WaAwareTriggerConfig, io_tracker: Arc<IoPressureTracker>) -> Self {
        Self { config, io_tracker }
    }

    /// Create with default configuration
    pub fn with_defaults(io_tracker: Arc<IoPressureTracker>) -> Self {
        Self::new(WaAwareTriggerConfig::default(), io_tracker)
    }

    /// Get the I/O pressure tracker reference
    pub fn io_tracker(&self) -> &Arc<IoPressureTracker> {
        &self.io_tracker
    }

    /// Evaluate compaction priority based on WA and I/O pressure
    ///
    /// # Arguments
    /// * `wa` - Current write amplification factor
    /// * `l0_segment_count` - Number of L0 segments
    /// * `_l0_total_size_bytes` - Total size of L0 segments in bytes (reserved for future use)
    pub fn evaluate_priority(&self, wa: f64, l0_segment_count: usize, _l0_total_size_bytes: u64) -> CompactionPriority {
        // Step 1: Determine base priority from WA
        let base_priority = if wa < self.config.wa_aggressive_threshold {
            CompactionPriority::Low
        } else if wa < self.config.wa_conservative_threshold {
            CompactionPriority::Normal
        } else if wa < self.config.wa_delay_threshold {
            CompactionPriority::High
        } else {
            CompactionPriority::Urgent
        };

        // Step 2: Adjust priority based on L0 pressure (apply before I/O pressure check)
        let l0_adjustment = if l0_segment_count >= self.config.l0_emergency_threshold {
            2 // Boost to Urgent
        } else if l0_segment_count >= self.config.l0_warning_threshold {
            1 // Boost one level
        } else {
            0
        };

        // Apply L0 adjustment first
        let priority_level = match base_priority {
            CompactionPriority::None => 0,
            CompactionPriority::Low => 1,
            CompactionPriority::Normal => 2,
            CompactionPriority::High => 3,
            CompactionPriority::Urgent => 4,
        };
        let adjusted_level = (priority_level + l0_adjustment as usize).min(4).max(priority_level);

        let l0_adjusted_priority = match adjusted_level {
            0 => CompactionPriority::None,
            1 => CompactionPriority::Low,
            2 => CompactionPriority::Normal,
            3 => CompactionPriority::High,
            _ => CompactionPriority::Urgent,
        };

        // Step 3: Check I/O pressure - may downgrade priority
        let io_pressure = self.io_tracker.evaluate_pressure(
            self.config.io_queue_depth_threshold,
            self.config.io_latency_threshold_us,
        );

        // Apply I/O pressure downgrade if needed
        match (l0_adjusted_priority, io_pressure) {
            // If I/O pressure is high, downgrade compaction priority
            (CompactionPriority::Urgent, IoPressure::High) => CompactionPriority::High,
            (CompactionPriority::High, IoPressure::High) => CompactionPriority::Normal,
            (CompactionPriority::Normal, IoPressure::High) => CompactionPriority::Low,
            (CompactionPriority::Low, IoPressure::High) => CompactionPriority::None,
            (CompactionPriority::None, IoPressure::High) => CompactionPriority::None,

            // Medium I/O pressure: conservative approach (no downgrade)
            (_, IoPressure::Medium) => l0_adjusted_priority,

            // Low I/O pressure: proceed normally
            (_, IoPressure::Low) => l0_adjusted_priority,
        }
    }

    /// Check if compaction should run based on current state
    ///
    /// # Returns
    /// (should_compact, priority, should_pause)
    pub fn should_compact(&self, state: &WaAwareState) -> (bool, CompactionPriority, bool) {
        let priority = self.evaluate_priority(
            state.write_amplification,
            state.l0_segment_count,
            state.l0_total_size_bytes,
        );

        let should_compact = !matches!(priority, CompactionPriority::None);
        let should_pause = self.io_tracker.should_compaction_pause();

        // If compaction should pause, only allow urgent compaction
        let effective_should_compact = if should_pause {
            matches!(priority, CompactionPriority::Urgent)
        } else {
            should_compact
        };

        (effective_should_compact, priority, should_pause)
    }

    /// Get compaction throttle delay based on priority and I/O pressure
    /// Higher delay = slower compaction to reduce I/O competition
    pub fn get_compaction_delay(&self, priority: CompactionPriority) -> Duration {
        let io_pressure = self.io_tracker.evaluate_pressure(
            self.config.io_queue_depth_threshold,
            self.config.io_latency_threshold_us,
        );

        match (priority, io_pressure) {
            (_, IoPressure::High) => Duration::from_millis(500), // 500ms delay when high pressure
            (CompactionPriority::Low, IoPressure::Medium) => Duration::from_millis(100),
            (CompactionPriority::Normal, IoPressure::Medium) => Duration::from_millis(50),
            (CompactionPriority::High, IoPressure::Medium) => Duration::from_millis(20),
            (CompactionPriority::Urgent, IoPressure::Medium) => Duration::from_millis(10),
            (_, IoPressure::Low) => Duration::ZERO, // No delay when low pressure
            // None priority: no delay regardless of I/O pressure
            (CompactionPriority::None, _) => Duration::ZERO,
        }
    }

    /// Build WA-aware state for evaluation
    pub fn build_state(
        &self,
        write_amplification: f64,
        l0_segment_count: usize,
        l0_total_size_bytes: u64,
    ) -> WaAwareState {
        WaAwareState {
            write_amplification,
            io_pressure: self.io_tracker.evaluate_pressure(
                self.config.io_queue_depth_threshold,
                self.config.io_latency_threshold_us,
            ),
            write_queue_depth: self.io_tracker.queue_depth(),
            write_latency_p99_us: self.io_tracker.p99_latency_us(),
            l0_segment_count,
            l0_total_size_bytes,
        }
    }
}

/// Compaction trigger strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    WriteCount,
    SizeThreshold,
    LevelBased,
    TimeBased,
    Composite,
}

/// Result of evaluating a trigger
#[derive(Debug, Clone)]
pub struct TriggerResult {
    pub should_trigger: bool,
    pub triggered_by: TriggerType,
    pub reason: String,
}

impl TriggerResult {
    pub fn none() -> Self {
        Self {
            should_trigger: false,
            triggered_by: TriggerType::WriteCount, // Default, won't be used
            reason: String::new(),
        }
    }

    pub fn triggered(by: TriggerType, reason: String) -> Self {
        Self {
            should_trigger: true,
            triggered_by: by,
            reason,
        }
    }
}

/// Compaction trigger strategies
#[derive(Debug, Clone)]
pub enum CompactionTrigger {
    /// Trigger after N writes
    WriteCount {
        /// Number of writes before triggering
        count: usize,
        /// Current write count since last trigger
        current_count: usize,
    },
    /// Trigger when total segment size exceeds threshold
    SizeThreshold {
        /// Maximum total bytes before triggering
        max_bytes: u64,
    },
    /// Trigger when L0 has too many files or a level exceeds its budget
    LevelBased {
        /// Maximum number of L0 segments before triggering
        l0_max_files: usize,
    },
    /// Trigger after a time interval
    TimeBased {
        /// Time interval between triggers
        interval: Duration,
        /// Time of last trigger (or start time)
        last_triggered: Instant,
    },
    /// Composite trigger - triggers when ANY sub-trigger fires
    Composite {
        /// Sub-triggers
        triggers: Vec<CompactionTrigger>,
    },
}

impl CompactionTrigger {
    /// Create a WriteCount trigger
    pub fn write_count(n: usize) -> Self {
        Self::WriteCount {
            count: n,
            current_count: 0,
        }
    }

    /// Create a SizeThreshold trigger
    pub fn size_threshold(max_bytes: u64) -> Self {
        Self::SizeThreshold { max_bytes }
    }

    /// Create a LevelBased trigger
    pub fn level_based(l0_max_files: usize) -> Self {
        Self::LevelBased { l0_max_files }
    }

    /// Create a TimeBased trigger
    pub fn time_based(interval: Duration) -> Self {
        Self::TimeBased {
            interval,
            last_triggered: Instant::now(),
        }
    }

    /// Create a Composite trigger
    pub fn composite(triggers: Vec<CompactionTrigger>) -> Self {
        Self::Composite { triggers }
    }

    /// Evaluate the trigger and return result if any fires
    pub fn evaluate(&mut self, state: &TriggerState) -> TriggerResult {
        match self {
            Self::WriteCount { count, current_count } => {
                *current_count += state.writes_since_last_check;
                if *current_count >= *count {
                    let result = TriggerResult::triggered(
                        TriggerType::WriteCount,
                        format!("Write count {} reached threshold {}", current_count, count),
                    );
                    *current_count = 0; // Reset counter
                    result
                } else {
                    TriggerResult::none()
                }
            }
            Self::SizeThreshold { max_bytes } => {
                if state.total_size_bytes >= *max_bytes {
                    TriggerResult::triggered(
                        TriggerType::SizeThreshold,
                        format!(
                            "Total size {} bytes exceeds threshold {} bytes",
                            state.total_size_bytes, max_bytes
                        ),
                    )
                } else {
                    TriggerResult::none()
                }
            }
            Self::LevelBased { l0_max_files } => {
                if state.l0_file_count >= *l0_max_files {
                    TriggerResult::triggered(
                        TriggerType::LevelBased,
                        format!(
                            "L0 file count {} exceeds threshold {}",
                            state.l0_file_count, l0_max_files
                        ),
                    )
                } else {
                    TriggerResult::none()
                }
            }
            Self::TimeBased {
                interval,
                last_triggered,
            } => {
                let elapsed = last_triggered.elapsed();
                if elapsed >= *interval {
                    let result = TriggerResult::triggered(
                        TriggerType::TimeBased,
                        format!("Time elapsed {:?} exceeds interval {:?}", elapsed, interval),
                    );
                    *last_triggered = Instant::now(); // Reset timer
                    result
                } else {
                    TriggerResult::none()
                }
            }
            Self::Composite { triggers } => {
                for trigger in triggers.iter_mut() {
                    let result = trigger.evaluate(state);
                    if result.should_trigger {
                        return result;
                    }
                }
                TriggerResult::none()
            }
        }
    }

    /// Reset all trigger states (after compaction completes)
    pub fn reset(&mut self) {
        match self {
            Self::WriteCount { current_count, .. } => {
                *current_count = 0;
            }
            Self::TimeBased { last_triggered, .. } => {
                *last_triggered = Instant::now();
            }
            Self::Composite { triggers } => {
                for trigger in triggers {
                    trigger.reset();
                }
            }
            _ => {}
        }
    }

    /// Get trigger type
    pub fn trigger_type(&self) -> TriggerType {
        match self {
            Self::WriteCount { .. } => TriggerType::WriteCount,
            Self::SizeThreshold { .. } => TriggerType::SizeThreshold,
            Self::LevelBased { .. } => TriggerType::LevelBased,
            Self::TimeBased { .. } => TriggerType::TimeBased,
            Self::Composite { .. } => TriggerType::Composite,
        }
    }
}

/// State information needed for trigger evaluation
#[derive(Debug, Clone, Default)]
pub struct TriggerState {
    /// Number of writes since last trigger check
    pub writes_since_last_check: usize,
    /// Total size of all segments in bytes
    pub total_size_bytes: u64,
    /// Number of L0 segments
    pub l0_file_count: usize,
}

impl TriggerState {
    pub fn new(writes_since_last_check: usize, total_size_bytes: u64, l0_file_count: usize) -> Self {
        Self {
            writes_since_last_check,
            total_size_bytes,
            l0_file_count,
        }
    }
}

/// Create default composite trigger (WriteCount(100) + LevelBased(l0_max_files: 3))
/// OPT-003: Reduced L0 threshold from 4 to 3 to trigger compaction earlier and avoid L0 buildup
pub fn default_compaction_trigger() -> CompactionTrigger {
    CompactionTrigger::composite(vec![
        CompactionTrigger::write_count(100),
        CompactionTrigger::level_based(3),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_count_trigger() {
        let mut trigger = CompactionTrigger::write_count(10);

        // Not triggered yet
        let state = TriggerState::new(5, 0, 0);
        let result = trigger.evaluate(&state);
        assert!(!result.should_trigger);

        // Trigger
        let state = TriggerState::new(5, 0, 0);
        let result = trigger.evaluate(&state);
        assert!(result.should_trigger);
        assert_eq!(result.triggered_by, TriggerType::WriteCount);

        // Counter reset after trigger
        let state = TriggerState::new(1, 0, 0);
        let result = trigger.evaluate(&state);
        assert!(!result.should_trigger);
    }

    #[test]
    fn test_size_threshold_trigger() {
        let mut trigger = CompactionTrigger::size_threshold(1000);

        // Not triggered
        let state = TriggerState::new(0, 500, 0);
        assert!(!trigger.evaluate(&state).should_trigger);

        // Triggered
        let state = TriggerState::new(0, 1500, 0);
        let result = trigger.evaluate(&state);
        assert!(result.should_trigger);
        assert_eq!(result.triggered_by, TriggerType::SizeThreshold);
    }

    #[test]
    fn test_level_based_trigger() {
        let mut trigger = CompactionTrigger::level_based(4);

        // Not triggered
        let state = TriggerState::new(0, 0, 3);
        assert!(!trigger.evaluate(&state).should_trigger);

        // Triggered
        let state = TriggerState::new(0, 0, 5);
        let result = trigger.evaluate(&state);
        assert!(result.should_trigger);
        assert_eq!(result.triggered_by, TriggerType::LevelBased);
    }

    #[test]
    fn test_time_based_trigger() {
        let mut trigger = CompactionTrigger::time_based(Duration::from_millis(100));

        // Not triggered immediately
        let state = TriggerState::default();
        assert!(!trigger.evaluate(&state).should_trigger);

        // Wait and trigger
        std::thread::sleep(Duration::from_millis(150));
        let state = TriggerState::default();
        let result = trigger.evaluate(&state);
        assert!(result.should_trigger);
        assert_eq!(result.triggered_by, TriggerType::TimeBased);
    }

    #[test]
    fn test_composite_trigger_any_fires() {
        let mut trigger = CompactionTrigger::composite(vec![
            CompactionTrigger::write_count(100), // Won't fire
            CompactionTrigger::level_based(2),   // Will fire
        ]);

        // LevelBased should fire
        let state = TriggerState::new(10, 0, 3);
        let result = trigger.evaluate(&state);
        assert!(result.should_trigger);
        assert_eq!(result.triggered_by, TriggerType::LevelBased);
    }

    #[test]
    fn test_composite_trigger_none_fires() {
        let mut trigger = CompactionTrigger::composite(vec![
            CompactionTrigger::write_count(100),
            CompactionTrigger::level_based(10),
        ]);

        let state = TriggerState::new(10, 0, 3);
        let result = trigger.evaluate(&state);
        assert!(!result.should_trigger);
    }

    #[test]
    fn test_trigger_reset() {
        let mut trigger = CompactionTrigger::write_count(10);

        // Build up counter
        let state = TriggerState::new(5, 0, 0);
        trigger.evaluate(&state);

        // Reset
        trigger.reset();

        // Should start from 0
        let state = TriggerState::new(3, 0, 0);
        assert!(!trigger.evaluate(&state).should_trigger);
    }

    #[test]
    fn test_default_compaction_trigger() {
        let mut trigger = default_compaction_trigger();

        // Should have 2 sub-triggers
        match &trigger {
            CompactionTrigger::Composite { triggers } => {
                assert_eq!(triggers.len(), 2);
            }
            _ => panic!("Expected Composite trigger"),
        }

        // Test that it fires on level based
        let state = TriggerState::new(10, 0, 5);
        let result = trigger.evaluate(&state);
        assert!(result.should_trigger);
    }

    // ========================================================================
    // OPT-003: WA-aware trigger tests
    // ========================================================================

    #[test]
    fn test_wa_aware_trigger_aggressive_priority() {
        // WA < 2.0x: should compact aggressively (Low priority)
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = WaAwareState {
            write_amplification: 1.5,
            l0_segment_count: 2,
            l0_total_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, should_pause) = trigger.should_compact(&state);
        assert!(should_compact);
        assert_eq!(priority, CompactionPriority::Low);
        assert!(!should_pause);
    }

    #[test]
    fn test_wa_aware_trigger_normal_priority() {
        // WA 2.0x ~ 3.0x: should compact conservatively (Normal priority)
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = WaAwareState {
            write_amplification: 2.5,
            l0_segment_count: 2,
            l0_total_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, _) = trigger.should_compact(&state);
        assert!(should_compact);
        assert_eq!(priority, CompactionPriority::Normal);
    }

    #[test]
    fn test_wa_aware_trigger_high_priority() {
        // WA > 3.0x: high priority compaction
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = WaAwareState {
            write_amplification: 3.5,
            l0_segment_count: 2,
            l0_total_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, _) = trigger.should_compact(&state);
        assert!(should_compact);
        assert_eq!(priority, CompactionPriority::High);
    }

    #[test]
    fn test_wa_aware_trigger_urgent_priority() {
        // WA > 4.0x: urgent priority
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = WaAwareState {
            write_amplification: 4.5,
            l0_segment_count: 2,
            l0_total_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, _) = trigger.should_compact(&state);
        assert!(should_compact);
        assert_eq!(priority, CompactionPriority::Urgent);
    }

    #[test]
    fn test_wa_aware_trigger_l0_pressure_boost() {
        // L0 segment count >= warning threshold: boost priority
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = WaAwareState {
            write_amplification: 1.5, // Normally Low
            l0_segment_count: 5,      // Warning threshold
            l0_total_size_bytes: 10 * 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, _) = trigger.should_compact(&state);
        assert!(should_compact);
        // Should be boosted from Low to Normal due to L0 pressure
        assert_eq!(priority, CompactionPriority::Normal);
    }

    #[test]
    fn test_wa_aware_trigger_l0_emergency_boost() {
        // L0 segment count >= emergency threshold: boost by 2 levels
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = WaAwareState {
            write_amplification: 1.5, // Normally Low (level 1)
            l0_segment_count: 8,      // Emergency threshold (+2 levels)
            l0_total_size_bytes: 20 * 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, _) = trigger.should_compact(&state);
        assert!(should_compact);
        // Low (1) + 2 = High (3), capped at 4 (Urgent) only if base is higher
        assert_eq!(priority, CompactionPriority::High);
    }

    #[test]
    fn test_io_pressure_tracker_queue_depth() {
        let tracker = IoPressureTracker::new(100);

        // Initial state
        assert_eq!(tracker.queue_depth(), 0);
        assert_eq!(tracker.p99_latency_us(), 0);

        // Record some writes
        for i in 0..50 {
            tracker.record_write_start();
            tracker.record_write_complete(i as u64);
        }

        assert_eq!(tracker.queue_depth(), 0); // All completed
        assert!(tracker.p99_latency_us() > 0);
    }

    #[test]
    fn test_io_pressure_high_pause_compaction() {
        let tracker = IoPressureTracker::new(100);

        // Simulate high queue depth (>= 64 * 2 = 128)
        for _ in 0..130 {
            tracker.record_write_start();
        }

        // Evaluate pressure
        let pressure = tracker.evaluate_pressure(64, 100);
        assert_eq!(pressure, IoPressure::High);
        assert!(tracker.should_compaction_pause());

        // Use the SAME tracker for the trigger (not a clone)
        let io_tracker = Arc::new(tracker);
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker.clone());

        let state = WaAwareState {
            write_amplification: 2.5, // Normal priority
            l0_segment_count: 2,
            l0_total_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let (should_compact, priority, should_pause) = trigger.should_compact(&state);
        // Normal priority should NOT proceed when I/O pressure is high
        // But due to L0 adjustment and I/O downgrade, it becomes Low
        assert!(!should_compact || priority == CompactionPriority::Low || priority == CompactionPriority::None);
        assert!(should_pause);
    }

    #[test]
    fn test_compaction_delay_throttling() {
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        // Low pressure: no delay
        let delay = trigger.get_compaction_delay(CompactionPriority::Low);
        assert_eq!(delay, Duration::ZERO);

        // Medium pressure: some delay
        // First simulate medium pressure
        let tracker = IoPressureTracker::new(100);
        for _ in 0..70 {
            // queue_depth >= 64
            tracker.record_write_start();
        }
        tracker.evaluate_pressure(64, 100);
        let io_tracker = Arc::new(tracker);
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let delay = trigger.get_compaction_delay(CompactionPriority::Normal);
        assert!(delay > Duration::ZERO);
        assert_eq!(delay, Duration::from_millis(50));
    }

    #[test]
    fn test_wa_aware_state_build() {
        let io_tracker = Arc::new(IoPressureTracker::new(100));
        let trigger = WriteAmplificationAwareTrigger::with_defaults(io_tracker);

        let state = trigger.build_state(2.5, 3, 5 * 1024 * 1024);

        assert_eq!(state.write_amplification, 2.5);
        assert_eq!(state.l0_segment_count, 3);
        assert_eq!(state.l0_total_size_bytes, 5 * 1024 * 1024);
    }
}
