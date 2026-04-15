//! Memory Tracker for FileKV
//!
//! 4.1 OPTIMIZATION: Global memory monitoring component that tracks
//! memory usage across all FileKV components (BlockCache, DenseIndex, MemTable, Segments).
//!
//! Provides:
//! - `get_memory_usage()` - Returns structured memory usage data
//! - Per-component memory tracking
//! - Optional memory limit enforcement

use std::sync::atomic::{AtomicU64, Ordering};

/// Memory usage breakdown per component
#[derive(Debug, Clone, Default)]
pub struct MemoryUsage {
    /// Block cache memory usage in bytes
    pub block_cache_bytes: u64,
    /// Dense index memory usage in bytes (all segments)
    pub dense_index_bytes: u64,
    /// MemTable memory usage in bytes
    pub memtable_bytes: u64,
    /// WAL buffer memory usage in bytes
    pub wal_buffer_bytes: u64,
    /// Mmap memory usage in bytes
    pub mmap_bytes: u64,
}

impl MemoryUsage {
    /// Total memory usage across all components
    pub fn total_bytes(&self) -> u64 {
        self.block_cache_bytes
            + self.dense_index_bytes
            + self.memtable_bytes
            + self.wal_buffer_bytes
            + self.mmap_bytes
    }

    /// Total memory in MB
    pub fn total_mb(&self) -> f64 {
        self.total_bytes() as f64 / 1024.0 / 1024.0
    }

    /// Human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Memory Usage: Total {:.2} MB (Cache: {:.2} MB, DenseIdx: {:.2} MB, MemTable: {:.2} MB, WAL: {:.2} MB, Mmap: {:.2} MB)",
            self.total_mb(),
            self.block_cache_bytes as f64 / 1024.0 / 1024.0,
            self.dense_index_bytes as f64 / 1024.0 / 1024.0,
            self.memtable_bytes as f64 / 1024.0 / 1024.0,
            self.wal_buffer_bytes as f64 / 1024.0 / 1024.0,
            self.mmap_bytes as f64 / 1024.0 / 1024.0,
        )
    }
}

/// 4.1 OPTIMIZATION: Memory tracker for FileKV
///
/// Tracks memory usage across all components and provides
/// methods to query and limit memory consumption.
#[derive(Debug)]
pub struct MemoryTracker {
    /// Block cache memory (tracked by cache itself)
    block_cache_bytes: AtomicU64,
    /// Dense index memory (sum of all segment dense indexes)
    dense_index_bytes: AtomicU64,
    /// MemTable memory (approximate)
    memtable_bytes: AtomicU64,
    /// WAL buffer memory
    wal_buffer_bytes: AtomicU64,
    /// Mmap memory (for segments with persistent mmap)
    mmap_bytes: AtomicU64,
    /// Optional memory limit in bytes (0 = unlimited)
    max_memory_bytes: u64,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new(max_memory_bytes: u64) -> Self {
        Self {
            block_cache_bytes: AtomicU64::new(0),
            dense_index_bytes: AtomicU64::new(0),
            memtable_bytes: AtomicU64::new(0),
            wal_buffer_bytes: AtomicU64::new(0),
            mmap_bytes: AtomicU64::new(0),
            max_memory_bytes,
        }
    }

    /// Update block cache memory usage
    pub fn set_block_cache_bytes(&self, bytes: u64) {
        self.block_cache_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update dense index memory usage
    pub fn set_dense_index_bytes(&self, bytes: u64) {
        self.dense_index_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update memtable memory usage
    pub fn set_memtable_bytes(&self, bytes: u64) {
        self.memtable_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update WAL buffer memory usage
    pub fn set_wal_buffer_bytes(&self, bytes: u64) {
        self.wal_buffer_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update mmap memory usage
    pub fn set_mmap_bytes(&self, bytes: u64) {
        self.mmap_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Get current memory usage snapshot
    pub fn get_usage(&self) -> MemoryUsage {
        MemoryUsage {
            block_cache_bytes: self.block_cache_bytes.load(Ordering::Relaxed),
            dense_index_bytes: self.dense_index_bytes.load(Ordering::Relaxed),
            memtable_bytes: self.memtable_bytes.load(Ordering::Relaxed),
            wal_buffer_bytes: self.wal_buffer_bytes.load(Ordering::Relaxed),
            mmap_bytes: self.mmap_bytes.load(Ordering::Relaxed),
        }
    }

    /// Check if memory limit is exceeded
    pub fn is_memory_limit_exceeded(&self) -> bool {
        if self.max_memory_bytes == 0 {
            return false; // Unlimited
        }
        self.get_usage().total_bytes() > self.max_memory_bytes
    }

    /// Get memory limit in bytes (0 = unlimited)
    pub fn max_memory_bytes(&self) -> u64 {
        self.max_memory_bytes
    }

    /// Check if dense index memory exceeds a per-segment budget
    /// 2.2 OPTIMIZATION: Returns true if dense index memory is within budget
    pub fn is_dense_index_within_budget(&self, per_segment_budget: u64) -> bool {
        // This is a simplified check - in production, you'd track per-segment usage
        let current = self.dense_index_bytes.load(Ordering::Relaxed);
        current <= per_segment_budget * 10 // Assume max 10 segments
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new(0) // Unlimited by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_basic() {
        let tracker = MemoryTracker::new(100 * 1024 * 1024); // 100MB limit

        tracker.set_block_cache_bytes(10 * 1024 * 1024);
        tracker.set_dense_index_bytes(5 * 1024 * 1024);
        tracker.set_memtable_bytes(20 * 1024 * 1024);

        let usage = tracker.get_usage();
        assert_eq!(usage.block_cache_bytes, 10 * 1024 * 1024);
        assert_eq!(usage.dense_index_bytes, 5 * 1024 * 1024);
        assert_eq!(usage.memtable_bytes, 20 * 1024 * 1024);
        assert_eq!(usage.total_bytes(), 35 * 1024 * 1024);
        assert!(!tracker.is_memory_limit_exceeded());
    }

    #[test]
    fn test_memory_tracker_limit_exceeded() {
        let tracker = MemoryTracker::new(50 * 1024 * 1024); // 50MB limit

        tracker.set_block_cache_bytes(30 * 1024 * 1024);
        tracker.set_dense_index_bytes(15 * 1024 * 1024);
        tracker.set_memtable_bytes(10 * 1024 * 1024);

        assert!(tracker.is_memory_limit_exceeded());
    }

    #[test]
    fn test_memory_usage_summary() {
        let tracker = MemoryTracker::new(0);
        tracker.set_block_cache_bytes(1_000_000);
        
        let usage = tracker.get_usage();
        let summary = usage.summary();
        assert!(summary.contains("Memory Usage"));
        assert!(summary.contains("Cache"));
    }

    #[test]
    fn test_dense_index_budget_check() {
        let tracker = MemoryTracker::new(0);
        tracker.set_dense_index_bytes(50 * 1024 * 1024);

        // Budget of 10MB per segment, 10 segments max = 100MB
        assert!(tracker.is_dense_index_within_budget(10 * 1024 * 1024));
        
        // Budget of 1MB per segment, 10 segments max = 10MB
        assert!(!tracker.is_dense_index_within_budget(1 * 1024 * 1024));
    }
}
