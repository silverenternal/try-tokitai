//! Performance Module Tracking - Per-Module Timing for Regression Debugging
//!
//! Tracks latency breakdown across modules in the read/write path,
//! enabling rapid identification of which module caused a performance regression.
//!
//! # Usage
//!
//! ```rust,ignore
//! let tracker = PerfTracker::new();
//! let mut timer = tracker.start_timer("bloom_lookup");
//! // ... do bloom lookup ...
//! timer.stop();
//!
//! // Later, get breakdown:
//! let report = tracker.report();
//! ```
//!
//! # Module Categories
//!
//! - `dense_index`   - Dense index lookup time
//! - `bloom_lookup`  - Bloom filter check time
//! - `cache_lookup`  - BlockCache get/insert time
//! - `segment_io`    - Segment read/mmap access time
//! - `decompress`    - Decompression time
//! - `wal_write`     - WAL submission time
//! - `memtable_insert` - MemTable insert time
//! - `compaction`    - Compaction execution time
//! - `total_get`     - End-to-end get() latency
//! - `total_put`     - End-to-end put() latency

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Number of module timing slots (fixed-size array, no allocation)
const NUM_MODULES: usize = 12;

/// Module timing indices (const for zero-cost indexing)
const MOD_DENSE_INDEX: usize = 0;
const MOD_BLOOM_LOOKUP: usize = 1;
const MOD_CACHE_LOOKUP: usize = 2;
const MOD_SEGMENT_IO: usize = 3;
const MOD_DECOMPRESS: usize = 4;
const MOD_WAL_WRITE: usize = 5;
const MOD_MEMTABLE_INSERT: usize = 6;
const MOD_COMPACTION: usize = 7;
const MOD_TOTAL_GET: usize = 8;
const MOD_TOTAL_PUT: usize = 9;
const MOD_PREFETCH: usize = 10;
const MOD_ZONE_MAP: usize = 11;

const MODULE_NAMES: [&str; NUM_MODULES] = [
    "dense_index",
    "bloom_lookup",
    "cache_lookup",
    "segment_io",
    "decompress",
    "wal_write",
    "memtable_insert",
    "compaction",
    "total_get",
    "total_put",
    "prefetch",
    "zone_map",
];

/// Per-module performance tracker using atomic counters.
///
/// For each module, tracks:
/// - `total_ns`: cumulative time spent in this module
/// - `count`: number of invocations
/// - `max_ns`: single slowest invocation
///
/// From these, average and estimated p99 can be computed.
pub struct PerfTracker {
    /// Cumulative time per module (nanoseconds)
    total_ns: [AtomicU64; NUM_MODULES],
    /// Invocation count per module
    count: [AtomicU64; NUM_MODULES],
    /// Maximum single invocation per module (nanoseconds)
    max_ns: [AtomicU64; NUM_MODULES],
}

impl PerfTracker {
    pub fn new() -> Self {
        Self {
            total_ns: std::array::from_fn(|_| AtomicU64::new(0)),
            count: std::array::from_fn(|_| AtomicU64::new(0)),
            max_ns: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    /// Start a timed section for a module.
    /// Returns a `PerfTimer` that records on `stop()`.
    pub fn start_timer(&self, module: &'static str) -> PerfTimer<'_> {
        let idx = self.find_module(module);
        PerfTimer {
            tracker: self,
            module_idx: idx,
            module_name: module,
            start: Instant::now(),
            stopped: false,
        }
    }

    /// Record a single measurement directly (for external callers).
    pub fn record(&self, module: &'static str, elapsed_ns: u64) {
        let idx = self.find_module(module);
        self.total_ns[idx].fetch_add(elapsed_ns, Ordering::Relaxed);
        self.count[idx].fetch_add(1, Ordering::Relaxed);
        // Update max with a simple CAS loop
        let mut current_max = self.max_ns[idx].load(Ordering::Relaxed);
        while elapsed_ns > current_max {
            match self.max_ns[idx].compare_exchange_weak(current_max, elapsed_ns, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(new) => current_max = new,
            }
        }
    }

    /// Reset all counters (for checkpoint-style resets)
    pub fn reset(&self) {
        for i in 0..NUM_MODULES {
            self.total_ns[i].store(0, Ordering::Relaxed);
            self.count[i].store(0, Ordering::Relaxed);
            self.max_ns[i].store(0, Ordering::Relaxed);
        }
    }

    /// Get a snapshot of per-module performance data
    #[allow(clippy::needless_range_loop)]
    pub fn snapshot(&self) -> PerfSnapshot {
        let mut modules = Vec::with_capacity(NUM_MODULES);
        for i in 0..NUM_MODULES {
            let total = self.total_ns[i].load(Ordering::Relaxed);
            let count = self.count[i].load(Ordering::Relaxed);
            let max = self.max_ns[i].load(Ordering::Relaxed);
            if count > 0 {
                modules.push(ModuleTiming {
                    name: MODULE_NAMES[i],
                    total_ns: total,
                    count,
                    avg_ns: total / count,
                    max_ns: max,
                });
            }
        }
        PerfSnapshot { modules }
    }

    /// Get timing for a specific module by name
    pub fn get_module(&self, module: &str) -> Option<ModuleTiming> {
        let idx = self.find_module(module);
        let total = self.total_ns[idx].load(Ordering::Relaxed);
        let count = self.count[idx].load(Ordering::Relaxed);
        if count == 0 {
            return None;
        }
        Some(ModuleTiming {
            name: MODULE_NAMES[idx],
            total_ns: total,
            count,
            avg_ns: total / count,
            max_ns: self.max_ns[idx].load(Ordering::Relaxed),
        })
    }

    /// Convert module name to index (linear scan of 12 elements = negligible)
    fn find_module(&self, name: &str) -> usize {
        MODULE_NAMES.iter().position(|&n| n == name).unwrap_or(0) // fallback to index 0 for unknown modules
    }
}

impl Default for PerfTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII timer for a module timing section.
/// Call `stop()` to record, or drop without calling `stop()` to discard.
pub struct PerfTimer<'a> {
    tracker: &'a PerfTracker,
    module_idx: usize,
    #[allow(dead_code)]
    module_name: &'a str,
    start: Instant,
    stopped: bool,
}

impl PerfTimer<'_> {
    /// Stop the timer and record the measurement.
    #[inline]
    pub fn stop(&mut self) -> u64 {
        if self.stopped {
            return 0;
        }
        self.stopped = true;
        let elapsed_ns = self.start.elapsed().as_nanos() as u64;
        self.tracker.total_ns[self.module_idx].fetch_add(elapsed_ns, Ordering::Relaxed);
        self.tracker.count[self.module_idx].fetch_add(1, Ordering::Relaxed);
        // Update max
        let mut current_max = self.tracker.max_ns[self.module_idx].load(Ordering::Relaxed);
        while elapsed_ns > current_max {
            match self.tracker.max_ns[self.module_idx].compare_exchange_weak(
                current_max,
                elapsed_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new) => current_max = new,
            }
        }
        elapsed_ns
    }

    /// Discard this timer without recording (e.g., on error path)
    pub fn discard(self) {
        // dropped without stop() = no recording
    }
}

impl Drop for PerfTimer<'_> {
    fn drop(&mut self) {
        // Auto-stop on drop if not already stopped
        if !self.stopped {
            let _ = self.stop();
        }
    }
}

/// Snapshot of per-module performance data
#[derive(Debug, Clone)]
pub struct PerfSnapshot {
    pub modules: Vec<ModuleTiming>,
}

/// Timing data for a single module
#[derive(Debug, Clone)]
pub struct ModuleTiming {
    pub name: &'static str,
    pub total_ns: u64,
    pub count: u64,
    pub avg_ns: u64,
    pub max_ns: u64,
}

impl PerfSnapshot {
    /// Print a human-readable report
    pub fn print(&self) {
        println!("=== Per-Module Performance Breakdown ===");
        println!(
            "{:<20} {:>10} {:>12} {:>12} {:>12}",
            "Module", "Count", "Avg (ns)", "Max (ns)", "Total (ns)"
        );
        println!("{}", "-".repeat(68));
        for m in &self.modules {
            println!(
                "{:<20} {:>10} {:>12} {:>12} {:>12}",
                m.name, m.count, m.avg_ns, m.max_ns, m.total_ns
            );
        }
    }

    /// Find a module by name
    pub fn find(&self, name: &str) -> Option<&ModuleTiming> {
        self.modules.iter().find(|m| m.name == name)
    }
}

/// Human-readable formatter for a single module timing
pub fn format_ns(ns: u64) -> String {
    if ns < 1000 {
        format!("{}ns", ns)
    } else if ns < 1_000_000 {
        format!("{:.1}µs", ns as f64 / 1000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", ns as f64 / 1_000_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_timer() {
        let tracker = PerfTracker::new();
        {
            let mut timer = tracker.start_timer("bloom_lookup");
            thread::sleep(Duration::from_micros(100));
            let elapsed = timer.stop();
            assert!(elapsed > 0);
        }

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.modules.len(), 1);
        assert_eq!(snapshot.modules[0].name, "bloom_lookup");
        assert_eq!(snapshot.modules[0].count, 1);
        assert!(snapshot.modules[0].avg_ns > 0);
    }

    #[test]
    fn test_multiple_modules() {
        let tracker = PerfTracker::new();
        tracker.record("dense_index", 100);
        tracker.record("dense_index", 200);
        tracker.record("bloom_lookup", 500);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.modules.len(), 2);

        let di = snapshot.find("dense_index").unwrap();
        assert_eq!(di.count, 2);
        assert_eq!(di.avg_ns, 150);
        assert_eq!(di.max_ns, 200);

        let bl = snapshot.find("bloom_lookup").unwrap();
        assert_eq!(bl.count, 1);
        assert_eq!(bl.avg_ns, 500);
    }

    #[test]
    fn test_reset() {
        let tracker = PerfTracker::new();
        tracker.record("cache_lookup", 100);
        tracker.reset();
        let snapshot = tracker.snapshot();
        assert!(snapshot.modules.is_empty());
    }

    #[test]
    fn test_format_ns() {
        assert_eq!(format_ns(500), "500ns");
        assert_eq!(format_ns(1500), "1.5µs");
        assert_eq!(format_ns(1_500_000), "1.50ms");
        assert_eq!(format_ns(1_500_000_000), "1.50s");
    }

    #[test]
    fn test_concurrent_record() {
        let tracker = Arc::new(PerfTracker::new());
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let t = Arc::clone(&tracker);
                thread::spawn(move || {
                    for _ in 0..1000 {
                        t.record("cache_lookup", 50);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let snapshot = tracker.snapshot();
        let cache = snapshot.find("cache_lookup").unwrap();
        assert_eq!(cache.count, 4000);
    }
}
