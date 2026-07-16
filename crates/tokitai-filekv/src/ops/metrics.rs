//! Prometheus Metrics Module for FileKV
//!
//! Provides comprehensive metrics collection and export for monitoring:
//! - Operation counters (reads, writes, deletes)
//! - Latency histograms
//! - Cache hit/miss statistics
//! - Compaction statistics
//! - Memory usage metrics
//! - Bloom Filter statistics
//! - Zone Map statistics

#[cfg(feature = "metrics")]
use metrics::{counter, gauge, histogram};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// FileKV comprehensive metrics collector
pub struct FileKVMetrics {
    // Operation counters
    write_count: AtomicU64,
    read_count: AtomicU64,
    delete_count: AtomicU64,
    write_errors: AtomicU64,
    read_errors: AtomicU64,
    delete_errors: AtomicU64,

    // Cache statistics
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    bloom_filter_hits: AtomicU64,
    bloom_filter_misses: AtomicU64,

    // Compaction statistics
    compaction_runs: AtomicU64,
    compaction_bytes_written: AtomicU64,
    compaction_segments_merged: AtomicU64,
    compaction_entries_removed: AtomicU64,
    compaction_tombstones_cleaned: AtomicU64,

    // Memory tracking
    memtable_bytes: AtomicU64,
    cache_bytes: AtomicU64,
    bloom_filter_bytes: AtomicU64,

    // Write amplification tracking
    user_bytes_written: AtomicU64,
    total_bytes_written: AtomicU64,

    // OPT-012: Read amplification tracking
    read_io_operations: AtomicU64,
    total_bytes_read: AtomicU64,

    // OPT-012: Space amplification tracking
    total_size_bytes: AtomicU64,

    // Latency tracking (in microseconds)
    write_latency_sum_us: AtomicU64,
    write_latency_count: AtomicU64,
    read_latency_sum_us: AtomicU64,
    read_latency_count: AtomicU64,
    delete_latency_sum_us: AtomicU64,
    delete_latency_count: AtomicU64,

    // Flush latency tracking (in microseconds)
    flush_count: AtomicU64,
    flush_errors: AtomicU64,
    flush_latency_sum_us: AtomicU64,
    flush_latency_count: AtomicU64,
}

impl FileKVMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            write_count: AtomicU64::new(0),
            read_count: AtomicU64::new(0),
            delete_count: AtomicU64::new(0),
            write_errors: AtomicU64::new(0),
            read_errors: AtomicU64::new(0),
            delete_errors: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            bloom_filter_hits: AtomicU64::new(0),
            bloom_filter_misses: AtomicU64::new(0),
            compaction_runs: AtomicU64::new(0),
            compaction_bytes_written: AtomicU64::new(0),
            compaction_segments_merged: AtomicU64::new(0),
            compaction_entries_removed: AtomicU64::new(0),
            compaction_tombstones_cleaned: AtomicU64::new(0),
            memtable_bytes: AtomicU64::new(0),
            cache_bytes: AtomicU64::new(0),
            bloom_filter_bytes: AtomicU64::new(0),
            user_bytes_written: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(0),
            read_io_operations: AtomicU64::new(0),
            total_bytes_read: AtomicU64::new(0),
            total_size_bytes: AtomicU64::new(0),
            write_latency_sum_us: AtomicU64::new(0),
            write_latency_count: AtomicU64::new(0),
            read_latency_sum_us: AtomicU64::new(0),
            read_latency_count: AtomicU64::new(0),
            delete_latency_sum_us: AtomicU64::new(0),
            delete_latency_count: AtomicU64::new(0),
            flush_count: AtomicU64::new(0),
            flush_errors: AtomicU64::new(0),
            flush_latency_sum_us: AtomicU64::new(0),
            flush_latency_count: AtomicU64::new(0),
        }
    }

    // ==================== Operation Recording ====================

    /// Record a successful write operation
    pub fn record_write_success(&self, latency_us: f64) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.add_write_latency(latency_us);
    }

    /// Record a failed write operation
    pub fn record_write_error(&self, _reason: &str) {
        self.write_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful read operation
    pub fn record_read_success(&self, latency_us: f64) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.add_read_latency(latency_us);
    }

    /// Record a failed read operation
    pub fn record_read_error(&self, _reason: &str) {
        self.read_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful delete operation
    pub fn record_delete_success(&self, latency_us: f64) {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        self.add_delete_latency(latency_us);
    }

    /// Record a failed delete operation
    pub fn record_delete_error(&self, _reason: &str) {
        self.delete_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful flush operation
    pub fn record_flush_success(&self, latency_us: f64) {
        self.flush_count.fetch_add(1, Ordering::Relaxed);
        self.add_flush_latency(latency_us);
    }

    /// Record a failed flush operation
    pub fn record_flush_error(&self, _reason: &str) {
        self.flush_errors.fetch_add(1, Ordering::Relaxed);
    }

    // ==================== Cache Statistics ====================

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a Bloom Filter hit (key might exist)
    pub fn record_bloom_hit(&self) {
        self.bloom_filter_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a Bloom Filter miss (key definitely doesn't exist)
    pub fn record_bloom_miss(&self) {
        self.bloom_filter_misses.fetch_add(1, Ordering::Relaxed);
    }

    // ==================== Compaction Statistics ====================

    /// Record a compaction run with all statistics
    pub fn record_compaction(
        &self,
        segments_merged: u64,
        bytes_written: u64,
        entries_removed: u64,
        tombstones_cleaned: u64,
    ) {
        self.compaction_runs.fetch_add(1, Ordering::Relaxed);
        self.compaction_bytes_written
            .fetch_add(bytes_written, Ordering::Relaxed);
        self.compaction_segments_merged
            .fetch_add(segments_merged, Ordering::Relaxed);
        self.compaction_entries_removed
            .fetch_add(entries_removed, Ordering::Relaxed);
        self.compaction_tombstones_cleaned
            .fetch_add(tombstones_cleaned, Ordering::Relaxed);
    }

    // ==================== Memory Tracking ====================

    /// Update memtable memory usage
    pub fn update_memtable_bytes(&self, bytes: u64) {
        self.memtable_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update cache memory usage
    pub fn update_cache_bytes(&self, bytes: u64) {
        self.cache_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update Bloom Filter memory usage
    pub fn update_bloom_filter_bytes(&self, bytes: u64) {
        self.bloom_filter_bytes.store(bytes, Ordering::Relaxed);
    }

    // ==================== Write Amplification Tracking ====================

    /// Record user data written
    pub fn record_user_bytes_written(&self, bytes: u64) {
        self.user_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record total bytes written (including WAL, compaction, etc.)
    pub fn record_total_bytes_written(&self, bytes: u64) {
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    // ==================== Latency Tracking ====================

    fn add_write_latency(&self, latency_us: f64) {
        self.write_latency_sum_us
            .fetch_add(latency_us as u64, Ordering::Relaxed);
        self.write_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    fn add_read_latency(&self, latency_us: f64) {
        self.read_latency_sum_us.fetch_add(latency_us as u64, Ordering::Relaxed);
        self.read_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    fn add_flush_latency(&self, latency_us: f64) {
        self.flush_latency_sum_us
            .fetch_add(latency_us as u64, Ordering::Relaxed);
        self.flush_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    fn add_delete_latency(&self, latency_us: f64) {
        self.delete_latency_sum_us
            .fetch_add(latency_us as u64, Ordering::Relaxed);
        self.delete_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    // ==================== Statistics Queries ====================

    /// Get average write latency in microseconds
    pub fn avg_write_latency_us(&self) -> f64 {
        let sum = self.write_latency_sum_us.load(Ordering::Relaxed) as f64;
        let count = self.write_latency_count.load(Ordering::Relaxed) as f64;
        if count == 0.0 {
            return 0.0;
        }
        sum / count
    }

    /// Get average read latency in microseconds
    pub fn avg_read_latency_us(&self) -> f64 {
        let sum = self.read_latency_sum_us.load(Ordering::Relaxed) as f64;
        let count = self.read_latency_count.load(Ordering::Relaxed) as f64;
        if count == 0.0 {
            return 0.0;
        }
        sum / count
    }

    /// Get average flush latency in microseconds
    pub fn avg_flush_latency_us(&self) -> f64 {
        let sum = self.flush_latency_sum_us.load(Ordering::Relaxed) as f64;
        let count = self.flush_latency_count.load(Ordering::Relaxed) as f64;
        if count == 0.0 {
            return 0.0;
        }
        sum / count
    }

    /// Get average delete latency in microseconds
    pub fn avg_delete_latency_us(&self) -> f64 {
        let sum = self.delete_latency_sum_us.load(Ordering::Relaxed) as f64;
        let count = self.delete_latency_count.load(Ordering::Relaxed) as f64;
        if count == 0.0 {
            return 0.0;
        }
        sum / count
    }

    /// Get cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 {
            return 0.0;
        }
        hits / total
    }

    /// Get Bloom Filter hit ratio
    pub fn bloom_hit_ratio(&self) -> f64 {
        let hits = self.bloom_filter_hits.load(Ordering::Relaxed) as f64;
        let misses = self.bloom_filter_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 {
            return 0.0;
        }
        hits / total
    }

    /// Get write amplification factor
    pub fn write_amplification_factor(&self) -> f64 {
        let user = self.user_bytes_written.load(Ordering::Relaxed) as f64;
        let total = self.total_bytes_written.load(Ordering::Relaxed) as f64;
        if user == 0.0 {
            return 1.0;
        }
        total / user
    }

    // ==================== OPT-012: Read Amplification ====================

    /// Record read I/O operations
    pub fn record_read_io(&self, io_ops: u64, bytes: u64) {
        self.read_io_operations.fetch_add(io_ops, Ordering::Relaxed);
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get read amplification factor
    pub fn read_amplification_factor(&self) -> f64 {
        let read_count = self.read_count.load(Ordering::Relaxed) as f64;
        let io_ops = self.read_io_operations.load(Ordering::Relaxed) as f64;
        if read_count == 0.0 {
            return 1.0;
        }
        io_ops / read_count
    }

    // ==================== OPT-012: Space Amplification ====================

    /// Record total size bytes on disk
    pub fn record_total_size(&self, bytes: u64) {
        self.total_size_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get space amplification factor
    pub fn space_amplification_factor(&self) -> f64 {
        let user = self.user_bytes_written.load(Ordering::Relaxed) as f64;
        let total_size = self.total_size_bytes.load(Ordering::Relaxed) as f64;
        if user == 0.0 {
            return 1.0;
        }
        total_size / user
    }

    /// Get total memory usage
    pub fn total_memory_bytes(&self) -> u64 {
        self.memtable_bytes.load(Ordering::Relaxed)
            + self.cache_bytes.load(Ordering::Relaxed)
            + self.bloom_filter_bytes.load(Ordering::Relaxed)
    }

    /// Get all statistics as a snapshot
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            write_count: self.write_count.load(Ordering::Relaxed),
            read_count: self.read_count.load(Ordering::Relaxed),
            delete_count: self.delete_count.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
            read_errors: self.read_errors.load(Ordering::Relaxed),
            delete_errors: self.delete_errors.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            bloom_filter_hits: self.bloom_filter_hits.load(Ordering::Relaxed),
            bloom_filter_misses: self.bloom_filter_misses.load(Ordering::Relaxed),
            compaction_runs: self.compaction_runs.load(Ordering::Relaxed),
            compaction_bytes_written: self.compaction_bytes_written.load(Ordering::Relaxed),
            compaction_segments_merged: self.compaction_segments_merged.load(Ordering::Relaxed),
            compaction_entries_removed: self.compaction_entries_removed.load(Ordering::Relaxed),
            compaction_tombstones_cleaned: self.compaction_tombstones_cleaned.load(Ordering::Relaxed),
            memtable_bytes: self.memtable_bytes.load(Ordering::Relaxed),
            cache_bytes: self.cache_bytes.load(Ordering::Relaxed),
            bloom_filter_bytes: self.bloom_filter_bytes.load(Ordering::Relaxed),
            user_bytes_written: self.user_bytes_written.load(Ordering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(Ordering::Relaxed),
            avg_write_latency_us: self.avg_write_latency_us(),
            avg_read_latency_us: self.avg_read_latency_us(),
            avg_delete_latency_us: self.avg_delete_latency_us(),
            avg_flush_latency_us: self.avg_flush_latency_us(),
            cache_hit_ratio: self.cache_hit_ratio(),
            bloom_hit_ratio: self.bloom_hit_ratio(),
            write_amplification_factor: self.write_amplification_factor(),
            read_amplification_factor: self.read_amplification_factor(),
            space_amplification_factor: self.space_amplification_factor(),
            flush_count: self.flush_count.load(Ordering::Relaxed),
            flush_errors: self.flush_errors.load(Ordering::Relaxed),
        }
    }
}

impl Default for FileKVMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of all metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub write_count: u64,
    pub read_count: u64,
    pub delete_count: u64,
    pub write_errors: u64,
    pub read_errors: u64,
    pub delete_errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub bloom_filter_hits: u64,
    pub bloom_filter_misses: u64,
    pub compaction_runs: u64,
    pub compaction_bytes_written: u64,
    pub compaction_segments_merged: u64,
    pub compaction_entries_removed: u64,
    pub compaction_tombstones_cleaned: u64,
    pub memtable_bytes: u64,
    pub cache_bytes: u64,
    pub bloom_filter_bytes: u64,
    pub user_bytes_written: u64,
    pub total_bytes_written: u64,
    pub avg_write_latency_us: f64,
    pub avg_read_latency_us: f64,
    pub avg_delete_latency_us: f64,
    pub avg_flush_latency_us: f64,
    pub cache_hit_ratio: f64,
    pub bloom_hit_ratio: f64,
    pub write_amplification_factor: f64,
    pub read_amplification_factor: f64,
    pub space_amplification_factor: f64,
    pub flush_count: u64,
    pub flush_errors: u64,
}

impl MetricsSnapshot {
    /// Print formatted statistics
    pub fn print(&self) {
        println!("=== FileKV Metrics Snapshot ===");
        println!();
        println!("Operations:");
        println!("  Writes: {} (errors: {})", self.write_count, self.write_errors);
        println!("  Reads: {} (errors: {})", self.read_count, self.read_errors);
        println!("  Deletes: {} (errors: {})", self.delete_count, self.delete_errors);
        println!();
        println!("Latency (avg):");
        println!("  Write: {:.2} µs", self.avg_write_latency_us);
        println!("  Read: {:.2} µs", self.avg_read_latency_us);
        println!();
        println!("Cache Performance:");
        println!("  Hit Ratio: {:.1}%", self.cache_hit_ratio * 100.0);
        println!("  Hits: {}, Misses: {}", self.cache_hits, self.cache_misses);
        println!();
        println!("Bloom Filter Performance:");
        println!("  Hit Ratio: {:.1}%", self.bloom_hit_ratio * 100.0);
        println!(
            "  Hits: {}, Misses: {}",
            self.bloom_filter_hits, self.bloom_filter_misses
        );
        println!();
        println!("Memory Usage:");
        println!(
            "  MemTable: {} bytes ({:.2} MB)",
            self.memtable_bytes,
            self.memtable_bytes as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Cache: {} bytes ({:.2} MB)",
            self.cache_bytes,
            self.cache_bytes as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Bloom Filter: {} bytes ({:.2} MB)",
            self.bloom_filter_bytes,
            self.bloom_filter_bytes as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Total: {} bytes ({:.2} MB)",
            self.total_memory_bytes(),
            self.total_memory_bytes() as f64 / 1024.0 / 1024.0
        );
        println!();
        println!("Amplification:");
        println!("  Write Amplification Factor: {:.2}x", self.write_amplification_factor);
        println!("  Read Amplification Factor: {:.2}x", self.read_amplification_factor);
        println!("  Space Amplification Factor: {:.2}x", self.space_amplification_factor);
        println!(
            "  User Bytes Written: {} ({:.2} MB)",
            self.user_bytes_written,
            self.user_bytes_written as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Total Bytes Written: {} ({:.2} MB)",
            self.total_bytes_written,
            self.total_bytes_written as f64 / 1024.0 / 1024.0
        );
        println!();
        println!("Compaction:");
        println!("  Runs: {}", self.compaction_runs);
        println!("  Segments Merged: {}", self.compaction_segments_merged);
        println!("  Entries Removed: {}", self.compaction_entries_removed);
        println!("  Tombstones Cleaned: {}", self.compaction_tombstones_cleaned);
        println!(
            "  Bytes Written: {} ({:.2} MB)",
            self.compaction_bytes_written,
            self.compaction_bytes_written as f64 / 1024.0 / 1024.0
        );
        println!("================================");
    }

    fn total_memory_bytes(&self) -> u64 {
        self.memtable_bytes + self.cache_bytes + self.bloom_filter_bytes
    }
}

/// Prometheus metrics exporter
#[cfg(feature = "metrics")]
pub struct PrometheusExporter {
    instance_id: String,
}

#[cfg(feature = "metrics")]
impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new(instance_id: &str) -> Self {
        Self {
            instance_id: instance_id.to_string(),
        }
    }

    /// Register all metrics with Prometheus
    /// NOTE: In metrics 0.23, metrics are auto-registered on first use via macros.
    /// This method is kept for API compatibility - actual registration happens lazily.
    pub fn register(&self) {
        // metrics 0.23 auto-registers on first counter!/gauge!/histogram! call
        // No explicit registration needed
    }

    /// Export current metrics to Prometheus
    pub fn export(&self, snapshot: &MetricsSnapshot) {
        // Operation counters
        counter!("filekv_writes_total", "instance" => self.instance_id.clone()).absolute(snapshot.write_count);
        counter!("filekv_reads_total", "instance" => self.instance_id.clone()).absolute(snapshot.read_count);
        counter!("filekv_deletes_total", "instance" => self.instance_id.clone()).absolute(snapshot.delete_count);
        counter!("filekv_write_errors_total", "instance" => self.instance_id.clone()).absolute(snapshot.write_errors);
        counter!("filekv_read_errors_total", "instance" => self.instance_id.clone()).absolute(snapshot.read_errors);
        counter!("filekv_delete_errors_total", "instance" => self.instance_id.clone()).absolute(snapshot.delete_errors);

        // Cache metrics
        counter!("filekv_cache_hits_total", "instance" => self.instance_id.clone()).absolute(snapshot.cache_hits);
        counter!("filekv_cache_misses_total", "instance" => self.instance_id.clone()).absolute(snapshot.cache_misses);
        counter!("filekv_bloom_hits_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.bloom_filter_hits);
        counter!("filekv_bloom_misses_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.bloom_filter_misses);

        // Compaction metrics
        counter!("filekv_compaction_runs_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.compaction_runs);
        counter!("filekv_compaction_bytes_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.compaction_bytes_written);
        counter!("filekv_compaction_segments_merged_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.compaction_segments_merged);
        counter!("filekv_compaction_entries_removed_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.compaction_entries_removed);
        counter!("filekv_compaction_tombstones_cleaned_total", "instance" => self.instance_id.clone())
            .absolute(snapshot.compaction_tombstones_cleaned);

        // Memory metrics
        gauge!("filekv_memtable_bytes", "instance" => self.instance_id.clone()).set(snapshot.memtable_bytes as f64);
        gauge!("filekv_cache_bytes", "instance" => self.instance_id.clone()).set(snapshot.cache_bytes as f64);
        gauge!("filekv_bloom_filter_bytes", "instance" => self.instance_id.clone())
            .set(snapshot.bloom_filter_bytes as f64);

        // Latency (convert µs to seconds)
        histogram!("filekv_write_latency_seconds", "instance" => self.instance_id.clone())
            .record(snapshot.avg_write_latency_us / 1_000_000.0);
        histogram!("filekv_read_latency_seconds", "instance" => self.instance_id.clone())
            .record(snapshot.avg_read_latency_us / 1_000_000.0);
        histogram!("filekv_delete_latency_seconds", "instance" => self.instance_id.clone())
            .record(snapshot.avg_delete_latency_us / 1_000_000.0);
        histogram!("filekv_flush_latency_seconds", "instance" => self.instance_id.clone())
            .record(snapshot.avg_flush_latency_us / 1_000_000.0);

        // Flush metrics
        counter!("filekv_flush_total", "instance" => self.instance_id.clone()).absolute(snapshot.flush_count);
        counter!("filekv_flush_errors_total", "instance" => self.instance_id.clone()).absolute(snapshot.flush_errors);

        // Amplification metrics
        gauge!("filekv_write_amplification_factor", "instance" => self.instance_id.clone())
            .set(snapshot.write_amplification_factor);
        gauge!("filekv_read_amplification_factor", "instance" => self.instance_id.clone())
            .set(snapshot.read_amplification_factor);
        gauge!("filekv_space_amplification_factor", "instance" => self.instance_id.clone())
            .set(snapshot.space_amplification_factor);
    }
}

/// Metrics timer for automatic latency recording
pub struct MetricsTimer<'a> {
    metrics: &'a FileKVMetrics,
    start: Instant,
    operation: OperationType,
}

enum OperationType {
    Write,
    Read,
    Delete,
    Flush,
}

impl<'a> MetricsTimer<'a> {
    /// Start a new timer for a write operation
    pub fn start_write(metrics: &'a FileKVMetrics) -> Self {
        Self {
            metrics,
            start: Instant::now(),
            operation: OperationType::Write,
        }
    }

    /// Start a new timer for a read operation
    pub fn start_read(metrics: &'a FileKVMetrics) -> Self {
        Self {
            metrics,
            start: Instant::now(),
            operation: OperationType::Read,
        }
    }

    /// Start a new timer for a delete operation
    pub fn start_delete(metrics: &'a FileKVMetrics) -> Self {
        Self {
            metrics,
            start: Instant::now(),
            operation: OperationType::Delete,
        }
    }

    /// Start a new timer for a flush operation
    pub fn start_flush(metrics: &'a FileKVMetrics) -> Self {
        Self {
            metrics,
            start: Instant::now(),
            operation: OperationType::Flush,
        }
    }

    /// Record the operation completion
    pub fn record(self, success: bool) {
        let latency_us = self.start.elapsed().as_secs_f64() * 1_000_000.0;

        match self.operation {
            OperationType::Write => {
                if success {
                    self.metrics.record_write_success(latency_us);
                } else {
                    self.metrics.record_write_error("operation_failed");
                }
            }
            OperationType::Read => {
                if success {
                    self.metrics.record_read_success(latency_us);
                } else {
                    self.metrics.record_read_error("operation_failed");
                }
            }
            OperationType::Delete => {
                if success {
                    self.metrics.record_delete_success(latency_us);
                } else {
                    self.metrics.record_delete_error("operation_failed");
                }
            }
            OperationType::Flush => {
                if success {
                    self.metrics.record_flush_success(latency_us);
                } else {
                    self.metrics.record_flush_error("operation_failed");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = FileKVMetrics::new();

        // Record some operations
        metrics.record_write_success(100.0);
        metrics.record_write_success(150.0);
        metrics.record_read_success(50.0);
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        // Verify counters
        assert_eq!(metrics.write_count.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.read_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.cache_hits.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.cache_misses.load(Ordering::Relaxed), 1);

        // Verify latency
        assert!((metrics.avg_write_latency_us() - 125.0).abs() < 0.01);
        assert!((metrics.avg_read_latency_us() - 50.0).abs() < 0.01);

        // Verify cache hit ratio
        assert!((metrics.cache_hit_ratio() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = FileKVMetrics::new();

        metrics.record_write_success(100.0);
        metrics.record_read_success(50.0);
        metrics.update_memtable_bytes(1024 * 1024); // 1 MB
        metrics.update_cache_bytes(2 * 1024 * 1024); // 2 MB

        let snapshot = metrics.get_snapshot();

        assert_eq!(snapshot.write_count, 1);
        assert_eq!(snapshot.read_count, 1);
        assert_eq!(snapshot.memtable_bytes, 1024 * 1024);
        assert_eq!(snapshot.cache_bytes, 2 * 1024 * 1024);
        assert!((snapshot.avg_write_latency_us - 100.0).abs() < 0.01);
        assert!((snapshot.avg_read_latency_us - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_write_amplification() {
        let metrics = FileKVMetrics::new();

        metrics.record_user_bytes_written(1000);
        metrics.record_total_bytes_written(3000);

        assert!((metrics.write_amplification_factor() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_read_amplification() {
        let metrics = FileKVMetrics::new();

        // Record 10 read operations, each triggering 3 I/O ops
        for _ in 0..10 {
            metrics.record_read_success(50.0);
            metrics.record_read_io(3, 4096);
        }

        // RA = total_io_ops / read_count = 30 / 10 = 3.0
        assert!((metrics.read_amplification_factor() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_space_amplification() {
        let metrics = FileKVMetrics::new();

        metrics.record_user_bytes_written(10000);
        metrics.record_total_size(25000);

        // SA = total_disk_size / user_data = 25000 / 10000 = 2.5
        assert!((metrics.space_amplification_factor() - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_all_amplification_factors() {
        let metrics = FileKVMetrics::new();

        metrics.record_user_bytes_written(1000);
        metrics.record_total_bytes_written(3000);
        metrics.record_total_size(2500);
        metrics.record_read_success(50.0);
        metrics.record_read_success(60.0);
        metrics.record_read_io(5, 8192);
        metrics.record_read_io(5, 8192);

        let snapshot = metrics.get_snapshot();

        assert!((snapshot.write_amplification_factor - 3.0).abs() < 0.01);
        assert!((snapshot.read_amplification_factor - 5.0).abs() < 0.01);
        assert!((snapshot.space_amplification_factor - 2.5).abs() < 0.01);
    }
}
