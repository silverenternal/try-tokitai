//! Adaptive Segment Pre-allocation Module (P2-008)
//!
//! This module implements dynamic segment pre-allocation sizing based on write patterns.
//! Instead of using a fixed pre-allocation size (e.g., 16MB), it adapts to actual usage:
//!
//! - Tracks entry sizes and segment fill rates
//! - Uses exponential weighted moving average (EWMA) for smooth adaptation
//! - Prevents over-allocation (wasted disk space) and under-allocation (fragmentation)
//!
//! # Algorithm
//!
//! The pre-allocation size is calculated using:
//! 1. EWMA of recent segment sizes to detect write patterns
//! 2. Minimum/maximum bounds to prevent extreme values
//! 3. Adaptive adjustment based on segment utilization rate

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Configuration for adaptive pre-allocation
#[derive(Debug, Clone)]
pub struct AdaptivePreallocatorConfig {
    /// Minimum pre-allocation size (default: 1MB)
    pub min_preallocate_bytes: u64,
    /// Maximum pre-allocation size (default: 64MB)
    pub max_preallocate_bytes: u64,
    /// Initial pre-allocation size (default: 16MB)
    pub initial_preallocate_bytes: u64,
    /// EWMA smoothing factor for size adaptation (0.0-1.0, default: 0.3)
    /// Higher = more responsive to recent changes, lower = smoother
    pub ewma_alpha: f64,
    /// Number of segments to track for pattern detection (default: 10)
    pub history_size: usize,
    /// Enable adaptive mode (if false, uses fixed initial size)
    pub enabled: bool,
}

impl Default for AdaptivePreallocatorConfig {
    fn default() -> Self {
        Self {
            min_preallocate_bytes: 1024 * 1024,            // 1MB minimum
            max_preallocate_bytes: 64 * 1024 * 1024,       // 64MB maximum
            initial_preallocate_bytes: 16 * 1024 * 1024,   // 16MB initial
            ewma_alpha: 0.3,
            history_size: 10,
            enabled: true,
        }
    }
}

/// Statistics about pre-allocation behavior
#[derive(Debug, Clone, Default)]
pub struct PreallocatorStats {
    /// Current pre-allocation size
    pub current_preallocate_size: u64,
    /// Average segment utilization (actual_size / preallocated_size)
    pub avg_utilization: f64,
    /// Number of segments tracked
    pub segments_tracked: usize,
    /// Total bytes pre-allocated
    pub total_preallocated_bytes: u64,
    /// Total bytes actually used
    pub total_used_bytes: u64,
}

/// Internal state for adaptive pre-allocation
struct AdaptiveState {
    /// EWMA of segment sizes
    ewma_segment_size: f64,
    /// History of recent segment actual sizes
    segment_sizes: Vec<u64>,
    /// Total pre-allocated bytes
    total_preallocated: u64,
    /// Total used bytes
    total_used: u64,
    /// Current pre-allocation size
    current_size: u64,
}

/// Adaptive Segment Pre-allocator
///
/// Dynamically adjusts segment pre-allocation size based on write patterns.
/// Uses exponential weighted moving average (EWMA) to smoothly adapt to
/// changing workloads while avoiding oscillation.
pub struct AdaptivePreallocator {
    config: AdaptivePreallocatorConfig,
    state: parking_lot::Mutex<AdaptiveState>,
    /// Counter for segments created
    segments_created: AtomicU64,
}

impl AdaptivePreallocator {
    /// Create a new adaptive pre-allocator with the given configuration
    pub fn new(config: AdaptivePreallocatorConfig) -> Self {
        let initial_size = config.initial_preallocate_bytes;

        Self {
            config,
            state: parking_lot::Mutex::new(AdaptiveState {
                ewma_segment_size: initial_size as f64,
                segment_sizes: Vec::new(),
                total_preallocated: 0,
                total_used: 0,
                current_size: initial_size,
            }),
            segments_created: AtomicU64::new(0),
        }
    }

    /// Get the next pre-allocation size based on current patterns
    ///
    /// This is called before creating a new segment file.
    pub fn next_preallocate_size(&self) -> u64 {
        let state = self.state.lock();
        
        if !self.config.enabled {
            return self.config.initial_preallocate_bytes;
        }

        state.current_size
    }

    /// Record that a segment was created with the given pre-allocated size
    ///
    /// Call this after creating a segment file to track pre-allocation.
    pub fn record_segment_created(&self, preallocated_size: u64) {
        self.segments_created.fetch_add(1, Ordering::Relaxed);
        
        let mut state = self.state.lock();
        state.total_preallocated += preallocated_size;
    }

    /// Record that a segment was closed with the given actual size
    ///
    /// Call this when a segment was sealed/closed to update the adaptation model.
    /// This triggers recalculation of the optimal pre-allocation size.
    pub fn record_segment_closed(&self, actual_size: u64) {
        let mut state = self.state.lock();

        state.total_used += actual_size;
        state.segment_sizes.push(actual_size);

        // Keep only recent history
        if state.segment_sizes.len() > self.config.history_size {
            state.segment_sizes.remove(0);
        }

        // Recalculate EWMA and optimal size
        if self.config.enabled && !state.segment_sizes.is_empty() {
            // Calculate average of recent segment sizes
            let avg_size: f64 = state.segment_sizes.iter()
                .map(|&s| s as f64)
                .sum::<f64>() / state.segment_sizes.len() as f64;

            // Update EWMA - use average directly for first segment, then EWMA
            if state.segment_sizes.len() == 1 {
                state.ewma_segment_size = avg_size;
            } else {
                state.ewma_segment_size = self.config.ewma_alpha * avg_size
                    + (1.0 - self.config.ewma_alpha) * state.ewma_segment_size;
            }

            // Calculate optimal pre-allocation size
            // We want to pre-allocate slightly more than average to reduce fragmentation
            // but not so much that we waste disk space
            let optimal_size = (state.ewma_segment_size * 1.1) as u64; // 10% buffer

            // Clamp to min/max bounds
            state.current_size = optimal_size
                .max(self.config.min_preallocate_bytes)
                .min(self.config.max_preallocate_bytes);
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> PreallocatorStats {
        let state = self.state.lock();
        
        let avg_utilization = if state.total_preallocated > 0 {
            state.total_used as f64 / state.total_preallocated as f64
        } else {
            0.0
        };

        PreallocatorStats {
            current_preallocate_size: state.current_size,
            avg_utilization,
            segments_tracked: state.segment_sizes.len(),
            total_preallocated_bytes: state.total_preallocated,
            total_used_bytes: state.total_used,
        }
    }

    /// Reset the adaptive state (useful for testing or manual recalibration)
    pub fn reset(&self) {
        let mut state = self.state.lock();
        state.ewma_segment_size = self.config.initial_preallocate_bytes as f64;
        state.segment_sizes.clear();
        state.current_size = self.config.initial_preallocate_bytes;
        // Keep total_preallocated and total_used for cumulative stats
    }

    /// Check if adaptive mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configured minimum pre-allocation size
    pub fn min_preallocate_size(&self) -> u64 {
        self.config.min_preallocate_bytes
    }

    /// Get the configured maximum pre-allocation size
    pub fn max_preallocate_size(&self) -> u64 {
        self.config.max_preallocate_bytes
    }
}

/// Shared adaptive pre-allocator (for use across multiple FileKV instances)
pub type SharedAdaptivePreallocator = Arc<AdaptivePreallocator>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AdaptivePreallocatorConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_preallocate_bytes, 1024 * 1024);
        assert_eq!(config.max_preallocate_bytes, 64 * 1024 * 1024);
        assert_eq!(config.initial_preallocate_bytes, 16 * 1024 * 1024);
        assert!((0.0..=1.0).contains(&config.ewma_alpha));
    }

    #[test]
    fn test_adaptive_preallocator_basic() {
        let config = AdaptivePreallocatorConfig::default();
        let preallocator = AdaptivePreallocator::new(config);

        // Initial size should be the default
        let initial_size = preallocator.next_preallocate_size();
        assert_eq!(initial_size, 16 * 1024 * 1024);

        // Record segment creation
        preallocator.record_segment_created(initial_size);

        let stats = preallocator.stats();
        assert_eq!(stats.segments_tracked, 0); // Not closed yet
        assert_eq!(stats.total_preallocated_bytes, initial_size);
    }

    #[test]
    fn test_adaptive_preallocator_adaptation() {
        let config = AdaptivePreallocatorConfig {
            history_size: 5,
            ewma_alpha: 0.5,
            initial_preallocate_bytes: 4 * 1024 * 1024,
            ..Default::default()
        };
        let preallocator = AdaptivePreallocator::new(config);

        // Simulate segments with increasing sizes
        let segment_sizes = vec![
            2 * 1024 * 1024,  // 2MB
            4 * 1024 * 1024,  // 4MB
            8 * 1024 * 1024,  // 8MB
            12 * 1024 * 1024, // 12MB
            16 * 1024 * 1024, // 16MB
        ];

        let initial_size = preallocator.next_preallocate_size();

        for &size in &segment_sizes {
            // Record segment created with current size
            let prealloc_size = preallocator.next_preallocate_size();
            preallocator.record_segment_created(prealloc_size);
            // Record segment closed with actual size
            preallocator.record_segment_closed(size);
        }

        // After several segments, the pre-allocation size should have adapted
        let final_size = preallocator.next_preallocate_size();
        let stats = preallocator.stats();

        // Size should have increased from initial due to growing segments
        assert!(final_size > initial_size, "Pre-allocation size should increase from initial with growing segments (initial: {}, final: {})", initial_size, final_size);
        // Final size should be around the average segment size (8.4MB) with EWMA weighting toward recent values
        // With EWMA alpha=0.5, the final EWMA is approximately 7-8MB, plus 10% buffer = ~7.7-8.8MB
        assert!(final_size >= 7 * 1024 * 1024, "Pre-allocation size should be at least 7MB (got: {})", final_size);
        assert_eq!(stats.segments_tracked, 5);
        assert!(stats.avg_utilization > 0.0);
    }

    #[test]
    fn test_adaptive_preallocator_min_max_bounds() {
        let config = AdaptivePreallocatorConfig {
            min_preallocate_bytes: 2 * 1024 * 1024,
            max_preallocate_bytes: 8 * 1024 * 1024,
            history_size: 3,
            ..Default::default()
        };
        let preallocator = AdaptivePreallocator::new(config);

        // Simulate very small segments
        for _ in 0..5 {
            let prealloc_size = preallocator.next_preallocate_size();
            preallocator.record_segment_created(prealloc_size);
            preallocator.record_segment_closed(100 * 1024); // 100KB segments
        }

        // Size should hit minimum bound
        let min_size = preallocator.next_preallocate_size();
        assert_eq!(min_size, 2 * 1024 * 1024, "Should respect minimum bound");

        // Reset and test maximum bound
        preallocator.reset();

        // Simulate very large segments
        for _ in 0..5 {
            let prealloc_size = preallocator.next_preallocate_size();
            preallocator.record_segment_created(prealloc_size);
            preallocator.record_segment_closed(100 * 1024 * 1024); // 100MB segments
        }

        // Size should hit maximum bound
        let max_size = preallocator.next_preallocate_size();
        assert_eq!(max_size, 8 * 1024 * 1024, "Should respect maximum bound");
    }

    #[test]
    fn test_adaptive_preallocator_disabled() {
        let config = AdaptivePreallocatorConfig {
            enabled: false,
            ..Default::default()
        };

        let preallocator = AdaptivePreallocator::new(config);

        // Should always return initial size
        assert_eq!(preallocator.next_preallocate_size(), 16 * 1024 * 1024);

        // Record some segments
        for _ in 0..10 {
            preallocator.record_segment_created(16 * 1024 * 1024);
            preallocator.record_segment_closed(32 * 1024 * 1024);
        }

        // Size should still be initial (no adaptation)
        assert_eq!(preallocator.next_preallocate_size(), 16 * 1024 * 1024);
        assert!(!preallocator.is_enabled());
    }

    #[test]
    fn test_adaptive_preallocator_stats() {
        let config = AdaptivePreallocatorConfig::default();
        let preallocator = AdaptivePreallocator::new(config);

        // Initial stats
        let stats = preallocator.stats();
        assert_eq!(stats.current_preallocate_size, 16 * 1024 * 1024);
        assert_eq!(stats.segments_tracked, 0);
        assert_eq!(stats.avg_utilization, 0.0);

        // After some segments
        let prealloc_size = 16 * 1024 * 1024;
        let actual_size = 12 * 1024 * 1024;
        
        preallocator.record_segment_created(prealloc_size);
        preallocator.record_segment_closed(actual_size);

        let stats = preallocator.stats();
        assert_eq!(stats.segments_tracked, 1);
        assert_eq!(stats.total_preallocated_bytes, prealloc_size);
        assert_eq!(stats.total_used_bytes, actual_size);
        // Utilization = 12MB / 16MB = 0.75
        assert!((stats.avg_utilization - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_preallocator_ewma_smoothing() {
        let config = AdaptivePreallocatorConfig {
            history_size: 10,
            ewma_alpha: 0.2,
            ..Default::default()
        };

        let preallocator = AdaptivePreallocator::new(config);

        // Simulate variable segment sizes
        let sizes = vec![
            10 * 1024 * 1024,
            20 * 1024 * 1024,
            10 * 1024 * 1024,
            20 * 1024 * 1024,
            10 * 1024 * 1024,
        ];

        let mut prev_size = preallocator.next_preallocate_size();
        
        for &size in &sizes {
            preallocator.record_segment_created(prev_size);
            preallocator.record_segment_closed(size);
            let new_size = preallocator.next_preallocate_size();
            
            // With low alpha, changes should be gradual
            let change_ratio = (new_size as f64 - prev_size as f64) / prev_size as f64;
            assert!(change_ratio.abs() < 0.5, "EWMA should smooth large changes");
            
            prev_size = new_size;
        }
    }
}
