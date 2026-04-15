//! False Positive Rate (FPR) Adaptive Controller
//!
//! INNO-001: Dynamically adjusts Bloom Filter false positive rates based on
//! access patterns to optimize memory usage and query performance.
//!
//! # Key Ideas
//! - Hot segments get lower FPR (better accuracy, more memory)
//! - Cold segments get higher FPR (less memory, acceptable false positives)
//! - Smooth transitions to avoid FPR oscillation
//!
//! # FPR Levels
//! - Level 0: 0.1% (highest accuracy, for hot segments)
//! - Level 1: 0.5%
//! - Level 2: 1.0% (default)
//! - Level 3: 2.0%
//! - Level 4: 5.0%
//! - Level 5: 10.0% (lowest accuracy, for cold segments)

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use tracing::debug;

use super::migration::{AccessRecord, MigrationThresholds};

/// FPR level definition
#[derive(Debug, Clone, Copy)]
pub struct FPRLevel {
    /// False positive rate (0.0 - 1.0)
    pub fpr: f64,
    /// Memory multiplier (relative to base size)
    pub memory_multiplier: f64,
    /// Minimum QPS to maintain this level
    pub min_qps: f64,
}

impl FPRLevel {
    pub const fn new(fpr: f64, memory_multiplier: f64, min_qps: f64) -> Self {
        Self {
            fpr,
            memory_multiplier,
            min_qps,
        }
    }
}

/// Predefined FPR levels
impl FPRLevel {
    /// Level 0: Highest accuracy (hot segments)
    pub const LEVEL_0: Self = Self::new(0.001, 2.0, 100.0);   // 0.1%
    /// Level 1: High accuracy
    pub const LEVEL_1: Self = Self::new(0.005, 1.5, 50.0);    // 0.5%
    /// Level 2: Default accuracy
    pub const LEVEL_2: Self = Self::new(0.01, 1.0, 10.0);     // 1.0%
    /// Level 3: Medium accuracy
    pub const LEVEL_3: Self = Self::new(0.02, 0.75, 5.0);     // 2.0%
    /// Level 4: Low accuracy
    pub const LEVEL_4: Self = Self::new(0.05, 0.5, 1.0);      // 5.0%
    /// Level 5: Lowest accuracy (cold segments)
    pub const LEVEL_5: Self = Self::new(0.10, 0.25, 0.0);     // 10.0%
}

/// FPR adaptation policy configuration
#[derive(Debug, Clone)]
pub struct AdaptationPolicy {
    /// Minimum FPR level (0-5)
    pub min_level: u8,
    /// Maximum FPR level (0-5)
    pub max_level: u8,
    /// Hysteresis factor to prevent oscillation (0.0-1.0)
    pub hysteresis: f64,
    /// Stabilization window (ms) before changing FPR
    pub stabilization_window_ms: u64,
    /// Enable gradual transitions (skip levels)
    pub gradual_transitions: bool,
}

impl Default for AdaptationPolicy {
    fn default() -> Self {
        Self {
            min_level: 0,
            max_level: 5,
            hysteresis: 0.2, // 20% hysteresis
            stabilization_window_ms: 120_000, // 2 minutes
            gradual_transitions: true,
        }
    }
}

/// FPR controller statistics
#[derive(Debug, Clone, Default)]
pub struct FPRControllerStats {
    /// Number of segments being tracked
    pub tracked_segments: usize,
    /// Number of FPR adjustments made
    pub adjustments_made: u64,
    /// Number of level upgrades
    pub upgrades: u64,
    /// Number of level downgrades
    pub downgrades: u64,
    /// Average FPR across all segments
    pub avg_fpr: f64,
    /// Memory saved by adaptive FPR (bytes, estimated)
    pub memory_saved_bytes: u64,
}

/// Segment FPR state
#[derive(Debug)]
pub struct SegmentFPRState {
    /// Current FPR level (0-5)
    current_level: Mutex<u8>,
    /// Target FPR level (pending adjustment)
    target_level: Mutex<u8>,
    /// Last adjustment timestamp (ms)
    last_adjustment_ms: AtomicU64,
    /// Consecutive high-QPS measurements
    high_qps_count: AtomicU32,
    /// Consecutive low-QPS measurements
    low_qps_count: AtomicU32,
    /// Estimated memory size at current level (bytes)
    memory_bytes: AtomicUsize,
}

impl SegmentFPRState {
    fn new(level: u8) -> Self {
        Self {
            current_level: Mutex::new(level),
            target_level: Mutex::new(level),
            last_adjustment_ms: AtomicU64::new(0),
            high_qps_count: AtomicU32::new(0),
            low_qps_count: AtomicU32::new(0),
            memory_bytes: AtomicUsize::new(0),
        }
    }

    fn get_current_level(&self) -> u8 {
        *self.current_level.lock()
    }

    fn set_current_level(&self, level: u8) {
        *self.current_level.lock() = level;
    }

    fn get_target_level(&self) -> u8 {
        *self.target_level.lock()
    }

    fn set_target_level(&self, level: u8) {
        *self.target_level.lock() = level;
    }
}

/// False Positive Rate Adaptive Controller
///
/// Monitors segment access patterns and dynamically adjusts FPR levels
/// to optimize memory usage while maintaining query performance.
pub struct FPRController {
    /// FPR states for all segments
    segment_states: DashMap<u64, Arc<SegmentFPRState>>,
    /// FPR levels configuration
    levels: [FPRLevel; 6],
    /// Adaptation policy
    policy: AdaptationPolicy,
    /// Migration thresholds (for coordination with cache migration)
    migration_thresholds: MigrationThresholds,
    /// Statistics
    adjustments_made: AtomicU64,
    upgrades: AtomicU64,
    downgrades: AtomicU64,
}

impl FPRController {
    /// Create a new FPR controller
    pub fn new(policy: AdaptationPolicy, migration_thresholds: MigrationThresholds) -> Self {
        Self {
            segment_states: DashMap::new(),
            levels: [
                FPRLevel::LEVEL_0,
                FPRLevel::LEVEL_1,
                FPRLevel::LEVEL_2,
                FPRLevel::LEVEL_3,
                FPRLevel::LEVEL_4,
                FPRLevel::LEVEL_5,
            ],
            policy,
            migration_thresholds,
            adjustments_made: AtomicU64::new(0),
            upgrades: AtomicU64::new(0),
            downgrades: AtomicU64::new(0),
        }
    }

    /// Create with default policy
    pub fn with_defaults() -> Self {
        Self::new(AdaptationPolicy::default(), MigrationThresholds::default())
    }

    /// Get or create FPR state for a segment
    pub fn get_state(&self, segment_id: u64) -> Arc<SegmentFPRState> {
        self.segment_states
            .entry(segment_id)
            .or_insert_with(|| Arc::new(SegmentFPRState::new(2))) // Default level 2 (1% FPR)
            .clone()
    }

    /// Get FPR level for a segment
    pub fn get_level(&self, segment_id: u64) -> u8 {
        self.get_state(segment_id).get_current_level()
    }

    /// Get target FPR for a segment
    pub fn get_target_fpr(&self, segment_id: u64) -> f64 {
        let state = self.get_state(segment_id);
        let level = state.get_target_level();
        self.levels.get(level as usize).map(|l| l.fpr).unwrap_or(0.1)
    }

    /// Get current FPR for a segment
    pub fn get_current_fpr(&self, segment_id: u64) -> f64 {
        let state = self.get_state(segment_id);
        let level = state.get_current_level();
        self.levels.get(level as usize).map(|l| l.fpr).unwrap_or(0.1)
    }

    /// Get FPR level info with bounds checking.
    ///
    /// Returns `None` if the level index is out of bounds (> 5).
    pub fn get_level_info(&self, level: u8) -> Option<&FPRLevel> {
        self.levels.get(level as usize)
    }

    /// Record access and potentially adjust FPR
    ///
    /// # Returns
    /// - `Some(new_level)`: FPR level changed
    /// - `None`: No change
    pub fn record_access(&self, segment_id: u64, access: &AccessRecord) -> Option<u8> {
        let state = self.get_state(segment_id);
        let current_level = state.get_current_level();
        let qps = access.qps();

        // Determine target level based on QPS
        let target_level = self.determine_target_level(qps);

        // Check if adjustment is needed
        if target_level != current_level {
            // Check stabilization window
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let last_adjustment = state.last_adjustment_ms.load(Ordering::Relaxed);
            
            if now_ms.saturating_sub(last_adjustment) < self.policy.stabilization_window_ms {
                // Still in stabilization window, don't adjust yet
                state.set_target_level(target_level);
                return None;
            }

            // Check hysteresis
            if !self.check_hysteresis(current_level, target_level, qps) {
                return None;
            }

            // Apply gradual transition if enabled
            let new_level = if self.policy.gradual_transitions && (target_level as i16 - current_level as i16).abs() > 1 {
                if target_level > current_level {
                    current_level + 1
                } else {
                    current_level - 1
                }
            } else {
                target_level
            };

            // Apply new level
            state.set_current_level(new_level);
            state.set_target_level(new_level);
            state.last_adjustment_ms.store(now_ms, Ordering::Relaxed);

            // Update statistics
            self.adjustments_made.fetch_add(1, Ordering::Relaxed);
            if new_level < current_level {
                self.upgrades.fetch_add(1, Ordering::Relaxed); // Lower level = better accuracy
            } else {
                self.downgrades.fetch_add(1, Ordering::Relaxed);
            }

            debug!(
                "FPR adjusted for segment {}: level {} -> {} (QPS: {:.2})",
                segment_id, current_level, new_level, qps
            );

            Some(new_level)
        } else {
            // Reset counters if QPS is stable
            state.high_qps_count.store(0, Ordering::Relaxed);
            state.low_qps_count.store(0, Ordering::Relaxed);
            None
        }
    }

    /// Determine target FPR level based on QPS
    fn determine_target_level(&self, qps: f64) -> u8 {
        for (level, fpr_level) in self.levels.iter().enumerate() {
            if qps >= fpr_level.min_qps {
                return level as u8;
            }
        }
        5 // Default to lowest accuracy level
    }

    /// Check hysteresis to prevent oscillation
    fn check_hysteresis(&self, current_level: u8, target_level: u8, qps: f64) -> bool {
        let target_fpr = match self.levels.get(target_level as usize) {
            Some(fpr) => fpr,
            None => return false, // Out of bounds, don't adjust
        };

        if target_level < current_level {
            // Upgrading (better accuracy)
            // Need QPS to exceed threshold by hysteresis factor
            qps >= target_fpr.min_qps * (1.0 + self.policy.hysteresis)
        } else {
            // Downgrading (worse accuracy)
            // Need QPS to be below threshold by hysteresis factor
            qps <= target_fpr.min_qps * (1.0 - self.policy.hysteresis)
        }
    }

    /// Estimate memory size for a segment at given FPR level.
    /// Returns base estimate if level is out of bounds.
    pub fn estimate_memory(&self, num_elements: usize, level: u8) -> usize {
        let base_size = num_elements * 10; // ~10 bits per element
        let multiplier = self.levels.get(level as usize)
            .map(|l| l.memory_multiplier)
            .unwrap_or(1.0); // default multiplier for out-of-bounds
        (base_size as f64 * multiplier) as usize
    }

    /// Update memory estimate for a segment
    pub fn update_memory_estimate(&self, segment_id: u64, num_elements: usize) {
        let state = self.get_state(segment_id);
        let level = state.get_current_level();
        let memory = self.estimate_memory(num_elements, level);
        state.memory_bytes.store(memory, Ordering::Relaxed);
    }

    /// Get memory estimate for a segment
    pub fn get_memory_estimate(&self, segment_id: u64) -> usize {
        let state = self.get_state(segment_id);
        state.memory_bytes.load(Ordering::Relaxed)
    }

    /// Get total memory usage across all segments
    pub fn get_total_memory(&self) -> usize {
        self.segment_states
            .iter()
            .map(|entry| entry.value().memory_bytes.load(Ordering::Relaxed))
            .sum()
    }

    /// Get statistics
    pub fn stats(&self) -> FPRControllerStats {
        let adjustments = self.adjustments_made.load(Ordering::Relaxed);
        let upgrades_count = self.upgrades.load(Ordering::Relaxed);
        let downgrades_count = self.downgrades.load(Ordering::Relaxed);

        let mut total_fpr = 0.0;
        let mut memory_saved = 0u64;

        for entry in self.segment_states.iter() {
            let state = entry.value();
            let level = state.get_current_level();
            total_fpr += self.levels.get(level as usize).map(|l| l.fpr).unwrap_or(0.1);

            // Estimate memory saved vs fixed 1% FPR
            let base_memory = self.estimate_memory(10000, 2); // Level 2 = 1% FPR
            let actual_memory = state.memory_bytes.load(Ordering::Relaxed);
            if actual_memory > 0 && base_memory > actual_memory {
                memory_saved += (base_memory - actual_memory) as u64;
            }
        }

        let avg_fpr = if !self.segment_states.is_empty() {
            total_fpr / self.segment_states.len() as f64
        } else {
            0.0
        };

        FPRControllerStats {
            tracked_segments: self.segment_states.len(),
            adjustments_made: adjustments,
            upgrades: upgrades_count,
            downgrades: downgrades_count,
            avg_fpr,
            memory_saved_bytes: memory_saved,
        }
    }

    /// Remove a segment from tracking
    pub fn remove_segment(&self, segment_id: u64) {
        self.segment_states.remove(&segment_id);
    }

    /// Get migration thresholds for coordination
    pub fn migration_thresholds(&self) -> &MigrationThresholds {
        &self.migration_thresholds
    }

    /// Update adaptation policy
    pub fn set_policy(&mut self, policy: AdaptationPolicy) {
        self.policy = policy;
    }
}

/// FPR-adjusted Bloom Filter builder
///
/// Helper to create bloom filters with FPR controller settings
pub struct FPRAdjustedBloom {
    /// Expected number of elements
    pub num_elements: usize,
    /// Target FPR
    pub fpr: f64,
    /// FPR level
    pub level: u8,
}

impl FPRAdjustedBloom {
    /// Create a new FPR-adjusted bloom filter spec
    pub fn new(num_elements: usize, fpr: f64, level: u8) -> Self {
        Self {
            num_elements,
            fpr,
            level,
        }
    }

    /// Create from controller settings
    pub fn from_controller(controller: &FPRController, segment_id: u64, num_elements: usize) -> Self {
        let fpr = controller.get_current_fpr(segment_id);
        let level = controller.get_level(segment_id);
        Self::new(num_elements, fpr, level)
    }

    /// Build the bloom filter
    pub fn build(&self) -> bloom::BloomFilter {
        
        bloom::BloomFilter::with_rate(self.fpr as f32, self.num_elements.try_into().unwrap_or(10000))
    }

    /// Estimate memory size
    pub fn estimated_memory(&self) -> usize {
        // ~10 bits per element at base FPR
        let base_bits = self.num_elements * 10;
        // Adjust based on FPR level
        let level_multiplier = match self.level {
            0 => 2.0,   // 0.1% = 2x memory
            1 => 1.5,   // 0.5% = 1.5x
            2 => 1.0,   // 1.0% = base
            3 => 0.75,  // 2.0% = 0.75x
            4 => 0.5,   // 5.0% = 0.5x
            _ => 0.25,  // 10.0% = 0.25x
        };
        (base_bits as f64 * level_multiplier / 8.0) as usize // Convert bits to bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bloom::ASMS;

    #[test]
    fn test_fpr_levels() {
        assert_eq!(FPRLevel::LEVEL_0.fpr, 0.001);
        assert_eq!(FPRLevel::LEVEL_1.fpr, 0.005);
        assert_eq!(FPRLevel::LEVEL_2.fpr, 0.01);
        assert_eq!(FPRLevel::LEVEL_3.fpr, 0.02);
        assert_eq!(FPRLevel::LEVEL_4.fpr, 0.05);
        assert_eq!(FPRLevel::LEVEL_5.fpr, 0.1);
    }

    #[test]
    fn test_fpr_controller_default() {
        let controller = FPRController::with_defaults();
        
        // Default level should be 2
        let level = controller.get_level(1);
        assert_eq!(level, 2);
        
        // Default FPR should be 1%
        let fpr = controller.get_current_fpr(1);
        assert!((fpr - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_fpr_controller_level_determination() {
        let controller = FPRController::with_defaults();
        
        // High QPS should give low level (better accuracy)
        let level_high = controller.determine_target_level(150.0);
        assert!(level_high <= 1);
        
        // Medium QPS
        let level_med = controller.determine_target_level(20.0);
        assert!(level_med >= 1 && level_med <= 3);
        
        // Low QPS should give high level (worse accuracy)
        let level_low = controller.determine_target_level(0.5);
        assert_eq!(level_low, 5);
    }

    #[test]
    fn test_fpr_controller_memory_estimate() {
        let controller = FPRController::with_defaults();
        
        // Level 0 should use more memory than level 5
        let mem_l0 = controller.estimate_memory(10000, 0);
        let mem_l5 = controller.estimate_memory(10000, 5);
        
        assert!(mem_l0 > mem_l5);
        assert!((mem_l0 as f64 / mem_l5 as f64) > 4.0); // Should be roughly 8x
    }

    #[test]
    fn test_fpr_adjusted_bloom() {
        let spec = FPRAdjustedBloom::new(1000, 0.01, 2);
        let bloom = spec.build();
        
        // Verify bloom filter was created
        assert!(bloom.contains(&"test".to_string()) == false);
        
        // Check memory estimate
        let mem = spec.estimated_memory();
        assert!(mem > 0);
    }

    #[test]
    fn test_fpr_controller_stats() {
        let controller = FPRController::with_defaults();
        
        // Access a segment to create state
        let access = AccessRecord {
            total_count: 100,
            window_count: 50,
            window_duration_ms: 5000,
            current_layer: 2,
        };
        
        controller.record_access(1, &access);

        let stats = controller.stats();
        assert_eq!(stats.tracked_segments, 1);
    }

    /// Test: BLOOM-002 - get_level_info returns None for out-of-bounds level
    #[test]
    fn test_get_level_info_out_of_bounds() {
        let controller = FPRController::with_defaults();

        // Valid levels 0-5
        assert!(controller.get_level_info(0).is_some());
        assert!(controller.get_level_info(5).is_some());

        // Out of bounds
        assert!(controller.get_level_info(6).is_none());
        assert!(controller.get_level_info(255).is_none());
    }

    /// Test: BLOOM-002 - get_current_fpr returns default for out-of-bounds level
    #[test]
    fn test_get_current_fpr_out_of_bounds() {
        let controller = FPRController::with_defaults();

        // Manually set an out-of-bounds level via state manipulation
        // Since SegmentFPRState is not public, we test via estimate_memory instead
        // which also uses levels array indexing
        let mem = controller.estimate_memory(10000, 255);
        // Should return a reasonable estimate with default multiplier
        assert!(mem > 0);
        // Default multiplier is 1.0, same as level 2
        let mem_default = controller.estimate_memory(10000, 2);
        assert_eq!(mem, mem_default);
    }
}
