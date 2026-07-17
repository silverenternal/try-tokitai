//! Prometheus Metrics Export (P2-016)
//!
//! This module provides Prometheus-compatible metrics export for the tokitai-context storage engine.
//! It exposes performance, capacity, and operational metrics for monitoring and alerting.
//!
//! # Metrics Categories
//!
//! - **Write Metrics**: Operations, latency, throughput
//! - **Read Metrics**: Hits, misses, latency
//! - **Storage Metrics**: Segment count, size, utilization
//! - **Memory Metrics**: MemTable size, cache usage
//! - **Compaction Metrics**: Operations, duration, bytes processed
//! - **WAL Metrics**: Writes, rotations, size
//!
//! # Usage
//!
//! ```rust
//! use tokitai_context::metrics::MetricsRegistry;
//!
//! // Create registry
//! let registry = MetricsRegistry::new();
//!
//! // Record operations
//! registry.record_write(1024, Duration::from_micros(50));
//! registry.record_read_hit(Duration::from_micros(5));
//!
//! // Export to Prometheus format
//! let metrics = registry.gather();
//! println!("{}", metrics);
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::fmt;

/// Metric types supported by the registry
#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// A single metric value
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

impl fmt::Display for MetricValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write metric help
        writeln!(f, "# HELP {} {}", self.name, self.help)?;
        
        // Write metric type
        let type_str = match self.metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
        };
        writeln!(f, "# TYPE {} {}", self.name, type_str)?;
        
        // Write metric value with labels
        if !self.labels.is_empty() {
            let labels: Vec<String> = self.labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            writeln!(f, "{}{{{}}} {}", self.name, labels.join(","), self.value)?;
        } else {
            writeln!(f, "{} {}", self.name, self.value)?;
        }
        
        Ok(())
    }
}

/// Write operation metrics
pub struct WriteMetrics {
    /// Total write operations
    pub write_count: AtomicU64,
    /// Total bytes written
    pub write_bytes: AtomicU64,
    /// Total write latency in microseconds
    pub write_latency_us: AtomicU64,
    /// Write errors
    pub write_errors: AtomicU64,
}

impl WriteMetrics {
    fn new() -> Self {
        Self {
            write_count: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            write_latency_us: AtomicU64::new(0),
            write_errors: AtomicU64::new(0),
        }
    }
    
    /// Record a write operation
    pub fn record(&self, bytes: usize, latency: Duration) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.write_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        self.write_latency_us.fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
    }
    
    /// Record a write error
    pub fn record_error(&self) {
        self.write_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get average write latency in microseconds
    pub fn avg_latency_us(&self) -> f64 {
        let count = self.write_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total_latency = self.write_latency_us.load(Ordering::Relaxed);
        total_latency as f64 / count as f64
    }
}

/// Read operation metrics
pub struct ReadMetrics {
    /// Total read operations
    pub read_count: AtomicU64,
    /// Cache hits
    pub cache_hits: AtomicU64,
    /// Cache misses
    pub cache_misses: AtomicU64,
    /// Total read latency in microseconds
    pub read_latency_us: AtomicU64,
    /// Read errors
    pub read_errors: AtomicU64,
}

impl ReadMetrics {
    fn new() -> Self {
        Self {
            read_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            read_latency_us: AtomicU64::new(0),
            read_errors: AtomicU64::new(0),
        }
    }
    
    /// Record a cache hit
    pub fn record_hit(&self, latency: Duration) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
        self.read_latency_us.fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
    }
    
    /// Record a cache miss
    pub fn record_miss(&self, latency: Duration) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        self.read_latency_us.fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
    }
    
    /// Record a read error
    pub fn record_error(&self) {
        self.read_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get cache hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }
        hits as f64 / total as f64
    }
    
    /// Get average read latency in microseconds
    pub fn avg_latency_us(&self) -> f64 {
        let count = self.read_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total_latency = self.read_latency_us.load(Ordering::Relaxed);
        total_latency as f64 / count as f64
    }
}

/// Storage metrics
pub struct StorageMetrics {
    /// Current segment count
    pub segment_count: AtomicU64,
    /// Total storage size in bytes
    pub total_size_bytes: AtomicU64,
    /// Total entries stored
    pub total_entries: AtomicU64,
    /// Compaction count
    pub compaction_count: AtomicU64,
}

impl StorageMetrics {
    fn new() -> Self {
        Self {
            segment_count: AtomicU64::new(0),
            total_size_bytes: AtomicU64::new(0),
            total_entries: AtomicU64::new(0),
            compaction_count: AtomicU64::new(0),
        }
    }
    
    /// Update segment count
    pub fn set_segment_count(&self, count: u64) {
        self.segment_count.store(count, Ordering::Relaxed);
    }
    
    /// Update total size
    pub fn set_total_size(&self, size: u64) {
        self.total_size_bytes.store(size, Ordering::Relaxed);
    }
    
    /// Update total entries
    pub fn set_total_entries(&self, entries: u64) {
        self.total_entries.store(entries, Ordering::Relaxed);
    }
    
    /// Record a compaction
    pub fn record_compaction(&self) {
        self.compaction_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Memory metrics
pub struct MemoryMetrics {
    /// MemTable size in bytes
    pub memtable_size_bytes: AtomicU64,
    /// MemTable entries
    pub memtable_entries: AtomicU64,
    /// Block cache size in bytes
    pub cache_size_bytes: AtomicU64,
    /// Block cache items
    pub cache_items: AtomicU64,
}

impl MemoryMetrics {
    fn new() -> Self {
        Self {
            memtable_size_bytes: AtomicU64::new(0),
            memtable_entries: AtomicU64::new(0),
            cache_size_bytes: AtomicU64::new(0),
            cache_items: AtomicU64::new(0),
        }
    }
    
    /// Update MemTable metrics
    pub fn set_memtable(&self, size: u64, entries: u64) {
        self.memtable_size_bytes.store(size, Ordering::Relaxed);
        self.memtable_entries.store(entries, Ordering::Relaxed);
    }
    
    /// Update cache metrics
    pub fn set_cache(&self, size: u64, items: u64) {
        self.cache_size_bytes.store(size, Ordering::Relaxed);
        self.cache_items.store(items, Ordering::Relaxed);
    }
}

/// WAL metrics
pub struct WalMetrics {
    /// WAL writes
    pub wal_writes: AtomicU64,
    /// WAL bytes written
    pub wal_bytes: AtomicU64,
    /// WAL rotations
    pub wal_rotations: AtomicU64,
    /// Current WAL size
    pub wal_size_bytes: AtomicU64,
}

impl WalMetrics {
    fn new() -> Self {
        Self {
            wal_writes: AtomicU64::new(0),
            wal_bytes: AtomicU64::new(0),
            wal_rotations: AtomicU64::new(0),
            wal_size_bytes: AtomicU64::new(0),
        }
    }
    
    /// Record a WAL write
    pub fn record_write(&self, bytes: usize) {
        self.wal_writes.fetch_add(1, Ordering::Relaxed);
        self.wal_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    /// Record a WAL rotation
    pub fn record_rotation(&self) {
        self.wal_rotations.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Update current WAL size
    pub fn set_size(&self, size: u64) {
        self.wal_size_bytes.store(size, Ordering::Relaxed);
    }
}

/// Prometheus metrics registry
pub struct MetricsRegistry {
    write: WriteMetrics,
    read: ReadMetrics,
    storage: StorageMetrics,
    memory: MemoryMetrics,
    wal: WalMetrics,
    /// Start time for uptime calculation
    start_time: Instant,
    /// Custom metrics
    custom_metrics: parking_lot::Mutex<HashMap<String, MetricValue>>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self {
            write: WriteMetrics::new(),
            read: ReadMetrics::new(),
            storage: StorageMetrics::new(),
            memory: MemoryMetrics::new(),
            wal: WalMetrics::new(),
            start_time: Instant::now(),
            custom_metrics: parking_lot::Mutex::new(HashMap::new()),
        }
    }
    
    /// Record a write operation
    pub fn record_write(&self, bytes: usize, latency: Duration) {
        self.write.record(bytes, latency);
    }
    
    /// Record a write error
    pub fn record_write_error(&self) {
        self.write.record_error();
    }
    
    /// Record a read cache hit
    pub fn record_read_hit(&self, latency: Duration) {
        self.read.record_hit(latency);
    }
    
    /// Record a read cache miss
    pub fn record_read_miss(&self, latency: Duration) {
        self.read.record_miss(latency);
    }
    
    /// Record a read error
    pub fn record_read_error(&self) {
        self.read.record_error();
    }
    
    /// Update storage metrics
    pub fn set_storage(&self, segments: u64, size: u64, entries: u64) {
        self.storage.set_segment_count(segments);
        self.storage.set_total_size(size);
        self.storage.set_total_entries(entries);
    }
    
    /// Record a compaction
    pub fn record_compaction(&self) {
        self.storage.record_compaction();
    }
    
    /// Update memory metrics
    pub fn set_memory(&self, memtable_size: u64, memtable_entries: u64, cache_size: u64, cache_items: u64) {
        self.memory.set_memtable(memtable_size, memtable_entries);
        self.memory.set_cache(cache_size, cache_items);
    }
    
    /// Record a WAL write
    pub fn record_wal_write(&self, bytes: usize) {
        self.wal.record_write(bytes);
    }
    
    /// Record a WAL rotation
    pub fn record_wal_rotation(&self) {
        self.wal.record_rotation();
    }
    
    /// Update WAL size
    pub fn set_wal_size(&self, size: u64) {
        self.wal.set_size(size);
    }
    
    /// Register a custom metric
    pub fn register_metric(&self, metric: MetricValue) {
        let mut metrics = self.custom_metrics.lock();
        metrics.insert(metric.name.clone(), metric);
    }
    
    /// Get write metrics
    pub fn write_metrics(&self) -> &WriteMetrics {
        &self.write
    }
    
    /// Get read metrics
    pub fn read_metrics(&self) -> &ReadMetrics {
        &self.read
    }
    
    /// Get storage metrics
    pub fn storage_metrics(&self) -> &StorageMetrics {
        &self.storage
    }
    
    /// Get memory metrics
    pub fn memory_metrics(&self) -> &MemoryMetrics {
        &self.memory
    }
    
    /// Get WAL metrics
    pub fn wal_metrics(&self) -> &WalMetrics {
        &self.wal
    }
    
    /// Gather all metrics in Prometheus format
    pub fn gather(&self) -> String {
        let mut output = String::new();
        
        // Uptime
        let uptime_secs = self.start_time.elapsed().as_secs();
        output.push_str(&format!(
            "# HELP tokitai_uptime_seconds Uptime in seconds\n\
             # TYPE tokitai_uptime_seconds gauge\n\
             tokitai_uptime_seconds {}\n\n",
            uptime_secs
        ));
        
        // Write metrics
        output.push_str(&format!(
            "# HELP tokitai_write_count Total write operations\n\
             # TYPE tokitai_write_count counter\n\
             tokitai_write_count {}\n\n",
            self.write.write_count.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_write_bytes_total Total bytes written\n\
             # TYPE tokitai_write_bytes_total counter\n\
             tokitai_write_bytes_total {}\n\n",
            self.write.write_bytes.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_write_latency_us_avg Average write latency (microseconds)\n\
             # TYPE tokitai_write_latency_us_avg gauge\n\
             tokitai_write_latency_us_avg {:.2}\n\n",
            self.write.avg_latency_us()
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_write_errors_total Total write errors\n\
             # TYPE tokitai_write_errors_total counter\n\
             tokitai_write_errors_total {}\n\n",
            self.write.write_errors.load(Ordering::Relaxed)
        ));
        
        // Read metrics
        output.push_str(&format!(
            "# HELP tokitai_read_count Total read operations\n\
             # TYPE tokitai_read_count counter\n\
             tokitai_read_count {}\n\n",
            self.read.read_count.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_cache_hit_rate Cache hit rate (0.0-1.0)\n\
             # TYPE tokitai_cache_hit_rate gauge\n\
             tokitai_cache_hit_rate {:.4}\n\n",
            self.read.hit_rate()
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_read_latency_us_avg Average read latency (microseconds)\n\
             # TYPE tokitai_read_latency_us_avg gauge\n\
             tokitai_read_latency_us_avg {:.2}\n\n",
            self.read.avg_latency_us()
        ));
        
        // Storage metrics
        output.push_str(&format!(
            "# HELP tokitai_segment_count Current segment count\n\
             # TYPE tokitai_segment_count gauge\n\
             tokitai_segment_count {}\n\n",
            self.storage.segment_count.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_storage_size_bytes Total storage size in bytes\n\
             # TYPE tokitai_storage_size_bytes gauge\n\
             tokitai_storage_size_bytes {}\n\n",
            self.storage.total_size_bytes.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_total_entries Total entries stored\n\
             # TYPE tokitai_total_entries gauge\n\
             tokitai_total_entries {}\n\n",
            self.storage.total_entries.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_compaction_count Total compactions performed\n\
             # TYPE tokitai_compaction_count counter\n\
             tokitai_compaction_count {}\n\n",
            self.storage.compaction_count.load(Ordering::Relaxed)
        ));
        
        // Memory metrics
        output.push_str(&format!(
            "# HELP tokitai_memtable_size_bytes Current MemTable size in bytes\n\
             # TYPE tokitai_memtable_size_bytes gauge\n\
             tokitai_memtable_size_bytes {}\n\n",
            self.memory.memtable_size_bytes.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_memtable_entries Current MemTable entries\n\
             # TYPE tokitai_memtable_entries gauge\n\
             tokitai_memtable_entries {}\n\n",
            self.memory.memtable_entries.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_cache_size_bytes Current cache size in bytes\n\
             # TYPE tokitai_cache_size_bytes gauge\n\
             tokitai_cache_size_bytes {}\n\n",
            self.memory.cache_size_bytes.load(Ordering::Relaxed)
        ));
        
        // WAL metrics
        output.push_str(&format!(
            "# HELP tokitai_wal_writes_total Total WAL writes\n\
             # TYPE tokitai_wal_writes_total counter\n\
             tokitai_wal_writes_total {}\n\n",
            self.wal.wal_writes.load(Ordering::Relaxed)
        ));
        
        output.push_str(&format!(
            "# HELP tokitai_wal_rotations_total Total WAL rotations\n\
             # TYPE tokitai_wal_rotations_total counter\n\
             tokitai_wal_rotations_total {}\n\n",
            self.wal.wal_rotations.load(Ordering::Relaxed)
        ));
        
        // Custom metrics
        let custom = self.custom_metrics.lock();
        for metric in custom.values() {
            output.push_str(&metric.to_string());
            output.push('\n');
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 保留：验证写指标记录逻辑
    #[test]
    fn test_write_metrics() {
        let registry = MetricsRegistry::new();

        registry.record_write(1024, Duration::from_micros(50));
        registry.record_write(2048, Duration::from_micros(100));

        assert_eq!(registry.write.write_count.load(Ordering::Relaxed), 2);
        assert_eq!(registry.write.write_bytes.load(Ordering::Relaxed), 3072);
        assert!((registry.write.avg_latency_us() - 75.0).abs() < 0.1);
    }

    /// 保留：验证读指标和缓存命中率计算
    #[test]
    fn test_read_metrics() {
        let registry = MetricsRegistry::new();

        registry.record_read_hit(Duration::from_micros(5));
        registry.record_read_hit(Duration::from_micros(5));
        registry.record_read_hit(Duration::from_micros(5));
        registry.record_read_miss(Duration::from_micros(100));

        assert_eq!(registry.read.cache_hits.load(Ordering::Relaxed), 3);
        assert_eq!(registry.read.cache_misses.load(Ordering::Relaxed), 1);
        assert!((registry.read.hit_rate() - 0.75).abs() < 0.01);
    }

    /// 保留：验证存储指标和 compaction 计数
    #[test]
    fn test_storage_metrics() {
        let registry = MetricsRegistry::new();

        registry.set_storage(10, 1024 * 1024, 1000);

        assert_eq!(registry.storage.segment_count.load(Ordering::Relaxed), 10);
        assert_eq!(registry.storage.total_size_bytes.load(Ordering::Relaxed), 1024 * 1024);
        assert_eq!(registry.storage.total_entries.load(Ordering::Relaxed), 1000);

        registry.record_compaction();
        assert_eq!(registry.storage.compaction_count.load(Ordering::Relaxed), 1);
    }

    /// 保留：验证 WAL 指标
    #[test]
    fn test_wal_metrics() {
        let registry = MetricsRegistry::new();

        registry.record_wal_write(100);
        registry.record_wal_write(200);
        registry.record_wal_rotation();

        assert_eq!(registry.wal.wal_writes.load(Ordering::Relaxed), 2);
        assert_eq!(registry.wal.wal_bytes.load(Ordering::Relaxed), 300);
        assert_eq!(registry.wal.wal_rotations.load(Ordering::Relaxed), 1);
    }

    /// 保留：验证指标收集输出
    #[test]
    fn test_gather_metrics() {
        let registry = MetricsRegistry::new();

        registry.record_write(1024, Duration::from_micros(50));
        registry.record_read_hit(Duration::from_micros(5));

        let output = registry.gather();

        assert!(output.contains("tokitai_write_count"));
        assert!(output.contains("tokitai_read_count"));
        assert!(output.contains("tokitai_cache_hit_rate"));
        assert!(output.contains("tokitai_uptime_seconds"));
    }
}
