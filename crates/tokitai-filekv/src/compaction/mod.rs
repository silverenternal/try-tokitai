//! Compaction module for FileKV
//!
//! This module implements LSM-Tree style compaction with streaming merge iterator:
//! - **Streaming Merge Iterator**: Merges multiple sorted segment streams without loading all data into memory
//! - Memory efficiency: O(num_segments) instead of O(total_keys)
//!
//! Submodules:
//! - `merge_iterator`: K-way merge iterator for sorted KV streams
//! - `segment_iterator`: Streaming iterator over segment files
//! - `manifest`: Crash-safe compaction manifest tracking
//! - `trigger`: Compaction trigger strategies

pub mod manifest;
#[cfg(test)]
mod manifest_crash_tests;
pub mod merge_iterator;
pub mod segment_iterator;
pub mod trigger;

// Re-export main types for convenience
pub use manifest::{recover_incomplete, CompactionExecutor, CompactionManifest, CompactionStatus, RecoveryAction};
pub use merge_iterator::{KVIterator, MergeIterator, MergeIteratorBuilder};
pub use segment_iterator::{SegmentIterator, SegmentIteratorBuilder};
pub use trigger::{default_compaction_trigger, CompactionTrigger, TriggerResult, TriggerState, TriggerType};

// Legacy compaction types (kept for backward compatibility)
use std::collections::BTreeMap;
use std::hash::Hasher;
use std::io::Write;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use rayon::prelude::*;

use crate::core::segment::{SegmentFile, SEGMENT_MAGIC, SEGMENT_VERSION};
use crate::core::sparse_index::{self, SparseIndex};
use crate::query::zone_map::ZoneMapEntry;

/// Trait providing the minimum interface needed for compaction operations
/// This decouples compaction from direct FileKV dependency
pub trait CompactionContext: Send + Sync {
    // Config access
    fn filekv_config(&self) -> &crate::FileKVConfig;
    fn compaction_config(&self) -> CompactionConfig;

    // Segment access
    fn segments_read(&self) -> arc_swap::Guard<Arc<BTreeMap<u64, Arc<SegmentFile>>>>;
    fn swap_segments(&self, new_segments: BTreeMap<u64, Arc<SegmentFile>>);

    // Index manager access
    fn index_manager_read(&self) -> parking_lot::RwLockReadGuard<'_, crate::core::sparse_index::IndexManager>;
    fn swap_index_manager(&self, new_index_manager: crate::core::sparse_index::IndexManager);

    // Segment ID generation
    fn next_segment_id(&self) -> u64;

    // Stats
    fn adjust_total_size_bytes(&self, delta: i64);

    // Cache invalidation
    fn invalidate_segment_cache(&self, segment_id: u64);

    // Bloom filter rebuild
    fn rebuild_bloom_for_segment(&self, segment_id: u64, keys: &BTreeMap<String, Vec<u8>>) -> crate::bloom::Result<()>;

    // V0.6.0: Global key index operations (split to avoid race conditions)
    fn mark_segments_stale_for_compaction(&self, segment_ids: &[u64]);
    fn clear_stale_segments_after_compaction(&self);
    fn remove_old_segments_from_global_index(&self, old_segment_ids: &[u64]);
    fn add_new_segments_to_global_index(
        &self,
        new_segment_id: u64,
        new_keys: &[(String, u64, usize)], // (key, offset, value_len)
    );

    // Compaction stats recording
    fn record_compaction_stats(&self, stats: &CompactionStats);

    // Prometheus metrics (feature-gated)
    #[cfg(feature = "metrics")]
    fn record_prometheus_metrics(&self, stats: &CompactionStats);
}

/// Compaction strategy for L0 segments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompactionStrategy {
    /// Size-Tiered Compaction Strategy: merges segments of similar size
    /// Best for L0 where key ranges overlap
    SizeTiered,
    /// Leveled Compaction Strategy: merges segments into sorted levels
    /// Best for L1+ where key ranges are non-overlapping
    #[default]
    Leveled,
}

/// Compaction configuration
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub min_segments: usize,
    pub auto_compact: bool,
    pub check_interval: usize,
    pub max_segment_size_bytes: u64,
    pub target_segment_size_bytes: u64,
    /// 1.1 OPTIMIZATION: Enable async compaction mode
    /// When true, compaction runs in background thread via channel
    /// When false, compaction runs synchronously (current behavior)
    pub async_compaction_enabled: bool,
    /// 1.2 OPTIMIZATION: Enable leveled compaction (L0/L1/L2/L3)
    /// When true, uses leveled compaction strategy
    /// When false, uses size-tiered compaction (current behavior)
    pub leveled_compaction_enabled: bool,
    /// 1.2 OPTIMIZATION: Level size multiplier (each level is N times larger than previous)
    /// Default: 10 (like LevelDB)
    pub level_size_multiplier: usize,
    /// 1.2 OPTIMIZATION: Maximum compaction level (default: 3 = L3)
    pub max_level: u8,
    /// 1.2 OPTIMIZATION: L0 file count trigger for compaction
    /// When L0 has >= this many segments, trigger compaction
    pub l0_file_count_threshold: usize,
    /// 1.3 OPTIMIZATION: Enable parallel compaction
    /// When true, segments are read in parallel during compaction
    pub parallel_compaction_enabled: bool,
    /// When true, uses MergeIterator instead of BTreeMap for compaction
    /// When false, uses legacy BTreeMap approach (loads all keys into memory)
    pub streaming_compaction_enabled: bool,
    /// OPT-003: Write amplification threshold for triggering compaction
    /// When WA exceeds this value, compaction is forced to reduce amplification
    /// Default: 3.0x (aggressive, keeps WA low)
    pub write_amplification_threshold: f64,
    /// OPT-003: Maximum number of background compaction threads
    /// Default: min(4, num_cpus/2) for parallel compaction without overwhelming the system
    pub max_background_compaction_threads: usize,
    /// OPT-003: L0 total size threshold in bytes - trigger compaction when L0 total size exceeds this
    /// This complements l0_file_count_threshold by also considering the actual data size
    /// Default: 64MB (triggers compaction even with few files if they're large)
    pub l0_size_bytes_threshold: u64,
    /// OPT-006: L0 compaction strategy (STCS vs LCS)
    /// Default: Leveled (backward compatible)
    pub l0_compaction_strategy: CompactionStrategy,
    /// OPT-006: Minimum number of L0 segments to trigger STCS
    /// Default: 3
    pub l0_stcs_min_segments: usize,
    /// OPT-006: Size ratio threshold for STCS segment grouping
    /// Segments are grouped together if their size ratio is < this value
    /// Default: 2.0 (segments within 2x size are merged together)
    pub l0_stcs_size_ratio: f64,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            min_segments: 4,
            auto_compact: true,
            check_interval: 100,
            max_segment_size_bytes: 256 * 1024 * 1024,    // 256MB
            target_segment_size_bytes: 128 * 1024 * 1024, // 128MB
            async_compaction_enabled: true,               // 1.1 OPTIMIZATION: Enabled by default
            leveled_compaction_enabled: true,             // 1.2 OPTIMIZATION: Enabled by default
            level_size_multiplier: 10,                    // 1.2 OPTIMIZATION: LevelDB-style sizing
            max_level: 3,                                 // 1.2 OPTIMIZATION: L0/L1/L2/L3
            l0_file_count_threshold: 3, // 1.2 OPTIMIZATION: Compact when L0 has 3+ files (reduced from 4 to avoid L0 buildup)
            parallel_compaction_enabled: true, // 1.3 OPTIMIZATION: Enabled by default
            streaming_compaction_enabled: true, // Enabled by default
            write_amplification_threshold: 3.0, // OPT-003: Force compaction when WA > 3x
            max_background_compaction_threads: std::cmp::min(4, (num_cpus::get() / 2).max(1)), // OPT-003: min(4, num_cpus/2)
            l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: 64MB L0 size trigger
            // OPT-006: STCS for L0 - defaults to Leveled for backward compatibility
            l0_compaction_strategy: CompactionStrategy::Leveled,
            l0_stcs_min_segments: 3,
            l0_stcs_size_ratio: 2.0,
        }
    }
}

impl CompactionConfig {
    pub fn validate(&self) -> crate::core::types::FileKVConfigValidation {
        let mut validation = crate::core::types::FileKVConfigValidation::default();

        if self.max_segment_size_bytes < self.target_segment_size_bytes {
            validation
                .errors
                .push(crate::core::types::FileKVConfigError::SegmentSizeMismatch {
                    max: self.max_segment_size_bytes,
                    target: self.target_segment_size_bytes,
                });
        }

        validation
    }
}

/// Compaction statistics
#[derive(Debug, Clone, Default)]
pub struct CompactionStats {
    pub compaction_runs: u64,
    pub segments_merged: u64,
    pub bytes_compacted: u64,
    pub entries_removed: u64,
    /// 1.5 OPTIMIZATION: Tombstones cleaned during compaction
    pub tombstones_cleaned: u64,
    /// OPT-008: Total bytes read from old segments during compaction
    pub bytes_read_from_segments: u64,
    /// OPT-008: Total bytes written to new segment during compaction
    pub bytes_written_to_segment: u64,
}

/// Compaction manager
pub struct CompactionManager {
    config: CompactionConfig,
    write_count: AtomicUsize,
    stats: Arc<CompactionStatsInner>,
    /// Channel sender for requesting async compaction
    pub(crate) tx: Option<mpsc::Sender<CompactionRequest>>,
    /// OPT-003: Write amplification tracking
    user_bytes_written: AtomicU64,
    total_bytes_written: AtomicU64,
    /// OPT-003: WA-aware trigger (optional, can be set externally)
    wa_aware_trigger: Option<Arc<crate::compaction::trigger::WriteAmplificationAwareTrigger>>,
    /// OPT-003: L0 segment count tracker
    l0_segment_count: AtomicUsize,
    /// OPT-003: L0 total size tracker
    l0_total_size_bytes: AtomicU64,
}

#[derive(Debug, Default)]
struct CompactionStatsInner {
    runs: AtomicU64,
    merged: AtomicU64,
    bytes: AtomicU64,
    entries_removed: AtomicU64,
    /// 1.5 OPTIMIZATION: Tombstones cleaned
    tombstones_cleaned: AtomicU64,
}

impl CompactionManager {
    pub fn new(config: CompactionConfig) -> Self {
        let tx = if config.async_compaction_enabled {
            let (tx, _rx) = mpsc::channel();
            Some(tx)
        } else {
            None
        };

        Self {
            config,
            write_count: AtomicUsize::new(0),
            stats: Arc::default(),
            tx,
            user_bytes_written: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(0),
            wa_aware_trigger: None,
            l0_segment_count: AtomicUsize::new(0),
            l0_total_size_bytes: AtomicU64::new(0),
        }
    }

    pub fn record_write(&self) -> bool {
        let count = self.write_count.fetch_add(1, Ordering::Relaxed);
        let should_run = count >= self.config.check_interval;
        if should_run {
            // Reset counter so next check happens after another check_interval writes
            self.write_count.store(0, Ordering::Relaxed);
        }
        should_run && self.config.auto_compact
    }

    pub fn should_run_compaction(&self) -> bool {
        self.config.auto_compact
    }

    pub fn reset_write_count(&self) {
        self.write_count.store(0, Ordering::Relaxed);
    }

    pub fn stats(&self) -> CompactionStats {
        CompactionStats {
            compaction_runs: self.stats.runs.load(Ordering::Relaxed),
            segments_merged: self.stats.merged.load(Ordering::Relaxed),
            bytes_compacted: self.stats.bytes.load(Ordering::Relaxed),
            entries_removed: self.stats.entries_removed.load(Ordering::Relaxed),
            tombstones_cleaned: self.stats.tombstones_cleaned.load(Ordering::Relaxed),
            bytes_read_from_segments: 0, // Aggregate stats only, per-compaction details not tracked here
            bytes_written_to_segment: 0,
        }
    }

    pub fn record_compaction(
        &self,
        segments_merged: u64,
        bytes_compacted: u64,
        entries_removed: u64,
        tombstones_cleaned: u64,
    ) {
        self.stats.runs.fetch_add(1, Ordering::Relaxed);
        self.stats.merged.fetch_add(segments_merged, Ordering::Relaxed);
        self.stats.bytes.fetch_add(bytes_compacted, Ordering::Relaxed);
        self.stats.entries_removed.fetch_add(entries_removed, Ordering::Relaxed);
        self.stats
            .tombstones_cleaned
            .fetch_add(tombstones_cleaned, Ordering::Relaxed);
    }

    pub fn config(&self) -> &CompactionConfig {
        &self.config
    }

    /// OPT-006: Send level-specific compaction request to background thread (non-blocking)
    ///
    /// # Returns
    /// `true` if request sent successfully, `false` if async compaction is disabled or channel closed
    pub fn request_level_compaction(&self, segment_count: usize, total_size_bytes: u64, target_level: u8) -> bool {
        if let Some(ref tx) = self.tx {
            let req = CompactionRequest {
                segment_count,
                total_size_bytes,
                target_level: Some(target_level),
            };
            tx.send(req).is_ok()
        } else {
            false
        }
    }

    /// OPT-003: Record user bytes written for write amplification tracking
    pub fn record_user_bytes(&self, bytes: u64) {
        self.user_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// OPT-003: Record total bytes written (including WAL, compaction overhead)
    pub fn record_total_bytes(&self, bytes: u64) {
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// OPT-003: Calculate current write amplification factor
    /// WA = total bytes written / user bytes written
    pub fn write_amplification_factor(&self) -> f64 {
        let user = self.user_bytes_written.load(Ordering::Relaxed) as f64;
        let total = self.total_bytes_written.load(Ordering::Relaxed) as f64;
        if user == 0.0 {
            return 1.0;
        }
        total / user
    }

    /// OPT-003: Check if compaction should be triggered due to high write amplification
    /// Returns true if WA exceeds the configured threshold
    pub fn should_compact_by_amplification(&self) -> bool {
        let wa = self.write_amplification_factor();
        wa > self.config.write_amplification_threshold
    }

    /// OPT-003: Reset write amplification counters (called after successful compaction)
    pub fn reset_amplification_counters(&self) {
        self.user_bytes_written.store(0, Ordering::Relaxed);
        self.total_bytes_written.store(0, Ordering::Relaxed);
    }

    /// OPT-003: Set WA-aware trigger
    pub fn set_wa_aware_trigger(&mut self, trigger: Arc<crate::compaction::trigger::WriteAmplificationAwareTrigger>) {
        self.wa_aware_trigger = Some(trigger);
    }

    /// OPT-003: Get WA-aware trigger reference
    pub fn wa_aware_trigger(&self) -> Option<&Arc<crate::compaction::trigger::WriteAmplificationAwareTrigger>> {
        self.wa_aware_trigger.as_ref()
    }

    /// OPT-003: Update L0 segment count and total size
    pub fn update_l0_segments(&self, count: usize, total_size_bytes: u64) {
        self.l0_segment_count.store(count, Ordering::Relaxed);
        self.l0_total_size_bytes.store(total_size_bytes, Ordering::Relaxed);
    }

    /// OPT-003: Get current L0 segment count
    pub fn l0_segment_count(&self) -> usize {
        self.l0_segment_count.load(Ordering::Relaxed)
    }

    /// OPT-003: Get current L0 total size
    pub fn l0_total_size_bytes(&self) -> u64 {
        self.l0_total_size_bytes.load(Ordering::Relaxed)
    }

    /// OPT-003: Evaluate WA-aware compaction priority
    /// Returns (should_compact, priority, should_pause)
    pub fn evaluate_wa_aware_priority(&self) -> Option<(bool, crate::compaction::trigger::CompactionPriority, bool)> {
        self.wa_aware_trigger.as_ref().map(|trigger| {
            let wa = self.write_amplification_factor();
            let l0_count = self.l0_segment_count();
            let l0_size = self.l0_total_size_bytes();

            let state = trigger.build_state(wa, l0_count, l0_size);
            trigger.should_compact(&state)
        })
    }

    /// 1.1 OPTIMIZATION: Send compaction request to background thread (non-blocking)
    ///
    /// # Returns
    /// `true` if request sent successfully, `false` if async compaction is disabled or channel closed
    pub fn request_compaction(&self, segment_count: usize, total_size_bytes: u64) -> bool {
        if let Some(ref tx) = self.tx {
            let req = CompactionRequest {
                segment_count,
                total_size_bytes,
                target_level: None,
            };
            tx.send(req).is_ok()
        } else {
            false
        }
    }
}

/// Message sent to background compaction thread
#[derive(Debug)]
pub struct CompactionRequest {
    /// Number of segments to consider for compaction
    pub segment_count: usize,
    /// Total size of segments
    pub total_size_bytes: u64,
    /// Target level for level-specific compaction (None = all levels)
    pub target_level: Option<u8>,
}

/// OPT-006: Select segments for Size-Tiered Compaction Strategy (STCS) for L0
///
/// Strategy:
/// 1. Filter only L0 segments (level == 0)
/// 2. Sort segments by size (smallest first)
/// 3. Group segments into "tiers" where segments within the same tier have size ratio < threshold
/// 4. Select the oldest tier that has >= min_segments segments
/// 5. Return selected segment IDs (all remain at L0 after compaction)
///
/// # Returns
/// (selected_segment_ids, output_level) - output_level is always 0 for STCS
fn select_size_tiered_segments(
    segments: &BTreeMap<u64, Arc<SegmentFile>>,
    config: &CompactionConfig,
) -> (Vec<u64>, u8) {
    // Step 1: Filter L0 segments only
    let mut l0_segments: Vec<(u64, u64)> = segments
        .iter()
        .filter(|(_, seg)| seg.level == 0)
        .map(|(&id, seg)| (id, seg.size()))
        .collect();

    if l0_segments.len() < config.l0_stcs_min_segments {
        tracing::debug!(
            "STCS: Not enough L0 segments ({} < min {}), falling back",
            l0_segments.len(),
            config.l0_stcs_min_segments
        );
        return (Vec::new(), 0);
    }

    // Step 2: Sort by size (smallest first), then by segment_id for stability
    l0_segments.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    // Step 3: Group into tiers based on size ratio
    // Segments are in the same tier if max_size / min_size < size_ratio
    let mut tiers: Vec<Vec<u64>> = Vec::new();
    let mut current_tier: Vec<u64> = Vec::new();
    let mut tier_min_size: u64 = u64::MAX;
    let mut tier_max_size: u64 = 0;

    for (id, size) in &l0_segments {
        let new_min = tier_min_size.min(*size);
        let new_max = tier_max_size.max(*size);

        // Check if adding this segment would exceed the size ratio threshold
        if !current_tier.is_empty() && new_max as f64 / new_min as f64 >= config.l0_stcs_size_ratio {
            // Current tier is full, start a new tier
            if current_tier.len() >= config.l0_stcs_min_segments {
                tiers.push(current_tier);
            }
            current_tier = vec![*id];
            tier_min_size = *size;
            tier_max_size = *size;
        } else {
            current_tier.push(*id);
            tier_min_size = new_min;
            tier_max_size = new_max;
        }
    }

    // Don't forget the last tier
    if current_tier.len() >= config.l0_stcs_min_segments {
        tiers.push(current_tier);
    }

    // Step 4: Select the first (smallest) eligible tier
    // This mimics Cassandra's STCS behavior: compact smallest tiers first
    if let Some(selected_tier) = tiers.first() {
        tracing::info!(
            "STCS: Selected tier with {} segments (size ratio threshold: {:.2})",
            selected_tier.len(),
            config.l0_stcs_size_ratio
        );
        return (selected_tier.clone(), 0); // Output stays at L0 for STCS
    }

    // Fallback: if no tier meets the criteria, select all L0 segments
    tracing::info!(
        "STCS: No eligible tier found, selecting all {} L0 segments",
        l0_segments.len()
    );
    let all_l0_ids: Vec<u64> = l0_segments.into_iter().map(|(id, _)| id).collect();
    (all_l0_ids, 0)
}

/// 1.2 OPTIMIZATION: Select segments for leveled compaction
///
/// Strategy:
/// 1. Group segments by level
/// 2. Check L0: if file count >= threshold, select all L0 segments
/// 3. Check L1+: if total size exceeds level budget, select overlapping segments
/// 4. Return selected segment IDs and the max level of selected segments
fn select_leveled_segments(segments: &BTreeMap<u64, Arc<SegmentFile>>, config: &CompactionConfig) -> (Vec<u64>, u8) {
    // Group segments by level
    let mut levels: BTreeMap<u8, Vec<u64>> = BTreeMap::new();
    for (&id, seg) in segments {
        levels.entry(seg.level).or_default().push(id);
    }

    // Check L0 first (most common trigger)
    if let Some(l0_segs) = levels.get(&0) {
        // OPT-003: Check L0 file count trigger
        if l0_segs.len() >= config.l0_file_count_threshold {
            // L0 trigger: compact all L0 segments
            tracing::info!(
                "Leveled compaction: L0 file count trigger ({} segments >= threshold {})",
                l0_segs.len(),
                config.l0_file_count_threshold
            );
            return (l0_segs.clone(), 0);
        }

        // OPT-003: Check L0 total size trigger (even with few files, large files should trigger)
        let l0_total_size: u64 = l0_segs.iter().filter_map(|id| segments.get(id)).map(|s| s.size()).sum();

        if l0_total_size >= config.l0_size_bytes_threshold {
            tracing::info!(
                "Leveled compaction: L0 size trigger ({} bytes >= threshold {} bytes)",
                l0_total_size,
                config.l0_size_bytes_threshold
            );
            return (l0_segs.clone(), 0);
        }
    }

    // Check higher levels for size-based triggers
    for (level, seg_ids) in &levels {
        if *level == 0 || *level >= config.max_level {
            continue;
        }

        // Calculate total size of this level
        let total_size: u64 = seg_ids.iter().filter_map(|id| segments.get(id)).map(|s| s.size()).sum();

        // Level budget: base_budget * level_multiplier^(level-1)
        // For L1: target_segment_size * 1 = 128MB
        // For L2: target_segment_size * 10 = 1.28GB
        // For L3: target_segment_size * 100 = 12.8GB
        let level_budget =
            config.target_segment_size_bytes * config.level_size_multiplier.pow(*level as u32 - 1) as u64;

        if total_size > level_budget {
            // Level exceeded budget: compact all segments at this level
            tracing::info!(
                "Leveled compaction: L{} size trigger ({} bytes > {} bytes budget)",
                level,
                total_size,
                level_budget
            );
            return (seg_ids.clone(), *level);
        }
    }

    // Fallback: if no leveled trigger, try size-tiered fallback
    // Select oldest segments as before
    let all_ids: Vec<u64> = segments.keys().cloned().collect();
    let selected: Vec<u64> = all_ids.iter().take(config.min_segments).cloned().collect();

    let max_lvl = selected
        .iter()
        .filter_map(|id| segments.get(id))
        .map(|s| s.level)
        .max()
        .unwrap_or(0);

    tracing::debug!(
        "Leveled compaction: falling back to size-tiered ({} segments)",
        selected.len()
    );
    (selected, max_lvl)
}

/// Execute actual compaction logic within FileKV context
///
/// # Steps
/// 0. Write compaction manifest (crash-safe)
/// 1. Select candidate segments (oldest N segments where N >= min_segments)
/// 2. Iterate all entries from selected segments
/// 3. Merge entries, keeping only the latest version of each key
/// 4. Write merged entries to a new segment file
/// 5. Build sparse/dense/zone map indexes
/// 6. Atomically update FileKV's segment map and index manager
/// 7. Remove old segment files
/// 8. Invalidate cache entries for compacted segments
/// 9. Commit manifest
pub fn execute_compaction<C: CompactionContext>(ctx: &C, req: &CompactionRequest) -> anyhow::Result<CompactionStats> {
    let config = ctx.compaction_config();

    // Step 1: Select candidate segments
    let segments_read = ctx.segments_read();

    // ENG-002 FIX: If target_level is specified, filter segments to only that level
    let (segments_to_compact, max_input_level) = if let Some(target_level) = req.target_level {
        // Level-specific compaction: only select segments at the target level
        let level_segments: Vec<u64> = segments_read
            .iter()
            .filter(|(_, seg)| seg.level == target_level)
            .map(|(&id, _)| id)
            .collect();

        if level_segments.is_empty() {
            tracing::info!("No segments found at level {}, skipping compaction", target_level);
            drop(segments_read);
            return Ok(CompactionStats::default());
        }

        let max_lvl = level_segments
            .iter()
            .filter_map(|sid| segments_read.get(sid))
            .map(|s| s.level)
            .max()
            .unwrap_or(target_level);

        tracing::info!(
            "Level-specific compaction: selected {} segments at level {}",
            level_segments.len(),
            target_level
        );
        (level_segments, max_lvl)
    } else if config.leveled_compaction_enabled {
        // Check if L0 should use STCS or LCS
        let l0_segments: Vec<u64> = segments_read
            .iter()
            .filter(|(_, seg)| seg.level == 0)
            .map(|(&id, _)| id)
            .collect();

        // OPT-006: Check if L0 has enough segments to trigger compaction
        let l0_trigger = l0_segments.len() >= config.l0_file_count_threshold || {
            let l0_total_size: u64 = l0_segments
                .iter()
                .filter_map(|id| segments_read.get(id))
                .map(|s| s.size())
                .sum();
            l0_total_size >= config.l0_size_bytes_threshold
        };

        if l0_trigger && !l0_segments.is_empty() {
            // OPT-006: Use configured strategy for L0 compaction
            match config.l0_compaction_strategy {
                CompactionStrategy::SizeTiered => {
                    tracing::info!("L0 compaction: using Size-Tiered Compaction Strategy (STCS)");
                    let (selected, output_level) = select_size_tiered_segments(&segments_read, &config);
                    if selected.is_empty() {
                        // Fallback to leveled if STCS returns empty
                        tracing::debug!("STCS returned empty selection, falling back to leveled");
                        select_leveled_segments(&segments_read, &config)
                    } else {
                        (selected, output_level)
                    }
                }
                CompactionStrategy::Leveled => {
                    tracing::info!("L0 compaction: using Leveled Compaction Strategy (LCS)");
                    select_leveled_segments(&segments_read, &config)
                }
            }
        } else {
            // L0 not triggered, check higher levels with leveled compaction
            select_leveled_segments(&segments_read, &config)
        }
    } else {
        // Fallback to size-tiered: select oldest segments
        let segment_ids: Vec<u64> = segments_read.keys().cloned().collect();
        let tiered: Vec<u64> = segment_ids.iter().take(config.min_segments).cloned().collect();
        let max_lvl = tiered
            .iter()
            .filter_map(|sid| segments_read.get(sid))
            .map(|s| s.level)
            .max()
            .unwrap_or(0);
        (tiered, max_lvl)
    };

    let segment_count = segments_to_compact.len();
    // OPT-006: Use strategy-specific minimum segment check
    let min_segments_required =
        if config.l0_compaction_strategy == CompactionStrategy::SizeTiered && max_input_level == 0 {
            config.l0_stcs_min_segments
        } else {
            config.min_segments.min(config.l0_file_count_threshold)
        };

    if segment_count < min_segments_required {
        tracing::debug!(
            "Skipping compaction: {} segments < threshold ({})",
            segment_count,
            min_segments_required
        );
        drop(segments_read);
        return Ok(CompactionStats::default());
    }

    drop(segments_read);

    tracing::info!(
        "Selected {} segments for compaction: {:?}",
        segments_to_compact.len(),
        segments_to_compact
    );

    // Phase 5: Write compaction manifest BEFORE starting compaction
    // OPT-006: STCS for L0 keeps output at L0, LCS promotes to next level
    let output_level = if config.l0_compaction_strategy == CompactionStrategy::SizeTiered && max_input_level == 0 {
        // STCS: L0 segments merge into L0 (no level promotion)
        0
    } else {
        // LCS: promote to next level, cap at L3
        max_input_level.saturating_add(1).min(3)
    };
    let new_segment_id = ctx.next_segment_id();

    let filekv_config = ctx.filekv_config();
    let manifest_dir = filekv_config.index_dir.join("compaction_manifests");
    filekv_config.fs.create_dir_all(&manifest_dir)?;

    let mut manifest = CompactionManifest::new(
        new_segment_id, // Use new segment ID as compaction ID
        segments_to_compact.clone(),
        vec![new_segment_id], // Output segment ID
        output_level,
    );

    let mut executor = CompactionExecutor::new(filekv_config.fs.clone(), manifest_dir);

    // Write manifest before compaction starts
    if let Err(e) = executor.prepare(&manifest) {
        tracing::error!("Failed to write compaction manifest: {}", e);
        return Err(anyhow::anyhow!("Compaction manifest preparation failed: {}", e));
    }

    // Now execute compaction - if we crash after this point, recovery will clean up
    let result = execute_compaction_inner(
        ctx,
        &config,
        &segments_to_compact,
        max_input_level,
        new_segment_id,
        output_level,
    );

    match result {
        Ok(stats) => {
            // Commit manifest on success
            manifest.mark_completed();
            if let Err(e) = executor.commit(&mut manifest) {
                tracing::warn!("Failed to commit compaction manifest: {}", e);
                // Non-critical, compaction itself succeeded
            }
            Ok(stats)
        }
        Err(e) => {
            // Abort manifest on failure
            let _ = executor.abort(&mut manifest);
            Err(e)
        }
    }
}

/// Inner compaction logic (separated from manifest handling)
fn execute_compaction_inner<C: CompactionContext>(
    ctx: &C,
    config: &CompactionConfig,
    segments_to_compact: &[u64],
    _max_input_level: u8,
    new_segment_id: u64,
    output_level: u8,
) -> anyhow::Result<CompactionStats> {
    // Choose between streaming and legacy compaction
    if config.streaming_compaction_enabled {
        execute_streaming_compaction(ctx, config, segments_to_compact, new_segment_id, output_level)
    } else {
        execute_legacy_compaction(ctx, config, segments_to_compact, new_segment_id, output_level)
    }
}

/// Streaming compaction using MergeIterator
/// Memory usage: O(num_segments) instead of O(total_keys)
fn execute_streaming_compaction<C: CompactionContext>(
    ctx: &C,
    _config: &CompactionConfig,
    segments_to_compact: &[u64],
    new_segment_id: u64,
    output_level: u8,
) -> anyhow::Result<CompactionStats> {
    // V0.6.0: Mark segments as stale before compaction to prevent stale reads
    ctx.mark_segments_stale_for_compaction(segments_to_compact);

    // Step 1: Collect segment references
    let segments_with_info = {
        let segments_read = ctx.segments_read();
        segments_to_compact
            .iter()
            .filter_map(|&segment_id| {
                segments_read.get(&segment_id).map(|segment| {
                    let size = segment.size();
                    (segment_id, segment.clone(), size)
                })
            })
            .collect::<Vec<_>>()
    };

    // Step 2: Create streaming iterators for each segment
    let mut total_bytes_before = 0u64;

    // Shared tombstone counter - all SegmentIterators will increment this
    let tombstones_cleaned = Arc::new(AtomicU64::new(0));

    let mut segment_iterators = Vec::with_capacity(segments_with_info.len());
    for (_segment_id, segment, size) in &segments_with_info {
        total_bytes_before += size;

        match SegmentIterator::with_tombstone_counter(segment.clone(), Some(tombstones_cleaned.clone())) {
            Ok(iter) => {
                segment_iterators.push(iter);
            }
            Err(e) => {
                tracing::warn!("Failed to create iterator for segment {}: {}", _segment_id, e);
            }
        }
    }

    if segment_iterators.is_empty() {
        tracing::info!("No valid segments to compact, skipping");
        return Ok(CompactionStats::default());
    }

    // Step 3: Create MergeIterator for k-way merge
    let mut merge_iter = MergeIterator::new(segment_iterators);

    // Step 4: Write merged entries to new segment file (streaming)
    let filekv_config = ctx.filekv_config();
    let temp_path = filekv_config
        .segment_dir
        .join(format!(".segment_{}.log.tmp", new_segment_id));
    let new_segment_path = filekv_config
        .segment_dir
        .join(format!("segment_{}.log", new_segment_id));

    let file = filekv_config.fs.create_file(&temp_path)?;
    // OPTIMIZATION: Use larger BufWriter buffer (256KB) to reduce syscalls during compaction
    let mut writer = std::io::BufWriter::with_capacity(256 * 1024, file);

    // Write segment header
    writer.write_all(&SEGMENT_MAGIC.to_le_bytes())?;
    writer.write_all(&SEGMENT_VERSION.to_le_bytes())?;

    let mut sparse_index = SparseIndex::new(new_segment_id);
    let block_size = filekv_config.block_size;
    let mut dense_index = sparse_index::DenseIndex::with_block_size(block_size);

    // Zone map tracking
    let mut current_block_entry_count = 0u32;
    let mut current_block_min_key: Option<String> = None;
    let mut current_block_max_key: Option<String> = None;
    let mut zone_map_entries: Vec<ZoneMapEntry> = Vec::new();
    let mut current_block_start = 8u64; // After header
    let estimated_avg_entry_size = 100u64;
    let block_entry_threshold = if estimated_avg_entry_size > 0 {
        (block_size / estimated_avg_entry_size).max(1) as u32
    } else {
        100u32
    };

    let mut current_pos = 8u64;
    let dense_index_enabled = filekv_config.aggressive.dense_index_enabled;

    // Collect keys for bloom filter (needed after compaction)
    let mut all_keys_for_bloom: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut unique_entries = 0u64;

    // Stream entries from merge iterator and write to new segment
    for (key, value) in merge_iter.by_ref() {
        let key_bytes = key.as_bytes();
        let value_bytes: &[u8] = value.as_ref();

        let key_len = key_bytes.len() as u32;
        let value_len = value_bytes.len() as u32;

        // Calculate checksum
        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(key_bytes);
        hasher.write(value_bytes);
        let checksum = hasher.finish() as u32;

        // Write entry
        writer.write_all(&key_len.to_le_bytes())?;
        writer.write_all(key_bytes)?;
        writer.write_all(&value_len.to_le_bytes())?;
        writer.write_all(value_bytes)?;
        writer.write_all(&checksum.to_le_bytes())?;

        // Update sparse index
        sparse_index.add(key.clone(), current_pos, 0);

        // Update dense index
        if dense_index_enabled {
            let block_id = dense_index.offset_to_block_id(current_pos);
            dense_index.entries.insert(
                key.clone(),
                sparse_index::DenseIndexEntry {
                    offset: current_pos,
                    key_len: key.len() as u32,
                    value_len,
                    checksum,
                    seq_num: 0,
                    block_id,
                },
            );
        }

        // Update zone map
        current_block_entry_count += 1;
        match &mut current_block_min_key {
            None => current_block_min_key = Some(key.clone()),
            Some(min_key) => {
                if key.as_str() < min_key.as_str() {
                    *min_key = key.clone();
                }
            }
        }
        match &mut current_block_max_key {
            None => current_block_max_key = Some(key.clone()),
            Some(max_key) => {
                if key.as_str() > max_key.as_str() {
                    *max_key = key.clone();
                }
            }
        }

        current_pos += (4 + key_bytes.len() + 4 + value_bytes.len() + 4) as u64;

        // Collect for bloom filter
        all_keys_for_bloom.insert(key.clone(), value.to_vec());
        unique_entries += 1;

        // Check if block is full
        if current_block_entry_count >= block_entry_threshold {
            finalize_block(
                &mut zone_map_entries,
                &mut current_block_min_key,
                &mut current_block_max_key,
                &mut current_block_start,
                &mut current_block_entry_count,
                current_pos,
            );
        }
    }

    // Finalize last block
    finalize_block(
        &mut zone_map_entries,
        &mut current_block_min_key,
        &mut current_block_max_key,
        &mut current_block_start,
        &mut current_block_entry_count,
        current_pos,
    );

    // Get precise statistics from the merge iterator
    let entries_removed = merge_iter.duplicates_removed();
    let tombstones_cleaned_count = tombstones_cleaned.load(Ordering::Relaxed);

    tracing::info!(
        "Streaming Compaction: {} unique entries, {} duplicates removed, {} tombstones cleaned (memory-efficient mode)",
        unique_entries,
        entries_removed,
        tombstones_cleaned_count
    );

    // Flush and sync the new segment file
    writer.flush()?;
    let file = writer
        .into_inner()
        .map_err(|e| anyhow::anyhow!("Failed to get inner file from BufWriter: {}", e))?;
    file.sync_all()?;

    // Atomically rename temp file to final path
    filekv_config.fs.rename(&temp_path, &new_segment_path)?;

    // Sync directory to ensure rename is persisted
    let _ = filekv_config.fs.sync_dir(&filekv_config.segment_dir);

    // Create new segment file object
    let new_segment = SegmentFile::create(
        filekv_config.fs.clone(),
        new_segment_id,
        output_level,
        &new_segment_path,
        0,
        filekv_config.aggressive.persistent_mmap_enabled,
        filekv_config.aggressive.readahead_multiplier,
        dense_index_enabled,
    )?;

    new_segment.populate_key_range_from_dense_index();
    new_segment.flush()?;

    // Update sparse_index with zone_map entries
    sparse_index.zone_map = Arc::new(zone_map_entries);

    // Step 5: Atomically update segments and indexes
    // V0.6.0: Collect key data for global index update
    let new_keys_for_index: Vec<(String, u64, usize)> = {
        let mut keys = Vec::with_capacity(all_keys_for_bloom.len());
        for (key, value) in &all_keys_for_bloom {
            if let Some(offset) = sparse_index.find(key) {
                keys.push((key.clone(), offset, value.len()));
            }
        }
        keys
    };

    // V0.6.0: Remove old entries from global index BEFORE segment swap
    // This ensures reads during the swap window fall back to segment traversal
    ctx.remove_old_segments_from_global_index(segments_to_compact);

    // COMP-001: Avoid cloning entire BTreeMap - only clone needed Arc references
    {
        let current_segments = ctx.segments_read();
        // COMP-001: Build new BTreeMap with only necessary changes
        // Start with existing segments, remove compacted ones, add new one
        let mut new_segments: BTreeMap<u64, Arc<SegmentFile>> = current_segments
            .iter()
            .filter(|(id, _)| !segments_to_compact.contains(id))
            .map(|(id, seg)| (*id, Arc::clone(seg)))
            .collect();
        drop(current_segments);

        let current_index = ctx.index_manager_read();
        let mut index_manager = (*current_index).clone();
        drop(current_index);

        // Track size delta for old segments being removed
        for &old_id in segments_to_compact {
            if let Some(old_segment) = new_segments.get(&old_id) {
                let old_size = old_segment.size();
                ctx.adjust_total_size_bytes(-(old_size as i64));
            }
        }

        // Add new segment
        new_segments.insert(new_segment_id, Arc::new(new_segment));
        ctx.adjust_total_size_bytes(current_pos as i64);

        index_manager.add_index(new_segment_id, std::sync::Arc::new(sparse_index.clone()));
        if dense_index_enabled {
            let dense_idx_path = filekv_config
                .segment_dir
                .join(format!("segment_{}.dense_idx", new_segment_id));
            match SegmentFile::save_dense_index(filekv_config.fs.as_ref(), &dense_index, &dense_idx_path) {
                Ok(_) => {
                    tracing::debug!(
                        segment_id = new_segment_id,
                        "Saved dense index to {}",
                        dense_idx_path.display()
                    );
                }
                Err(e) => {
                    tracing::warn!(segment_id = new_segment_id, "Failed to save dense index: {}", e);
                }
            }

            index_manager.add_dense_index(new_segment_id, dense_index);
        }
        index_manager.save_index(new_segment_id)?;

        // Remove old segments from the map
        for &old_id in segments_to_compact {
            new_segments.remove(&old_id);
            let old_idx_path = filekv_config.index_dir.join(format!("segment_{}.idx", old_id));
            if old_idx_path.exists() {
                let _ = filekv_config.fs.remove_file(&old_idx_path);
            }
            let old_dense_idx_path = filekv_config.segment_dir.join(format!("segment_{}.dense_idx", old_id));
            if old_dense_idx_path.exists() {
                let _ = filekv_config.fs.remove_file(&old_dense_idx_path);
            }
        }

        // Atomically swap the updated state
        ctx.swap_segments(new_segments);
        ctx.swap_index_manager(index_manager);
    }

    // V0.6.0: Add new entries to global index AFTER segment swap
    ctx.add_new_segments_to_global_index(new_segment_id, &new_keys_for_index);

    // V0.6.0: Clear stale segment tracking
    ctx.clear_stale_segments_after_compaction();

    // Build bloom filter for the new compacted segment
    if filekv_config.enable_bloom {
        if let Err(e) = ctx.rebuild_bloom_for_segment(new_segment_id, &all_keys_for_bloom) {
            tracing::warn!(
                "Failed to rebuild bloom filter for new segment {}: {}",
                new_segment_id,
                e
            );
        }
    }

    // Delete old segment files and invalidate cache
    for &old_id in segments_to_compact {
        let old_path = filekv_config.segment_dir.join(format!("segment_{}.log", old_id));
        if old_path.exists() {
            match filekv_config.fs.remove_file(&old_path) {
                Ok(_) => {
                    tracing::debug!("Deleted old segment file: {}", old_path.display());
                }
                Err(e) => {
                    tracing::warn!("Failed to delete old segment file {}: {}", old_path.display(), e);
                }
            }
        }

        ctx.invalidate_segment_cache(old_id);
    }

    let stats = CompactionStats {
        compaction_runs: 1,
        segments_merged: segments_to_compact.len() as u64,
        bytes_compacted: total_bytes_before,
        entries_removed,
        tombstones_cleaned: tombstones_cleaned_count,
        bytes_read_from_segments: total_bytes_before,
        bytes_written_to_segment: current_pos,
    };

    ctx.record_compaction_stats(&stats);

    Ok(stats)
}

/// Legacy compaction using BTreeMap (loads all keys into memory)
/// Kept for backward compatibility and fallback
fn execute_legacy_compaction<C: CompactionContext>(
    ctx: &C,
    config: &CompactionConfig,
    segments_to_compact: &[u64],
    new_segment_id: u64,
    output_level: u8,
) -> anyhow::Result<CompactionStats> {
    // V0.6.0: Mark segments as stale before compaction to prevent stale reads
    ctx.mark_segments_stale_for_compaction(segments_to_compact);

    // Step 2: Collect all entries from selected segments
    let mut all_entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut total_entries_before = 0u64;
    let mut total_bytes_before = 0u64;
    let mut total_tombstones = 0u64;

    let parallel_enabled = config.parallel_compaction_enabled;

    // Collect segment references for parallel iteration
    let segments_with_info = {
        let segments_read = ctx.segments_read();
        segments_to_compact
            .iter()
            .filter_map(|&segment_id| {
                segments_read.get(&segment_id).map(|segment| {
                    let size = segment.size();
                    (segment_id, segment.clone(), size)
                })
            })
            .collect::<Vec<_>>()
    };

    // 1.3 OPTIMIZATION: Read segments in parallel
    if parallel_enabled && segments_with_info.len() >= 2 {
        tracing::info!(
            "Parallel compaction: reading {} segments in parallel",
            segments_with_info.len()
        );

        let results: Vec<_> = segments_with_info
            .par_iter()
            .map(|(_segment_id, segment, size)| {
                let mut local_entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
                let mut local_entry_count = 0u64;
                let mut local_tombstones = 0u64;

                let iterate_result = segment.iterate_all(|key: &str, value: &[u8], _deleted: bool| {
                    if value.is_empty() {
                        local_tombstones += 1;
                        local_entry_count += 1;
                        return Ok(());
                    }

                    local_entries.insert(key.to_string(), value.to_vec());
                    local_entry_count += 1;
                    Ok(())
                });

                (
                    local_entries,
                    local_entry_count,
                    local_tombstones,
                    *size,
                    iterate_result,
                )
            })
            .collect();

        for (local_entries, entry_count, tombstones, size, result) in results {
            match result {
                Ok(()) => {
                    all_entries.extend(local_entries);
                    total_entries_before += entry_count;
                    total_bytes_before += size;
                    total_tombstones += tombstones;
                }
                Err(e) => {
                    tracing::warn!("Error reading segment during parallel compaction: {}", e);
                }
            }
        }
    } else {
        for (_segment_id, segment, size) in segments_with_info {
            total_bytes_before += size;

            let mut segment_entry_count = 0u64;
            let mut segment_tombstones = 0u64;
            segment.iterate_all(|key: &str, value: &[u8], _deleted: bool| {
                if value.is_empty() {
                    segment_tombstones += 1;
                    segment_entry_count += 1;
                    return Ok(());
                }

                all_entries.insert(key.to_string(), value.to_vec());
                segment_entry_count += 1;
                Ok(())
            })?;

            total_entries_before += segment_entry_count;
            total_tombstones += segment_tombstones;
        }
    }

    let unique_entries = all_entries.len() as u64;
    let entries_removed = total_entries_before.saturating_sub(unique_entries);

    tracing::info!(
        "Compaction: {} total entries ({} tombstones) -> {} unique entries (removed {} duplicates/tombstones)",
        total_entries_before,
        total_tombstones,
        unique_entries,
        entries_removed
    );

    if all_entries.is_empty() {
        tracing::info!("No entries to compact, skipping");
        return Ok(CompactionStats::default());
    }

    // Step 3: Write merged entries to a new segment file
    let filekv_config = ctx.filekv_config();
    let temp_path = filekv_config
        .segment_dir
        .join(format!(".segment_{}.log.tmp", new_segment_id));
    let new_segment_path = filekv_config
        .segment_dir
        .join(format!("segment_{}.log", new_segment_id));

    let mut writer = filekv_config.fs.create_file(&temp_path)?;

    // Write segment header
    writer.write_all(&SEGMENT_MAGIC.to_le_bytes())?;
    writer.write_all(&SEGMENT_VERSION.to_le_bytes())?;

    let mut sparse_index = SparseIndex::new(new_segment_id);
    let block_size = filekv_config.block_size;
    let mut dense_index = sparse_index::DenseIndex::with_block_size(block_size);

    // Zone map tracking
    let mut current_block_entry_count = 0u32;
    let mut current_block_min_key: Option<String> = None;
    let mut current_block_max_key: Option<String> = None;
    let mut zone_map_entries: Vec<ZoneMapEntry> = Vec::new();
    let mut current_block_start = 8u64;
    let estimated_avg_entry_size = 100u64;
    let block_entry_threshold = if estimated_avg_entry_size > 0 {
        (block_size / estimated_avg_entry_size).max(1) as u32
    } else {
        100u32
    };

    let mut current_pos = 8u64;
    let dense_index_enabled = filekv_config.aggressive.dense_index_enabled;

    for (key, value) in &all_entries {
        let key_bytes = key.as_bytes();
        let value_bytes: &[u8] = value.as_ref();

        let key_len = key_bytes.len() as u32;
        let value_len = value_bytes.len() as u32;

        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(key_bytes);
        hasher.write(value_bytes);
        let checksum = hasher.finish() as u32;

        writer.write_all(&key_len.to_le_bytes())?;
        writer.write_all(key_bytes)?;
        writer.write_all(&value_len.to_le_bytes())?;
        writer.write_all(value_bytes)?;
        writer.write_all(&checksum.to_le_bytes())?;

        sparse_index.add(key.clone(), current_pos, 0);

        if dense_index_enabled {
            let block_id = dense_index.offset_to_block_id(current_pos);
            dense_index.entries.insert(
                key.clone(),
                sparse_index::DenseIndexEntry {
                    offset: current_pos,
                    key_len: key.len() as u32,
                    value_len,
                    checksum,
                    seq_num: 0,
                    block_id,
                },
            );
        }

        current_block_entry_count += 1;
        match &mut current_block_min_key {
            None => current_block_min_key = Some(key.clone()),
            Some(min_key) => {
                if key.as_str() < min_key.as_str() {
                    *min_key = key.clone();
                }
            }
        }
        match &mut current_block_max_key {
            None => current_block_max_key = Some(key.clone()),
            Some(max_key) => {
                if key.as_str() > max_key.as_str() {
                    *max_key = key.clone();
                }
            }
        }

        current_pos += (4 + key_bytes.len() + 4 + value_bytes.len() + 4) as u64;

        if current_block_entry_count >= block_entry_threshold {
            finalize_block(
                &mut zone_map_entries,
                &mut current_block_min_key,
                &mut current_block_max_key,
                &mut current_block_start,
                &mut current_block_entry_count,
                current_pos,
            );
        }
    }

    finalize_block(
        &mut zone_map_entries,
        &mut current_block_min_key,
        &mut current_block_max_key,
        &mut current_block_start,
        &mut current_block_entry_count,
        current_pos,
    );

    writer.flush()?;
    writer.sync_all()?;

    filekv_config.fs.rename(&temp_path, &new_segment_path)?;
    let _ = filekv_config.fs.sync_dir(&filekv_config.segment_dir);

    let new_segment = SegmentFile::create(
        filekv_config.fs.clone(),
        new_segment_id,
        output_level,
        &new_segment_path,
        0,
        filekv_config.aggressive.persistent_mmap_enabled,
        filekv_config.aggressive.readahead_multiplier,
        dense_index_enabled,
    )?;

    new_segment.populate_key_range_from_dense_index();
    new_segment.flush()?;

    sparse_index.zone_map = Arc::new(zone_map_entries);

    // Step 4: Atomically update segments and indexes
    // V0.6.0: Collect key data for global index update
    let new_keys_for_index: Vec<(String, u64, usize)> = {
        let mut keys = Vec::with_capacity(all_entries.len());
        for (key, value) in &all_entries {
            if let Some(offset) = sparse_index.find(key) {
                keys.push((key.clone(), offset, value.len()));
            }
        }
        keys
    };

    // V0.6.0: Remove old entries from global index BEFORE segment swap
    ctx.remove_old_segments_from_global_index(segments_to_compact);

    // COMP-001: Avoid cloning entire BTreeMap - only clone needed Arc references
    {
        let current_segments = ctx.segments_read();
        // COMP-001: Build new BTreeMap with only necessary changes
        let mut new_segments: BTreeMap<u64, Arc<SegmentFile>> = current_segments
            .iter()
            .filter(|(id, _)| !segments_to_compact.contains(id))
            .map(|(id, seg)| (*id, Arc::clone(seg)))
            .collect();
        drop(current_segments);

        let current_index = ctx.index_manager_read();
        let mut index_manager = (*current_index).clone();
        drop(current_index);

        // Track size delta for old segments being removed
        for &old_id in segments_to_compact {
            if let Some(old_segment) = new_segments.get(&old_id) {
                let old_size = old_segment.size();
                ctx.adjust_total_size_bytes(-(old_size as i64));
            }
        }

        // Add new segment
        new_segments.insert(new_segment_id, Arc::new(new_segment));
        ctx.adjust_total_size_bytes(current_pos as i64);

        index_manager.add_index(new_segment_id, std::sync::Arc::new(sparse_index.clone()));
        if dense_index_enabled {
            let dense_idx_path = filekv_config
                .segment_dir
                .join(format!("segment_{}.dense_idx", new_segment_id));
            match SegmentFile::save_dense_index(filekv_config.fs.as_ref(), &dense_index, &dense_idx_path) {
                Ok(_) => {
                    tracing::debug!(
                        segment_id = new_segment_id,
                        "Saved dense index to {}",
                        dense_idx_path.display()
                    );
                }
                Err(e) => {
                    tracing::warn!(segment_id = new_segment_id, "Failed to save dense index: {}", e);
                }
            }

            index_manager.add_dense_index(new_segment_id, dense_index);
        }
        index_manager.save_index(new_segment_id)?;

        // Remove old segments
        for &old_id in segments_to_compact {
            new_segments.remove(&old_id);
            let old_idx_path = filekv_config.index_dir.join(format!("segment_{}.idx", old_id));
            if old_idx_path.exists() {
                let _ = filekv_config.fs.remove_file(&old_idx_path);
            }
            let old_dense_idx_path = filekv_config.segment_dir.join(format!("segment_{}.dense_idx", old_id));
            if old_dense_idx_path.exists() {
                let _ = filekv_config.fs.remove_file(&old_dense_idx_path);
            }
        }

        // Atomically swap the updated state
        ctx.swap_segments(new_segments);
        ctx.swap_index_manager(index_manager);
    }

    // V0.6.0: Add new entries to global index AFTER segment swap
    ctx.add_new_segments_to_global_index(new_segment_id, &new_keys_for_index);

    // V0.6.0: Clear stale segment tracking
    ctx.clear_stale_segments_after_compaction();

    // BLOOM FILTER
    if filekv_config.enable_bloom {
        if let Err(e) = ctx.rebuild_bloom_for_segment(new_segment_id, &all_entries) {
            tracing::warn!(
                "Failed to rebuild bloom filter for new segment {}: {}",
                new_segment_id,
                e
            );
        }
    }

    // Step 5: Delete old segment files
    for &old_id in segments_to_compact {
        let old_path = filekv_config.segment_dir.join(format!("segment_{}.log", old_id));
        if old_path.exists() {
            match filekv_config.fs.remove_file(&old_path) {
                Ok(_) => {
                    tracing::debug!("Deleted old segment file: {}", old_path.display());
                }
                Err(e) => {
                    tracing::warn!("Failed to delete old segment file {}: {}", old_path.display(), e);
                }
            }
        }

        ctx.invalidate_segment_cache(old_id);
    }

    let stats = CompactionStats {
        compaction_runs: 1,
        segments_merged: segments_to_compact.len() as u64,
        bytes_compacted: total_bytes_before,
        entries_removed,
        tombstones_cleaned: total_tombstones,
        bytes_read_from_segments: total_bytes_before,
        bytes_written_to_segment: current_pos,
    };

    ctx.record_compaction_stats(&stats);

    Ok(stats)
}

/// Helper to finalize a block and create zone map entry
fn finalize_block(
    zone_map_entries: &mut Vec<ZoneMapEntry>,
    current_block_min_key: &mut Option<String>,
    current_block_max_key: &mut Option<String>,
    current_block_start: &mut u64,
    current_block_entry_count: &mut u32,
    current_pos: u64,
) {
    if let (Some(min_key), Some(max_key)) = (current_block_min_key.take(), current_block_max_key.take()) {
        let block_id = (zone_map_entries.len() + 1) as u64;
        zone_map_entries.push(ZoneMapEntry::new(
            block_id,
            min_key,
            max_key,
            *current_block_start,
            (current_pos - *current_block_start) as u32,
            *current_block_entry_count,
        ));
        *current_block_start = current_pos;
        *current_block_entry_count = 0;
    }
}

// ============================================================
// CompactionContext implementation for FileKV
// ============================================================

impl CompactionContext for crate::FileKV {
    fn filekv_config(&self) -> &crate::FileKVConfig {
        &self.config
    }

    fn compaction_config(&self) -> CompactionConfig {
        self.compaction_engine.compaction_manager().config().clone()
    }

    fn segments_read(&self) -> arc_swap::Guard<Arc<BTreeMap<u64, Arc<SegmentFile>>>> {
        self.engine_state.segment_state.segments.load()
    }

    fn swap_segments(&self, new_segments: BTreeMap<u64, Arc<SegmentFile>>) {
        // Also update atomic counters
        let new_count = new_segments.len();
        let new_total_size: u64 = new_segments.values().map(|s| s.size()).sum();
        self.engine_state
            .segment_state
            .segment_count
            .store(new_count, std::sync::atomic::Ordering::Relaxed);
        self.engine_state
            .segment_state
            .total_size_bytes
            .store(new_total_size, std::sync::atomic::Ordering::Relaxed);
        self.engine_state.segment_state.segments.store(Arc::new(new_segments));
    }

    fn index_manager_read(&self) -> parking_lot::RwLockReadGuard<'_, crate::core::sparse_index::IndexManager> {
        self.engine_state.index_state.index_manager.read()
    }

    fn swap_index_manager(&self, new_index_manager: crate::core::sparse_index::IndexManager) {
        let mut idx = self.engine_state.index_state.index_manager.write();
        *idx = new_index_manager;
    }

    fn next_segment_id(&self) -> u64 {
        self.engine_state
            .segment_state
            .next_segment_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    fn adjust_total_size_bytes(&self, delta: i64) {
        if delta >= 0 {
            self.engine_state
                .stats_state
                .stats
                .total_size_bytes
                .fetch_add(delta as u64, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.engine_state
                .stats_state
                .stats
                .total_size_bytes
                .fetch_sub((-delta) as u64, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn invalidate_segment_cache(&self, segment_id: u64) {
        self.block_cache_ref().invalidate_by_segment(segment_id);
    }

    fn rebuild_bloom_for_segment(&self, segment_id: u64, keys: &BTreeMap<String, Vec<u8>>) -> crate::bloom::Result<()> {
        self.rebuild_bloom_filter_for_segment(segment_id, keys)
    }

    fn mark_segments_stale_for_compaction(&self, segment_ids: &[u64]) {
        self.engine_state
            .global_index_state
            .global_index
            .mark_segments_stale(segment_ids);
    }

    fn clear_stale_segments_after_compaction(&self) {
        self.engine_state.global_index_state.global_index.clear_stale_segments();
    }

    fn remove_old_segments_from_global_index(&self, old_segment_ids: &[u64]) {
        self.engine_state
            .global_index_state
            .global_index
            .remove_segments(old_segment_ids);
    }

    fn add_new_segments_to_global_index(&self, new_segment_id: u64, new_keys: &[(String, u64, usize)]) {
        let generation = self.engine_state.global_index_state.global_index.current_generation();
        let new_key_locations: Vec<_> = new_keys
            .iter()
            .map(|(key, offset, value_len)| {
                (
                    Arc::from(key.as_str()),
                    crate::core::global_index::KeyLocation {
                        segment_id: new_segment_id,
                        offset: *offset,
                        generation,
                        value_len: *value_len,
                    },
                )
            })
            .collect();
        self.engine_state
            .global_index_state
            .global_index
            .bulk_insert(new_key_locations);
    }

    fn record_compaction_stats(&self, stats: &CompactionStats) {
        self.compaction_engine.compaction_manager().record_compaction(
            stats.segments_merged,
            stats.bytes_compacted,
            stats.entries_removed,
            stats.tombstones_cleaned,
        );
    }

    #[cfg(feature = "metrics")]
    fn record_prometheus_metrics(&self, stats: &CompactionStats) {
        self.metrics.record_compaction(
            stats.segments_merged,
            stats.bytes_compacted,
            stats.entries_removed,
            stats.tombstones_cleaned,
        );
    }
}
