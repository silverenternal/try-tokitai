//! Prometheus Metrics Exporter
//!
//! P2-016: Export performance metrics to Prometheus for monitoring:
//! - Operation latencies (histograms)
//! - Operation counts (counters)
//! - Resource usage (gauges)
//! - Cache statistics
//! - Compaction metrics
//!
//! Usage:
//! ```rust
//! let exporter = PrometheusExporter::new("tokitai", "0.1.0");
//! exporter.register();
//! // Metrics available at /metrics endpoint
//! ```

use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use tracing::{info, warn};

#[cfg(feature = "metrics")]
use prometheus::{
    Registry, Counter, Gauge, Histogram, HistogramOpts, Opts, TextEncoder,
    IntCounterVec, IntGaugeVec, HistogramVec,
};

/// Prometheus metrics configuration
#[derive(Debug, Clone)]
pub struct PrometheusConfig {
    /// Metrics namespace (e.g., "tokitai")
    pub namespace: String,
    /// Subsystem name (e.g., "storage", "context")
    pub subsystem: String,
    /// Version label
    pub version: String,
    /// HTTP bind address for metrics endpoint
    pub bind_address: String,
    /// Enable automatic metrics collection
    pub auto_collect: bool,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            namespace: "tokitai".to_string(),
            subsystem: "storage".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            bind_address: "0.0.0.0:9090".to_string(),
            auto_collect: true,
        }
    }
}

/// Metrics collector for FileKV operations
#[cfg(feature = "metrics")]
pub struct PrometheusExporter {
    registry: Registry,
    config: PrometheusConfig,
    
    // Operation counters
    read_count: IntCounterVec,
    write_count: IntCounterVec,
    delete_count: IntCounterVec,
    hit_count: IntCounterVec,
    miss_count: IntCounterVec,
    
    // Operation latency histograms (in microseconds)
    read_latency: HistogramVec,
    write_latency: HistogramVec,
    delete_latency: HistogramVec,
    
    // Resource gauges
    memtable_size: IntGaugeVec,
    segment_count: IntGaugeVec,
    cache_size: IntGaugeVec,
    bloom_filter_count: IntGaugeVec,
    
    // Compaction metrics
    compaction_count: IntCounterVec,
    compaction_duration: HistogramVec,
    bytes_compacted: IntCounterVec,
    
    // WAL metrics
    wal_bytes_written: IntCounterVec,
    wal_rotation_count: IntCounterVec,
    
    // Error counters
    error_count: IntCounterVec,
    timeout_count: IntCounterVec,
    
    // Cache metrics
    cache_hits: IntCounterVec,
    cache_misses: IntCounterVec,
    cache_evictions: IntCounterVec,
    
    // Bloom filter metrics
    bloom_filter_checks: IntCounterVec,
    bloom_filter_false_positives: IntCounterVec,
    
    // Write coalescing metrics
    write_coalesced_count: IntCounterVec,
    write_coalesced_batches: IntCounterVec,
}

#[cfg(feature = "metrics")]
impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new(namespace: &str, version: &str) -> Self {
        Self::with_config(PrometheusConfig {
            namespace: namespace.to_string(),
            version: version.to_string(),
            ..Default::default()
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: PrometheusConfig) -> Self {
        let registry = Registry::new();

        // Operation counters
        let read_count = IntCounterVec::new(
            Opts::new("read_total", "Total number of read operations"),
            &["type", "status"],
        ).unwrap();
        registry.register(Box::new(read_count.clone())).unwrap();

        let write_count = IntCounterVec::new(
            Opts::new("write_total", "Total number of write operations"),
            &["type", "status"],
        ).unwrap();
        registry.register(Box::new(write_count.clone())).unwrap();

        let delete_count = IntCounterVec::new(
            Opts::new("delete_total", "Total number of delete operations"),
            &["status"],
        ).unwrap();
        registry.register(Box::new(delete_count.clone())).unwrap();

        let hit_count = IntCounterVec::new(
            Opts::new("hit_total", "Total cache hit count"),
            &["cache_type"],
        ).unwrap();
        registry.register(Box::new(hit_count.clone())).unwrap();

        let miss_count = IntCounterVec::new(
            Opts::new("miss_total", "Total cache miss count"),
            &["cache_type"],
        ).unwrap();
        registry.register(Box::new(miss_count.clone())).unwrap();

        // Latency histograms (microseconds)
        let latency_buckets = vec![
            0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0,
            1000.0, 2500.0, 5000.0, 10000.0,
        ];

        let read_latency = HistogramVec::new(
            HistogramOpts::new("read_latency_us", "Read operation latency in microseconds")
                .buckets(latency_buckets.clone()),
            &["cache_status"],
        ).unwrap();
        registry.register(Box::new(read_latency.clone())).unwrap();

        let write_latency = HistogramVec::new(
            HistogramOpts::new("write_latency_us", "Write operation latency in microseconds")
                .buckets(latency_buckets.clone()),
            &["batch_size"],
        ).unwrap();
        registry.register(Box::new(write_latency.clone())).unwrap();

        let delete_latency = HistogramVec::new(
            HistogramOpts::new("delete_latency_us", "Delete operation latency in microseconds")
                .buckets(latency_buckets),
            &[],
        ).unwrap();
        registry.register(Box::new(delete_latency.clone())).unwrap();

        // Resource gauges
        let memtable_size = IntGaugeVec::new(
            Opts::new("memtable_size_bytes", "Current MemTable size in bytes"),
            &[],
        ).unwrap();
        registry.register(Box::new(memtable_size.clone())).unwrap();

        let segment_count = IntGaugeVec::new(
            Opts::new("segment_count", "Number of active segments"),
            &[],
        ).unwrap();
        registry.register(Box::new(segment_count.clone())).unwrap();

        let cache_size = IntGaugeVec::new(
            Opts::new("cache_size_bytes", "Current cache size in bytes"),
            &["cache_type"],
        ).unwrap();
        registry.register(Box::new(cache_size.clone())).unwrap();

        let bloom_filter_count = IntGaugeVec::new(
            Opts::new("bloom_filter_count", "Number of Bloom filters in memory"),
            &[],
        ).unwrap();
        registry.register(Box::new(bloom_filter_count.clone())).unwrap();

        // Compaction metrics
        let compaction_count = IntCounterVec::new(
            Opts::new("compaction_total", "Total number of compactions"),
            &["strategy", "reason"],
        ).unwrap();
        registry.register(Box::new(compaction_count.clone())).unwrap();

        let compaction_duration = HistogramVec::new(
            HistogramOpts::new("compaction_duration_ms", "Compaction duration in milliseconds")
                .buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0, 30000.0]),
            &["strategy"],
        ).unwrap();
        registry.register(Box::new(compaction_duration.clone())).unwrap();

        let bytes_compacted = IntCounterVec::new(
            Opts::new("compacted_bytes_total", "Total bytes compacted"),
            &["strategy"],
        ).unwrap();
        registry.register(Box::new(bytes_compacted.clone())).unwrap();

        // WAL metrics
        let wal_bytes_written = IntCounterVec::new(
            Opts::new("wal_bytes_written_total", "Total WAL bytes written"),
            &[],
        ).unwrap();
        registry.register(Box::new(wal_bytes_written.clone())).unwrap();

        let wal_rotation_count = IntCounterVec::new(
            Opts::new("wal_rotation_total", "Total WAL rotations"),
            &[],
        ).unwrap();
        registry.register(Box::new(wal_rotation_count.clone())).unwrap();

        // Error counters
        let error_count = IntCounterVec::new(
            Opts::new("error_total", "Total error count"),
            &["operation", "error_type"],
        ).unwrap();
        registry.register(Box::new(error_count.clone())).unwrap();

        let timeout_count = IntCounterVec::new(
            Opts::new("timeout_total", "Total timeout count"),
            &["operation"],
        ).unwrap();
        registry.register(Box::new(timeout_count.clone())).unwrap();

        // Cache metrics
        let cache_hits = IntCounterVec::new(
            Opts::new("cache_hits_total", "Total cache hits"),
            &["cache_type"],
        ).unwrap();
        registry.register(Box::new(cache_hits.clone())).unwrap();

        let cache_misses = IntCounterVec::new(
            Opts::new("cache_misses_total", "Total cache misses"),
            &["cache_type"],
        ).unwrap();
        registry.register(Box::new(cache_misses.clone())).unwrap();

        let cache_evictions = IntCounterVec::new(
            Opts::new("cache_evictions_total", "Total cache evictions"),
            &["cache_type"],
        ).unwrap();
        registry.register(Box::new(cache_evictions.clone())).unwrap();

        // Bloom filter metrics
        let bloom_filter_checks = IntCounterVec::new(
            Opts::new("bloom_filter_checks_total", "Total Bloom filter checks"),
            &[],
        ).unwrap();
        registry.register(Box::new(bloom_filter_checks.clone())).unwrap();

        let bloom_filter_false_positives = IntCounterVec::new(
            Opts::new("bloom_filter_false_positives_total", "Total Bloom filter false positives"),
            &[],
        ).unwrap();
        registry.register(Box::new(bloom_filter_false_positives.clone())).unwrap();

        // Write coalescing metrics
        let write_coalesced_count = IntCounterVec::new(
            Opts::new("write_coalesced_total", "Total coalesced writes"),
            &[],
        ).unwrap();
        registry.register(Box::new(write_coalesced_count.clone())).unwrap();

        let write_coalesced_batches = IntCounterVec::new(
            Opts::new("write_coalesced_batches_total", "Total coalesced write batches"),
            &[],
        ).unwrap();
        registry.register(Box::new(write_coalesced_batches.clone())).unwrap();

        Self {
            registry,
            config,
            read_count,
            write_count,
            delete_count,
            hit_count,
            miss_count,
            read_latency,
            write_latency,
            delete_latency,
            memtable_size,
            segment_count,
            cache_size,
            bloom_filter_count,
            compaction_count,
            compaction_duration,
            bytes_compacted,
            wal_bytes_written,
            wal_rotation_count,
            error_count,
            timeout_count,
            cache_hits,
            cache_misses,
            cache_evictions,
            bloom_filter_checks,
            bloom_filter_false_positives,
            write_coalesced_count,
            write_coalesced_batches,
        }
    }

    /// Register the exporter (for global metrics)
    pub fn register(&self) {
        info!(
            namespace = %self.config.namespace,
            subsystem = %self.config.subsystem,
            "Prometheus exporter registered"
        );
    }

    /// Get the metrics registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Encode metrics to Prometheus text format
    pub fn encode(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = String::new();
        encoder.encode_utf8(&metric_families, &mut buffer)?;
        Ok(buffer)
    }

    // === Counter Methods ===

    /// Record a read operation
    pub fn record_read(&self, status: &str) {
        self.read_count
            .with_label_values(&["sync", status])
            .inc();
    }

    /// Record a write operation
    pub fn record_write(&self, status: &str) {
        self.write_count
            .with_label_values(&["sync", status])
            .inc();
    }

    /// Record a delete operation
    pub fn record_delete(&self, status: &str) {
        self.delete_count
            .with_label_values(&[status])
            .inc();
    }

    /// Record cache hit
    pub fn record_cache_hit(&self, cache_type: &str) {
        self.hit_count
            .with_label_values(&[cache_type])
            .inc();
        self.cache_hits
            .with_label_values(&[cache_type])
            .inc();
    }

    /// Record cache miss
    pub fn record_cache_miss(&self, cache_type: &str) {
        self.miss_count
            .with_label_values(&[cache_type])
            .inc();
        self.cache_misses
            .with_label_values(&[cache_type])
            .inc();
    }

    // === Latency Methods ===

    /// Record read latency (in microseconds)
    pub fn record_read_latency(&self, latency_us: f64, cache_status: &str) {
        self.read_latency
            .with_label_values(&[cache_status])
            .observe(latency_us);
    }

    /// Record write latency (in microseconds)
    pub fn record_write_latency(&self, latency_us: f64, batch_size: &str) {
        self.write_latency
            .with_label_values(&[batch_size])
            .observe(latency_us);
    }

    /// Record delete latency (in microseconds)
    pub fn record_delete_latency(&self, latency_us: f64) {
        self.delete_latency
            .with_label_values(&[])
            .observe(latency_us);
    }

    // === Gauge Methods ===

    /// Update MemTable size
    pub fn set_memtable_size(&self, size_bytes: i64) {
        self.memtable_size
            .with_label_values(&[])
            .set(size_bytes);
    }

    /// Update segment count
    pub fn set_segment_count(&self, count: i64) {
        self.segment_count
            .with_label_values(&[])
            .set(count);
    }

    /// Update cache size
    pub fn set_cache_size(&self, cache_type: &str, size_bytes: i64) {
        self.cache_size
            .with_label_values(&[cache_type])
            .set(size_bytes);
    }

    /// Update Bloom filter count
    pub fn set_bloom_filter_count(&self, count: i64) {
        self.bloom_filter_count
            .with_label_values(&[])
            .set(count);
    }

    // === Compaction Methods ===

    /// Record a compaction event
    pub fn record_compaction(&self, strategy: &str, reason: &str) {
        self.compaction_count
            .with_label_values(&[strategy, reason])
            .inc();
    }

    /// Record compaction duration (in milliseconds)
    pub fn record_compaction_duration(&self, strategy: &str, duration_ms: f64) {
        self.compaction_duration
            .with_label_values(&[strategy])
            .observe(duration_ms);
    }

    /// Record bytes compacted
    pub fn record_bytes_compacted(&self, strategy: &str, bytes: i64) {
        if bytes > 0 {
            self.bytes_compacted
                .with_label_values(&[strategy])
                .inc_by(bytes as u64);
        }
    }

    // === WAL Methods ===

    /// Record WAL bytes written
    pub fn record_wal_bytes_written(&self, bytes: i64) {
        if bytes > 0 {
            self.wal_bytes_written
                .with_label_values(&[])
                .inc_by(bytes as u64);
        }
    }

    /// Record WAL rotation
    pub fn record_wal_rotation(&self) {
        self.wal_rotation_count
            .with_label_values(&[])
            .inc();
    }

    // === Error Methods ===

    /// Record an error
    pub fn record_error(&self, operation: &str, error_type: &str) {
        self.error_count
            .with_label_values(&[operation, error_type])
            .inc();
    }

    /// Record a timeout
    pub fn record_timeout(&self, operation: &str) {
        self.timeout_count
            .with_label_values(&[operation])
            .inc();
    }

    // === Cache Methods ===

    /// Record cache eviction
    pub fn record_cache_eviction(&self, cache_type: &str) {
        self.cache_evictions
            .with_label_values(&[cache_type])
            .inc();
    }

    // === Bloom Filter Methods ===

    /// Record Bloom filter check
    pub fn record_bloom_check(&self) {
        self.bloom_filter_checks
            .with_label_values(&[])
            .inc();
    }

    /// Record Bloom filter false positive
    pub fn record_bloom_false_positive(&self) {
        self.bloom_filter_false_positives
            .with_label_values(&[])
            .inc();
    }

    // === Write Coalescing Methods ===

    /// Record coalesced write
    pub fn record_write_coalesced(&self, count: i64) {
        if count > 0 {
            self.write_coalesced_count
                .with_label_values(&[])
                .inc_by(count as u64);
        }
    }

    /// Record coalesced batch
    pub fn record_write_coalesced_batch(&self) {
        self.write_coalesced_batches
            .with_label_values(&[])
            .inc();
    }

    /// Get a summary of all metrics as text
    pub fn gather_text(&self) -> String {
        self.encode().unwrap_or_else(|e| {
            tracing::error!("Failed to encode metrics: {}", e);
            String::new()
        })
    }
}

/// Stub implementation when metrics feature is disabled
#[cfg(not(feature = "metrics"))]
pub struct PrometheusExporter {
    _private: (),
}

#[cfg(not(feature = "metrics"))]
impl PrometheusExporter {
    pub fn new(_namespace: &str, _version: &str) -> Self {
        Self { _private: () }
    }

    pub fn with_config(_config: PrometheusConfig) -> Self {
        Self { _private: () }
    }

    pub fn register(&self) {}
    pub fn registry(&self) -> &() { &() }
    pub fn encode(&self) -> String { String::new() }

    pub fn record_read(&self, _status: &str) {}
    pub fn record_write(&self, _status: &str) {}
    pub fn record_delete(&self, _status: &str) {}
    pub fn record_cache_hit(&self, _cache_type: &str) {}
    pub fn record_cache_miss(&self, _cache_type: &str) {}
    pub fn record_read_latency(&self, _latency_us: f64, _cache_status: &str) {}
    pub fn record_write_latency(&self, _latency_us: f64, _batch_size: &str) {}
    pub fn record_delete_latency(&self, _latency_us: f64) {}
    pub fn set_memtable_size(&self, _size_bytes: i64) {}
    pub fn set_segment_count(&self, _count: i64) {}
    pub fn set_cache_size(&self, _cache_type: &str, _size_bytes: i64) {}
    pub fn set_bloom_filter_count(&self, _count: i64) {}
    pub fn record_compaction(&self, _strategy: &str, _reason: &str) {}
    pub fn record_compaction_duration(&self, _strategy: &str, _duration_ms: f64) {}
    pub fn record_bytes_compacted(&self, _strategy: &str, _bytes: i64) {}
    pub fn record_wal_bytes_written(&self, _bytes: i64) {}
    pub fn record_wal_rotation(&self) {}
    pub fn record_error(&self, _operation: &str, _error_type: &str) {}
    pub fn record_timeout(&self, _operation: &str) {}
    pub fn record_cache_eviction(&self, _cache_type: &str) {}
    pub fn record_bloom_check(&self) {}
    pub fn record_bloom_false_positive(&self) {}
    pub fn record_write_coalesced(&self, _count: i64) {}
    pub fn record_write_coalesced_batch(&self) {}
    pub fn gather_text(&self) -> String { String::new() }
}

/// Metrics collector wrapper for FileKV
pub struct FileKVMetrics {
    exporter: Arc<PrometheusExporter>,
}

impl FileKVMetrics {
    pub fn new(exporter: Arc<PrometheusExporter>) -> Self {
        Self { exporter }
    }

    /// Record a successful read
    pub fn record_read_success(&self, latency_us: f64, cache_hit: bool) {
        let cache_status = if cache_hit { "hit" } else { "miss" };
        self.exporter.record_read("success");
        self.exporter.record_read_latency(latency_us, cache_status);
        if cache_hit {
            self.exporter.record_cache_hit("block");
        } else {
            self.exporter.record_cache_miss("block");
        }
    }

    /// Record a failed read
    pub fn record_read_error(&self, error_type: &str) {
        self.exporter.record_read("error");
        self.exporter.record_error("read", error_type);
    }

    /// Record a successful write
    pub fn record_write_success(&self, latency_us: f64, is_batch: bool) {
        let batch_label = if is_batch { "batch" } else { "single" };
        self.exporter.record_write("success");
        self.exporter.record_write_latency(latency_us, batch_label);
    }

    /// Record a failed write
    pub fn record_write_error(&self, error_type: &str) {
        self.exporter.record_write("error");
        self.exporter.record_error("write", error_type);
    }

    /// Record a successful delete
    pub fn record_delete_success(&self, latency_us: f64) {
        self.exporter.record_delete("success");
        self.exporter.record_delete_latency(latency_us);
    }

    /// Record a failed delete
    pub fn record_delete_error(&self, error_type: &str) {
        self.exporter.record_delete("error");
        self.exporter.record_error("delete", error_type);
    }

    /// Update resource metrics
    pub fn update_resources(&self, memtable_size: i64, segment_count: i64, cache_size: i64, bloom_count: i64) {
        self.exporter.set_memtable_size(memtable_size);
        self.exporter.set_segment_count(segment_count);
        self.exporter.set_cache_size("block", cache_size);
        self.exporter.set_bloom_filter_count(bloom_count);
    }

    /// Record compaction event
    pub fn record_compaction(&self, strategy: &str, reason: &str, duration_ms: f64, bytes: i64) {
        self.exporter.record_compaction(strategy, reason);
        self.exporter.record_compaction_duration(strategy, duration_ms);
        self.exporter.record_bytes_compacted(strategy, bytes);
    }

    /// Record WAL write
    pub fn record_wal_write(&self, bytes: i64) {
        self.exporter.record_wal_bytes_written(bytes);
    }

    /// Record WAL rotation
    pub fn record_wal_rotation(&self) {
        self.exporter.record_wal_rotation();
    }

    /// Record Bloom filter check
    pub fn record_bloom_check(&self, is_false_positive: bool) {
        self.exporter.record_bloom_check();
        if is_false_positive {
            self.exporter.record_bloom_false_positive();
        }
    }

    /// Record coalesced writes
    pub fn record_coalesced_writes(&self, count: usize) {
        if count > 0 {
            self.exporter.record_write_coalesced(count as i64);
            self.exporter.record_write_coalesced_batch();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 保留：验证 Prometheus 计数器记录
    #[test]
    #[cfg(feature = "metrics")]
    fn test_prometheus_counters() {
        let exporter = PrometheusExporter::new("test", "1.0.0");

        exporter.record_read("success");
        exporter.record_write("success");
        exporter.record_delete("success");

        let text = exporter.gather_text();
        assert!(text.contains("read_total"));
        assert!(text.contains("write_total"));
        assert!(text.contains("delete_total"));
    }

    /// 保留：验证 Prometheus 延迟记录
    #[test]
    #[cfg(feature = "metrics")]
    fn test_prometheus_latencies() {
        let exporter = PrometheusExporter::new("test", "1.0.0");

        exporter.record_read_latency(10.5, "hit");
        exporter.record_write_latency(25.3, "single");
        exporter.record_delete_latency(5.2);

        let text = exporter.gather_text();
        assert!(text.contains("read_latency_us"));
        assert!(text.contains("write_latency_us"));
        assert!(text.contains("delete_latency_us"));
    }

    /// 保留：验证 Prometheus Gauge 指标
    #[test]
    #[cfg(feature = "metrics")]
    fn test_prometheus_gauges() {
        let exporter = PrometheusExporter::new("test", "1.0.0");

        exporter.set_memtable_size(1024 * 1024);
        exporter.set_segment_count(5);
        exporter.set_cache_size("block", 512 * 1024);
        exporter.set_bloom_filter_count(10);

        let text = exporter.gather_text();
        assert!(text.contains("memtable_size_bytes"));
        assert!(text.contains("segment_count"));
        assert!(text.contains("cache_size_bytes"));
        assert!(text.contains("bloom_filter_count"));
    }
}
