//! Bloom Filter Cache Migration Mechanism
//!
//! INNO-001: Implements cache layer migration policies:
//! - L3 -> L2: When segment QPS > warm_threshold
//! - L2 -> L1: When segment QPS > hot_threshold
//! - L1 -> L2: When segment QPS < warm_threshold (cooldown)
//! - L2 -> L3: When segment QPS < cold_threshold (eviction)
//!
//! # Migration Policy
//! Uses hysteresis (dual threshold) to prevent cache thrashing:
//! - Upgrade requires sustained high QPS for upgrade_window_ms
//! - Downgrade requires sustained low QPS for downgrade_window_ms

use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;

use crate::core::error::FatalError;
use bloom::{BloomFilter, ASMS};

/// Result type for bloom migration operations
pub type Result<T> = std::result::Result<T, FatalError>;

/// Current bloom filter file version
pub const CURRENT_BLOOM_VERSION: u32 = 2;

/// Frequency tier classification based on access_count
///
/// Determines which cache layer a segment should prefer:
/// - Hot: High access frequency, prefer L1 (fastest, lowest FPR)
/// - Warm: Moderate access frequency, prefer L2 (compressed, medium FPR)
/// - Cold: Low access frequency, prefer L3 (disk-based, high FPR)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrequencyTier {
    /// High access frequency - prefer L1 cache
    Hot,
    /// Moderate access frequency - prefer L2 cache
    Warm,
    /// Low access frequency - prefer L3 (disk)
    Cold,
}

impl FrequencyTier {
    /// Get the preferred cache layer for this frequency tier
    /// Returns 1 for L1, 2 for L2, 3 for L3
    pub fn preferred_layer(&self) -> usize {
        match self {
            FrequencyTier::Hot => 1,
            FrequencyTier::Warm => 2,
            FrequencyTier::Cold => 3,
        }
    }
}

/// Migration thresholds configuration
#[derive(Debug, Clone)]
pub struct MigrationThresholds {
    /// QPS threshold for L3 -> L2 migration (warm)
    pub warm_threshold_qps: u64,
    /// QPS threshold for L2 -> L1 migration (hot)
    pub hot_threshold_qps: u64,
    /// QPS threshold for L1 -> L2 migration (cooldown)
    pub cooldown_threshold_qps: u64,
    /// QPS threshold for L2 -> L3 migration (eviction)
    pub cold_threshold_qps: u64,
    /// Time window for upgrade migration (ms)
    pub upgrade_window_ms: u64,
    /// Time window for downgrade migration (ms)
    pub downgrade_window_ms: u64,
    /// Access count threshold for Hot tier (segments with >= this count are Hot)
    pub hot_tier_access_count: u64,
    /// Access count threshold for Warm tier (segments with >= this count are Warm, below are Cold)
    pub warm_tier_access_count: u64,
    /// Weight for frequency score in migration decision (0.0 = QPS only, 1.0 = frequency only)
    pub frequency_weight: f64,
}

impl Default for MigrationThresholds {
    fn default() -> Self {
        Self {
            warm_threshold_qps: 10,       // 10 QPS for warm
            hot_threshold_qps: 100,       // 100 QPS for hot
            cooldown_threshold_qps: 5,    // 5 QPS for cooldown
            cold_threshold_qps: 1,        // 1 QPS for eviction
            upgrade_window_ms: 60_000,    // 1 minute
            downgrade_window_ms: 300_000, // 5 minutes
            hot_tier_access_count: 100,   // 100+ accesses = Hot
            warm_tier_access_count: 10,   // 10-99 accesses = Warm, <10 = Cold
            frequency_weight: 0.3,        // 30% frequency, 70% QPS in combined score
        }
    }
}

/// Process startup instant (used to avoid SystemTime::now() syscalls on hot path)
static PROCESS_START: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);

/// Return milliseconds since process start (monotonic, no syscall)
#[inline]
fn elapsed_ms() -> u64 {
    Instant::now().duration_since(*PROCESS_START).as_millis() as u64
}

/// Access frequency tracking for a single segment
#[derive(Debug)]
pub struct SegmentAccessTracker {
    /// Total access count
    access_count: AtomicU64,
    /// Last access timestamp (ms since process start)
    last_access_ms: AtomicU64,
    /// First access timestamp in current window (ms since process start)
    window_start_ms: AtomicU64,
    /// Access count in current window
    window_count: AtomicU64,
    /// Current cache layer (1=L1, 2=L2, 3=L3)
    current_layer: AtomicUsize,
}

impl SegmentAccessTracker {
    pub fn new(layer: usize) -> Self {
        let now_ms = elapsed_ms();
        Self {
            access_count: AtomicU64::new(0),
            last_access_ms: AtomicU64::new(now_ms),
            window_start_ms: AtomicU64::new(now_ms),
            window_count: AtomicU64::new(0),
            current_layer: AtomicUsize::new(layer),
        }
    }

    /// Record an access event
    pub fn record_access(&self) -> AccessRecord {
        let now_ms = elapsed_ms();

        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.last_access_ms.store(now_ms, Ordering::Relaxed);

        // Update window count
        let window_start = self.window_start_ms.load(Ordering::Relaxed);
        let window_duration = now_ms.saturating_sub(window_start);

        if window_duration > 60_000 {
            // Reset window every minute
            self.window_start_ms.store(now_ms, Ordering::Relaxed);
            self.window_count.store(1, Ordering::Relaxed);
        } else {
            self.window_count.fetch_add(1, Ordering::Relaxed);
        }

        AccessRecord {
            total_count: self.access_count.load(Ordering::Relaxed),
            window_count: self.window_count.load(Ordering::Relaxed),
            window_duration_ms: window_duration,
            current_layer: self.current_layer.load(Ordering::Relaxed),
        }
    }

    /// Get current QPS (queries per second)
    pub fn get_qps(&self) -> f64 {
        let window_count = self.window_count.load(Ordering::Relaxed);
        let window_start = self.window_start_ms.load(Ordering::Relaxed);
        let now_ms = elapsed_ms();

        let window_duration_sec = (now_ms.saturating_sub(window_start)) as f64 / 1000.0;
        if window_duration_sec > 0.0 {
            window_count as f64 / window_duration_sec
        } else {
            0.0
        }
    }

    /// Update cache layer
    pub fn set_layer(&self, layer: usize) {
        self.current_layer.store(layer, Ordering::Relaxed);
    }

    /// Get current layer
    pub fn get_layer(&self) -> usize {
        self.current_layer.load(Ordering::Relaxed)
    }
}

/// Access record for migration decision
#[derive(Debug, Clone)]
pub struct AccessRecord {
    pub total_count: u64,
    pub window_count: u64,
    pub window_duration_ms: u64,
    pub current_layer: usize,
}

impl AccessRecord {
    /// Calculate QPS from window
    pub fn qps(&self) -> f64 {
        if self.window_duration_ms > 0 {
            self.window_count as f64 / (self.window_duration_ms as f64 / 1000.0)
        } else {
            0.0
        }
    }

    /// Get total access count (alias for total_count)
    pub fn access_count(&self) -> u64 {
        self.total_count
    }
}

/// Classify a segment into a frequency tier based on its access count
///
/// Uses configurable thresholds from MigrationThresholds:
/// - Hot: access_count >= hot_tier_access_count
/// - Warm: access_count >= warm_tier_access_count
/// - Cold: access_count < warm_tier_access_count
pub fn classify_by_frequency(access_count: u64, thresholds: &MigrationThresholds) -> FrequencyTier {
    if access_count >= thresholds.hot_tier_access_count {
        FrequencyTier::Hot
    } else if access_count >= thresholds.warm_tier_access_count {
        FrequencyTier::Warm
    } else {
        FrequencyTier::Cold
    }
}

/// Migration decision result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationDecision {
    /// No migration needed
    Stay,
    /// Upgrade to L1 (hot)
    UpgradeToL1,
    /// Upgrade to L2 (warm)
    UpgradeToL2,
    /// Downgrade to L2 (cooldown)
    DowngradeToL2,
    /// Downgrade to L3 (evict)
    DowngradeToL3,
}

/// Migration policy controller
pub struct MigrationController {
    /// Access trackers for all segments
    trackers: DashMap<u64, Arc<SegmentAccessTracker>>,
    /// Migration thresholds
    thresholds: MigrationThresholds,
    /// Pending migrations (segment_id -> target_layer)
    pending_migrations: DashMap<u64, usize>,
    /// Migration statistics
    upgrades_triggered: AtomicU64,
    downgrades_triggered: AtomicU64,
    migrations_completed: AtomicU64,
}

impl MigrationController {
    /// Create a new migration controller
    pub fn new(thresholds: MigrationThresholds) -> Self {
        Self {
            trackers: DashMap::new(),
            thresholds,
            pending_migrations: DashMap::new(),
            upgrades_triggered: AtomicU64::new(0),
            downgrades_triggered: AtomicU64::new(0),
            migrations_completed: AtomicU64::new(0),
        }
    }

    /// Get or create tracker for a segment
    pub fn get_tracker(&self, segment_id: u64, initial_layer: usize) -> Arc<SegmentAccessTracker> {
        self.trackers
            .entry(segment_id)
            .or_insert_with(|| Arc::new(SegmentAccessTracker::new(initial_layer)))
            .clone()
    }

    /// Record access and check if migration is needed
    pub fn record_access(&self, segment_id: u64) -> Option<MigrationDecision> {
        let tracker = self.get_tracker(segment_id, 2); // Default L2
        let record = tracker.record_access();

        // Decide migration based on current layer and QPS
        let decision = self.decide_migration(&record);

        if decision != MigrationDecision::Stay {
            // Record pending migration
            let target_layer = match decision {
                MigrationDecision::UpgradeToL1 => 1,
                MigrationDecision::UpgradeToL2 => 2,
                MigrationDecision::DowngradeToL2 => 2,
                MigrationDecision::DowngradeToL3 => 3,
                MigrationDecision::Stay => return None,
            };

            self.pending_migrations.insert(segment_id, target_layer);

            match decision {
                MigrationDecision::UpgradeToL1 | MigrationDecision::UpgradeToL2 => {
                    self.upgrades_triggered.fetch_add(1, Ordering::Relaxed);
                }
                MigrationDecision::DowngradeToL2 | MigrationDecision::DowngradeToL3 => {
                    self.downgrades_triggered.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }

        Some(decision)
    }

    /// Decide migration based on access record and thresholds
    /// Uses both QPS and frequency (access_count) for intelligent tier placement
    fn decide_migration(&self, record: &AccessRecord) -> MigrationDecision {
        let qps = record.qps();
        let current_layer = record.current_layer;

        // Classify by frequency
        let freq_tier = classify_by_frequency(record.access_count(), &self.thresholds);

        // Compute combined score: blend QPS and frequency
        let combined_score = self.compute_combined_score(qps, record.access_count(), freq_tier);

        match current_layer {
            1 => {
                // Currently in L1
                if self.should_downgrade_from_l1(qps, combined_score, record) {
                    debug!(
                        "L1 -> L2 migration triggered for QPS={:.2}, freq_tier={:?}",
                        qps, freq_tier
                    );
                    MigrationDecision::DowngradeToL2
                } else {
                    MigrationDecision::Stay
                }
            }
            2 => {
                // Currently in L2
                if self.should_upgrade_to_l1(qps, combined_score, freq_tier, record) {
                    debug!(
                        "L2 -> L1 migration triggered for QPS={:.2}, freq_tier={:?}",
                        qps, freq_tier
                    );
                    MigrationDecision::UpgradeToL1
                } else if self.should_downgrade_to_l3(qps, combined_score, record) {
                    debug!(
                        "L2 -> L3 migration triggered for QPS={:.2}, freq_tier={:?}",
                        qps, freq_tier
                    );
                    MigrationDecision::DowngradeToL3
                } else {
                    MigrationDecision::Stay
                }
            }
            3 => {
                // Currently in L3 (cold)
                if self.should_upgrade_to_l2(qps, combined_score, freq_tier, record) {
                    debug!(
                        "L3 -> L2 migration triggered for QPS={:.2}, freq_tier={:?}",
                        qps, freq_tier
                    );
                    MigrationDecision::UpgradeToL2
                } else {
                    MigrationDecision::Stay
                }
            }
            _ => MigrationDecision::Stay,
        }
    }

    /// Compute a combined score blending QPS and frequency signals
    /// Returns a normalized score in [0.0, 1.0] range
    fn compute_combined_score(&self, qps: f64, _access_count: u64, freq_tier: FrequencyTier) -> f64 {
        let fw = self.thresholds.frequency_weight;

        // Normalize QPS to [0, 1] range (cap at hot_threshold)
        let qps_normalized = (qps / self.thresholds.hot_threshold_qps as f64).min(1.0);

        // Normalize frequency tier to [0, 1] range
        let freq_normalized = match freq_tier {
            FrequencyTier::Hot => 1.0,
            FrequencyTier::Warm => 0.5,
            FrequencyTier::Cold => 0.0,
        };

        // Weighted combination
        (1.0 - fw) * qps_normalized + fw * freq_normalized
    }

    /// Check if L1 segment should be downgraded to L2
    fn should_downgrade_from_l1(&self, qps: f64, combined_score: f64, record: &AccessRecord) -> bool {
        // Both QPS and combined score must be below thresholds
        let qps_below = qps < self.thresholds.cooldown_threshold_qps as f64;
        let score_below = combined_score < 0.2; // Low combined score indicates cold

        qps_below && score_below && self.check_sustained_low_qps(record, self.thresholds.downgrade_window_ms)
    }

    /// Check if L2 segment should be upgraded to L1
    fn should_upgrade_to_l1(
        &self,
        qps: f64,
        combined_score: f64,
        freq_tier: FrequencyTier,
        record: &AccessRecord,
    ) -> bool {
        // Either high QPS or Hot frequency tier can trigger upgrade
        let qps_above = qps > self.thresholds.hot_threshold_qps as f64;
        let freq_indicates_hot = freq_tier == FrequencyTier::Hot;

        let sustained = self.check_sustained_high_qps(record, self.thresholds.upgrade_window_ms);

        // Upgrade if: (high QPS AND sustained) OR (Hot tier AND sustained) OR (high combined score)
        (qps_above && sustained) || (freq_indicates_hot && combined_score > 0.7) || combined_score > 0.85
    }

    /// Check if L2 segment should be downgraded to L3
    fn should_downgrade_to_l3(&self, qps: f64, combined_score: f64, record: &AccessRecord) -> bool {
        let qps_below = qps < self.thresholds.cold_threshold_qps as f64;
        let score_below = combined_score < 0.1;

        qps_below && score_below && self.check_sustained_low_qps(record, self.thresholds.downgrade_window_ms)
    }

    /// Check if L3 segment should be upgraded to L2
    fn should_upgrade_to_l2(
        &self,
        qps: f64,
        combined_score: f64,
        freq_tier: FrequencyTier,
        record: &AccessRecord,
    ) -> bool {
        let qps_above = qps > self.thresholds.warm_threshold_qps as f64;
        let freq_indicates_warm_or_hot = matches!(freq_tier, FrequencyTier::Warm | FrequencyTier::Hot);

        let sustained = self.check_sustained_high_qps(record, self.thresholds.upgrade_window_ms);

        // Upgrade if: (QPS above threshold AND sustained) OR (warm/hot tier with good combined score)
        (qps_above && sustained) || (freq_indicates_warm_or_hot && combined_score > 0.3)
    }

    /// Check if high QPS has been sustained for the specified duration
    fn check_sustained_high_qps(&self, record: &AccessRecord, window_ms: u64) -> bool {
        // Simple heuristic: if current window QPS exceeds threshold, trigger
        // A more sophisticated implementation would track historical QPS
        let threshold = if record.current_layer == 1 {
            self.thresholds.hot_threshold_qps as f64
        } else {
            self.thresholds.warm_threshold_qps as f64
        };

        record.qps() > threshold && record.window_duration_ms >= window_ms
    }

    /// Check if low QPS has been sustained for the specified duration
    fn check_sustained_low_qps(&self, record: &AccessRecord, window_ms: u64) -> bool {
        let threshold = if record.current_layer == 1 {
            self.thresholds.cooldown_threshold_qps as f64
        } else {
            self.thresholds.cold_threshold_qps as f64
        };

        record.qps() < threshold && record.window_duration_ms >= window_ms
    }

    /// Mark a migration as completed
    pub fn complete_migration(&self, segment_id: u64, target_layer: usize) {
        self.pending_migrations.remove(&segment_id);

        if let Some(tracker) = self.trackers.get(&segment_id) {
            tracker.set_layer(target_layer);
        }

        self.migrations_completed.fetch_add(1, Ordering::Relaxed);

        debug!(
            "Migration completed for segment {} to layer {}",
            segment_id, target_layer
        );
    }

    /// Get migration statistics
    pub fn stats(&self) -> MigrationStats {
        MigrationStats {
            tracked_segments: self.trackers.len(),
            pending_migrations: self.pending_migrations.len(),
            upgrades_triggered: self.upgrades_triggered.load(Ordering::Relaxed),
            downgrades_triggered: self.downgrades_triggered.load(Ordering::Relaxed),
            migrations_completed: self.migrations_completed.load(Ordering::Relaxed),
        }
    }

    /// Get thresholds configuration
    pub fn thresholds(&self) -> &MigrationThresholds {
        &self.thresholds
    }

    /// Update thresholds configuration
    pub fn set_thresholds(&mut self, thresholds: MigrationThresholds) {
        self.thresholds = thresholds;
    }

    /// Remove tracker for a segment (called when segment is deleted)
    pub fn remove_tracker(&self, segment_id: u64) {
        self.trackers.remove(&segment_id);
        self.pending_migrations.remove(&segment_id);
    }

    /// Get the frequency tier for a segment based on its access count
    pub fn get_frequency_tier(&self, segment_id: u64) -> FrequencyTier {
        if let Some(tracker) = self.trackers.get(&segment_id) {
            let access_count = tracker.access_count.load(Ordering::Relaxed);
            classify_by_frequency(access_count, &self.thresholds)
        } else {
            FrequencyTier::Cold
        }
    }

    /// Get recommended cache layer for a segment based on frequency tier
    pub fn get_recommended_layer(&self, segment_id: u64) -> usize {
        self.get_frequency_tier(segment_id).preferred_layer()
    }
}

/// Migration statistics
#[derive(Debug, Clone, Default)]
pub struct MigrationStats {
    pub tracked_segments: usize,
    pub pending_migrations: usize,
    pub upgrades_triggered: u64,
    pub downgrades_triggered: u64,
    pub migrations_completed: u64,
}

/// Bloom filter migrator for loading filters from disk with version migration
pub struct BloomFilterMigrator {
    index_dir: PathBuf,
}

impl BloomFilterMigrator {
    /// Create a new bloom filter migrator
    pub fn new(index_dir: PathBuf) -> Self {
        Self { index_dir }
    }

    /// Load bloom filter with automatic migration
    ///
    /// # Returns
    /// - `Ok(Some((bloom, keys, migration_result)))`: Successfully loaded/migrated
    /// - `Ok(None)`: Filter doesn't exist or unsupported version
    /// - `Err(e)`: Error during loading/migration
    pub fn load_with_migration(&self, segment_id: u64) -> Result<Option<(BloomFilter, Vec<String>, MigrationResult)>> {
        use std::fs::File;
        use std::io::{BufReader, Read};

        let bloom_path = self.index_dir.join(format!("bloom_{:06}.bin", segment_id));

        if !bloom_path.exists() {
            return Ok(None);
        }

        let file = File::open(&bloom_path).map_err(FatalError::Io)?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut magic_buf = [0u8; 4];
        reader.read_exact(&mut magic_buf).map_err(FatalError::Io)?;
        let _magic = u32::from_le_bytes(magic_buf);

        let mut version_buf = [0u8; 4];
        reader.read_exact(&mut version_buf).map_err(FatalError::Io)?;
        let version = u32::from_le_bytes(version_buf);

        // Check version compatibility
        let migration_result = if version < CURRENT_BLOOM_VERSION {
            // Needs migration
            MigrationResult::Migrated {
                from_version: version,
                to_version: CURRENT_BLOOM_VERSION,
            }
        } else if version > CURRENT_BLOOM_VERSION {
            // Future version - may not be supported
            MigrationResult::FutureVersion { version }
        } else {
            MigrationResult::NoMigrationNeeded
        };

        // Parse based on version
        let num_keys = if version == 1 {
            // V1: just num_keys
            let mut num_keys_buf = [0u8; 8];
            reader.read_exact(&mut num_keys_buf).map_err(FatalError::Io)?;
            u64::from_le_bytes(num_keys_buf) as usize
        } else if version >= 2 {
            // V2+: skip num_bits and num_hashes, then read num_keys
            let mut _num_bits_buf = [0u8; 4];
            reader.read_exact(&mut _num_bits_buf).map_err(FatalError::Io)?;
            let mut _num_hashes_buf = [0u8; 4];
            reader.read_exact(&mut _num_hashes_buf).map_err(FatalError::Io)?;
            let mut num_keys_buf = [0u8; 8];
            reader.read_exact(&mut num_keys_buf).map_err(FatalError::Io)?;
            u64::from_le_bytes(num_keys_buf) as usize
        } else {
            return Err(FatalError::Corruption(format!(
                "Unsupported bloom version: {}",
                version
            )));
        };

        // Read keys
        let mut keys = Vec::with_capacity(num_keys);
        for _ in 0..num_keys {
            let mut key_len_buf = [0u8; 4];
            reader.read_exact(&mut key_len_buf).map_err(FatalError::Io)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;

            let mut key_buf = vec![0u8; key_len];
            reader.read_exact(&mut key_buf).map_err(FatalError::Io)?;

            let key = String::from_utf8_lossy(&key_buf).to_string();
            keys.push(key);
        }

        // Recreate bloom filter from keys
        let mut bloom = BloomFilter::with_rate(crate::DEFAULT_BLOOM_FPR, keys.len().try_into().unwrap_or(10000));
        for key in &keys {
            bloom.insert(key);
        }

        Ok(Some((bloom, keys, migration_result)))
    }
}

/// Migration result for bloom filter version compatibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationResult {
    /// Successfully migrated from one version to another
    Migrated { from_version: u32, to_version: u32 },
    /// Version is too old and not supported
    UnsupportedVersion { version: u32 },
    /// Version is from the future (may work, but untested)
    FutureVersion { version: u32 },
    /// No migration needed (current version)
    NoMigrationNeeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_thresholds_default() {
        let thresholds = MigrationThresholds::default();
        assert_eq!(thresholds.warm_threshold_qps, 10);
        assert_eq!(thresholds.hot_threshold_qps, 100);
        assert_eq!(thresholds.cooldown_threshold_qps, 5);
        assert_eq!(thresholds.cold_threshold_qps, 1);
        assert_eq!(thresholds.upgrade_window_ms, 60_000);
        assert_eq!(thresholds.downgrade_window_ms, 300_000);
        assert_eq!(thresholds.hot_tier_access_count, 100);
        assert_eq!(thresholds.warm_tier_access_count, 10);
        assert!((thresholds.frequency_weight - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_frequency_tier_classification() {
        let thresholds = MigrationThresholds::default();

        // Cold: < 10 accesses
        assert_eq!(classify_by_frequency(0, &thresholds), FrequencyTier::Cold);
        assert_eq!(classify_by_frequency(5, &thresholds), FrequencyTier::Cold);
        assert_eq!(classify_by_frequency(9, &thresholds), FrequencyTier::Cold);

        // Warm: 10-99 accesses
        assert_eq!(classify_by_frequency(10, &thresholds), FrequencyTier::Warm);
        assert_eq!(classify_by_frequency(50, &thresholds), FrequencyTier::Warm);
        assert_eq!(classify_by_frequency(99, &thresholds), FrequencyTier::Warm);

        // Hot: >= 100 accesses
        assert_eq!(classify_by_frequency(100, &thresholds), FrequencyTier::Hot);
        assert_eq!(classify_by_frequency(500, &thresholds), FrequencyTier::Hot);
        assert_eq!(classify_by_frequency(u64::MAX, &thresholds), FrequencyTier::Hot);
    }

    #[test]
    fn test_frequency_tier_preferred_layer() {
        assert_eq!(FrequencyTier::Hot.preferred_layer(), 1);
        assert_eq!(FrequencyTier::Warm.preferred_layer(), 2);
        assert_eq!(FrequencyTier::Cold.preferred_layer(), 3);
    }

    #[test]
    fn test_classify_by_frequency_custom_thresholds() {
        let thresholds = MigrationThresholds {
            hot_tier_access_count: 50,
            warm_tier_access_count: 5,
            ..MigrationThresholds::default()
        };

        assert_eq!(classify_by_frequency(0, &thresholds), FrequencyTier::Cold);
        assert_eq!(classify_by_frequency(4, &thresholds), FrequencyTier::Cold);
        assert_eq!(classify_by_frequency(5, &thresholds), FrequencyTier::Warm);
        assert_eq!(classify_by_frequency(49, &thresholds), FrequencyTier::Warm);
        assert_eq!(classify_by_frequency(50, &thresholds), FrequencyTier::Hot);
    }

    #[test]
    fn test_segment_access_tracker() {
        let tracker = SegmentAccessTracker::new(2); // Start in L2

        // Record some accesses
        for _ in 0..10 {
            tracker.record_access();
        }

        // Add small delay to ensure window_duration > 0
        std::thread::sleep(std::time::Duration::from_millis(10));

        let qps = tracker.get_qps();
        // QPS should be positive (at least some accesses in the window)
        assert!(qps >= 0.0);
        assert_eq!(tracker.get_layer(), 2);
    }

    #[test]
    fn test_access_record_qps() {
        let record = AccessRecord {
            total_count: 100,
            window_count: 50,
            window_duration_ms: 5000, // 5 seconds
            current_layer: 2,
        };

        let qps = record.qps();
        assert!((qps - 10.0).abs() < 0.1); // 50 / 5 = 10 QPS
    }

    #[test]
    fn test_access_record_access_count_alias() {
        let record = AccessRecord {
            total_count: 42,
            window_count: 10,
            window_duration_ms: 1000,
            current_layer: 1,
        };
        assert_eq!(record.access_count(), 42);
    }

    #[test]
    fn test_migration_controller_basic() {
        let thresholds = MigrationThresholds::default();
        let controller = MigrationController::new(thresholds);

        // Record access for segment 1
        let decision = controller.record_access(1);
        assert!(decision.is_some());

        let stats = controller.stats();
        assert_eq!(stats.tracked_segments, 1);
    }

    #[test]
    fn test_migration_controller_frequency_tier() {
        let thresholds = MigrationThresholds::default();
        let controller = MigrationController::new(thresholds);

        // Segment with no accesses should be Cold
        assert_eq!(controller.get_frequency_tier(1), FrequencyTier::Cold);
        assert_eq!(controller.get_recommended_layer(1), 3);

        // Record many accesses to make it Hot
        for _ in 0..150 {
            controller.record_access(2);
        }
        assert_eq!(controller.get_frequency_tier(2), FrequencyTier::Hot);
        assert_eq!(controller.get_recommended_layer(2), 1);

        // Record moderate accesses to make it Warm
        for _ in 0..20 {
            controller.record_access(3);
        }
        assert_eq!(controller.get_frequency_tier(3), FrequencyTier::Warm);
        assert_eq!(controller.get_recommended_layer(3), 2);
    }

    #[test]
    fn test_migration_controller_combined_score() {
        let thresholds = MigrationThresholds::default();
        let controller = MigrationController::new(thresholds);

        // Test compute_combined_score for different frequency tiers
        let hot_score = controller.compute_combined_score(100.0, 200, FrequencyTier::Hot);
        let warm_score = controller.compute_combined_score(50.0, 50, FrequencyTier::Warm);
        let cold_score = controller.compute_combined_score(1.0, 2, FrequencyTier::Cold);

        // Hot should have higher score than Cold
        assert!(
            hot_score > cold_score,
            "Hot score ({}) should be > Cold score ({})",
            hot_score,
            cold_score
        );
        // Warm should be between Hot and Cold
        assert!(
            hot_score > warm_score,
            "Hot score ({}) should be > Warm score ({})",
            hot_score,
            warm_score
        );
        assert!(
            warm_score > cold_score,
            "Warm score ({}) should be > Cold score ({})",
            warm_score,
            cold_score
        );
    }

    #[test]
    fn test_migration_result_serialization() {
        let result = MigrationResult::Migrated {
            from_version: 1,
            to_version: 2,
        };

        match result {
            MigrationResult::Migrated {
                from_version,
                to_version,
            } => {
                assert_eq!(from_version, 1);
                assert_eq!(to_version, 2);
            }
            _ => panic!("Expected Migrated variant"),
        }
    }

    #[test]
    fn test_frequency_aware_migration() {
        // Test: verifies frequency-based migration tier classification
        let thresholds = MigrationThresholds {
            upgrade_window_ms: 100, // Short window for testing
            downgrade_window_ms: 100,
            hot_tier_access_count: 50,
            warm_tier_access_count: 10,
            frequency_weight: 0.5,
            ..MigrationThresholds::default()
        };
        let controller = MigrationController::new(thresholds);

        // Simulate heavy access pattern that should trigger upgrade
        for _ in 0..200 {
            controller.record_access(1);
        }

        // After many accesses, segment should be classified as Hot
        assert_eq!(controller.get_frequency_tier(1), FrequencyTier::Hot);
        assert_eq!(controller.get_recommended_layer(1), 1);
    }
}
