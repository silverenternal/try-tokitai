//! Compaction Trigger Strategies
//!
//! This module defines various strategies for triggering compaction:
//! - WriteCount: Trigger after N writes
//! - SizeThreshold: Trigger when total size exceeds threshold
//! - LevelBased: Trigger when a level exceeds its size budget
//! - TimeBased: Trigger after a time interval
//! - Composite: Trigger when ANY sub-trigger fires

use std::time::{Duration, Instant};

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
                        format!("Total size {} bytes exceeds threshold {} bytes",
                            state.total_size_bytes, max_bytes),
                    )
                } else {
                    TriggerResult::none()
                }
            }
            Self::LevelBased { l0_max_files } => {
                if state.l0_file_count >= *l0_max_files {
                    TriggerResult::triggered(
                        TriggerType::LevelBased,
                        format!("L0 file count {} exceeds threshold {}",
                            state.l0_file_count, l0_max_files),
                    )
                } else {
                    TriggerResult::none()
                }
            }
            Self::TimeBased { interval, last_triggered } => {
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
    pub fn new(
        writes_since_last_check: usize,
        total_size_bytes: u64,
        l0_file_count: usize,
    ) -> Self {
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
}
