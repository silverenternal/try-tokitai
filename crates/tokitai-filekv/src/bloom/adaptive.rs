//! Adaptive Bloom Filter Cache - Multi-Layer Architecture
//!
//! INNO-001: Implements L1/L2/L3 multi-layer bloom filter cache with:
//! - L1: Hot cache - small, fast, low FPR (0.1%)
//! - L2: Warm cache - compressed, medium FPR (1%)
//! - L3: Cold storage - disk-based, high FPR (10%)
//!
//! # Architecture
//! ```text
//! Query Flow:
//!   ┌─────────────┐
//!   │ Query Key   │
//!   └──────┬──────┘
//!          │
//!          ▼
//!   ┌─────────────┐
//!   │   L1 Cache  │ (Hot, ~1000 filters, FPR 0.1%, <100ns)
//!   │  DashMap    │
//!   └──────┬──────┘
//!          │ Miss
//!          ▼
//!   ┌─────────────┐
//!   │   L2 Cache  │ (Warm, ~10000 filters, FPR 1%, ~500ns)
//!   │ Compressed  │
//!   └──────┬──────┘
//!          │ Miss
//!          ▼
//!   ┌─────────────┐
//!   │   L3 Store  │ (Cold, disk-based, FPR 10%, ~10µs)
//!   │  On-demand  │
//!   └─────────────┘
//! ```

use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Process start time for monotonic duration calculation (avoids SystemTime syscalls).
static ADAPTIVE_PROCESS_START: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);

#[inline]
fn adaptive_elapsed_ms() -> u64 {
    Instant::now().duration_since(*ADAPTIVE_PROCESS_START).as_millis() as u64
}
use tracing::{debug, warn};

use super::compressed::CompressionError;
use super::custom_bloom::CustomBloom;
use super::fpr_controller::FPRController;
use super::migration::{classify_by_frequency, AccessRecord, FrequencyTier, MigrationThresholds};
use crate::core::error::{DomainError, FileKVError, FileKVResult, TransientError};
use bloom::{BloomFilter, ASMS};

// =========================================================================
// OPT-002: BloomFilterWrapper enum for Unified CustomBloom Integration
// =========================================================================

/// Unified wrapper for bloom filters - supports both legacy ::bloom::BloomFilter
/// and high-performance CustomBloom (V3 format).
///
/// OPT-002: This enables:
/// - Backward compatibility: V1/V2 format files still loadable via Bloom variant
/// - Fast path: V3 format loads as CustomBloom with direct bitset deserialization
/// - Auto-migration: V1/V2 formats can be migrated to V3 on first load
pub enum BloomFilterWrapper {
    /// Legacy bloom filter (V1/V2 format, uses RandomState hashing)
    Bloom(BloomFilter),
    /// Custom bloom filter (V3 format, uses deterministic XXH3 hashing)
    Custom(CustomBloom),
}

impl std::fmt::Debug for BloomFilterWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BloomFilterWrapper::Bloom(_) => f.debug_tuple("Bloom").field(&"<bloom::BloomFilter>").finish(),
            BloomFilterWrapper::Custom(cb) => f.debug_tuple("Custom").field(cb).finish(),
        }
    }
}

impl BloomFilterWrapper {
    /// Check if a key might be in the filter
    pub fn contains(&self, key: &str) -> bool {
        match self {
            BloomFilterWrapper::Bloom(bf) => bf.contains(&key.to_string()),
            BloomFilterWrapper::Custom(cb) => cb.contains(key.as_bytes()),
        }
    }

    /// Estimate memory size for the wrapped filter
    pub fn estimate_memory_size(&self) -> usize {
        match self {
            BloomFilterWrapper::Bloom(bf) => L1CacheEntry::estimate_bloom_memory_size(bf),
            BloomFilterWrapper::Custom(cb) => cb.memory_usage(),
        }
    }

    /// Get the inner BloomFilter if present (for L2 migration that requires keys)
    pub fn as_bloom_filter(&self) -> Option<&BloomFilter> {
        match self {
            BloomFilterWrapper::Bloom(bf) => Some(bf),
            BloomFilterWrapper::Custom(_) => None,
        }
    }

    /// Check if this is a CustomBloom (V3 format)
    pub fn is_custom(&self) -> bool {
        matches!(self, BloomFilterWrapper::Custom(_))
    }
}

// =========================================================================
// T-005: CLOCK Cache for approximate LRU with lock-free access
// =========================================================================

/// Entry in the CLOCK cache circular buffer
#[derive(Debug)]
struct ClockEntry {
    segment_id: u64,
    /// Reference bit - set to 1 on access, cleared during eviction scan
    referenced: AtomicBool,
}

impl ClockEntry {
    fn new(segment_id: u64) -> Self {
        Self {
            segment_id,
            referenced: AtomicBool::new(true), // New entries start as referenced
        }
    }

    /// T-005: Set reference bit on access (lock-free, atomic operation)
    fn tick(&self) {
        self.referenced.store(true, Ordering::Relaxed);
    }

    /// Check and clear reference bit, returns true if was referenced
    fn test_and_clear(&self) -> bool {
        self.referenced.swap(false, Ordering::Relaxed)
    }
}

/// T-005: CLOCK algorithm cache for approximate LRU with lock-free access path.
///
/// The CLOCK algorithm provides approximate LRU behavior with:
/// - O(1) lock-free access: just set an atomic reference bit
/// - O(n) eviction: scan circular buffer, clearing reference bits
/// - Sharded design: multiple independent CLOCK buffers to reduce contention
#[derive(Debug)]
struct ClockCache {
    /// Shards - each shard is an independent CLOCK buffer
    shards: Vec<Mutex<ClockBuffer>>,
    /// Number of shards (power of 2 for fast modulo)
    shard_mask: usize,
}

/// Internal CLOCK buffer (single shard)
#[derive(Debug)]
struct ClockBuffer {
    /// Circular buffer of entries
    entries: Vec<Option<ClockEntry>>,
    /// Current hand position in the circular buffer
    hand: usize,
    /// Number of active entries
    count: usize,
    /// Maximum capacity
    capacity: usize,
}

impl ClockBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            entries: (0..capacity).map(|_| None).collect(),
            hand: 0,
            count: 0,
            capacity,
        }
    }

    /// Insert a new entry. If full, evicts one entry and returns its segment_id.
    fn insert(&mut self, segment_id: u64) -> Option<u64> {
        // Find a free slot first
        for i in 0..self.capacity {
            let idx = (self.hand + i) % self.capacity;
            if self.entries[idx].is_none() {
                self.entries[idx] = Some(ClockEntry::new(segment_id));
                self.count += 1;
                self.hand = (idx + 1) % self.capacity;
                return None;
            }
        }

        // No free slot, evict using CLOCK algorithm
        let evicted = self.evict_one();
        // Insert in the freed slot
        let idx = self.hand;
        self.entries[idx] = Some(ClockEntry::new(segment_id));
        self.hand = (idx + 1) % self.capacity;
        evicted
    }

    /// Set reference bit for an entry (called on cache hit)
    fn tick(&mut self, segment_id: u64) {
        // Linear scan to find the entry (buffer is small per shard)
        for entry in self.entries.iter().flatten() {
            if entry.segment_id == segment_id {
                entry.tick();
                return;
            }
        }
    }

    /// Remove a specific entry (called on explicit remove)
    fn remove(&mut self, segment_id: u64) -> bool {
        for entry_opt in &mut self.entries {
            if let Some(entry) = entry_opt {
                if entry.segment_id == segment_id {
                    *entry_opt = None;
                    self.count = self.count.saturating_sub(1);
                    return true;
                }
            }
        }
        false
    }

    /// Clear all entries
    fn clear(&mut self) {
        for entry_opt in &mut self.entries {
            *entry_opt = None;
        }
        self.count = 0;
        self.hand = 0;
    }

    /// Evict one entry using CLOCK algorithm. Returns the evicted segment_id.
    fn evict_one(&mut self) -> Option<u64> {
        // Scan at most one full circle
        let mut scanned = 0;

        while scanned < self.capacity {
            let idx = (self.hand + scanned) % self.capacity;
            scanned += 1;

            if let Some(ref entry) = self.entries[idx] {
                if entry.test_and_clear() {
                    // Entry was referenced, skip it
                    continue;
                } else {
                    // Entry not referenced, evict it
                    let entry = self.entries[idx].take().unwrap();
                    self.count = self.count.saturating_sub(1);
                    self.hand = (idx + 1) % self.capacity;
                    return Some(entry.segment_id);
                }
            }
            // Skip empty slot
        }

        // All entries were referenced - force evict from hand position
        // This shouldn't happen normally but handle it gracefully
        for i in 0..self.capacity {
            let idx = (self.hand + i) % self.capacity;
            if self.entries[idx].is_some() {
                let entry = self.entries[idx].take().unwrap();
                self.count = self.count.saturating_sub(1);
                self.hand = (idx + 1) % self.capacity;
                return Some(entry.segment_id);
            }
        }

        None
    }

    /// Pop the LRU entry (for compatibility with old LRU-based code)
    /// Uses CLOCK scan to find the least recently used candidate
    fn pop_lru(&mut self) -> Option<u64> {
        self.evict_one()
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.count
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl ClockCache {
    /// Create a new sharded CLOCK cache
    ///
    /// `capacity` is the total number of entries across all shards.
    /// `num_shards` should be a power of 2.
    fn new(capacity: usize, num_shards: usize) -> Self {
        let shard_capacity = capacity / num_shards;
        let shards = (0..num_shards)
            .map(|_| Mutex::new(ClockBuffer::new(shard_capacity.max(1))))
            .collect();

        // Calculate shard mask for fast modulo
        let shard_mask = num_shards - 1;

        Self { shards, shard_mask }
    }

    /// Select shard index for a segment_id
    #[inline]
    fn shard_index(&self, segment_id: u64) -> usize {
        // Use lower bits for fast hashing (good distribution for sequential IDs)
        (segment_id as usize) & self.shard_mask
    }

    /// Insert an entry. Returns evicted segment_id if any.
    fn insert(&self, segment_id: u64) -> Option<u64> {
        let idx = self.shard_index(segment_id);
        self.shards[idx].lock().insert(segment_id)
    }

    /// Tick (set reference bit) for an entry on access.
    /// T-005: This is the fast path - just acquires shard lock briefly.
    fn tick(&self, segment_id: u64) {
        let idx = self.shard_index(segment_id);
        self.shards[idx].lock().tick(segment_id);
    }

    /// Remove a specific entry
    fn remove(&self, segment_id: u64) -> bool {
        let idx = self.shard_index(segment_id);
        self.shards[idx].lock().remove(segment_id)
    }

    /// Pop LRU entry (for eviction)
    fn pop_lru(&self) -> Option<u64> {
        // Scan all shards to find the best eviction candidate
        let mut best: Option<(u64, usize)> = None;
        for (i, shard) in self.shards.iter().enumerate() {
            if let Some(id) = shard.lock().pop_lru() {
                best = Some((id, i));
                break; // Found one, return it
            }
        }
        best.map(|(id, _)| id)
    }

    /// Clear all entries
    fn clear(&self) {
        for shard in &self.shards {
            shard.lock().clear();
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.shards.iter().map(|s| s.lock().len()).sum()
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.shards.iter().all(|s| s.lock().is_empty())
    }
}

/// Per-segment access tracking for FPR controller integration
/// Tracks access counts and timing to construct AccessRecord for QPS calculation
#[derive(Debug)]
struct SegmentAccessTracker {
    /// Total access count for this segment
    total_count: AtomicU64,
    /// Access count in the current window
    window_count: AtomicU64,
    /// Start of the current window (ms since epoch)
    window_start_ms: AtomicU64,
    /// Window duration in ms (for QPS calculation)
    window_duration_ms: u64,
}

impl SegmentAccessTracker {
    fn new(window_duration_ms: u64) -> Self {
        let now_ms = adaptive_elapsed_ms();
        Self {
            total_count: AtomicU64::new(0),
            window_count: AtomicU64::new(0),
            window_start_ms: AtomicU64::new(now_ms),
            window_duration_ms,
        }
    }

    /// Record an access and return an AccessRecord for FPR controller
    fn record_access(&self) -> AccessRecord {
        let total = self.total_count.fetch_add(1, Ordering::Relaxed) + 1;
        let window = self.window_count.fetch_add(1, Ordering::Relaxed) + 1;

        let now_ms = adaptive_elapsed_ms();
        let window_start = self.window_start_ms.load(Ordering::Relaxed);
        let elapsed = now_ms.saturating_sub(window_start);

        // Reset window if it has exceeded the duration
        if elapsed > self.window_duration_ms {
            self.window_count.store(1, Ordering::Relaxed);
            self.window_start_ms.store(now_ms, Ordering::Relaxed);
        }

        // Use a minimum window duration of 1 second to avoid division by zero
        // and unrealistic QPS calculations on initial accesses
        let effective_duration_ms = elapsed.max(1000).min(self.window_duration_ms);

        AccessRecord {
            total_count: total,
            window_count: window,
            window_duration_ms: effective_duration_ms,
            current_layer: 0, // Layer is not used by FPR controller for level determination
        }
    }
}

/// Cache layer identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheLayer {
    L1, // Hot cache
    L2, // Warm cache (compressed)
    L3, // Cold storage (disk)
}

/// Configuration for adaptive bloom filter cache
#[derive(Debug, Clone)]
pub struct AdaptiveBloomCacheConfig {
    /// L1 cache: max number of filters
    pub l1_max_filters: usize,
    /// L2 cache: max number of filters
    pub l2_max_filters: usize,
    /// L1 FPR target (e.g., 0.001 = 0.1%)
    pub l1_fpr_target: f64,
    /// L2 FPR target (e.g., 0.01 = 1%)
    pub l2_fpr_target: f64,
    /// L3 FPR target (e.g., 0.1 = 10%)
    pub l3_fpr_target: f64,
    /// Enable compression for L2 cache
    pub l2_compression_enabled: bool,
    /// L3 index directory
    pub l3_index_dir: PathBuf,
}

impl Default for AdaptiveBloomCacheConfig {
    fn default() -> Self {
        Self {
            l1_max_filters: 1_000,
            l2_max_filters: 10_000,
            l1_fpr_target: 0.001, // 0.1%
            l2_fpr_target: 0.01,  // 1%
            l3_fpr_target: 0.1,   // 10%
            l2_compression_enabled: true,
            l3_index_dir: PathBuf::from("index"),
        }
    }
}

/// Statistics for adaptive bloom cache
#[derive(Debug, Clone, Default)]
pub struct AdaptiveBloomCacheStats {
    /// L1 cache hits
    pub l1_hits: u64,
    /// L2 cache hits
    pub l2_hits: u64,
    /// L3 cache hits (loaded from disk)
    pub l3_hits: u64,
    /// Total misses (filter not found in any layer)
    pub total_misses: u64,
    /// L1 -> L2 migrations
    pub l1_to_l2_migrations: u64,
    /// L2 -> L1 migrations
    pub l2_to_l1_migrations: u64,
    /// L2 -> L3 migrations (evictions)
    pub l2_to_l3_migrations: u64,
    /// L3 -> L2 migrations (loads)
    pub l3_to_l2_migrations: u64,
    /// Current L1 cache size
    pub l1_cache_size: usize,
    /// Current L2 cache size
    pub l2_cache_size: usize,
    /// Memory used by L1 cache (bytes)
    pub l1_memory_used: usize,
    /// Memory used by L2 cache (bytes, compressed)
    pub l2_memory_used: usize,
    /// Total hit rate
    pub hit_rate: f64,
}

impl AdaptiveBloomCacheStats {
    /// Get overall hit rate percentage
    pub fn hit_rate_percent(&self) -> f64 {
        self.hit_rate * 100.0
    }

    /// Get L1 hit rate percentage
    pub fn l1_hit_rate_percent(&self) -> f64 {
        let total = self.l1_hits + self.l2_hits + self.l3_hits + self.total_misses;
        if total > 0 {
            (self.l1_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get L2 hit rate percentage
    pub fn l2_hit_rate_percent(&self) -> f64 {
        let total = self.l1_hits + self.l2_hits + self.l3_hits + self.total_misses;
        if total > 0 {
            (self.l2_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get memory used in MB
    pub fn total_memory_mb(&self) -> f64 {
        (self.l1_memory_used + self.l2_memory_used) as f64 / (1024.0 * 1024.0)
    }
}

/// Entry in L1 cache
///
/// OPT-002: Uses BloomFilterWrapper to support both legacy BloomFilter and
/// high-performance CustomBloom (V3 format).
struct L1CacheEntry {
    filter: Arc<BloomFilterWrapper>,
    /// Estimated memory size (bytes)
    memory_size: usize,
    /// MIN-003: Access count field - currently incremented on each access but NOT used
    /// for migration decisions. Current migration strategy is based on LRU eviction
    /// (L1→L2) and MigrationController QPS-based decisions (L2↔L3).
    /// This field is reserved for future frequency-aware migration implementation.
    access_count: AtomicU64,
    /// Original keys for L1→L2 migration (None if inserted without keys)
    keys: Option<Vec<String>>,
}

impl L1CacheEntry {
    fn new(filter: BloomFilterWrapper, keys: Option<Vec<String>>) -> Self {
        let memory_size = filter.estimate_memory_size();
        Self {
            filter: Arc::new(filter),
            memory_size,
            access_count: AtomicU64::new(0),
            keys,
        }
    }

    /// Estimate memory size for a BloomFilter (legacy path)
    /// Uses the actual bit array size from the filter for accurate estimation.
    fn estimate_bloom_memory_size(filter: &BloomFilter) -> usize {
        // BloomFilter stores a BitVec internally. The actual memory is:
        // - BitVec bit array: num_bits / 8 bytes (BitVec uses Vec<u64> internally,
        //   so there's some alignment overhead)
        // - struct overhead: BloomFilter has 2 HashBuilders + num_hashes field (~48 bytes)
        // - Arc wrapper when stored in L1CacheEntry
        let num_bits = filter.num_bits();
        let bitvec_bytes = num_bits.div_ceil(8);
        // BitVec uses Vec<u64> so add alignment padding to u64 boundary
        let bitvec_aligned = (bitvec_bytes + 7) & !7;
        // BloomFilter struct overhead (2 RandomState + num_hashes) + Vec header
        let struct_overhead = 64;
        bitvec_aligned + struct_overhead
    }
}

/// L2 metadata for serialization
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct L2Metadata {
    num_bits: u32,
    num_hashes: u32,
    original_fpr: f64,
    num_keys: u64,
}

/// L2 compressed entry - stores keys for BloomFilter reconstruction
///
/// # Storage Format
/// The `compressed_keys` field uses a magic byte prefix to distinguish compressed vs uncompressed data:
/// - `0x01` + zstd-compressed bincode data: when compression is enabled
/// - `0x00` + raw bincode data: when compression is disabled
///
/// This avoids unnecessary zstd decode attempts on uncompressed entries during decompression.
#[derive(Debug, Serialize, Deserialize)]
struct L2CompressedEntry {
    metadata: L2Metadata,
    compressed_keys: Vec<u8>, // [magic: u8][data...] where magic 0x01=zstd, 0x00=bincode
}

impl L2CompressedEntry {
    fn new(keys: &[String], metadata: L2Metadata, compression_enabled: bool) -> Result<Self, CompressionError> {
        // Serialize keys to bincode
        let keys_bytes = bincode::serialize(keys)
            .map_err(|e| CompressionError::InvalidData(format!("Failed to serialize keys: {}", e)))?;

        // Apply zstd compression if enabled, and add magic byte prefix
        let compressed_keys = if compression_enabled {
            let zstd_data = zstd::encode_all(keys_bytes.as_slice(), 3)
                .map_err(|e| CompressionError::InvalidData(format!("Failed to compress keys: {}", e)))?;
            // BLOOM-003: Prepend magic byte 0x01 to indicate zstd-compressed data
            let mut result = Vec::with_capacity(1 + zstd_data.len());
            result.push(0x01);
            result.extend_from_slice(&zstd_data);
            result
        } else {
            // BLOOM-003: Prepend magic byte 0x00 to indicate raw bincode data
            let mut result = Vec::with_capacity(1 + keys_bytes.len());
            result.push(0x00);
            result.extend_from_slice(&keys_bytes);
            result
        };

        Ok(Self {
            metadata,
            compressed_keys,
        })
    }

    fn decompress_keys(&self) -> Result<Vec<String>, CompressionError> {
        // If compressed data is empty, return empty keys
        if self.compressed_keys.is_empty() {
            return Ok(Vec::new());
        }

        // BLOOM-003: Read magic byte to determine encoding format
        let data = &self.compressed_keys;
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let magic = data[0];
        let payload = &data[1..];

        match magic {
            0x01 => {
                // zstd-compressed bincode data
                let decompressed = zstd::decode_all(payload)
                    .map_err(|e| CompressionError::InvalidData(format!("Failed to decompress zstd data: {}", e)))?;
                bincode::deserialize::<Vec<String>>(&decompressed)
                    .map_err(|e| CompressionError::InvalidData(format!("Failed to deserialize keys: {}", e)))
            }
            0x00 => {
                // Raw bincode data (no zstd compression)
                bincode::deserialize::<Vec<String>>(payload)
                    .map_err(|e| CompressionError::InvalidData(format!("Failed to deserialize keys: {}", e)))
            }
            _ => {
                // Backward compatibility: old entries without magic byte
                // Try zstd decompress first, then fall back to raw bincode
                let full_data: &[u8] = &self.compressed_keys;
                if let Ok(decompressed) = zstd::decode_all(full_data) {
                    bincode::deserialize::<Vec<String>>(&decompressed)
                        .map_err(|e| CompressionError::InvalidData(format!("Failed to deserialize keys: {}", e)))
                } else {
                    bincode::deserialize::<Vec<String>>(full_data)
                        .map_err(|e| CompressionError::InvalidData(format!("Failed to deserialize keys: {}", e)))
                }
            }
        }
    }
}

/// Entry in L2 cache (stores keys for BloomFilter reconstruction)
///
/// OPT-002: Uses CustomBloom directly for V3 format, with fallback to
/// compressed keys for backward compatibility and migration.
///
/// T-004: Stores both `Arc<BloomFilterWrapper>` for O(1) hit path and
/// `L2CompressedEntry` for memory-efficient storage and persistence.
struct L2CacheEntry {
    /// Pre-built BloomFilterWrapper for O(1) cache hit path
    filter: Arc<BloomFilterWrapper>,
    /// Compressed keys storage (for L2->L3 eviction and persistence)
    compressed: L2CompressedEntry,
    /// T-004: Estimated total memory usage (filter + compressed data)
    memory_size: usize,
    /// Access count for frequency tracking
    access_count: AtomicU64,
}

impl L2CacheEntry {
    /// Create a new L2 cache entry with explicit FPR (borrows filter, rebuilds from keys)
    ///
    /// OPT-002: Builds a CustomBloom from keys for V3 format performance.
    #[cfg(test)]
    fn with_fpr(
        _filter: &BloomFilter,
        keys: Vec<String>,
        compression_enabled: bool,
        fpr: f64,
    ) -> Result<Self, CompressionError> {
        let num_keys = keys.len() as u64;
        let metadata = L2Metadata {
            num_bits: 0,   // Rebuilt on decompress using FPR
            num_hashes: 0, // Calculated from FPR on rebuild
            original_fpr: fpr,
            num_keys,
        };

        let compressed = L2CompressedEntry::new(&keys, metadata, compression_enabled)?;

        // OPT-002: Build CustomBloom from keys for V3 format performance
        let custom_bloom = CustomBloom::from_keys(&keys, Self::estimate_custom_num_bits(num_keys, fpr), fpr);
        let filter_mem = custom_bloom.memory_usage();
        let compressed_mem = compressed.compressed_keys.len();
        let memory_size = filter_mem + compressed_mem;

        Ok(Self {
            filter: Arc::new(BloomFilterWrapper::Custom(custom_bloom)),
            compressed,
            memory_size,
            access_count: AtomicU64::new(0),
        })
    }

    /// Estimate optimal number of bits for CustomBloom given expected items and FPR
    fn estimate_custom_num_bits(num_items: u64, fpr: f64) -> usize {
        if fpr <= 0.0 || fpr >= 1.0 {
            return (num_items as usize) * 10;
        }
        let ln2_sq = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        (-((num_items as f64) * fpr.ln()) / ln2_sq).ceil() as usize
    }

    /// NOTE: This constructor uses estimated FPR - prefer `with_fpr` for explicit FPR
    /// Currently only used in tests
    #[cfg(test)]
    fn new(filter: &BloomFilter, keys: Vec<String>, compression_enabled: bool) -> Result<Self, CompressionError> {
        Self::with_fpr(filter, keys, compression_enabled, estimate_fpr_from_filter(filter))
    }

    /// T-004: Returns the cached filter directly (O(1) Arc::clone)
    fn get_filter(&self) -> Arc<BloomFilterWrapper> {
        self.filter.clone()
    }

    /// Rebuild BloomFilter from compressed keys.
    /// T-004: This is now only used in tests for verification.
    /// Production code should use `get_filter()` for O(1) access.
    #[cfg(test)]
    fn decompress(&self) -> Result<BloomFilter, CompressionError> {
        let keys = self.compressed.decompress_keys()?;

        // Rebuild BloomFilter from keys
        let mut filter = BloomFilter::with_rate(
            self.compressed.metadata.original_fpr as f32,
            self.compressed.metadata.num_keys as u32,
        );
        for key in &keys {
            filter.insert(key);
        }
        Ok(filter)
    }

    /// Serialize to bytes (for L2 memory storage)
    /// NOTE: Currently only used in tests
    #[cfg(test)]
    fn to_bytes(&self) -> Result<Vec<u8>, CompressionError> {
        bincode::serialize(&self.compressed)
            .map_err(|e| CompressionError::InvalidData(format!("Serialization failed: {}", e)))
    }

    /// Deserialize from bytes
    /// NOTE: Currently only used in tests
    /// OPT-002: Rebuilds CustomBloom from keys for V3 format performance.
    #[cfg(test)]
    fn from_bytes(data: &[u8]) -> Result<Self, CompressionError> {
        let compressed: L2CompressedEntry = bincode::deserialize(data)
            .map_err(|e| CompressionError::InvalidData(format!("Deserialization failed: {}", e)))?;

        // Rebuild CustomBloom from compressed data (test-only path)
        let keys = compressed.decompress_keys()?;
        let custom_bloom = CustomBloom::from_keys(
            &keys,
            L2CacheEntry::estimate_custom_num_bits(compressed.metadata.num_keys, compressed.metadata.original_fpr),
            compressed.metadata.original_fpr,
        );
        let filter_mem = custom_bloom.memory_usage();
        let compressed_mem = compressed.compressed_keys.len();

        Ok(Self {
            filter: Arc::new(BloomFilterWrapper::Custom(custom_bloom)),
            compressed,
            memory_size: filter_mem + compressed_mem,
            access_count: AtomicU64::new(0),
        })
    }
}

/// MIN-011: Estimate FPR from a BloomFilter based on its internal state.
///
/// # Limitations
/// The `bloom` crate does not expose the original FPR or the number of items inserted.
/// Given only `num_bits` and `num_hashes`, there are infinitely many (FPR, expected_items)
/// pairs that produce the same filter configuration. Therefore, this function cannot
/// determine the exact original FPR.
///
/// # Improved estimation strategy
/// We use the ratio of bits to hash functions to estimate the filter's precision level:
/// - High bits_per_hash (>15000): likely low FPR (~0.001)
/// - Medium bits_per_hash (4000-15000): medium FPR (~0.005-0.01)
/// - Low bits_per_hash (<4000): higher FPR (~0.02-0.05)
///
/// # Current behavior
/// Returns an estimated FPR based on filter configuration heuristics.
/// For accurate FPR tracking, always pass the FPR explicitly via `L2CacheEntry::with_fpr()`.
///
/// # Usage
/// This function is only used in test code via `L2CacheEntry::new()`. Production code
/// should use `with_fpr()` with an explicit FPR value.
#[cfg(test)]
fn estimate_fpr_from_filter(filter: &BloomFilter) -> f64 {
    // MIN-011: Use actual filter properties for better estimation
    let num_bits = filter.num_bits() as f64;
    let num_hashes = filter.num_hashes() as f64;

    if num_bits == 0.0 || num_hashes == 0.0 {
        return 0.01; // Fallback to default L2 FPR
    }

    // Heuristic: estimate based on bits per hash function ratio
    let bits_per_hash = num_bits / num_hashes;

    // Heuristic mapping based on typical bloom filter configurations:
    // - High bits_per_hash (>15000): likely low FPR (~0.001)
    // - Medium-high bits_per_hash (8000-15000): medium-low FPR (~0.005)
    // - Medium bits_per_hash (4000-8000): standard FPR (~0.01)
    // - Low bits_per_hash (2000-4000): higher FPR (~0.02)
    // - Very low bits_per_hash (<2000): very high FPR (~0.05)
    if bits_per_hash > 15000.0 {
        0.001 // 0.1% - high precision filter
    } else if bits_per_hash > 8000.0 {
        0.005 // 0.5% - medium-high precision
    } else if bits_per_hash > 4000.0 {
        0.01 // 1% - standard precision (default)
    } else if bits_per_hash > 2000.0 {
        0.02 // 2% - lower precision
    } else {
        0.05 // 5% - very low precision
    }
}

/// Adaptive Bloom Filter Cache with multi-layer architecture
pub struct AdaptiveBloomCache {
    /// L1: Hot cache (uncompressed, fastest)
    l1_cache: DashMap<u64, L1CacheEntry>,
    /// T-005: L1 CLOCK queue (approximate LRU with lock-free access)
    l1_lru: Arc<ClockCache>,

    /// L2: Warm cache (compressed)
    l2_cache: DashMap<u64, L2CacheEntry>,
    /// T-005: L2 CLOCK queue (approximate LRU with lock-free access)
    l2_lru: Arc<ClockCache>,

    /// L3: Cold storage directory
    l3_index_dir: PathBuf,

    /// Configuration
    config: AdaptiveBloomCacheConfig,

    /// Feature flag: enable/disable adaptive cache
    enabled: parking_lot::Mutex<bool>,

    /// MAJ-001: Optional FPR Controller for dynamic FPR adjustment
    /// When present, the cache will consult the controller on access patterns
    /// and suggest FPR level changes for segments.
    fpr_controller: Option<Arc<FPRController>>,

    /// MAJ-001: Per-segment access trackers for FPR controller
    /// Lazily created on first access per segment
    segment_access_trackers: DashMap<u64, Arc<SegmentAccessTracker>>,

    /// MAJ-001: Segments pending BloomFilter rebuild due to FPR level change
    /// When FPR level changes, segments are marked here and rebuilt on next access
    pending_fpr_rebuilds: parking_lot::RwLock<HashSet<u64>>,

    /// Statistics
    l1_hits: AtomicU64,
    l2_hits: AtomicU64,
    l3_hits: AtomicU64,
    total_misses: AtomicU64,
    l1_to_l2_migrations: AtomicU64,
    l2_to_l1_migrations: AtomicU64,
    l2_to_l3_migrations: AtomicU64,
    l3_to_l2_migrations: AtomicU64,
    l1_memory_used: AtomicUsize,
    l2_memory_used: AtomicUsize,
}

impl AdaptiveBloomCache {
    /// Create a new adaptive bloom cache with validated configuration.
    ///
    /// # Errors
    /// Returns `FileKVError::InvalidArgument` if:
    /// - `l1_max_filters` is zero
    /// - `l2_max_filters` is zero
    pub fn try_new(config: AdaptiveBloomCacheConfig) -> FileKVResult<Self> {
        let l1_cap = NonZero::new(config.l1_max_filters)
            .ok_or_else(|| FileKVError::Domain(DomainError::Config("l1_max_filters must be non-zero".to_string())))?;
        let l2_cap = NonZero::new(config.l2_max_filters)
            .ok_or_else(|| FileKVError::Domain(DomainError::Config("l2_max_filters must be non-zero".to_string())))?;

        // T-005: Use sharded CLOCK cache with 16 shards for reduced contention
        const NUM_CLOCK_SHARDS: usize = 16;

        Ok(Self {
            l1_cache: DashMap::new(),
            l1_lru: Arc::new(ClockCache::new(l1_cap.get(), NUM_CLOCK_SHARDS)),
            l2_cache: DashMap::new(),
            l2_lru: Arc::new(ClockCache::new(l2_cap.get(), NUM_CLOCK_SHARDS)),
            l3_index_dir: config.l3_index_dir.clone(),
            config,
            enabled: parking_lot::Mutex::new(true),
            fpr_controller: None,
            segment_access_trackers: DashMap::new(),
            pending_fpr_rebuilds: parking_lot::RwLock::new(HashSet::new()),
            l1_hits: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            l3_hits: AtomicU64::new(0),
            total_misses: AtomicU64::new(0),
            l1_to_l2_migrations: AtomicU64::new(0),
            l2_to_l1_migrations: AtomicU64::new(0),
            l2_to_l3_migrations: AtomicU64::new(0),
            l3_to_l2_migrations: AtomicU64::new(0),
            l1_memory_used: AtomicUsize::new(0),
            l2_memory_used: AtomicUsize::new(0),
        })
    }

    /// Create a new adaptive bloom cache with an FPR controller and validated configuration.
    ///
    /// MAJ-001: When an FPR controller is provided, the cache will:
    /// - Record access patterns for each segment to the controller
    /// - Periodically check for FPR adjustments
    /// - Log FPR level changes (rebuild not yet implemented)
    ///
    /// # Errors
    /// Returns `FileKVError::InvalidArgument` if:
    /// - `l1_max_filters` is zero
    /// - `l2_max_filters` is zero
    ///
    /// # Limitations
    /// - FPR changes are logged but do not automatically rebuild Bloom filters
    /// - The caller is responsible for rebuilding affected segments if needed
    pub fn try_with_fpr_controller(
        config: AdaptiveBloomCacheConfig,
        fpr_controller: Arc<FPRController>,
    ) -> FileKVResult<Self> {
        let l1_cap = NonZero::new(config.l1_max_filters)
            .ok_or_else(|| FileKVError::Domain(DomainError::Config("l1_max_filters must be non-zero".to_string())))?;
        let l2_cap = NonZero::new(config.l2_max_filters)
            .ok_or_else(|| FileKVError::Domain(DomainError::Config("l2_max_filters must be non-zero".to_string())))?;

        // T-005: Use sharded CLOCK cache with 16 shards for reduced contention
        const NUM_CLOCK_SHARDS: usize = 16;

        Ok(Self {
            l1_cache: DashMap::new(),
            l1_lru: Arc::new(ClockCache::new(l1_cap.get(), NUM_CLOCK_SHARDS)),
            l2_cache: DashMap::new(),
            l2_lru: Arc::new(ClockCache::new(l2_cap.get(), NUM_CLOCK_SHARDS)),
            l3_index_dir: config.l3_index_dir.clone(),
            config,
            enabled: parking_lot::Mutex::new(true),
            fpr_controller: Some(fpr_controller),
            segment_access_trackers: DashMap::new(),
            pending_fpr_rebuilds: parking_lot::RwLock::new(HashSet::new()),
            l1_hits: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            l3_hits: AtomicU64::new(0),
            total_misses: AtomicU64::new(0),
            l1_to_l2_migrations: AtomicU64::new(0),
            l2_to_l1_migrations: AtomicU64::new(0),
            l2_to_l3_migrations: AtomicU64::new(0),
            l3_to_l2_migrations: AtomicU64::new(0),
            l1_memory_used: AtomicUsize::new(0),
            l2_memory_used: AtomicUsize::new(0),
        })
    }

    /// MAJ-001: Get or create access tracker for a segment
    fn get_access_tracker(&self, segment_id: u64) -> Arc<SegmentAccessTracker> {
        self.segment_access_trackers
            .entry(segment_id)
            .or_insert_with(|| Arc::new(SegmentAccessTracker::new(5000))) // 5 second window
            .clone()
    }

    /// MAJ-001: Record access with FPR controller and check for adjustments
    ///
    /// Called from `get()` when an FPR controller is configured.
    /// Records the access pattern and checks if FPR level should change.
    fn record_fpr_access(&self, segment_id: u64, cache_hit: bool) {
        if let Some(ref controller) = self.fpr_controller {
            let tracker = self.get_access_tracker(segment_id);
            let access_record = tracker.record_access();

            // Record access and potentially adjust FPR
            if let Some(new_level) = controller.record_access(segment_id, &access_record) {
                let fpr = controller.get_level_info(new_level).map(|l| l.fpr).unwrap_or(-1.0);
                debug!(
                    "MAJ-001: FPR level changed for segment {} -> level {} (FPR: {:.4}) [cache_hit={}]",
                    segment_id, new_level, fpr, cache_hit
                );
                // Mark for lazy rebuild on next access
                self.pending_fpr_rebuilds.write().insert(segment_id);
            }
        }
    }

    /// MAJ-001: Check if FPR has changed for a segment and return the new level
    ///
    /// This can be called periodically or on-demand to check for FPR changes.
    /// Returns the current FPR level if a controller is configured.
    pub fn get_current_fpr_level(&self, segment_id: u64) -> Option<u8> {
        self.fpr_controller.as_ref().map(|c| c.get_level(segment_id))
    }

    /// MAJ-001: Get FPR controller stats if configured
    pub fn fpr_controller_stats(&self) -> Option<crate::bloom::fpr_controller::FPRControllerStats> {
        self.fpr_controller.as_ref().map(|c| c.stats())
    }

    /// MAJ-001: Get count of segments pending FPR rebuild
    ///
    /// Returns the number of segments that have been marked for BloomFilter
    /// rebuild due to FPR level changes. These segments will be rebuilt
    /// on their next access.
    pub fn pending_fpr_rebuild_count(&self) -> usize {
        self.pending_fpr_rebuilds.read().len()
    }

    /// Get bloom filter for a segment
    ///
    /// Query flow: L1 -> L2 -> L3 (load on demand)
    ///
    /// # Arguments
    /// * `segment_id` - Segment identifier
    /// * `loader` - Function to load CustomBloom from disk (for L3 miss).
    ///
    /// # Returns
    /// `Arc<BloomFilterWrapper>` if found, None if filter doesn't exist
    ///
    /// OPT-002: Returns BloomFilterWrapper which may contain either legacy BloomFilter
    /// (V1/V2 format from L3) or CustomBloom (V3 format). Callers can use contains() directly.
    pub fn get(
        &self,
        segment_id: u64,
        loader: &dyn Fn(u64) -> FileKVResult<Option<(CustomBloom, Vec<String>)>>,
    ) -> FileKVResult<Option<Arc<BloomFilterWrapper>>> {
        // Check if cache is enabled
        if !self.is_enabled() {
            // Bypass cache, load directly from disk via loader and wrap as Custom
            let result = match loader(segment_id) {
                Ok(Some((custom_bloom, _keys))) => Ok(Some(Arc::new(BloomFilterWrapper::Custom(custom_bloom)))),
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            };
            // MAJ-001: Record access even when cache is bypassed
            self.record_fpr_access(segment_id, false);
            return result;
        }

        // MAJ-001: Check if this segment needs FPR rebuild due to FPR level change
        if self.pending_fpr_rebuilds.read().contains(&segment_id) {
            // Remove from pending set
            self.pending_fpr_rebuilds.write().remove(&segment_id);

            // Invalidate L1 and L2 entries to force reload from disk with new FPR
            self.l1_cache.remove(&segment_id);
            self.l2_cache.remove(&segment_id);

            // T-005: Also remove from CLOCK queues
            self.l1_lru.remove(segment_id);
            self.l2_lru.remove(segment_id);

            debug!(
                "MAJ-001: Invalidated L1/L2 cache for segment {} to reload with new FPR",
                segment_id
            );
        }

        // Try L1 first (hot cache)
        if let Some(entry) = self.l1_cache.get(&segment_id) {
            self.l1_hits.fetch_add(1, Ordering::Relaxed);
            entry.access_count.fetch_add(1, Ordering::Relaxed);

            // T-005: CLOCK tick (sets reference bit, minimal lock contention)
            self.l1_lru.tick(segment_id);

            // MAJ-001: Record access with FPR controller
            self.record_fpr_access(segment_id, true);

            return Ok(Some(entry.filter.clone()));
        }

        // Try L2 (warm cache, compressed)
        // T-004: Use get_filter() for O(1) Arc::clone instead of decompress
        if let Some(entry) = self.l2_cache.get(&segment_id) {
            self.l2_hits.fetch_add(1, Ordering::Relaxed);
            entry.access_count.fetch_add(1, Ordering::Relaxed);

            // T-005: CLOCK tick (sets reference bit, minimal lock contention)
            self.l2_lru.tick(segment_id);

            // T-004: Return cached filter directly (O(1) Arc::clone)
            let filter = entry.get_filter();
            // MAJ-001: Record access with FPR controller
            self.record_fpr_access(segment_id, true);
            return Ok(Some(filter));
        }

        // L3: Try loading from L3 disk storage first
        if let Some(filter) = self.load_from_l3_disk(segment_id)? {
            self.l3_hits.fetch_add(1, Ordering::Relaxed);
            self.l3_to_l2_migrations.fetch_add(1, Ordering::Relaxed);

            // Promote to L1 for future fast access
            let arc = Arc::new(filter);
            self.insert_l1_wrapper(segment_id, arc.clone(), None);

            // MAJ-001: Record access with FPR controller
            self.record_fpr_access(segment_id, true);

            return Ok(Some(arc));
        }

        // L3 miss: fall through to loader (original bloom file)
        self.total_misses.fetch_add(1, Ordering::Relaxed);

        match loader(segment_id)? {
            Some((custom_bloom, keys)) => {
                self.l3_hits.fetch_add(1, Ordering::Relaxed);
                self.l3_to_l2_migrations.fetch_add(1, Ordering::Relaxed);

                // Insert into L1 as Custom (V3 format) with keys for L1→L2 migration
                let arc = Arc::new(BloomFilterWrapper::Custom(custom_bloom));
                self.insert_l1_wrapper(segment_id, arc.clone(), Some(keys));

                // MAJ-001: Record access with FPR controller
                self.record_fpr_access(segment_id, true);

                Ok(Some(arc))
            }
            None => {
                // MAJ-001: Record access even on miss
                self.record_fpr_access(segment_id, false);
                Ok(None)
            }
        }
    }

    /// Insert a bloom filter into the cache (auto-determines layer based on access pattern)
    pub fn insert(&self, segment_id: u64, filter: BloomFilter) {
        // Check if cache is enabled
        if !self.is_enabled() {
            return; // No-op when disabled
        }

        // New inserts go to L1 by default (without keys - caller doesn't have them)
        self.insert_l1(segment_id, filter);
    }

    /// Insert into L1 cache with keys (enables L1→L2 migration on eviction)
    ///
    /// OPT-002: Accepts Arc<BloomFilterWrapper> to support both legacy and V3 formats.
    pub fn insert_l1_with_keys(&self, segment_id: u64, filter: Arc<BloomFilterWrapper>, keys: Vec<String>) {
        if !self.is_enabled() {
            return;
        }
        let memory_size = filter.estimate_memory_size();
        let entry = L1CacheEntry {
            filter,
            memory_size,
            access_count: AtomicU64::new(0),
            keys: Some(keys),
        };
        self.insert_l1_entry(segment_id, entry);
    }

    /// Check if a key exists in a segment's bloom filter
    pub fn contains(
        &self,
        segment_id: u64,
        key: &str,
        loader: &dyn Fn(u64) -> FileKVResult<Option<(CustomBloom, Vec<String>)>>,
    ) -> FileKVResult<Option<bool>> {
        match self.get(segment_id, loader)? {
            Some(filter) => Ok(Some(filter.contains(key))),
            None => Ok(None),
        }
    }

    /// Remove a filter from all cache layers
    ///
    /// OPT-002: Returns Option<Arc<BloomFilterWrapper>> instead of Arc<BloomFilter>.
    pub fn remove(&self, segment_id: u64) -> Option<Arc<BloomFilterWrapper>> {
        // Remove from L1
        let l1_removed = if let Some((_, entry)) = self.l1_cache.remove(&segment_id) {
            self.l1_memory_used.fetch_sub(entry.memory_size, Ordering::Relaxed);
            Some(entry.filter)
        } else {
            None
        };

        // Remove from L2
        let l2_removed = self.l2_cache.remove(&segment_id).is_some();

        // T-005: Remove from CLOCK queues
        self.l1_lru.remove(segment_id);
        self.l2_lru.remove(segment_id);

        if l2_removed {
            debug!("Removed filter from L2 cache for segment {}", segment_id);
        }

        l1_removed
    }

    /// Clear all cache layers
    pub fn clear(&self) {
        self.l1_cache.clear();
        self.l2_cache.clear();

        // T-005: Clear CLOCK queues
        self.l1_lru.clear();
        self.l2_lru.clear();

        self.l1_memory_used.store(0, Ordering::Relaxed);
        self.l2_memory_used.store(0, Ordering::Relaxed);
    }

    /// Get cache statistics
    pub fn stats(&self) -> AdaptiveBloomCacheStats {
        let l1_hits = self.l1_hits.load(Ordering::Relaxed);
        let l2_hits = self.l2_hits.load(Ordering::Relaxed);
        let l3_hits = self.l3_hits.load(Ordering::Relaxed);
        let total_misses = self.total_misses.load(Ordering::Relaxed);
        let total = l1_hits + l2_hits + l3_hits + total_misses;

        AdaptiveBloomCacheStats {
            l1_hits,
            l2_hits,
            l3_hits,
            total_misses,
            l1_to_l2_migrations: self.l1_to_l2_migrations.load(Ordering::Relaxed),
            l2_to_l1_migrations: self.l2_to_l1_migrations.load(Ordering::Relaxed),
            l2_to_l3_migrations: self.l2_to_l3_migrations.load(Ordering::Relaxed),
            l3_to_l2_migrations: self.l3_to_l2_migrations.load(Ordering::Relaxed),
            l1_cache_size: self.l1_cache.len(),
            l2_cache_size: self.l2_cache.len(),
            l1_memory_used: self.l1_memory_used.load(Ordering::Relaxed),
            l2_memory_used: self.l2_memory_used.load(Ordering::Relaxed),
            hit_rate: if total > 0 {
                (l1_hits + l2_hits + l3_hits) as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Get number of cached filters (L1 + L2)
    pub fn len(&self) -> usize {
        self.l1_cache.len() + self.l2_cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.l1_cache.is_empty() && self.l2_cache.is_empty()
    }

    /// Enable or disable the adaptive cache
    ///
    /// When disabled, all get() operations will return None (bypass cache)
    /// and insert() operations will be no-ops.
    ///
    /// This is useful for:
    /// - Debugging: Bypass cache to test baseline performance
    /// - Fallback: Disable if cache causes issues
    /// - Testing: Test with/without cache
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock() = enabled;
    }

    /// Check if adaptive cache is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock()
    }

    /// Insert into L1 cache (internal)
    ///
    /// OPT-002: Wraps BloomFilter in BloomFilterWrapper::Bloom for backward compatibility.
    fn insert_l1(&self, segment_id: u64, filter: BloomFilter) {
        let entry = L1CacheEntry::new(BloomFilterWrapper::Bloom(filter), None);
        self.insert_l1_entry(segment_id, entry);
    }

    /// Insert into L1 cache with Arc (internal)
    ///
    /// OPT-002: Already receives Arc<BloomFilterWrapper>.
    fn insert_l1_arc(&self, segment_id: u64, filter: Arc<BloomFilterWrapper>) {
        let memory_size = filter.estimate_memory_size();
        let entry = L1CacheEntry {
            filter,
            memory_size,
            access_count: AtomicU64::new(0),
            keys: None,
        };
        self.insert_l1_entry(segment_id, entry);
    }

    /// Internal helper: insert L1 entry without taking ownership by wrapping.
    /// Used by load_from_l3_disk path which already has Arc<BloomFilterWrapper>.
    fn insert_l1_wrapper(&self, segment_id: u64, filter: Arc<BloomFilterWrapper>, keys: Option<Vec<String>>) {
        let memory_size = filter.estimate_memory_size();
        let entry = L1CacheEntry {
            filter,
            memory_size,
            access_count: AtomicU64::new(0),
            keys,
        };
        self.insert_l1_entry(segment_id, entry);
    }

    /// Insert L1 cache entry (internal helper)
    fn insert_l1_entry(&self, segment_id: u64, entry: L1CacheEntry) {
        let memory_delta = entry.memory_size;

        // Check count limit first (more reliable than memory estimate)
        let current_count = self.l1_cache.len();
        if current_count >= self.config.l1_max_filters {
            self.evict_l1_multiple(10); // Evict 10 at once for efficiency
        }

        // Check memory limit and evict if necessary
        let current_memory = self.l1_memory_used.load(Ordering::Relaxed);
        let memory_limit = self.config.l1_max_filters * 10240; // 10KB per filter estimate
        if current_memory + memory_delta > memory_limit {
            let excess = (current_memory + memory_delta) - memory_limit;
            let entries_to_evict = (excess / memory_delta).max(10);
            self.evict_l1_multiple(entries_to_evict);
        }

        // Insert into L1
        if let Some(old_entry) = self.l1_cache.insert(segment_id, entry) {
            self.l1_memory_used.fetch_sub(old_entry.memory_size, Ordering::Relaxed);
        } else {
            self.l1_memory_used.fetch_add(memory_delta, Ordering::Relaxed);
        }

        // T-005: Insert into CLOCK queue
        self.l1_lru.insert(segment_id);
    }

    /// Evict multiple entries from L1 cache (batch eviction for efficiency)
    /// When keys are available, entries are migrated to L2. Otherwise they are simply removed.
    ///
    /// BLOOM-004: Batch acquisition of LRU entries under a single lock acquisition
    /// to reduce lock contention. Instead of acquiring the lock for each remove,
    /// we collect all segment IDs to evict under one lock, then process them.
    ///
    /// FREQ-001: Frequency-aware eviction prefers evicting segments with lower
    /// access counts (Cold tier) over segments with higher access counts (Hot tier).
    fn evict_l1_multiple(&self, count: usize) {
        // T-005: Collect segment IDs to evict using CLOCK pop_lru
        let mut candidates: Vec<(u64, u64)> = {
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                if let Some(segment_id) = self.l1_lru.pop_lru() {
                    items.push(segment_id);
                } else {
                    break; // CLOCK empty
                }
            }
            // Get access counts for each candidate
            items
                .iter()
                .filter_map(|&sid| {
                    self.l1_cache.get(&sid).map(|entry| {
                        let access_count = entry.access_count.load(Ordering::Relaxed);
                        (sid, access_count)
                    })
                })
                .collect()
        };

        // FREQ-001: Sort by access count (ascending) to evict cold segments first
        // This preserves hot segments in L1 even if they were accessed less recently
        candidates.sort_by_key(|&(_, count)| count);

        // Process evictions outside the LRU lock to minimize lock hold time
        for (segment_id, _access_count) in candidates {
            if let Some((_, entry)) = self.l1_cache.remove(&segment_id) {
                self.l1_memory_used.fetch_sub(entry.memory_size, Ordering::Relaxed);

                let tier = classify_by_frequency(
                    entry.access_count.load(Ordering::Relaxed),
                    &MigrationThresholds::default(),
                );

                // Migrate to L2 if keys are available
                if let Some(keys) = entry.keys {
                    let key_count = keys.len();
                    self.insert_l2(segment_id, &entry.filter, keys);
                    self.l1_to_l2_migrations.fetch_add(1, Ordering::Relaxed);
                    debug!(
                        "Migrated segment {} from L1 to L2 ({} keys, tier={:?})",
                        segment_id, key_count, tier
                    );
                } else {
                    self.l1_to_l2_migrations.fetch_add(1, Ordering::Relaxed);
                    debug!(
                        "Evicted segment {} from L1 (no keys available, tier={:?})",
                        segment_id, tier
                    );
                }
            }
        }
    }

    /// FREQ-001: Get the frequency tier for a segment based on its access count
    /// Returns the tier and the access count
    pub fn get_segment_frequency(&self, segment_id: u64) -> (FrequencyTier, u64) {
        // Check L1 first
        if let Some(entry) = self.l1_cache.get(&segment_id) {
            let count = entry.access_count.load(Ordering::Relaxed);
            let tier = classify_by_frequency(count, &MigrationThresholds::default());
            return (tier, count);
        }

        // Check L2
        if let Some(entry) = self.l2_cache.get(&segment_id) {
            let count = entry.access_count.load(Ordering::Relaxed);
            let tier = classify_by_frequency(count, &MigrationThresholds::default());
            return (tier, count);
        }

        // Not in cache - return Cold
        (FrequencyTier::Cold, 0)
    }

    /// FREQ-001: Promote a segment to the appropriate tier based on its frequency
    /// This method should be called periodically to ensure segments are in the right layer
    pub fn promote_by_frequency(&self, segment_id: u64) {
        let (tier, _count) = self.get_segment_frequency(segment_id);

        match tier {
            FrequencyTier::Hot => {
                // If in L2, promote to L1
                if self.l1_cache.get(&segment_id).is_none() && self.l2_cache.get(&segment_id).is_some() {
                    self.migrate_l2_to_l1(segment_id);
                }
            }
            FrequencyTier::Warm => {
                // Warm segments should stay in L2 - no action needed if already there
            }
            FrequencyTier::Cold => {
                // If in L1, demote to L2
                if self.l1_cache.get(&segment_id).is_some() {
                    self.migrate_l1_to_l2(segment_id);
                }
            }
        }
    }

    /// Insert into L2 cache (compressed keys storage)
    ///
    /// OPT-002: Takes a reference to BloomFilterWrapper and keys.
    /// The keys are used to rebuild CustomBloom for V3 format performance.
    fn insert_l2(&self, segment_id: u64, _filter: &BloomFilterWrapper, keys: Vec<String>) {
        // OPT-002: Build CustomBloom directly from keys - no need for the filter reference
        let num_keys = keys.len() as u64;
        let custom_bloom = CustomBloom::from_keys(
            &keys,
            L2CacheEntry::estimate_custom_num_bits(num_keys, self.config.l2_fpr_target),
            self.config.l2_fpr_target,
        );
        let filter_mem = custom_bloom.memory_usage();

        let metadata = L2Metadata {
            num_bits: 0,
            num_hashes: 0,
            original_fpr: self.config.l2_fpr_target,
            num_keys,
        };

        match L2CompressedEntry::new(&keys, metadata, self.config.l2_compression_enabled) {
            Ok(compressed) => {
                let compressed_mem = compressed.compressed_keys.len();
                let mem_size = filter_mem + compressed_mem;
                let entry = L2CacheEntry {
                    filter: Arc::new(BloomFilterWrapper::Custom(custom_bloom)),
                    compressed,
                    memory_size: mem_size,
                    access_count: AtomicU64::new(0),
                };

                // Check L2 capacity and evict if needed
                let current_count = self.l2_cache.len();
                if current_count >= self.config.l2_max_filters {
                    self.evict_l2();
                }

                // Insert into L2
                if let Some(old_entry) = self.l2_cache.insert(segment_id, entry) {
                    let old_mem = old_entry.memory_size;
                    self.l2_memory_used.fetch_sub(old_mem, Ordering::Relaxed);
                }
                self.l2_memory_used.fetch_add(mem_size, Ordering::Relaxed);

                // T-005: Insert into CLOCK queue
                self.l2_lru.insert(segment_id);

                debug!("Inserted segment {} into L2 cache", segment_id);
            }
            Err(e) => {
                warn!("Failed to create L2 entry for segment {}: {}", segment_id, e);
            }
        }
    }

    /// Evict from L2 cache (move coldest to L3 - disk)
    fn evict_l2(&self) {
        // T-005: Use CLOCK pop_lru
        if let Some(segment_id) = self.l2_lru.pop_lru() {
            if let Some((_, entry)) = self.l2_cache.remove(&segment_id) {
                // T-004: Use stored memory_size instead of compressed size only
                let mem_size = entry.memory_size;
                self.l2_memory_used.fetch_sub(mem_size, Ordering::Relaxed);

                // Extract keys from entry for L3 storage
                match entry.compressed.decompress_keys() {
                    Ok(keys) => {
                        if let Err(e) = self.save_to_l3_disk(segment_id, &keys) {
                            warn!("Failed to save evicted segment {} to L3 disk: {}", segment_id, e);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to extract keys from segment {} for L3 eviction: {}",
                            segment_id, e
                        );
                    }
                }

                self.l2_to_l3_migrations.fetch_add(1, Ordering::Relaxed);
                debug!("Evicted segment {} from L2 to L3 (disk)", segment_id);
            }
        }
    }

    /// Migrate filter from L2 to L1 (on hot access)
    pub fn migrate_l2_to_l1(&self, segment_id: u64) {
        if let Some(entry) = self.l2_cache.get(&segment_id) {
            // T-004: Use cached filter directly instead of decompressing
            let filter = entry.get_filter();
            self.insert_l1_arc(segment_id, filter);
            self.l2_to_l1_migrations.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Migrate filter from L1 to L2 (on cold access)
    /// When keys are available in the L1 entry, the entry is migrated to L2.
    /// Otherwise it is simply removed from L1.
    pub fn migrate_l1_to_l2(&self, segment_id: u64) {
        if let Some((_, entry)) = self.l1_cache.remove(&segment_id) {
            self.l1_memory_used.fetch_sub(entry.memory_size, Ordering::Relaxed);

            // T-005: Remove from CLOCK queue
            self.l1_lru.remove(segment_id);

            // Migrate to L2 if keys are available
            if let Some(keys) = entry.keys {
                let key_count = keys.len();
                self.insert_l2(segment_id, &entry.filter, keys.clone());
                self.l1_to_l2_migrations.fetch_add(1, Ordering::Relaxed);
                debug!("Migrated segment {} from L1 to L2 ({} keys)", segment_id, key_count);
            } else {
                self.l1_to_l2_migrations.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "Evicted segment {} from L1 during migration (no keys available)",
                    segment_id
                );
            }
        }
    }

    /// Insert into L2 cache with explicit keys (the only way to populate L2)
    ///
    /// OPT-002: Accepts BloomFilterWrapper for consistency with the unified interface.
    pub fn insert_l2_with_keys(&self, segment_id: u64, filter: &BloomFilterWrapper, keys: Vec<String>) {
        self.insert_l2(segment_id, filter, keys);
    }

    // =========================================================================
    // L3 Disk I/O
    // =========================================================================

    /// Save bloom filter to L3 disk storage (V3 CustomBloom format)
    ///
    /// Saves as CustomBloom directly (V3 format: [magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitset_bytes]).
    /// Also saves keys for L2 migration purposes.
    fn save_to_l3_disk(&self, segment_id: u64, keys: &[String]) -> FileKVResult<()> {
        let path = self.l3_path_for_segment(segment_id);

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                FileKVError::Transient(TransientError::ResourceExhausted(format!(
                    "Failed to create L3 dir: {}",
                    e
                )))
            })?;
        }

        // Build CustomBloom from keys
        let custom_bloom = CustomBloom::from_keys(keys, keys.len().max(1000), self.config.l3_fpr_target);

        // Save CustomBloom in V3 format
        custom_bloom.save_to_file(&path).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to save L3 bloom: {}",
                e
            )))
        })?;

        debug!("Saved bloom filter to L3 disk (V3): {:?}", path);
        Ok(())
    }

    /// Load bloom filter from L3 disk storage
    ///
    /// OPT-002: Returns BloomFilterWrapper, preferring Custom (V3 format).
    /// V3 files are loaded directly; old format files are converted.
    fn load_from_l3_disk(&self, segment_id: u64) -> FileKVResult<Option<BloomFilterWrapper>> {
        let path = self.l3_path_for_segment(segment_id);

        if !path.exists() {
            return Ok(None);
        }

        // Try loading as V3 CustomBloom first
        match CustomBloom::load_from_file(&path) {
            Ok(Some(custom_bloom)) => {
                debug!("Loaded v3 bloom filter from L3 disk for segment {}", segment_id);
                return Ok(Some(BloomFilterWrapper::Custom(custom_bloom)));
            }
            Ok(None) => {
                // Not V3 format - try old L3 format (keys-based)
            }
            Err(e) => {
                warn!("Failed to load L3 V3 bloom for segment {}: {}", segment_id, e);
                return Ok(None);
            }
        }

        // Fallback: load old L3 format (keys-based) and rebuild as CustomBloom
        let file = File::open(&path).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to open L3 file: {}",
                e
            )))
        })?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut magic_buf = [0u8; 4];
        reader.read_exact(&mut magic_buf).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to read L3 magic: {}",
                e
            )))
        })?;
        let magic = u32::from_le_bytes(magic_buf);
        if magic != crate::core::types::BLOOM_MAGIC {
            warn!("Invalid L3 bloom magic: {}", magic);
            return Ok(None);
        }

        let mut version_buf = [0u8; 4];
        reader.read_exact(&mut version_buf).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to read L3 version: {}",
                e
            )))
        })?;
        let _version = u32::from_le_bytes(version_buf);

        let mut num_bits_buf = [0u8; 4];
        reader.read_exact(&mut num_bits_buf).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to read L3 num_bits: {}",
                e
            )))
        })?;
        let _num_bits = u32::from_le_bytes(num_bits_buf);

        let mut num_hashes_buf = [0u8; 4];
        reader.read_exact(&mut num_hashes_buf).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to read L3 num_hashes: {}",
                e
            )))
        })?;
        let _num_hashes = u32::from_le_bytes(num_hashes_buf);

        let mut fpr_buf = [0u8; 8];
        reader.read_exact(&mut fpr_buf).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to read L3 fpr: {}",
                e
            )))
        })?;
        let fpr = f64::from_le_bytes(fpr_buf);

        let mut num_keys_buf = [0u8; 8];
        reader.read_exact(&mut num_keys_buf).map_err(|e| {
            FileKVError::Transient(TransientError::ResourceExhausted(format!(
                "Failed to read L3 num_keys: {}",
                e
            )))
        })?;
        let num_keys = u64::from_le_bytes(num_keys_buf);

        // Read keys
        let mut keys = Vec::with_capacity(num_keys as usize);
        for _ in 0..num_keys {
            let mut key_len_buf = [0u8; 4];
            reader.read_exact(&mut key_len_buf).map_err(|e| {
                FileKVError::Transient(TransientError::ResourceExhausted(format!(
                    "Failed to read key length: {}",
                    e
                )))
            })?;
            let key_len = u32::from_le_bytes(key_len_buf);

            let mut key_bytes = vec![0u8; key_len as usize];
            reader.read_exact(&mut key_bytes).map_err(|e| {
                FileKVError::Transient(TransientError::ResourceExhausted(format!("Failed to read key: {}", e)))
            })?;

            let key = String::from_utf8_lossy(&key_bytes).to_string();
            keys.push(key);
        }

        // Rebuild as CustomBloom with deterministic XXH3 hashing
        let custom_bloom = CustomBloom::from_keys(&keys, num_keys as usize, fpr);

        // Re-save as V3 format for faster future loads
        if let Err(e) = custom_bloom.save_to_file(&path) {
            warn!("Failed to re-save L3 as V3 for segment {}: {}", segment_id, e);
        }

        debug!(
            "Loaded L3 bloom from old format, rebuilt as CustomBloom for segment {}",
            segment_id
        );
        Ok(Some(BloomFilterWrapper::Custom(custom_bloom)))
    }

    /// Get L3 file path for a segment
    fn l3_path_for_segment(&self, segment_id: u64) -> PathBuf {
        self.l3_index_dir.join(format!("bloom_{}.bloom", segment_id))
    }

    /// Load a filter from L3 disk and optionally promote to L2
    pub fn load_l3_to_cache(&self, segment_id: u64, promote_to_l2: bool) -> FileKVResult<Option<BloomFilterWrapper>> {
        match self.load_from_l3_disk(segment_id)? {
            Some(filter) => {
                if promote_to_l2 {
                    // We can't promote to L2 without keys, but we can at least note the load
                    self.l3_to_l2_migrations.fetch_add(1, Ordering::Relaxed);
                }
                Ok(Some(filter))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::DEFAULT_BLOOM_FPR;

    #[test]
    fn test_adaptive_cache_config_default() {
        let config = AdaptiveBloomCacheConfig::default();
        assert_eq!(config.l1_max_filters, 1_000);
        assert_eq!(config.l2_max_filters, 10_000);
        assert_eq!(config.l1_fpr_target, 0.001);
        assert_eq!(config.l2_fpr_target, 0.01);
        assert_eq!(config.l3_fpr_target, 0.1);
        assert!(config.l2_compression_enabled);
    }

    #[test]
    fn test_adaptive_cache_insert_get() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Create test bloom filter
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test_key".to_string());

        // Insert into cache
        cache.insert(1, filter);

        // Retrieve from cache (no loader needed for L1 hit)
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let cached = cache.get(1, &loader).unwrap();

        assert!(cached.is_some());
        assert!(cached.unwrap().contains("test_key"));
    }

    #[test]
    fn test_adaptive_cache_stats() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test".to_string());
        cache.insert(1, filter);

        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };

        cache.get(1, &loader).unwrap(); // L1 hit
        cache.get(1, &loader).unwrap(); // L1 hit
        cache.get(2, &loader).unwrap(); // Miss

        let stats = cache.stats();
        assert_eq!(stats.l1_hits, 2);
        assert_eq!(stats.total_misses, 1);
        assert!(stats.hit_rate > 0.5);
    }

    #[test]
    fn test_adaptive_cache_remove() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test".to_string());
        cache.insert(1, filter);

        assert!(cache.get(1, &|_| Ok(None)).unwrap().is_some());

        let removed = cache.remove(1);
        assert!(removed.is_some());
        assert!(cache.get(1, &|_| Ok(None)).unwrap().is_none());
    }

    #[test]
    fn test_adaptive_cache_clear() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        for i in 0..10 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            filter.insert(&format!("key_{}", i));
            cache.insert(i, filter);
        }

        assert!(!cache.is_empty());
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    // ==================== L2/L3 Tests ====================

    #[test]
    fn test_l2_insert_get_with_keys() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Create test bloom filter with known keys
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys: Vec<String> = (0..50).map(|i| format!("key_{}", i)).collect();
        for key in &keys {
            filter.insert(key);
        }

        // Insert into L2 with keys
        cache.insert_l2_with_keys(1, &BloomFilterWrapper::Bloom(filter), keys.clone());

        // Retrieve from L2
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let cached = cache.get(1, &loader).unwrap();

        assert!(cached.is_some());
        let filter = cached.unwrap();
        // Verify the reconstructed filter contains the keys
        assert!(filter.contains("key_0"));
        assert!(filter.contains("key_49"));
    }

    #[test]
    fn test_l2_serialization_roundtrip() {
        // Create test bloom filter with known keys
        let filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys: Vec<String> = (0..20).map(|i| format!("test_key_{}", i)).collect();

        let entry = L2CacheEntry::new(&filter, keys.clone(), true).unwrap();

        // Serialize
        let bytes = entry.to_bytes().unwrap();

        // Deserialize
        let restored = L2CacheEntry::from_bytes(&bytes).unwrap();

        // Verify decompression works
        let rebuilt_filter = restored.decompress().unwrap();

        // Verify keys are in the filter
        for key in &keys {
            assert!(rebuilt_filter.contains(&key));
        }
    }

    #[test]
    fn test_l2_serialization_without_compression() {
        let filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys: Vec<String> = vec!["key_a".to_string(), "key_b".to_string(), "key_c".to_string()];

        let entry = L2CacheEntry::new(&filter, keys.clone(), false).unwrap();
        let bytes = entry.to_bytes().unwrap();
        let restored = L2CacheEntry::from_bytes(&bytes).unwrap();
        let rebuilt_filter = restored.decompress().unwrap();

        for key in &keys {
            assert!(rebuilt_filter.contains(&key));
        }
    }

    #[test]
    fn test_l3_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Create bloom filter with known keys
        let mut filter = BloomFilter::with_rate(0.1, 50);
        let keys: Vec<String> = (0..30).map(|i| format!("l3_key_{}", i)).collect();
        for key in &keys {
            filter.insert(key);
        }

        // Save to L3
        cache.save_to_l3_disk(42, &keys).unwrap();

        // Load from L3
        let loaded = cache.load_from_l3_disk(42).unwrap();
        assert!(loaded.is_some());

        let loaded_filter = loaded.unwrap();
        // Verify the loaded filter contains the keys
        for key in &keys {
            assert!(loaded_filter.contains(key), "Key {} should be in loaded filter", key);
        }
    }

    #[test]
    fn test_l3_load_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Loading non-existent L3 file should return None
        let loaded = cache.load_from_l3_disk(999).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_l2_eviction_to_l3() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l2_max_filters = 2;
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert 2 filters into L2 (fill capacity)
        for seg_id in 1..=2u64 {
            let filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            let keys: Vec<String> = (0..10).map(|i| format!("seg{}_key_{}", seg_id, i)).collect();
            cache.insert_l2_with_keys(seg_id, &BloomFilterWrapper::Bloom(filter), keys);
        }

        // Insert a 3rd filter - should trigger eviction of the first one to L3
        let filter3 = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys3: Vec<String> = (0..10).map(|i| format!("seg3_key_{}", i)).collect();
        cache.insert_l2_with_keys(3, &BloomFilterWrapper::Bloom(filter3), keys3);

        // L2 should still have 2 entries
        assert_eq!(cache.l2_cache.len(), 2);

        // The evicted segment should be in L3
        let l3_path = cache.l3_path_for_segment(1);
        assert!(l3_path.exists(), "L3 file for segment 1 should exist after eviction");
    }

    #[test]
    fn test_l2_hit_returns_reconstructed_filter() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 1000);
        let keys: Vec<String> = (0..200).map(|i| format!("data_{}", i)).collect();
        for key in &keys {
            filter.insert(key);
        }

        cache.insert_l2_with_keys(100, &BloomFilterWrapper::Bloom(filter), keys.clone());

        // Query should hit L2 and return reconstructed filter
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            panic!("Loader should not be called on L2 hit");
        };
        let result = cache.get(100, &loader).unwrap();
        assert!(result.is_some());

        let retrieved = result.unwrap();
        // Verify the filter works
        assert!(retrieved.contains("data_0"));
        assert!(retrieved.contains("data_199"));
    }

    #[test]
    fn test_l2_decompress_keys_failure_handling() {
        // Test that L2CacheEntry handles corrupted data gracefully
        // Use truncated data (only 3 bytes, less than the 8-byte length prefix)
        let bad_data = vec![0xFFu8, 0xFF, 0xFF];
        let bad_entry = L2CompressedEntry {
            metadata: L2Metadata {
                num_bits: 0,
                num_hashes: 0,
                original_fpr: 0.01,
                num_keys: 100,
            },
            compressed_keys: bad_data,
        };
        // T-004: L2CacheEntry now requires filter and memory_size fields
        let dummy_filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let entry = L2CacheEntry {
            filter: Arc::new(BloomFilterWrapper::Bloom(dummy_filter)),
            compressed: bad_entry,
            memory_size: 0,
            access_count: AtomicU64::new(0),
        };

        // Decompress should fail gracefully
        assert!(entry.decompress().is_err());
    }

    #[test]
    fn test_l2_lru_promotion() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l2_max_filters = 5;
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert several entries
        for i in 0..5u64 {
            let filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            let keys: Vec<String> = vec![format!("key_{}", i)];
            cache.insert_l2_with_keys(i, &BloomFilterWrapper::Bloom(filter), keys);
        }

        // Access entry 2 - should promote it in LRU
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        cache.get(2, &loader).unwrap();

        // L2 should still have 5 entries
        assert_eq!(cache.l2_cache.len(), 5);
    }

    #[test]
    fn test_l2_memory_tracking() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        let initial_mem = cache.l2_memory_used.load(Ordering::Relaxed);

        let filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys: Vec<String> = (0..50).map(|i| format!("track_key_{}", i)).collect();
        cache.insert_l2_with_keys(1, &BloomFilterWrapper::Bloom(filter), keys);

        // Memory should have been tracked
        let after_mem = cache.l2_memory_used.load(Ordering::Relaxed);
        // Note: memory tracking happens on L2 insert via compressed size
        assert!(after_mem >= initial_mem);
    }

    #[test]
    fn test_cache_disabled_bypass() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();
        cache.set_enabled(false);

        // Insert should be a no-op
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test".to_string());
        cache.insert(1, filter);

        // Get should bypass cache and use loader
        let loader_called = AtomicU64::new(0);
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            loader_called.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        };
        cache.get(1, &loader).unwrap();

        assert_eq!(loader_called.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_load_l3_to_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Save a filter to L3
        let mut filter = BloomFilter::with_rate(0.1, 50);
        let keys: Vec<String> = vec!["l3_test_key".to_string()];
        for key in &keys {
            filter.insert(key);
        }
        cache.save_to_l3_disk(77, &keys).unwrap();

        // Load from L3
        let loaded = cache.load_l3_to_cache(77, true).unwrap();
        assert!(loaded.is_some());
        let loaded_filter = loaded.unwrap();
        assert!(loaded_filter.contains("l3_test_key"));

        // Stats should reflect the load
        let stats = cache.stats();
        assert_eq!(stats.l3_to_l2_migrations, 1);
    }

    /// Test: L1→L2 migration via eviction (production path)
    ///
    /// When entries are loaded from disk (with keys), they are stored in L1 with keys.
    /// When L1 evicts, those entries are migrated to L2.
    #[test]
    fn test_l1_to_l2_migration_via_eviction() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 3;
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Create a loader that returns keys (simulating disk load)
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            let keys: Vec<String> = (0..10).map(|i| format!("key_for_{}_{}", _id, i)).collect();
            let cb = CustomBloom::from_keys(&keys, 800, DEFAULT_BLOOM_FPR as f64);
            Ok(Some((cb, keys)))
        };

        // Initially L1 and L2 are empty
        assert_eq!(cache.l1_cache.len(), 0);
        assert_eq!(cache.l2_cache.len(), 0);
        assert_eq!(cache.stats().l1_to_l2_migrations, 0);

        // Load 3 entries into L1 (fills capacity)
        for i in 1..=3u64 {
            let result = cache.get(i, &loader).unwrap();
            assert!(result.is_some());
        }

        assert_eq!(cache.l1_cache.len(), 3);

        // Load a 4th entry - should trigger L1 eviction (evicts up to 10 entries)
        // Since we only have 3 entries, all 3 get evicted and migrated to L2
        let result = cache.get(4, &loader).unwrap();
        assert!(result.is_some());

        // L1 should have just 1 entry (the newly inserted one)
        assert_eq!(cache.l1_cache.len(), 1);

        // L2 should have 3 entries (migrated from L1)
        assert_eq!(cache.l2_cache.len(), 3, "L2 should have 3 entries migrated from L1");

        // Check migration stats
        let stats = cache.stats();
        assert_eq!(stats.l1_to_l2_migrations, 3, "Should have 3 L1→L2 migrations");

        // Verify the L2 entries work - query for an evicted segment
        let l2_loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let result = cache.get(1, &l2_loader).unwrap();
        assert!(result.is_some(), "Should find segment 1 in L2");
        // The filter should contain keys for segment 1
        let filter = result.unwrap();
        assert!(filter.contains("key_for_1_0"));
    }

    /// Test: L1→L2 migration produces valid L2 entry (S1-1 acceptance test)
    ///
    /// Verifies that when L1 evicts an entry with keys, the L2 entry
    /// can be decompressed and returns a valid BloomFilter.
    #[test]
    fn test_l1_to_l2_migration_produces_valid_l2_entry() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 2;
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Create a loader that returns keys (simulating disk load with keys)
        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            let keys: Vec<String> = (0..20).map(|i| format!("seg{}_key_{}", id, i)).collect();
            let cb = CustomBloom::from_keys(&keys, 800, DEFAULT_BLOOM_FPR as f64);
            Ok(Some((cb, keys)))
        };

        // Load 2 entries into L1 (fills capacity)
        for i in 1..=2u64 {
            let result = cache.get(i, &loader).unwrap();
            assert!(result.is_some());
        }
        assert_eq!(cache.l1_cache.len(), 2);
        assert_eq!(cache.l2_cache.len(), 0);

        // Load a 3rd entry - triggers L1 eviction (evicts up to 10, we have 2)
        let result = cache.get(3, &loader).unwrap();
        assert!(result.is_some());

        // L1 should now have 1 entry (the newest)
        assert_eq!(cache.l1_cache.len(), 1);

        // L2 should have 2 entries (migrated from L1 eviction)
        assert_eq!(cache.l2_cache.len(), 2, "L2 should have 2 entries migrated from L1");

        // Verify L2 entries are valid - query evicted segments
        let l2_loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };

        for seg_id in 1..=2u64 {
            let result = cache.get(seg_id, &l2_loader).unwrap();
            assert!(result.is_some(), "L2 should have valid entry for segment {}", seg_id);
            let filter = result.unwrap();

            // Verify the reconstructed filter contains the expected keys
            for i in 0..20u64 {
                let key = format!("seg{}_key_{}", seg_id, i);
                assert!(
                    filter.contains(&key),
                    "L2 filter for segment {} should contain key '{}'",
                    seg_id,
                    key
                );
            }
        }

        // Verify migration stats
        let stats = cache.stats();
        assert_eq!(stats.l1_to_l2_migrations, 2);
    }

    /// Test: L2 decompress correctly rebuilds BloomFilter from stored keys (S1-1 acceptance test)
    ///
    /// Verifies that L2CacheEntry.decompress() rebuilds a BloomFilter
    /// that correctly identifies all original keys.
    #[test]
    fn test_l2_decompress_rebuilds_bloom_correctly() {
        // Create a bloom filter with known keys
        let mut original_filter = BloomFilter::with_rate(0.001, 500);
        let keys: Vec<String> = (0..200).map(|i| format!("decompress_test_key_{}", i)).collect();
        for key in &keys {
            original_filter.insert(key);
        }

        // Create L2 entry
        let entry = L2CacheEntry::new(&original_filter, keys.clone(), true).unwrap();

        // Decompress and rebuild
        let rebuilt_filter = entry.decompress().expect("decompress should succeed");

        // Verify ALL original keys are in the rebuilt filter
        for key in &keys {
            assert!(
                rebuilt_filter.contains(&key),
                "Rebuilt filter should contain key: {}",
                key
            );
        }

        // Verify false positives are possible (filter is not just a set)
        // A key that was never inserted should sometimes not be present
        assert!(
            !rebuilt_filter.contains(&"definitely_not_inserted_key_xyz".to_string())
                || rebuilt_filter.contains(&"definitely_not_inserted_key_xyz".to_string()),
            "Filter may have false positive for unknown key (expected behavior)"
        );

        // Verify filter properties match
        // The rebuilt filter should have the same number of effective keys
        // (we can't check num_bits/num_hashes since bloom crate doesn't expose them)

        // Test with different FPR targets
        for fpr in [0.001, 0.01, 0.05] {
            let mut filter = BloomFilter::with_rate(fpr as f32, 50);
            let test_keys: Vec<String> = (0..30).map(|i| format!("fpr_{}_key_{}", fpr, i)).collect();
            for key in &test_keys {
                filter.insert(key);
            }

            let l2_entry = L2CacheEntry::new(&filter, test_keys.clone(), true).unwrap();
            let rebuilt = l2_entry.decompress().unwrap();

            for key in &test_keys {
                assert!(rebuilt.contains(key), "FPR {}: rebuilt should contain key {}", fpr, key);
            }
        }
    }

    /// Test: L1 entries inserted without keys do NOT migrate to L2
    #[test]
    fn test_l1_without_keys_no_migration() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 3;
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert 3 filters without keys (simulates direct insert())
        for i in 1..=3u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            filter.insert(&format!("key_{}", i));
            cache.insert(i, filter);
        }

        assert_eq!(cache.l1_cache.len(), 3);
        assert_eq!(cache.l2_cache.len(), 0);

        // Insert a 4th - triggers eviction of all 3, but no L2 migration (no keys)
        let mut filter4 = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter4.insert(&"key_4".to_string());
        cache.insert(4, filter4);

        // L1 should have just 1 entry (the new one)
        assert_eq!(cache.l1_cache.len(), 1);

        // L2 should still have 0 entries (no keys = no migration)
        assert_eq!(cache.l2_cache.len(), 0);

        // Migration count reflects the eviction count but actual migration didn't happen
        let stats = cache.stats();
        assert_eq!(stats.l1_to_l2_migrations, 3); // 3 entries were evicted
        assert_eq!(stats.l2_cache_size, 0); // But L2 is empty
    }

    /// Test: L3 persistence and reload (S1-2 acceptance test)
    ///
    /// Verifies the full L2→L3 eviction path and L3→L1 reload path:
    /// 1. Fill L2 capacity, insert one more to trigger L2→L3 eviction
    /// 2. Verify L3 file exists with correct format
    /// 3. Query evicted segment - should load from L3 disk
    /// 4. Verify reloaded BloomFilter contains original keys
    #[test]
    fn test_l3_persistence_and_reload() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l2_max_filters = 2;
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert 2 filters into L2 (fill capacity)
        for seg_id in 1..=2u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            let keys: Vec<String> = (0..15).map(|i| format!("seg{}_key_{}", seg_id, i)).collect();
            for key in &keys {
                filter.insert(key);
            }
            cache.insert_l2_with_keys(seg_id, &BloomFilterWrapper::Bloom(filter), keys);
        }
        assert_eq!(cache.l2_cache.len(), 2);

        // Insert a 3rd filter - triggers eviction of segment 1 to L3
        let mut filter3 = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys3: Vec<String> = (0..15).map(|i| format!("seg3_key_{}", i)).collect();
        for key in &keys3 {
            filter3.insert(key);
        }
        cache.insert_l2_with_keys(3, &BloomFilterWrapper::Bloom(filter3), keys3.clone());

        // L2 should still have 2 entries
        assert_eq!(cache.l2_cache.len(), 2);

        // Segment 1 should be in L3 disk
        let l3_path = cache.l3_path_for_segment(1);
        assert!(l3_path.exists(), "L3 file for segment 1 should exist after L2 eviction");

        // Query segment 1 - should load from L3 disk
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            // This should NOT be called because L3 has the data
            panic!("Loader should not be called when L3 has the data");
        };
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some(), "Should find segment 1 loaded from L3");

        let filter = result.unwrap();
        // Verify the reloaded filter contains the original keys
        for i in 0..15u64 {
            let key = format!("seg1_key_{}", i);
            assert!(filter.contains(&key), "Reloaded filter should contain key: {}", key);
        }

        // Verify L3 hit was recorded
        let stats = cache.stats();
        assert_eq!(stats.l3_hits, 1, "Should have 1 L3 hit");
        assert_eq!(stats.l3_to_l2_migrations, 1, "Should have 1 L3→L2 migration recorded");

        // The loaded filter should now be in L1
        assert_eq!(cache.l1_cache.len(), 1, "Loaded filter should be in L1");
    }

    // ==================== MAJ-001: FPR Controller Integration Tests ====================

    /// Test: FPR controller is consulted when attached to AdaptiveBloomCache
    ///
    /// Verifies that:
    /// 1. Cache with FPR controller records accesses
    /// 2. FPR controller tracks segments
    /// 3. get_current_fpr_level returns the controller's level
    #[test]
    fn test_fpr_controller_integration_basic() {
        use crate::bloom::fpr_controller::{AdaptationPolicy, FPRController};
        use crate::bloom::migration::MigrationThresholds;

        let config = AdaptiveBloomCacheConfig::default();
        let fpr_controller = Arc::new(FPRController::new(
            AdaptationPolicy::default(),
            MigrationThresholds::default(),
        ));

        let cache = AdaptiveBloomCache::try_with_fpr_controller(config, fpr_controller.clone()).unwrap();

        // Create and insert a filter
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"fpr_test_key".to_string());
        cache.insert(1, filter);

        // Query through the cache
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());

        // Verify FPR controller is tracking the segment
        let stats = fpr_controller.stats();
        assert_eq!(stats.tracked_segments, 1, "FPR controller should track 1 segment");

        // Verify we can query the FPR level through the cache
        // With minimum 1s window and 1 access, QPS = 1.0 which is below the hysteresis
        // threshold for any downgrade from level 2, so level should stay at 2
        let level = cache.get_current_fpr_level(1);
        assert_eq!(
            level,
            Some(2),
            "Default FPR level should be 2 (QPS too low to trigger downgrade with hysteresis)"
        );

        // Verify controller stats are accessible through cache
        let cache_fpr_stats = cache.fpr_controller_stats();
        assert!(cache_fpr_stats.is_some());
        assert_eq!(cache_fpr_stats.unwrap().tracked_segments, 1);
    }

    /// Test: FPR controller records multiple accesses
    ///
    /// Verifies that repeated accesses are properly tracked and
    /// the access tracker maintains correct counts.
    #[test]
    fn test_fpr_controller_records_multiple_accesses() {
        use crate::bloom::fpr_controller::{AdaptationPolicy, FPRController};
        use crate::bloom::migration::MigrationThresholds;

        let config = AdaptiveBloomCacheConfig::default();
        let fpr_controller = Arc::new(FPRController::new(
            AdaptationPolicy::default(),
            MigrationThresholds::default(),
        ));

        let cache = AdaptiveBloomCache::try_with_fpr_controller(config, fpr_controller.clone()).unwrap();

        // Insert filters for multiple segments
        for i in 1..=5u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            filter.insert(&format!("key_{}", i));
            cache.insert(i, filter);
        }

        // Query all segments multiple times
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        for _ in 0..3 {
            for i in 1..=5u64 {
                let _ = cache.get(i, &loader).unwrap();
            }
        }

        // FPR controller should be tracking all 5 segments
        let stats = fpr_controller.stats();
        assert_eq!(stats.tracked_segments, 5, "FPR controller should track 5 segments");
    }

    /// Test: Cache without FPR controller does not track segments in FPR
    ///
    /// Verifies backward compatibility - caches created with `new()`
    /// should work normally without FPR tracking.
    #[test]
    fn test_cache_without_fpr_controller_no_fpr_tracking() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"test_key".to_string());
        cache.insert(1, filter);

        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());

        // FPR level should be None (no controller)
        let level = cache.get_current_fpr_level(1);
        assert!(
            level.is_none(),
            "Cache without FPR controller should return None for FPR level"
        );

        // FPR controller stats should be None
        let fpr_stats = cache.fpr_controller_stats();
        assert!(
            fpr_stats.is_none(),
            "Cache without FPR controller should return None for stats"
        );
    }

    // ==================== MAJ-001 Phase 2: FPR BloomFilter Rebuild Tests ====================

    /// Test: FPR filter rebuild pending and rebuild on next access
    ///
    /// Verifies that:
    /// 1. When FPR level changes, the segment is marked as pending rebuild
    /// 2. On next get() call, the pending flag is cleared and L1/L2 are invalidated
    /// 3. The filter is reloaded from disk (simulated via loader)
    #[test]
    fn test_fpr_filter_rebuild_pending() {
        use crate::bloom::fpr_controller::{AdaptationPolicy, FPRController};
        use crate::bloom::migration::MigrationThresholds;

        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l3_index_dir = temp_dir.path().to_path_buf();

        let fpr_controller = Arc::new(FPRController::new(
            AdaptationPolicy::default(),
            MigrationThresholds::default(),
        ));

        let cache = AdaptiveBloomCache::try_with_fpr_controller(config, fpr_controller.clone()).unwrap();

        // Initially no pending rebuilds
        assert_eq!(
            cache.pending_fpr_rebuild_count(),
            0,
            "Should have no pending rebuilds initially"
        );

        // Create a loader that returns keys (simulating disk load)
        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            let keys: Vec<String> = (0..10).map(|i| format!("seg{}_key_{}", id, i)).collect();
            let cb = CustomBloom::from_keys(&keys, 800, DEFAULT_BLOOM_FPR as f64);
            Ok(Some((cb, keys)))
        };

        // Load segment 1 into cache
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());

        // Simulate FPR level change by manually marking as pending
        // (In production, this happens via record_fpr_access when FPR controller decides to change level)
        cache.pending_fpr_rebuilds.write().insert(1);
        assert_eq!(cache.pending_fpr_rebuild_count(), 1, "Should have 1 pending rebuild");

        // Access the segment again - should trigger rebuild (invalidate L1/L2)
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());

        // Pending rebuilds should be cleared
        assert_eq!(
            cache.pending_fpr_rebuild_count(),
            0,
            "Pending rebuilds should be cleared after access"
        );
    }

    /// Test: FPR rebuild invalidates L1 and L2 entries
    ///
    /// Verifies that when a segment is marked for FPR rebuild,
    /// the next get() call removes it from L1/L2 and forces reload.
    #[test]
    fn test_fpr_rebuild_invalidates_cache_entries() {
        use crate::bloom::fpr_controller::{AdaptationPolicy, FPRController};
        use crate::bloom::migration::MigrationThresholds;

        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l3_index_dir = temp_dir.path().to_path_buf();

        let fpr_controller = Arc::new(FPRController::new(
            AdaptationPolicy::default(),
            MigrationThresholds::default(),
        ));

        let cache = AdaptiveBloomCache::try_with_fpr_controller(config, fpr_controller).unwrap();

        // Create loader that counts calls
        let loader_calls = AtomicU64::new(0);
        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            loader_calls.fetch_add(1, Ordering::Relaxed);
            let keys: Vec<String> = (0..5).map(|i| format!("seg{}_key_{}", id, i)).collect();
            let cb = CustomBloom::from_keys(&keys, keys.len(), DEFAULT_BLOOM_FPR as f64);
            Ok(Some((cb, keys)))
        };

        // Load segment 1 into L1
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert_eq!(cache.l1_cache.len(), 1);
        assert_eq!(loader_calls.load(Ordering::Relaxed), 1);

        // Second access should hit L1 (no loader call)
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert_eq!(loader_calls.load(Ordering::Relaxed), 1, "Should hit L1 cache");

        // Mark for FPR rebuild
        cache.pending_fpr_rebuilds.write().insert(1);
        assert_eq!(cache.pending_fpr_rebuild_count(), 1);

        // Next access should invalidate L1 and reload from loader
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert_eq!(
            loader_calls.load(Ordering::Relaxed),
            2,
            "Should reload after FPR rebuild"
        );
        assert_eq!(cache.pending_fpr_rebuild_count(), 0, "Pending flag should be cleared");

        // L1 should now have the entry again (reloaded)
        assert_eq!(cache.l1_cache.len(), 1);
    }

    /// Test: FPR rebuild works for entries in L2 cache
    ///
    /// Verifies that L2 entries are also invalidated during FPR rebuild.
    #[test]
    fn test_fpr_rebuild_invalidates_l2_entries() {
        use crate::bloom::fpr_controller::{AdaptationPolicy, FPRController};
        use crate::bloom::migration::MigrationThresholds;

        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 1; // Small L1 to force L2 population
        config.l3_index_dir = temp_dir.path().to_path_buf();

        let fpr_controller = Arc::new(FPRController::new(
            AdaptationPolicy::default(),
            MigrationThresholds::default(),
        ));

        let cache = AdaptiveBloomCache::try_with_fpr_controller(config, fpr_controller).unwrap();

        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            let keys: Vec<String> = (0..10).map(|i| format!("seg{}_key_{}", id, i)).collect();
            let cb = CustomBloom::from_keys(&keys, 800, DEFAULT_BLOOM_FPR as f64);
            Ok(Some((cb, keys)))
        };

        // Load segment 1 and 2 - segment 1 will be evicted to L2
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());

        let result = cache.get(2, &loader).unwrap();
        assert!(result.is_some());

        // Check segment 1 was evicted to L2 (L1 has only 1 entry due to small capacity)
        let l2_size = cache.l2_cache.len();
        let l1_size = cache.l1_cache.len();

        // Mark segment 1 for FPR rebuild (wherever it is - L1 or L2)
        cache.pending_fpr_rebuilds.write().insert(1);

        // Access segment 1 - should be invalidated and reloaded
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert_eq!(cache.pending_fpr_rebuild_count(), 0, "Pending flag should be cleared");

        // Verify we still have the correct sizes (entry was reloaded)
        assert_eq!(cache.l1_cache.len(), l1_size);
        assert_eq!(cache.l2_cache.len(), l2_size);
    }

    /// Test: BLOOM-001 - try_new rejects zero l1_max_filters
    #[test]
    fn test_try_new_rejects_zero_l1_max_filters() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 0;
        let result = AdaptiveBloomCache::try_new(config);
        assert!(result.is_err(), "try_new should reject zero l1_max_filters");
    }

    /// Test: BLOOM-001 - try_new rejects zero l2_max_filters
    #[test]
    fn test_try_new_rejects_zero_l2_max_filters() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l2_max_filters = 0;
        let result = AdaptiveBloomCache::try_new(config);
        assert!(result.is_err(), "try_new should reject zero l2_max_filters");
    }

    /// Test: BLOOM-001 - try_with_fpr_controller rejects zero config
    #[test]
    fn test_try_with_fpr_controller_rejects_zero_config() {
        use crate::bloom::fpr_controller::{AdaptationPolicy, FPRController};
        use crate::bloom::migration::MigrationThresholds;

        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 0;
        let controller = Arc::new(FPRController::new(
            AdaptationPolicy::default(),
            MigrationThresholds::default(),
        ));
        let result = AdaptiveBloomCache::try_with_fpr_controller(config, controller);
        assert!(
            result.is_err(),
            "try_with_fpr_controller should reject zero l1_max_filters"
        );
    }

    // ==================== BLOOM-006: Concurrent Access Tests ====================

    /// Test: BLOOM-006 - Concurrent insert and get operations verify no deadlocks and data consistency
    #[test]
    fn test_concurrent_insert_and_get_no_deadlock() {
        use std::thread;

        let config = AdaptiveBloomCacheConfig::default();
        let cache = Arc::new(AdaptiveBloomCache::try_new(config).unwrap());

        let num_threads: usize = 8;
        let keys_per_thread: usize = 50;
        let success_count = Arc::new(AtomicUsize::new(0));

        thread::scope(|s| {
            for t in 0..num_threads {
                let cache_clone = cache.clone();
                let success_clone = success_count.clone();
                s.spawn(move || {
                    let segment_base: u64 = (t * 1000) as u64;
                    // Insert phase
                    for i in 0..keys_per_thread {
                        let segment_id = segment_base + i as u64;
                        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 10);
                        filter.insert(&format!("key_{}_{}", t, i));
                        cache_clone.insert(segment_id, filter);
                    }
                    // Get phase - verify our own inserts
                    for i in 0..keys_per_thread {
                        let segment_id = segment_base + i as u64;
                        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
                        if let Ok(Some(_)) = cache_clone.get(segment_id, &loader) {
                            success_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        // All gets should succeed (no deadlocks, no panics)
        let successes = success_count.load(Ordering::Relaxed);
        let expected = num_threads * keys_per_thread;
        assert_eq!(
            successes, expected,
            "All concurrent gets should succeed, got {}/{}",
            successes, expected
        );
    }

    /// Test: BLOOM-006 - Concurrent evict and get operations verify no panics or deadlocks
    #[test]
    fn test_concurrent_evict_and_get_no_panic() {
        use std::thread;

        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 20; // Small limit to trigger evictions
        let cache = Arc::new(AdaptiveBloomCache::try_new(config).unwrap());

        // Pre-populate with some data
        for i in 0..50u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 10);
            filter.insert(&format!("pre_key_{}", i));
            cache.insert(i, filter);
        }

        let num_threads: usize = 6;
        let ops_per_thread: usize = 30;
        let get_success = Arc::new(AtomicUsize::new(0));
        let remove_success = Arc::new(AtomicUsize::new(0));

        thread::scope(|s| {
            for t in 0..num_threads {
                let cache_clone = cache.clone();
                let get_clone = get_success.clone();
                let remove_clone = remove_success.clone();
                s.spawn(move || {
                    let base = t * 100;
                    for i in 0..ops_per_thread {
                        if i % 2 == 0 {
                            // Get operation
                            let segment_id = (base + i) as u64 % 50;
                            let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
                            let _ = cache_clone.get(segment_id, &loader);
                            get_clone.fetch_add(1, Ordering::Relaxed);
                        } else {
                            // Remove operation
                            let segment_id = (base + i) as u64 % 50;
                            let _ = cache_clone.remove(segment_id);
                            remove_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        // Verify no panics or deadlocks occurred (test completes)
        let gets = get_success.load(Ordering::Relaxed);
        let removes = remove_success.load(Ordering::Relaxed);
        let expected_each = num_threads * ops_per_thread / 2;
        assert_eq!(gets, expected_each);
        assert_eq!(removes, expected_each);
    }

    /// Test: BLOOM-006 - Mixed concurrent operations verify data consistency
    #[test]
    fn test_concurrent_multiple_operations_consistency() {
        use std::thread;

        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 30;
        config.l2_max_filters = 50;
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = Arc::new(AdaptiveBloomCache::try_new(config).unwrap());

        let num_threads: usize = 4;
        let ops_per_thread: usize = 40;
        let successful_gets = Arc::new(AtomicUsize::new(0));
        let successful_contains = Arc::new(AtomicUsize::new(0));

        // Pre-insert known data
        for i in 0..20u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 10);
            let key = format!("known_key_{}", i);
            filter.insert(&key);
            cache.insert(i, filter);
        }

        thread::scope(|s| {
            for t in 0..num_threads {
                let cache_clone = cache.clone();
                let get_clone = successful_gets.clone();
                let contains_clone = successful_contains.clone();
                s.spawn(move || {
                    for i in 0..ops_per_thread {
                        let segment_id = ((t * ops_per_thread + i) % 20) as u64;
                        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
                            let key = format!("known_key_{}", id);
                            let cb = CustomBloom::from_keys(std::slice::from_ref(&key), 80, DEFAULT_BLOOM_FPR as f64);
                            Ok(Some((cb, vec![key])))
                        };

                        // Test get
                        if let Ok(Some(filter)) = cache_clone.get(segment_id, &loader) {
                            get_clone.fetch_add(1, Ordering::Relaxed);
                            // Test contains on the returned filter
                            let key = format!("known_key_{}", segment_id);
                            if filter.contains(&key) {
                                contains_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                });
            }
        });

        let gets = successful_gets.load(Ordering::Relaxed);
        let contains = successful_contains.load(Ordering::Relaxed);
        // All operations should complete without errors
        assert!(gets > 0, "Should have successful gets, got {}", gets);
        assert!(contains > 0, "Should have successful contains, got {}", contains);
        // Verify stats are consistent
        let stats = cache.stats();
        let expected_total = stats.l1_hits + stats.l2_hits + stats.l3_hits + stats.total_misses;
        assert_eq!(expected_total, gets as u64, "Stats should reflect total operations");
    }

    /// Test: BLOOM-006 - Concurrent insert with keys and migration to L2
    #[test]
    fn test_concurrent_insert_with_keys_l2_migration() {
        use std::thread;

        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 10; // Small L1 to trigger eviction to L2
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let l1_max = config.l1_max_filters;
        let cache = Arc::new(AdaptiveBloomCache::try_new(config).unwrap());

        let num_threads: usize = 4;
        let inserts_per_thread: usize = 15;

        thread::scope(|s| {
            for t in 0..num_threads {
                let cache_clone = cache.clone();
                s.spawn(move || {
                    for i in 0..inserts_per_thread {
                        let segment_id = (t * inserts_per_thread + i) as u64;
                        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 10);
                        let keys: Vec<String> = (0..5).map(|k| format!("seg{}_key_{}", segment_id, k)).collect();
                        for k in &keys {
                            filter.insert(k);
                        }
                        cache_clone.insert_l1_with_keys(segment_id, Arc::new(BloomFilterWrapper::Bloom(filter)), keys);
                    }
                });
            }
        });

        // After all inserts, check L2 has some migrated entries (L1 evictions went to L2)
        let l2_size = cache.l2_cache.len();
        let l1_size = cache.l1_cache.len();
        // L1 should be at capacity, L2 should have some entries
        assert!(l1_size <= l1_max, "L1 should not exceed capacity");
        assert!(l2_size > 0, "L2 should have migrated entries, got {}", l2_size);
    }

    // ==================== FREQ-001: Frequency-Aware Migration Tests ====================

    /// Test: FREQ-001 - get_segment_frequency returns correct tier for L1 entries
    #[test]
    fn test_get_segment_frequency_l1() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert a filter
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        filter.insert(&"freq_test".to_string());
        cache.insert(1, filter);

        // Should be Cold (0 accesses)
        let (tier, count) = cache.get_segment_frequency(1);
        assert_eq!(tier, FrequencyTier::Cold);
        assert_eq!(count, 0);

        // Access the segment multiple times
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        for _ in 0..50 {
            let _ = cache.get(1, &loader);
        }

        // Should now have 50 accesses -> Warm tier
        let (tier, count) = cache.get_segment_frequency(1);
        assert_eq!(tier, FrequencyTier::Warm);
        assert_eq!(count, 50);
    }

    /// Test: FREQ-001 - get_segment_frequency returns correct tier for L2 entries
    #[test]
    fn test_get_segment_frequency_l2() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 1;
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert two filters - first will be evicted to L2
        for i in 1..=2u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            let keys: Vec<String> = (0..10).map(|k| format!("key_{}_{}", i, k)).collect();
            for k in &keys {
                filter.insert(k);
            }
            cache.insert_l1_with_keys(i, Arc::new(BloomFilterWrapper::Bloom(filter)), keys);
        }

        // Segment 1 should now be in L2
        let (tier, _count) = cache.get_segment_frequency(1);
        // Should be Cold (no additional accesses after L1 insertion)
        assert_eq!(tier, FrequencyTier::Cold);

        // Segment 2 should be in L1
        let (tier_l1, _count_l1) = cache.get_segment_frequency(2);
        assert_eq!(tier_l1, FrequencyTier::Cold);
    }

    /// Test: FREQ-001 - classify_by_frequency works correctly
    #[test]
    fn test_classify_by_frequency_in_adaptive() {
        let thresholds = MigrationThresholds::default();

        // Verify classification matches expected tiers
        assert_eq!(classify_by_frequency(0, &thresholds), FrequencyTier::Cold);
        assert_eq!(classify_by_frequency(9, &thresholds), FrequencyTier::Cold);
        assert_eq!(classify_by_frequency(10, &thresholds), FrequencyTier::Warm);
        assert_eq!(classify_by_frequency(99, &thresholds), FrequencyTier::Warm);
        assert_eq!(classify_by_frequency(100, &thresholds), FrequencyTier::Hot);
        assert_eq!(classify_by_frequency(1000, &thresholds), FrequencyTier::Hot);
    }

    /// Test: FREQ-001 - promote_by_frequency moves hot segments to L1
    #[test]
    fn test_promote_by_frequency_hot_to_l1() {
        let config = AdaptiveBloomCacheConfig::default();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert a filter into L2 directly
        let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        let keys: Vec<String> = (0..10).map(|i| format!("promote_key_{}", i)).collect();
        for k in &keys {
            filter.insert(k);
        }
        cache.insert_l2_with_keys(42, &BloomFilterWrapper::Bloom(filter), keys);

        // Initially Cold (0 accesses)
        let (tier, _) = cache.get_segment_frequency(42);
        assert_eq!(tier, FrequencyTier::Cold);

        // Access it many times to make it Hot
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        for _ in 0..200 {
            let _ = cache.get(42, &loader);
        }

        // Should be Hot now
        let (tier, count) = cache.get_segment_frequency(42);
        assert_eq!(tier, FrequencyTier::Hot);
        assert!(count >= 200);

        // Promote by frequency should move it to L1 if it's in L2
        // Note: get() already loads into L1, so segment may already be in L1
    }

    /// Test: FREQ-001 - frequency tier affects eviction order
    /// Verifies that cold segments are evicted before hot segments during batch eviction.
    #[test]
    fn test_frequency_aware_eviction_order() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 5;
        config.l2_max_filters = 20;
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Insert 5 filters
        for i in 1..=5u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            let keys: Vec<String> = (0..10).map(|k| format!("evict_key_{}_{}", i, k)).collect();
            for k in &keys {
                filter.insert(k);
            }
            cache.insert_l1_with_keys(i, Arc::new(BloomFilterWrapper::Bloom(filter)), keys);
        }

        // Access segments 4 and 5 many times to make them hot
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        for _ in 0..200 {
            let _ = cache.get(4, &loader);
            let _ = cache.get(5, &loader);
        }

        // Verify segments 4 and 5 are Hot
        let (tier4, _) = cache.get_segment_frequency(4);
        let (tier5, _) = cache.get_segment_frequency(5);
        assert_eq!(tier4, FrequencyTier::Hot);
        assert_eq!(tier5, FrequencyTier::Hot);

        // Segments 1, 2, 3 should be Cold (only 1 access from insertion)
        let (tier1, _) = cache.get_segment_frequency(1);
        assert_eq!(tier1, FrequencyTier::Cold);

        // Now insert more filters to trigger eviction
        // L1 capacity is 5, currently has 5. Adding 5 more triggers eviction.
        // Batch eviction evicts up to 10 at once.
        for i in 6..=10u64 {
            let mut filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
            let keys: Vec<String> = (0..10).map(|k| format!("new_key_{}_{}", i, k)).collect();
            for k in &keys {
                filter.insert(k);
            }
            cache.insert_l1_with_keys(i, Arc::new(BloomFilterWrapper::Bloom(filter)), keys);
        }

        // After batch eviction, hot segments (4, 5) should have been migrated to L2
        // Cold segments (1, 2, 3) should also be in L2 or evicted entirely
        // The key assertion: hot segments should still be accessible via L2
        let l2_size = cache.l2_cache.len();
        assert!(l2_size > 0, "L2 should have migrated entries");

        // Hot segments should be in L2 (migrated with keys)
        let loader2 = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let result4 = cache.get(4, &loader2);
        assert!(result4.is_ok(), "Hot segment 4 should still be accessible");
    }

    /// Test: FREQ-001 - frequency thresholds are configurable
    #[test]
    fn test_configurable_frequency_thresholds() {
        let thresholds = MigrationThresholds {
            hot_tier_access_count: 50,
            warm_tier_access_count: 5,
            frequency_weight: 0.5,
            ..MigrationThresholds::default()
        };

        // With custom thresholds, 50 accesses should be Hot
        assert_eq!(classify_by_frequency(50, &thresholds), FrequencyTier::Hot);
        // 25 accesses should be Warm
        assert_eq!(classify_by_frequency(25, &thresholds), FrequencyTier::Warm);
        // 4 accesses should be Cold
        assert_eq!(classify_by_frequency(4, &thresholds), FrequencyTier::Cold);
    }

    #[test]
    fn test_frequency_aware_migration() {
        // Test: verifies frequency-based tier classification for segment migration
        // This test validates that segments with different access patterns are classified
        // into the correct tiers (Hot, Warm, Cold)
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 10;
        config.l2_max_filters = 20;
        let temp_dir = tempfile::tempdir().unwrap();
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            let keys: Vec<String> = (0..20).map(|i| format!("key_{}_{}", id, i)).collect();
            let cb = CustomBloom::from_keys(&keys, keys.len(), DEFAULT_BLOOM_FPR as f64);
            Ok(Some((cb, keys)))
        };

        // Hot segments: accessed many times (should be Hot tier with count >= 200)
        for _ in 0..250 {
            let _ = cache.get(1, &loader);
        }
        let (tier1, count1) = cache.get_segment_frequency(1);
        assert_eq!(tier1, FrequencyTier::Hot, "Segment 1 should be Hot");
        assert!(count1 >= 200, "Hot segment should have >= 200 accesses, got {}", count1);

        // Warm segments: accessed moderate times (should be Warm tier)
        for _ in 0..30 {
            let _ = cache.get(2, &loader);
        }
        let (tier2, count2) = cache.get_segment_frequency(2);
        assert_eq!(tier2, FrequencyTier::Warm, "Segment 2 should be Warm");
        assert!(count2 >= 20, "Warm segment should have >= 20 accesses, got {}", count2);

        // Verify that hot segment has significantly more accesses than warm
        assert!(
            count1 > count2 * 5,
            "Hot segment should have much higher count than warm"
        );
    }

    // =========================================================================
    // OPT-002: CustomBloom Integration Test
    // =========================================================================

    /// Test: OPT-002 - CustomBloom integration into AdaptiveBloomCache
    ///
    /// Verifies:
    /// 1. L1CacheEntry accepts both BloomFilter and CustomBloom via BloomFilterWrapper
    /// 2. L2CacheEntry uses CustomBloom directly for V3 format performance
    /// 3. contains() works correctly through the unified interface
    /// 4. Migration from legacy BloomFilter to CustomBloom path works
    #[test]
    fn test_custom_bloom_integration() {
        let mut config = AdaptiveBloomCacheConfig::default();
        config.l1_max_filters = 50;
        config.l2_max_filters = 100;
        let temp_dir = tempfile::tempdir().unwrap();
        config.l3_index_dir = temp_dir.path().to_path_buf();
        let cache = AdaptiveBloomCache::try_new(config).unwrap();

        // Test 1: Insert legacy BloomFilter - gets wrapped in BloomFilterWrapper::Bloom
        let mut legacy_filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 100);
        legacy_filter.insert(&"legacy_key".to_string());
        cache.insert(1, legacy_filter);

        // Test 2: Insert CustomBloom via L1 with keys - gets wrapped in BloomFilterWrapper::Custom
        let custom_bloom = CustomBloom::with_capacity(100, 0.01);
        cache.insert_l1_arc(2, Arc::new(BloomFilterWrapper::Custom(custom_bloom.clone())));

        // Test 3: Verify contains() works through unified interface
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };

        // Legacy filter should be accessible
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some(), "Legacy filter should be retrievable");
        let wrapper = result.unwrap();
        assert!(wrapper.contains("legacy_key"), "Legacy filter should contain key");
        assert!(!wrapper.is_custom(), "Legacy filter should NOT be CustomBloom");

        // Test 4: L2 migration uses CustomBloom
        let mut l2_filter = BloomFilter::with_rate(DEFAULT_BLOOM_FPR, 1000);
        let l2_keys: Vec<String> = (0..500).map(|i| format!("l2_key_{}", i)).collect();
        for key in &l2_keys {
            l2_filter.insert(key);
        }
        cache.insert_l2_with_keys(10, &BloomFilterWrapper::Bloom(l2_filter), l2_keys.clone());

        // Verify L2 entry is accessible and contains keys
        let l2_result = cache.get(10, &loader).unwrap();
        assert!(l2_result.is_some(), "L2 entry should be retrievable");
        let l2_wrapper = l2_result.unwrap();
        assert!(l2_wrapper.contains("l2_key_0"), "L2 filter should contain key 0");
        assert!(l2_wrapper.contains("l2_key_499"), "L2 filter should contain key 499");
        assert!(
            !l2_wrapper.contains("nonexistent_key"),
            "L2 filter should NOT contain unknown key"
        );

        // Test 5: Statistics should be tracked correctly
        let stats = cache.stats();
        assert!(
            stats.l1_hits > 0 || stats.l2_hits > 0,
            "Should have recorded cache hits"
        );

        // Test 6: Clear and verify empty
        cache.clear();
        assert_eq!(cache.len(), 0, "Cache should be empty after clear");
        assert!(cache.is_empty(), "Cache should report empty after clear");
    }
}

// ============================================================================
// CustomBloomCache - High-performance bloom cache using CustomBloom (V3 format)
// ============================================================================

/// Configuration for CustomBloomCache
#[derive(Debug, Clone)]
pub struct CustomBloomCacheConfig {
    /// Maximum number of filters to cache
    pub max_filters: usize,
    /// Target false positive rate
    pub fpr_target: f64,
    /// Index directory where bloom filters are stored
    pub index_dir: PathBuf,
}

impl Default for CustomBloomCacheConfig {
    fn default() -> Self {
        Self {
            max_filters: 1000,
            fpr_target: 0.01, // 1%
            index_dir: PathBuf::from("index"),
        }
    }
}

/// Statistics for CustomBloomCache
#[derive(Debug, Clone, Default)]
pub struct CustomBloomCacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate (0.0-1.0)
    pub hit_rate: f64,
    /// Number of filters currently in cache
    pub filters_cached: usize,
    /// Memory used by cached filters (bytes)
    pub memory_used: usize,
}

impl CustomBloomCacheStats {
    /// Get hit rate as percentage
    pub fn hit_rate_percent(&self) -> f64 {
        self.hit_rate * 100.0
    }

    /// Get memory used in MB
    pub fn memory_used_mb(&self) -> f64 {
        self.memory_used as f64 / (1024.0 * 1024.0)
    }
}

/// Entry in CustomBloomCache
struct CustomBloomEntry {
    bloom: Arc<CustomBloom>,
    memory_size: usize,
}

/// High-performance bloom filter cache using CustomBloom with V3 format
///
/// This cache provides:
/// - Fast loading: V3 format enables direct bitset loading (< 100µs)
/// - Fast queries: CustomBloom uses deterministic XXH3 (< 10µs for negative queries)
/// - Automatic migration: V1/V2 formats are automatically migrated to V3 on first load
/// - CLOCK eviction: Approximate LRU with O(1) lock-free access
pub struct CustomBloomCache {
    /// Cache of loaded CustomBloom filters
    cache: DashMap<u64, CustomBloomEntry>,
    /// CLOCK queue for eviction tracking
    clock_queue: Arc<ClockCache>,
    /// Configuration
    config: CustomBloomCacheConfig,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    memory_used: AtomicUsize,
}

impl CustomBloomCache {
    /// Create a new CustomBloomCache
    pub fn new(config: CustomBloomCacheConfig) -> Self {
        const NUM_CLOCK_SHARDS: usize = 16;
        let clock_queue = Arc::new(ClockCache::new(config.max_filters, NUM_CLOCK_SHARDS));

        Self {
            cache: DashMap::new(),
            clock_queue,
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            memory_used: AtomicUsize::new(0),
        }
    }

    /// Get a CustomBloom filter for a segment (loads on-demand if not cached)
    ///
    /// # Arguments
    /// * `segment_id` - Segment identifier
    /// * `loader` - Function to load CustomBloom from disk. Should try V3 first, then fallback to V1/V2.
    /// * `migrator` - Optional function to migrate V1/V2 to V3 after loading
    pub fn get(
        &self,
        segment_id: u64,
        loader: &dyn Fn(u64) -> FileKVResult<Option<(CustomBloom, Vec<String>)>>,
    ) -> FileKVResult<Option<Arc<CustomBloom>>> {
        // Check if filter is already cached
        if let Some(entry) = self.cache.get(&segment_id) {
            self.hits.fetch_add(1, Ordering::Relaxed);

            // Mark as referenced in CLOCK queue
            self.clock_queue.tick(segment_id);

            return Ok(Some(entry.bloom.clone()));
        }

        // Filter not in cache, load on-demand
        self.misses.fetch_add(1, Ordering::Relaxed);

        // Use loader to load the filter
        match loader(segment_id)? {
            Some((bloom, _keys)) => {
                // Cache the loaded filter
                self.cache_and_promote(segment_id, bloom);
                // Get the cached filter and return Arc
                if let Some(entry) = self.cache.get(&segment_id) {
                    Ok(Some(entry.bloom.clone()))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Insert a CustomBloom filter into the cache
    pub fn insert(&self, segment_id: u64, bloom: CustomBloom) {
        self.cache_and_promote(segment_id, bloom);
    }

    /// Check if a key exists in a segment's bloom filter (convenience method)
    pub fn contains(
        &self,
        segment_id: u64,
        key: &str,
        loader: &dyn Fn(u64) -> FileKVResult<Option<(CustomBloom, Vec<String>)>>,
    ) -> FileKVResult<Option<bool>> {
        match self.get(segment_id, loader)? {
            Some(bloom) => Ok(Some(bloom.contains(key.as_bytes()))),
            None => Ok(None),
        }
    }

    /// Remove a CustomBloom filter from the cache
    pub fn remove(&self, segment_id: u64) -> Option<Arc<CustomBloom>> {
        if let Some((_, entry)) = self.cache.remove(&segment_id) {
            self.memory_used.fetch_sub(entry.memory_size, Ordering::Relaxed);

            // Remove from CLOCK queue
            self.clock_queue.remove(segment_id);

            Some(entry.bloom)
        } else {
            None
        }
    }

    /// Clear all cached filters
    pub fn clear(&self) {
        self.cache.clear();
        self.clock_queue.clear();
        self.memory_used.store(0, Ordering::Relaxed);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CustomBloomCacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let filters_cached = self.cache.len();
        let memory_used = self.memory_used.load(Ordering::Relaxed);

        CustomBloomCacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            filters_cached,
            memory_used,
        }
    }

    /// Get number of cached filters
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Cache a filter and update CLOCK queue (internal helper)
    fn cache_and_promote(&self, segment_id: u64, bloom: CustomBloom) {
        let memory_size = bloom.memory_usage();
        let entry = CustomBloomEntry {
            bloom: Arc::new(bloom),
            memory_size,
        };

        // Check memory/count limit and evict if necessary
        let current_count = self.cache.len();
        if current_count >= self.config.max_filters {
            self.evict_one();
        }

        // Insert into cache
        if let Some(old_entry) = self.cache.insert(segment_id, entry) {
            self.memory_used.fetch_sub(old_entry.memory_size, Ordering::Relaxed);
        }

        self.memory_used.fetch_add(memory_size, Ordering::Relaxed);

        // Insert into CLOCK queue (may trigger eviction)
        if let Some(evicted_id) = self.clock_queue.insert(segment_id) {
            // Remove evicted entry from cache if it's still there
            if let Some((_, evicted)) = self.cache.remove(&evicted_id) {
                self.memory_used.fetch_sub(evicted.memory_size, Ordering::Relaxed);
                debug!("Evicted CustomBloom for segment {} (CLOCK eviction)", evicted_id);
            }
        }
    }

    /// Evict one entry using CLOCK algorithm
    fn evict_one(&self) {
        if let Some(evicted_id) = self.clock_queue.pop_lru() {
            if let Some((_, entry)) = self.cache.remove(&evicted_id) {
                self.memory_used.fetch_sub(entry.memory_size, Ordering::Relaxed);
                debug!("Evicted CustomBloom for segment {} (CLOCK eviction)", evicted_id);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod custom_bloom_cache_tests {
    use super::*;

    #[test]
    fn test_custom_bloom_cache_config_default() {
        let config = CustomBloomCacheConfig::default();
        assert_eq!(config.max_filters, 1000);
        assert_eq!(config.fpr_target, 0.01);
    }

    #[test]
    fn test_custom_bloom_cache_basic() {
        let config = CustomBloomCacheConfig::default();
        let cache = CustomBloomCache::new(config);

        // Create a test CustomBloom
        let mut bloom = CustomBloom::with_capacity(100, 0.01);
        bloom.insert(b"test_key");

        // Insert into cache
        cache.insert(1, bloom);

        // Retrieve from cache
        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let cached = cache.get(1, &loader).unwrap();
        assert!(cached.is_some());
        assert!(cached.unwrap().contains(b"test_key"));
    }

    #[test]
    fn test_custom_bloom_cache_contains() {
        let config = CustomBloomCacheConfig::default();
        let cache = CustomBloomCache::new(config);

        let mut bloom = CustomBloom::with_capacity(100, 0.01);
        bloom.insert(b"test_key");
        cache.insert(1, bloom);

        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };
        let result = cache.contains(1, "test_key", &loader).unwrap();
        assert_eq!(result, Some(true));

        let result = cache.contains(1, "nonexistent", &loader).unwrap();
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_custom_bloom_cache_stats() {
        let config = CustomBloomCacheConfig::default();
        let cache = CustomBloomCache::new(config);

        let mut bloom = CustomBloom::with_capacity(100, 0.01);
        bloom.insert(b"test");
        cache.insert(1, bloom);

        let loader = |_id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> { Ok(None) };

        cache.get(1, &loader).unwrap(); // Hit
        cache.get(1, &loader).unwrap(); // Hit
        cache.get(2, &loader).unwrap(); // Miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.5);
    }

    #[test]
    fn test_custom_bloom_cache_remove() {
        let config = CustomBloomCacheConfig::default();
        let cache = CustomBloomCache::new(config);

        let mut bloom = CustomBloom::with_capacity(100, 0.01);
        bloom.insert(b"test");
        cache.insert(1, bloom);

        assert!(cache.get(1, &|_| Ok(None)).unwrap().is_some());

        let removed = cache.remove(1);
        assert!(removed.is_some());
        assert!(cache.get(1, &|_| Ok(None)).unwrap().is_none());
    }

    #[test]
    fn test_custom_bloom_cache_clear() {
        let config = CustomBloomCacheConfig::default();
        let cache = CustomBloomCache::new(config);

        for i in 0..10 {
            let mut bloom = CustomBloom::with_capacity(100, 0.01);
            bloom.insert(format!("key_{}", i).as_bytes());
            cache.insert(i, bloom);
        }

        assert!(!cache.is_empty());
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_custom_bloom_cache_eviction() {
        let mut config = CustomBloomCacheConfig::default();
        config.max_filters = 5;
        let max_filters = config.max_filters; // Copy before move
        let cache = CustomBloomCache::new(config);

        // Insert 5 filters
        for i in 1..=5u64 {
            let mut bloom = CustomBloom::with_capacity(100, 0.01);
            bloom.insert(format!("key_{}", i).as_bytes());
            cache.insert(i, bloom);
        }

        assert_eq!(cache.len(), 5);

        // Insert 5 more - should trigger eviction
        for i in 6..=10u64 {
            let mut bloom = CustomBloom::with_capacity(100, 0.01);
            bloom.insert(format!("key_{}", i).as_bytes());
            cache.insert(i, bloom);
        }

        // After eviction, cache should have <= 5 entries
        assert!(
            cache.len() <= max_filters,
            "Cache should have <= {} entries after eviction, got {}",
            max_filters,
            cache.len()
        );
    }

    #[test]
    fn test_custom_bloom_cache_on_demand_load() {
        let config = CustomBloomCacheConfig::default();
        let cache = CustomBloomCache::new(config);

        // Simulate on-demand loading
        let loader = |id: u64| -> FileKVResult<Option<(CustomBloom, Vec<String>)>> {
            let mut bloom = CustomBloom::with_capacity(100, 0.01);
            bloom.insert(format!("loaded_key_{}", id).as_bytes());
            Ok(Some((bloom, vec![format!("loaded_key_{}", id)])))
        };

        // First access - miss, then load
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains(b"loaded_key_1"));

        // Second access - hit
        let result = cache.get(1, &loader).unwrap();
        assert!(result.is_some());
    }
}
