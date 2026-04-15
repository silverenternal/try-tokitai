//! Cache Rebalance Configuration and Decision Logic
//!
//! This module implements the background rebalance algorithm for `UnifiedCacheManager`.
//! The rebalance thread periodically evaluates cache performance and redistributes
//! memory budget from underperforming caches to high-performing ones.
//!
//! # Rebalance Algorithm
//!
//! The algorithm follows a conservative strategy to avoid oscillation:
//!
//! 1. **Collect Stats**: Gather hit rates and memory usage from all caches.
//! 2. **Check Minimum Samples**: Skip if caches haven't been accessed enough.
//! 3. **Identify Candidates**:
//!    - A cache is a "donor" candidate if its hit rate < `low_hit_rate_threshold`
//!    - A cache is a "receiver" candidate if its hit rate > `high_hit_rate_threshold`
//! 4. **Evaluate Gap**: Only transfer if receiver hit rate - donor hit rate >= `min_hit_rate_gap`
//! 5. **Calculate Transfer**: Transfer amount = donor_memory * `max_transfer_ratio` (capped)
//! 6. **Enforce Bounds**: Ensure each cache stays within [min_budget, max_budget]
//! 7. **Apply**: Execute shrink/grow operations on the caches.
//!
//! # Thread Safety
//!
//! All rebalance types are `Clone` and `Send` for safe cross-thread usage.
//! The background thread uses `AtomicBool` for shutdown signaling.

use std::time::Duration;

/// Configuration for the background cache rebalance thread.
///
/// # Example
/// ```ignore
/// let config = RebalanceConfig {
///     interval: Duration::from_secs(30),
///     low_hit_rate_threshold: 0.3,       // Below 30% = candidate for shrinking
///     high_hit_rate_threshold: 0.8,      // Above 80% = candidate for growing
///     min_hit_rate_gap: 0.2,             // Need at least 20% gap to transfer
///     max_transfer_ratio: 0.1,           // Move at most 10% of donor's budget per cycle
///     min_budget_bytes: 1024 * 1024,     // Never shrink below 1MB
///     max_budget_bytes: 256 * 1024 * 1024, // Never grow above 256MB
///     min_access_samples: 100,            // Need at least 100 accesses before deciding
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RebalanceConfig {
    /// How often the rebalance thread runs (default: 30 seconds)
    pub interval: Duration,

    /// Hit rate below which a cache becomes a candidate for shrinking (default: 0.3 = 30%)
    pub low_hit_rate_threshold: f64,

    /// Hit rate above which a cache becomes a candidate for growing (default: 0.8 = 80%)
    pub high_hit_rate_threshold: f64,

    /// Minimum hit rate gap between receiver and donor for budget transfer (default: 0.2 = 20%)
    pub min_hit_rate_gap: f64,

    /// Maximum fraction of a cache's budget to transfer per cycle (default: 0.1 = 10%)
    /// This prevents oscillation and ensures gradual rebalancing.
    pub max_transfer_ratio: f64,

    /// Minimum budget in bytes for any cache (default: 1MB)
    /// Ensures no cache is starved completely.
    pub min_budget_bytes: u64,

    /// Maximum budget in bytes for any cache (default: 256MB)
    /// Prevents a single cache from consuming all memory.
    pub max_budget_bytes: u64,

    /// Minimum number of access samples before rebalance decisions are made (default: 100)
    /// Prevents premature decisions when caches have barely been used.
    pub min_access_samples: u64,
}

impl Default for RebalanceConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            low_hit_rate_threshold: 0.3,
            high_hit_rate_threshold: 0.8,
            min_hit_rate_gap: 0.2,
            max_transfer_ratio: 0.1,
            min_budget_bytes: 1024 * 1024,        // 1MB
            max_budget_bytes: 256 * 1024 * 1024,   // 256MB
            min_access_samples: 100,
        }
    }
}

/// A single rebalance decision
#[derive(Debug, Clone)]
pub enum RebalanceDecision {
    /// Shrink BlockCache by the specified number of bytes
    ShrinkBlock(u64),
    /// Grow BlockCache by the specified number of bytes
    GrowBlock(u64),
    /// Shrink BloomFilterCache by the specified number of bytes
    ShrinkBloom(u64),
    /// Grow BloomFilterCache by the specified number of bytes
    GrowBloom(u64),
}

impl RebalanceDecision {
    /// Evaluate rebalance decisions based on current cache stats and configuration.
    ///
    /// Returns a list of decisions to apply. The decisions are designed to be
    /// applied together atomically (e.g., ShrinkBlock + GrowBloom in one cycle).
    pub fn evaluate(
        config: &RebalanceConfig,
        block_hit_rate: f64,
        bloom_hit_rate: f64,
        block_memory: u64,
        bloom_memory: u64,
    ) -> Vec<RebalanceDecision> {
        let mut decisions = Vec::new();

        // Determine performance categories
        let block_is_low = block_hit_rate < config.low_hit_rate_threshold;
        let block_is_high = block_hit_rate > config.high_hit_rate_threshold;
        let bloom_is_low = bloom_hit_rate < config.low_hit_rate_threshold;
        let bloom_is_high = bloom_hit_rate > config.high_hit_rate_threshold;

        // Calculate hit rate gap
        let gap = bloom_hit_rate - block_hit_rate;

        // Case 1: Block is low, Bloom is high -> transfer from Block to Bloom
        if block_is_low && bloom_is_high && gap >= config.min_hit_rate_gap {
            let transfer_bytes = Self::calculate_transfer(block_memory, config);
            let transfer_bytes = Self::clamp_transfer(
                transfer_bytes,
                block_memory,
                bloom_memory,
                config,
            );
            if transfer_bytes > 0 {
                decisions.push(RebalanceDecision::ShrinkBlock(transfer_bytes));
                decisions.push(RebalanceDecision::GrowBloom(transfer_bytes));
            }
        }

        // Case 2: Bloom is low, Block is high -> transfer from Bloom to Block
        if bloom_is_low && block_is_high && (-gap) >= config.min_hit_rate_gap {
            let transfer_bytes = Self::calculate_transfer(bloom_memory, config);
            let transfer_bytes = Self::clamp_transfer(
                transfer_bytes,
                bloom_memory,
                block_memory,
                config,
            );
            if transfer_bytes > 0 {
                decisions.push(RebalanceDecision::ShrinkBloom(transfer_bytes));
                decisions.push(RebalanceDecision::GrowBlock(transfer_bytes));
            }
        }

        // Case 3: Both low -> no transfer, both may need tuning elsewhere
        // Case 4: Both high -> no transfer, both are performing well
        // Case 5: One medium (neither low nor high) -> no transfer

        decisions
    }

    /// Calculate transfer amount as a fraction of donor's current memory.
    fn calculate_transfer(donor_memory: u64, config: &RebalanceConfig) -> u64 {
        let max_transfer = (donor_memory as f64 * config.max_transfer_ratio) as u64;
        max_transfer.max(1) // At least 1 byte if any transfer is warranted
    }

    /// Clamp transfer to respect min/max budget bounds.
    fn clamp_transfer(
        transfer_bytes: u64,
        donor_memory: u64,
        receiver_memory: u64,
        config: &RebalanceConfig,
    ) -> u64 {
        // Donor cannot go below min_budget
        let donor_min = donor_memory.saturating_sub(transfer_bytes);
        let donor_safe = if donor_min < config.min_budget_bytes {
            donor_memory.saturating_sub(config.min_budget_bytes)
        } else {
            transfer_bytes
        };

        // Receiver cannot go above max_budget
        let receiver_max = receiver_memory.saturating_add(donor_safe);
        if receiver_max > config.max_budget_bytes {
            config.max_budget_bytes.saturating_sub(receiver_memory)
        } else {
            donor_safe
        }
    }
}

/// Statistics from a rebalance cycle
#[derive(Debug, Clone)]
pub struct RebalanceStats {
    /// Block cache hit rate at time of rebalance
    pub block_hit_rate: f64,
    /// Bloom cache hit rate at time of rebalance
    pub bloom_hit_rate: f64,
    /// Block cache memory usage at time of rebalance (bytes)
    pub block_memory_bytes: u64,
    /// Bloom cache memory usage at time of rebalance (bytes)
    pub bloom_memory_bytes: u64,
    /// Decisions made in this cycle
    pub decisions: Vec<RebalanceDecision>,
    /// Whether the cycle was completed, skipped, or disabled
    pub status: RebalanceStatus,
}

impl RebalanceStats {
    /// Create stats for a disabled rebalance (no config provided)
    pub fn disabled() -> Self {
        Self {
            block_hit_rate: 0.0,
            bloom_hit_rate: 0.0,
            block_memory_bytes: 0,
            bloom_memory_bytes: 0,
            decisions: Vec::new(),
            status: RebalanceStatus::Disabled,
        }
    }

    /// Create stats for a skipped cycle (insufficient samples)
    pub fn skipped(
        block_hit_rate: f64,
        bloom_hit_rate: f64,
        block_memory_bytes: u64,
        bloom_memory_bytes: u64,
    ) -> Self {
        Self {
            block_hit_rate,
            bloom_hit_rate,
            block_memory_bytes,
            bloom_memory_bytes,
            decisions: Vec::new(),
            status: RebalanceStatus::SkippedInsufficientSamples,
        }
    }

    /// Create stats for a completed cycle
    pub fn completed(
        block_hit_rate: f64,
        bloom_hit_rate: f64,
        block_memory_bytes: u64,
        bloom_memory_bytes: u64,
        decisions: Vec<RebalanceDecision>,
    ) -> Self {
        Self {
            block_hit_rate,
            bloom_hit_rate,
            block_memory_bytes,
            bloom_memory_bytes,
            decisions,
            status: RebalanceStatus::Completed,
        }
    }

    /// Get the total bytes transferred (sum of all shrink decisions)
    pub fn total_bytes_transferred(&self) -> u64 {
        self.decisions
            .iter()
            .filter_map(|d| match d {
                RebalanceDecision::ShrinkBlock(b) | RebalanceDecision::ShrinkBloom(b) => Some(*b),
                _ => None,
            })
            .sum()
    }

    /// Check if any action was taken in this cycle
    pub fn had_action(&self) -> bool {
        !self.decisions.is_empty()
    }
}

impl std::fmt::Display for RebalanceStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RebalanceStats {{ status: {:?}, block_hit_rate: {:.3}, bloom_hit_rate: {:.3}, decisions: {} }}",
            self.status,
            self.block_hit_rate,
            self.bloom_hit_rate,
            self.decisions.len(),
        )
    }
}

/// Status of a rebalance cycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebalanceStatus {
    /// Rebalance is not configured (no RebalanceConfig provided)
    Disabled,
    /// Cycle was skipped due to insufficient access samples
    SkippedInsufficientSamples,
    /// Cycle completed and decisions were evaluated
    Completed,
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebalance_config_defaults() {
        let config = RebalanceConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert!((config.low_hit_rate_threshold - 0.3).abs() < f64::EPSILON);
        assert!((config.high_hit_rate_threshold - 0.8).abs() < f64::EPSILON);
        assert!((config.min_hit_rate_gap - 0.2).abs() < f64::EPSILON);
        assert!((config.max_transfer_ratio - 0.1).abs() < f64::EPSILON);
        assert_eq!(config.min_budget_bytes, 1024 * 1024);
        assert_eq!(config.max_budget_bytes, 256 * 1024 * 1024);
        assert_eq!(config.min_access_samples, 100);
    }

    #[test]
    fn test_no_decision_when_both_medium() {
        let config = RebalanceConfig::default();

        // Both hit rates are in the medium range (0.3-0.8)
        let decisions = RebalanceDecision::evaluate(
            &config,
            0.5,  // block: medium
            0.6,  // bloom: medium
            10 * 1024 * 1024,
            5 * 1024 * 1024,
        );

        assert!(decisions.is_empty(), "No decisions expected when both caches are medium");
    }

    #[test]
    fn test_no_decision_when_both_high() {
        let config = RebalanceConfig::default();

        let decisions = RebalanceDecision::evaluate(
            &config,
            0.9,  // block: high
            0.85, // bloom: high
            10 * 1024 * 1024,
            5 * 1024 * 1024,
        );

        assert!(decisions.is_empty(), "No decisions expected when both caches are high performing");
    }

    #[test]
    fn test_no_decision_when_both_low() {
        let config = RebalanceConfig::default();

        let decisions = RebalanceDecision::evaluate(
            &config,
            0.1, // block: low
            0.2, // bloom: low
            10 * 1024 * 1024,
            5 * 1024 * 1024,
        );

        assert!(decisions.is_empty(), "No decisions expected when both caches are low performing");
    }

    #[test]
    fn test_transfer_from_block_to_bloom() {
        let config = RebalanceConfig::default();

        // Block is low, Bloom is high, gap is sufficient
        let decisions = RebalanceDecision::evaluate(
            &config,
            0.1,  // block: low (< 0.3)
            0.9,  // bloom: high (> 0.8)
            10 * 1024 * 1024, // 10MB block
            5 * 1024 * 1024,  // 5MB bloom
        );

        assert_eq!(decisions.len(), 2, "Expected 2 decisions (shrink + grow)");
        // First decision should be shrinking block
        match &decisions[0] {
            RebalanceDecision::ShrinkBlock(bytes) => {
                assert!(*bytes > 0, "Should transfer some bytes");
                // Transfer should be at most 10% of block memory
                assert!(*bytes <= (10 * 1024 * 1024) / 10, "Transfer should not exceed max_transfer_ratio");
            }
            _ => panic!("Expected ShrinkBlock as first decision"),
        }
        // Second decision should be growing bloom
        match &decisions[1] {
            RebalanceDecision::GrowBloom(bytes) => {
                assert_eq!(*bytes, match &decisions[0] {
                    RebalanceDecision::ShrinkBlock(b) => *b,
                    _ => panic!("Mismatched decisions"),
                }, "Grow amount should match shrink amount");
            }
            _ => panic!("Expected GrowBloom as second decision"),
        }
    }

    #[test]
    fn test_transfer_from_bloom_to_block() {
        let config = RebalanceConfig::default();

        // Bloom is low, Block is high, gap is sufficient
        let decisions = RebalanceDecision::evaluate(
            &config,
            0.9,  // block: high
            0.1,  // bloom: low
            10 * 1024 * 1024,
            5 * 1024 * 1024,
        );

        assert_eq!(decisions.len(), 2, "Expected 2 decisions (shrink + grow)");
        match &decisions[0] {
            RebalanceDecision::ShrinkBloom(bytes) => {
                assert!(*bytes > 0, "Should transfer some bytes");
                assert!(*bytes <= (5 * 1024 * 1024) / 10, "Transfer should not exceed max_transfer_ratio");
            }
            _ => panic!("Expected ShrinkBloom as first decision"),
        }
        match &decisions[1] {
            RebalanceDecision::GrowBlock(bytes) => {
                assert_eq!(*bytes, match &decisions[0] {
                    RebalanceDecision::ShrinkBloom(b) => *b,
                    _ => panic!("Mismatched decisions"),
                }, "Grow amount should match shrink amount");
            }
            _ => panic!("Expected GrowBlock as second decision"),
        }
    }

    #[test]
    fn test_no_transfer_when_gap_insufficient() {
        let config = RebalanceConfig::default();

        // Block is low, Bloom is high, but gap is too small
        let decisions = RebalanceDecision::evaluate(
            &config,
            0.25, // block: low
            0.40, // bloom: not high enough, and gap = 0.15 < 0.2
            10 * 1024 * 1024,
            5 * 1024 * 1024,
        );

        assert!(decisions.is_empty(), "No transfer expected when gap is below threshold");
    }

    #[test]
    fn test_transfer_respects_min_budget() {
        let config = RebalanceConfig {
            min_budget_bytes: 5 * 1024 * 1024, // 5MB minimum
            ..RebalanceConfig::default()
        };

        // Block has only 6MB, min is 5MB, so transfer should be limited to 1MB
        let decisions = RebalanceDecision::evaluate(
            &config,
            0.1,  // block: low
            0.9,  // bloom: high
            6 * 1024 * 1024, // 6MB block
            5 * 1024 * 1024,
        );

        assert_eq!(decisions.len(), 2);
        match &decisions[0] {
            RebalanceDecision::ShrinkBlock(bytes) => {
                // Cannot shrink below 5MB, so max transfer is 1MB
                assert!(*bytes <= 1 * 1024 * 1024, "Transfer should respect min_budget");
            }
            _ => panic!("Expected ShrinkBlock"),
        }
    }

    #[test]
    fn test_transfer_respects_max_budget() {
        let config = RebalanceConfig {
            max_budget_bytes: 6 * 1024 * 1024, // 6MB maximum
            ..RebalanceConfig::default()
        };

        // Bloom is at 5.5MB, max is 6MB, so can only receive 0.5MB
        let decisions = RebalanceDecision::evaluate(
            &config,
            0.1,  // block: low
            0.9,  // bloom: high
            10 * 1024 * 1024,
            5500 * 1024, // ~5.5MB bloom
        );

        assert_eq!(decisions.len(), 2);
        match &decisions[1] {
            RebalanceDecision::GrowBloom(bytes) => {
                // Cannot grow beyond 6MB, so max transfer is ~0.5MB
                let max_transfer: u64 = (6u64 * 1024 * 1024).saturating_sub(5500 * 1024) + 1;
                assert!(*bytes <= max_transfer, "Transfer should respect max_budget");
            }
            _ => panic!("Expected GrowBloom"),
        }
    }

    #[test]
    fn test_rebalance_stats_disabled() {
        let stats = RebalanceStats::disabled();
        assert_eq!(stats.status, RebalanceStatus::Disabled);
        assert!(stats.decisions.is_empty());
        assert!(!stats.had_action());
        assert_eq!(stats.total_bytes_transferred(), 0);
    }

    #[test]
    fn test_rebalance_stats_skipped() {
        let stats = RebalanceStats::skipped(0.5, 0.6, 1024, 2048);
        assert_eq!(stats.status, RebalanceStatus::SkippedInsufficientSamples);
        assert_eq!(stats.block_hit_rate, 0.5);
        assert_eq!(stats.bloom_hit_rate, 0.6);
        assert_eq!(stats.block_memory_bytes, 1024);
        assert_eq!(stats.bloom_memory_bytes, 2048);
        assert!(!stats.had_action());
    }

    #[test]
    fn test_rebalance_stats_completed_with_action() {
        let decisions = vec![
            RebalanceDecision::ShrinkBlock(1024),
            RebalanceDecision::GrowBloom(1024),
        ];
        let stats = RebalanceStats::completed(0.1, 0.9, 10 * 1024, 5 * 1024, decisions);

        assert_eq!(stats.status, RebalanceStatus::Completed);
        assert!(stats.had_action());
        assert_eq!(stats.total_bytes_transferred(), 1024);
    }

    #[test]
    fn test_rebalance_stats_display() {
        let stats = RebalanceStats::completed(0.123, 0.456, 1024, 2048, vec![]);
        let display = format!("{}", stats);
        assert!(display.contains("Completed"));
        assert!(display.contains("0.123"));
        assert!(display.contains("0.456"));
    }

    #[test]
    fn test_calculate_transfer_basic() {
        let config = RebalanceConfig::default();
        let transfer = RebalanceDecision::calculate_transfer(10 * 1024 * 1024, &config);
        // 10% of 10MB = 1MB
        assert_eq!(transfer, 1024 * 1024);
    }

    #[test]
    fn test_calculate_transfer_small_donor() {
        let config = RebalanceConfig::default();
        let transfer = RebalanceDecision::calculate_transfer(100, &config);
        // 10% of 100 = 10, but minimum is 1
        assert!(transfer >= 1);
        assert!(transfer <= 10);
    }

    #[test]
    fn test_clamp_transfer_respects_min_budget() {
        let config = RebalanceConfig {
            min_budget_bytes: 1000,
            ..RebalanceConfig::default()
        };

        // Donor has 1100 bytes, min is 1000, so can only transfer 100
        let clamped = RebalanceDecision::clamp_transfer(500, 1100, 500, &config);
        assert!(clamped <= 100, "Should not transfer below min_budget");
    }

    #[test]
    fn test_clamp_transfer_respects_max_budget() {
        let config = RebalanceConfig {
            max_budget_bytes: 1500,
            ..RebalanceConfig::default()
        };

        // Receiver has 1400 bytes, max is 1500, so can only receive 100
        let clamped = RebalanceDecision::clamp_transfer(500, 2000, 1400, &config);
        assert!(clamped <= 100, "Should not transfer above max_budget");
    }
}
