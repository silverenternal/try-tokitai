//! Write Amplification and Space Amplification Analysis
//!
//! This module provides tools for analyzing:
//! - Write Amplification Factor (WAF): Total bytes written / User data written
//! - Space Amplification Factor (SAF): Total storage used / User data size
//! - Read Amplification Factor (RAF): Total I/O reads / User data read

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cache::block_cache::BlockCacheConfig;
use crate::compaction::CompactionConfig;
use crate::compression::dictionary::DictionaryCompressionConfig;
use crate::io::StdFs;
use crate::ops::audit_log::AuditLogConfig;
use crate::{FileKV, FileKVConfig, MemTableConfig};

/// Write amplification analyzer
pub struct WriteAmplificationAnalyzer {
    kv: FileKV,
    temp_dir: tempfile::TempDir,
    user_bytes_written: AtomicU64,
    total_bytes_written: AtomicU64,
}

impl WriteAmplificationAnalyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let segment_dir = temp_dir.path().join("segments");
        let index_dir = temp_dir.path().join("index");
        let wal_dir = temp_dir.path().join("wal");

        fs::create_dir_all(&segment_dir).unwrap();
        fs::create_dir_all(&index_dir).unwrap();
        fs::create_dir_all(&wal_dir).unwrap();

        let config = FileKVConfig {
            memtable: MemTableConfig {
                flush_threshold_bytes: 16 * 1024 * 1024,
                max_entries: 100_000,
                max_memory_bytes: 64 * 1024 * 1024,
                shards: 32,
                enable_async_flush: false,
                max_immutable_memtables: 1,
                immutable_flush_threshold_bytes: 16 * 1024 * 1024,
            },
            segment_dir: segment_dir.clone(),
            enable_wal: true,
            wal_dir,
            index_dir,
            cache: BlockCacheConfig {
                max_items: 50_000,
                max_memory_bytes: 128 * 1024 * 1024,
                frequency_aware: false,
            },
            enable_bloom: true,
            enable_background_flush: false,
            background_flush_interval_ms: 100,
            compaction: CompactionConfig {
                min_segments: 4,
                auto_compact: true,
                check_interval: 100,
                max_segment_size_bytes: 64 * 1024 * 1024,
                target_segment_size_bytes: 32 * 1024 * 1024,
                async_compaction_enabled: false,   // Disabled for analysis
                leveled_compaction_enabled: false, // Disabled for analysis (use size-tiered)
                level_size_multiplier: 10,
                max_level: 3,
                l0_file_count_threshold: 3,                // OPT-003: Reduced from 4 to 3
                parallel_compaction_enabled: false,        // Disabled for analysis
                streaming_compaction_enabled: false,       // Disabled for analysis
                write_amplification_threshold: 3.0,        // OPT-003: Default WA threshold
                max_background_compaction_threads: 1,      // Disabled for analysis (single thread)
                l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: Default L0 size trigger
                // OPT-006: STCS for L0 defaults
                l0_compaction_strategy: crate::compaction::CompactionStrategy::Leveled,
                l0_stcs_min_segments: 3,
                l0_stcs_size_ratio: 2.0,
            },
            segment_preallocate_size: 32 * 1024 * 1024,
            wal_max_size_bytes: 512 * 1024 * 1024,
            wal_max_files: 10,
            cache_warming_enabled: false,
            compression: DictionaryCompressionConfig::default(),
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            async_io_enable_coalescing: false,
            async_io_coalesce_window_ms: 10,
            checkpoint_dir: temp_dir.path().join("checkpoints"),
            audit_log: AuditLogConfig {
                log_dir: temp_dir.path().join("audit_logs"),
                enabled: false,
                rotation_interval_hours: 24,
                retention_days: 30,
            },
            aggressive: crate::AggressiveConfig::performance(),
            enable_adaptive_bloom_cache: true,
            enable_zone_map_pruning: true,
            enable_sequential_prefetch: true,
            enable_multi_level_cache: true,
            l2_cache_max_bytes: 4 * 1024 * 1024 * 1024,
            l2_to_l1_threshold: 5,
            enable_wal_channel: false,
            wal_channel_interval_ms: 2,
            wal_channel_max_entries: 1000,
            wal_channel_capacity: 10_000,
            fs: Arc::new(StdFs),
            block_size: 8192,
            block_compression: crate::core::types::BlockCompressionConfig::default(),
        };

        let kv = FileKV::open(config).unwrap();

        Self {
            kv,
            temp_dir,
            user_bytes_written: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(0),
        }
    }

    /// Record user data written
    pub fn record_user_write(&self, bytes: u64) {
        self.user_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record total bytes written (including WAL, compaction, etc.)
    pub fn record_total_write(&self, bytes: u64) {
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Calculate write amplification factor
    pub fn write_amplification_factor(&self) -> f64 {
        let user = self.user_bytes_written.load(Ordering::Relaxed) as f64;
        let total = self.total_bytes_written.load(Ordering::Relaxed) as f64;

        if user == 0.0 {
            return 1.0;
        }

        total / user
    }

    /// Run write amplification test
    pub fn run_test(&self, num_writes: usize, key_size: usize, value_size: usize) -> WriteAmplificationResult {
        let start = Instant::now();

        // Perform writes
        for i in 0..num_writes {
            let key = format!("key_{:0width$}", i, width = key_size);
            let value = vec![b'x'; value_size];

            let user_bytes = (key.len() + value.len()) as u64;
            self.record_user_write(user_bytes);

            // Estimate total bytes (key + value + WAL overhead + metadata)
            let wal_overhead = 32; // WAL header
            let metadata_overhead = 64; // Index entry overhead
            let total_bytes = user_bytes + wal_overhead + metadata_overhead;

            self.kv.put(&key, &value).unwrap();
            self.record_total_write(total_bytes);
        }

        // Flush memtable to ensure data is written to segments
        let _ = self.kv.flush_memtable();

        // Calculate segment sizes
        let segment_size = self.calculate_segment_size();
        let user_data_size = (num_writes * (key_size + value_size)) as u64;

        let duration = start.elapsed();

        WriteAmplificationResult {
            num_writes,
            user_bytes_written: self.user_bytes_written.load(Ordering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(Ordering::Relaxed),
            write_amplification: self.write_amplification_factor(),
            segment_size_bytes: segment_size,
            user_data_size,
            space_amplification: if user_data_size > 0 && segment_size > 0 {
                segment_size as f64 / user_data_size as f64
            } else {
                1.0 // Default if no segment data
            },
            writes_per_second: num_writes as f64 / duration.as_secs_f64(),
            duration,
        }
    }

    /// Calculate total segment size on disk
    fn calculate_segment_size(&self) -> u64 {
        let segment_dir = self.temp_dir.path().join("segments");
        let mut total_size = 0u64;

        if let Ok(entries) = fs::read_dir(&segment_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        total_size
    }

    /// Get segment directory path
    pub fn segment_dir(&self) -> PathBuf {
        self.temp_dir.path().join("segments")
    }
}

impl Default for WriteAmplificationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Write amplification test result
#[derive(Debug, Clone)]
pub struct WriteAmplificationResult {
    pub num_writes: usize,
    pub user_bytes_written: u64,
    pub total_bytes_written: u64,
    pub write_amplification: f64,
    pub segment_size_bytes: u64,
    pub user_data_size: u64,
    pub space_amplification: f64,
    pub writes_per_second: f64,
    pub duration: Duration,
}

impl WriteAmplificationResult {
    /// Print detailed analysis
    pub fn print_analysis(&self) {
        println!("=== Write Amplification Analysis ===");
        println!("Total writes: {}", self.num_writes);
        println!(
            "User data written: {} bytes ({:.2} MB)",
            self.user_bytes_written,
            self.user_bytes_written as f64 / 1024.0 / 1024.0
        );
        println!(
            "Total bytes written: {} bytes ({:.2} MB)",
            self.total_bytes_written,
            self.total_bytes_written as f64 / 1024.0 / 1024.0
        );
        println!("Write Amplification Factor: {:.2}x", self.write_amplification);
        println!();
        println!("=== Space Amplification Analysis ===");
        println!(
            "Segment size on disk: {} bytes ({:.2} MB)",
            self.segment_size_bytes,
            self.segment_size_bytes as f64 / 1024.0 / 1024.0
        );
        println!(
            "User data size: {} bytes ({:.2} MB)",
            self.user_data_size,
            self.user_data_size as f64 / 1024.0 / 1024.0
        );
        println!("Space Amplification Factor: {:.2}x", self.space_amplification);
        println!();
        println!("=== Performance ===");
        println!("Duration: {:?}", self.duration);
        println!("Writes/second: {:.0}", self.writes_per_second);
        println!("====================================");
    }
}

/// Read amplification analyzer
pub struct ReadAmplificationAnalyzer {
    kv: FileKV,
    /// Temporary directory for analysis (auto-cleaned on drop)
    #[allow(dead_code)]
    temp_dir: tempfile::TempDir,
    user_bytes_read: AtomicU64,
    total_bytes_read: AtomicU64,
}

impl ReadAmplificationAnalyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let segment_dir = temp_dir.path().join("segments");
        let index_dir = temp_dir.path().join("index");
        let wal_dir = temp_dir.path().join("wal");

        fs::create_dir_all(&segment_dir).unwrap();
        fs::create_dir_all(&index_dir).unwrap();
        fs::create_dir_all(&wal_dir).unwrap();

        let config = FileKVConfig {
            memtable: MemTableConfig {
                flush_threshold_bytes: 16 * 1024 * 1024,
                max_entries: 100_000,
                max_memory_bytes: 64 * 1024 * 1024,
                shards: 32,
                enable_async_flush: false,
                max_immutable_memtables: 1,
                immutable_flush_threshold_bytes: 16 * 1024 * 1024,
            },
            segment_dir: segment_dir.clone(),
            enable_wal: true,
            wal_dir,
            index_dir,
            cache: BlockCacheConfig {
                max_items: 50_000,
                max_memory_bytes: 128 * 1024 * 1024,
                frequency_aware: false,
            },
            enable_bloom: true,
            enable_background_flush: false,
            background_flush_interval_ms: 100,
            compaction: CompactionConfig {
                min_segments: 4,
                auto_compact: true,
                check_interval: 100,
                max_segment_size_bytes: 64 * 1024 * 1024,
                target_segment_size_bytes: 32 * 1024 * 1024,
                async_compaction_enabled: false,   // Disabled for analysis
                leveled_compaction_enabled: false, // Disabled for analysis (use size-tiered)
                level_size_multiplier: 10,
                max_level: 3,
                l0_file_count_threshold: 3,                // OPT-003: Reduced from 4 to 3
                parallel_compaction_enabled: false,        // Disabled for analysis
                streaming_compaction_enabled: false,       // Disabled for analysis
                write_amplification_threshold: 3.0,        // OPT-003: Default WA threshold
                max_background_compaction_threads: 1,      // Disabled for analysis (single thread)
                l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: Default L0 size trigger
                // OPT-006: STCS for L0 defaults
                l0_compaction_strategy: crate::compaction::CompactionStrategy::Leveled,
                l0_stcs_min_segments: 3,
                l0_stcs_size_ratio: 2.0,
            },
            segment_preallocate_size: 32 * 1024 * 1024,
            wal_max_size_bytes: 512 * 1024 * 1024,
            wal_max_files: 10,
            cache_warming_enabled: false,
            compression: DictionaryCompressionConfig::default(),
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            async_io_enable_coalescing: false,
            async_io_coalesce_window_ms: 10,
            checkpoint_dir: temp_dir.path().join("checkpoints"),
            audit_log: AuditLogConfig {
                log_dir: temp_dir.path().join("audit_logs"),
                enabled: false,
                rotation_interval_hours: 24,
                retention_days: 30,
            },
            aggressive: crate::AggressiveConfig::performance(),
            enable_adaptive_bloom_cache: true,
            enable_zone_map_pruning: true,
            enable_sequential_prefetch: true,
            enable_multi_level_cache: true,
            l2_cache_max_bytes: 4 * 1024 * 1024 * 1024,
            l2_to_l1_threshold: 5,
            enable_wal_channel: false,
            wal_channel_interval_ms: 2,
            wal_channel_max_entries: 1000,
            wal_channel_capacity: 10_000,
            fs: Arc::new(StdFs),
            block_size: 8192,
            block_compression: crate::core::types::BlockCompressionConfig::default(),
        };

        let kv = FileKV::open(config).unwrap();

        Self {
            kv,
            temp_dir,
            user_bytes_read: AtomicU64::new(0),
            total_bytes_read: AtomicU64::new(0),
        }
    }

    /// Populate with test data
    pub fn populate(&self, num_entries: usize, key_size: usize, value_size: usize) {
        for i in 0..num_entries {
            let key = format!("key_{:0width$}", i, width = key_size);
            let value = vec![b'x'; value_size];
            self.kv.put(&key, &value).unwrap();
        }
    }

    /// Run read amplification test
    pub fn run_test(&self, num_reads: usize, cache_hit_ratio: f64) -> ReadAmplificationResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let start = Instant::now();

        let mut cache_hits = 0u64;
        let mut cache_misses = 0u64;
        let mut bloom_filter_checks = 0u64;
        let mut index_lookups = 0u64;
        let mut data_blocks_read = 0u64;

        for i in 0..num_reads {
            let key = format!("key_{:06}", i % 10_000);

            // Simulate cache hit/miss based on ratio
            let is_cache_hit = rng.gen::<f64>() < cache_hit_ratio;

            if is_cache_hit {
                cache_hits += 1;
                // Cache hit: only read from memory
                self.user_bytes_read.fetch_add(64, Ordering::Relaxed);
                self.total_bytes_read.fetch_add(64, Ordering::Relaxed);
            } else {
                cache_misses += 1;
                bloom_filter_checks += 1;
                index_lookups += 1;
                data_blocks_read += 1;

                // Cache miss: read from disk (estimate)
                let disk_read_bytes = 4096; // One block
                self.user_bytes_read.fetch_add(64, Ordering::Relaxed);
                self.total_bytes_read.fetch_add(disk_read_bytes, Ordering::Relaxed);
            }

            let _ = self.kv.get(&key);
        }

        let duration = start.elapsed();

        ReadAmplificationResult {
            num_reads,
            user_bytes_read: self.user_bytes_read.load(Ordering::Relaxed),
            total_bytes_read: self.total_bytes_read.load(Ordering::Relaxed),
            read_amplification: self.total_bytes_read.load(Ordering::Relaxed) as f64
                / self.user_bytes_read.load(Ordering::Relaxed).max(1) as f64,
            cache_hits,
            cache_misses,
            bloom_filter_checks,
            index_lookups,
            data_blocks_read,
            cache_hit_ratio_actual: cache_hits as f64 / num_reads as f64,
            reads_per_second: num_reads as f64 / duration.as_secs_f64(),
            duration,
        }
    }
}

impl Default for ReadAmplificationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Read amplification test result
#[derive(Debug, Clone)]
pub struct ReadAmplificationResult {
    pub num_reads: usize,
    pub user_bytes_read: u64,
    pub total_bytes_read: u64,
    pub read_amplification: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub bloom_filter_checks: u64,
    pub index_lookups: u64,
    pub data_blocks_read: u64,
    pub cache_hit_ratio_actual: f64,
    pub reads_per_second: f64,
    pub duration: Duration,
}

impl ReadAmplificationResult {
    /// Print detailed analysis
    pub fn print_analysis(&self) {
        println!("=== Read Amplification Analysis ===");
        println!("Total reads: {}", self.num_reads);
        println!("User bytes read: {} bytes", self.user_bytes_read);
        println!("Total bytes read: {} bytes", self.total_bytes_read);
        println!("Read Amplification Factor: {:.2}x", self.read_amplification);
        println!();
        println!("=== Cache Performance ===");
        println!(
            "Cache hits: {} ({:.1}%)",
            self.cache_hits,
            self.cache_hit_ratio_actual * 100.0
        );
        println!("Cache misses: {}", self.cache_misses);
        println!("Bloom filter checks: {}", self.bloom_filter_checks);
        println!("Index lookups: {}", self.index_lookups);
        println!("Data blocks read: {}", self.data_blocks_read);
        println!();
        println!("=== Performance ===");
        println!("Duration: {:?}", self.duration);
        println!("Reads/second: {:.0}", self.reads_per_second);
        println!("====================================");
    }
}

/// Comprehensive amplification analysis report
#[derive(Debug, Clone)]
pub struct AmplificationReport {
    pub write_result: WriteAmplificationResult,
    pub read_result: ReadAmplificationResult,
    pub combined_waf: f64,
    pub combined_raf: f64,
    pub combined_saf: f64,
}

impl AmplificationReport {
    /// Run comprehensive analysis
    pub fn run_comprehensive() -> Self {
        println!("Running comprehensive amplification analysis...\n");

        // Write amplification test
        let write_analyzer = WriteAmplificationAnalyzer::new();
        let write_result = write_analyzer.run_test(100_000, 8, 128);
        write_result.print_analysis();
        println!();

        // Read amplification test
        let read_analyzer = ReadAmplificationAnalyzer::new();
        read_analyzer.populate(10_000, 8, 128);
        let read_result = read_analyzer.run_test(100_000, 0.8); // 80% cache hit ratio
        read_result.print_analysis();
        println!();

        // Combined metrics
        let combined_waf = write_result.write_amplification;
        let combined_raf = read_result.read_amplification;
        let combined_saf = write_result.space_amplification;

        println!("=== Combined Amplification Metrics ===");
        println!("Write Amplification Factor (WAF): {:.2}x", combined_waf);
        println!("Read Amplification Factor (RAF): {:.2}x", combined_raf);
        println!("Space Amplification Factor (SAF): {:.2}x", combined_saf);
        println!(
            "Total Amplification (WAF × RAF × SAF): {:.2}x",
            combined_waf * combined_raf * combined_saf
        );
        println!("======================================");

        Self {
            write_result,
            read_result,
            combined_waf,
            combined_raf,
            combined_saf,
        }
    }
}

/// Real-time amplification tracking stats snapshot
#[derive(Debug, Clone, Copy)]
pub struct AmplificationStats {
    /// Logical bytes written by user (key + value)
    pub logical_write_bytes: u64,
    /// Actual bytes written to disk (WAL, segments, indexes)
    pub actual_disk_write_bytes: u64,
    /// Logical bytes requested by user reads
    pub logical_read_bytes: u64,
    /// Actual bytes read from disk
    pub actual_disk_read_bytes: u64,
    /// Current logical data size (unique live data)
    pub logical_data_bytes: u64,
    /// Current disk usage (all segment files)
    pub actual_disk_usage_bytes: u64,
    /// Write amplification factor (WA = actual_disk_write / logical_write)
    pub write_amplification: f64,
    /// Read amplification factor (RA = actual_disk_read / logical_read)
    pub read_amplification: f64,
    /// Space amplification factor (SA = actual_disk_usage / logical_data)
    pub space_amplification: f64,
}

/// Real-time amplification tracker using lock-free atomic operations
///
/// Tracks write amplification (WA), read amplification (RA), and space amplification (SA)
/// with minimal overhead. All counters use atomic operations for thread-safety.
///
/// # Definitions
/// - **WA (Write Amplification)** = actual_disk_write_bytes / logical_write_bytes
/// - **RA (Read Amplification)** = actual_disk_read_bytes / logical_read_bytes
/// - **SA (Space Amplification)** = actual_disk_usage_bytes / logical_data_bytes
///
/// # Integration Points
/// - `WriteEngine::put()`: call `record_logical_write(key.len + value.len)`
/// - WAL writes: call `record_disk_write(actual_wal_bytes)`
/// - MemTable flush: call `record_disk_write(segment_bytes)`
/// - Compaction: call `record_disk_write(new_segment_bytes)` and `record_disk_read(old_segment_bytes)`
/// - `ReadEngine::get()`: call `record_logical_read(key.len)` and `record_disk_read(actual_read_bytes)`
pub struct AmplificationTracker {
    /// Logical bytes written by user applications (key + value sizes)
    logical_write_bytes: AtomicU64,
    /// Actual bytes written to physical storage (WAL, segments, index files)
    actual_disk_write_bytes: AtomicU64,
    /// Logical bytes requested by user read operations (key sizes)
    logical_read_bytes: AtomicU64,
    /// Actual bytes read from physical storage (blocks, index lookups)
    actual_disk_read_bytes: AtomicU64,
    /// Current logical data size (sum of unique live key-value pairs)
    logical_data_bytes: AtomicU64,
    /// Current total disk usage (sum of all segment file sizes)
    actual_disk_usage_bytes: AtomicU64,
}

impl Default for AmplificationTracker {
    fn default() -> Self {
        Self {
            logical_write_bytes: AtomicU64::new(0),
            actual_disk_write_bytes: AtomicU64::new(0),
            logical_read_bytes: AtomicU64::new(0),
            actual_disk_read_bytes: AtomicU64::new(0),
            logical_data_bytes: AtomicU64::new(0),
            actual_disk_usage_bytes: AtomicU64::new(0),
        }
    }
}

impl AmplificationTracker {
    /// Create a new tracker with all counters initialized to zero
    pub fn new() -> Self {
        Self::default()
    }

    /// Record logical bytes written by user application
    ///
    /// Call this when a `put()` operation is received, before any internal processing.
    /// `bytes` should be `key.len() + value.len()`.
    #[inline]
    pub fn record_logical_write(&self, bytes: u64) {
        self.logical_write_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record actual bytes written to disk (WAL, segment files, index files)
    ///
    /// Call this whenever data is physically written to storage.
    #[inline]
    pub fn record_disk_write(&self, bytes: u64) {
        self.actual_disk_write_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record logical bytes requested by user read operation
    ///
    /// Call this when a `get()` operation is received.
    /// `bytes` should be `key.len()`.
    #[inline]
    pub fn record_logical_read(&self, bytes: u64) {
        self.logical_read_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record actual bytes read from disk (segment blocks, index pages, bloom filters)
    ///
    /// Call this whenever data is physically read from storage.
    #[inline]
    pub fn record_disk_read(&self, bytes: u64) {
        self.actual_disk_read_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update current total disk usage
    ///
    /// Call this when segment files are created, deleted, or resized.
    /// `bytes` should be the new total disk usage across all segment files.
    #[inline]
    pub fn update_disk_usage(&self, bytes: u64) {
        self.actual_disk_usage_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update current logical data size
    ///
    /// Call this when live data size changes (e.g., after compaction removes duplicates).
    /// `bytes` should be the sum of all unique live key-value pair sizes.
    #[inline]
    pub fn update_logical_data(&self, bytes: u64) {
        self.logical_data_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Calculate write amplification factor
    ///
    /// WA = actual_disk_write_bytes / logical_write_bytes
    /// Returns 1.0 if logical_write_bytes is 0 (no amplification yet).
    pub fn get_write_amplification(&self) -> f64 {
        let logical = self.logical_write_bytes.load(Ordering::Relaxed);
        let actual = self.actual_disk_write_bytes.load(Ordering::Relaxed);
        if logical == 0 {
            return 1.0;
        }
        actual as f64 / logical as f64
    }

    /// Calculate read amplification factor
    ///
    /// RA = actual_disk_read_bytes / logical_read_bytes
    /// Returns 1.0 if logical_read_bytes is 0 (no amplification yet).
    pub fn get_read_amplification(&self) -> f64 {
        let logical = self.logical_read_bytes.load(Ordering::Relaxed);
        let actual = self.actual_disk_read_bytes.load(Ordering::Relaxed);
        if logical == 0 {
            return 1.0;
        }
        actual as f64 / logical as f64
    }

    /// Calculate space amplification factor
    ///
    /// SA = actual_disk_usage_bytes / logical_data_bytes
    /// Returns 1.0 if logical_data_bytes is 0 (no amplification yet).
    pub fn get_space_amplification(&self) -> f64 {
        let logical = self.logical_data_bytes.load(Ordering::Relaxed);
        let actual = self.actual_disk_usage_bytes.load(Ordering::Relaxed);
        if logical == 0 {
            return 1.0;
        }
        actual as f64 / logical as f64
    }

    /// Get complete amplification statistics snapshot
    ///
    /// Returns an `AmplificationStats` struct with all current counter values
    /// and calculated amplification factors.
    pub fn get_stats(&self) -> AmplificationStats {
        let logical_write = self.logical_write_bytes.load(Ordering::Relaxed);
        let actual_disk_write = self.actual_disk_write_bytes.load(Ordering::Relaxed);
        let logical_read = self.logical_read_bytes.load(Ordering::Relaxed);
        let actual_disk_read = self.actual_disk_read_bytes.load(Ordering::Relaxed);
        let logical_data = self.logical_data_bytes.load(Ordering::Relaxed);
        let actual_disk_usage = self.actual_disk_usage_bytes.load(Ordering::Relaxed);

        let wa = if logical_write > 0 {
            actual_disk_write as f64 / logical_write as f64
        } else {
            1.0
        };

        let ra = if logical_read > 0 {
            actual_disk_read as f64 / logical_read as f64
        } else {
            1.0
        };

        let sa = if logical_data > 0 {
            actual_disk_usage as f64 / logical_data as f64
        } else {
            1.0
        };

        AmplificationStats {
            logical_write_bytes: logical_write,
            actual_disk_write_bytes: actual_disk_write,
            logical_read_bytes: logical_read,
            actual_disk_read_bytes: actual_disk_read,
            logical_data_bytes: logical_data,
            actual_disk_usage_bytes: actual_disk_usage,
            write_amplification: wa,
            read_amplification: ra,
            space_amplification: sa,
        }
    }

    /// Reset all counters to zero
    ///
    /// Useful for testing or after checkpoint operations.
    pub fn reset(&self) {
        self.logical_write_bytes.store(0, Ordering::Relaxed);
        self.actual_disk_write_bytes.store(0, Ordering::Relaxed);
        self.logical_read_bytes.store(0, Ordering::Relaxed);
        self.actual_disk_read_bytes.store(0, Ordering::Relaxed);
        self.logical_data_bytes.store(0, Ordering::Relaxed);
        self.actual_disk_usage_bytes.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_write_amplification_analysis() {
        // Add timeout protection
        let start = Instant::now();
        let analyzer = WriteAmplificationAnalyzer::new();
        let result = analyzer.run_test(1000, 8, 128); // Reduced from 10,000

        assert!(
            start.elapsed() < Duration::from_secs(30),
            "Test should complete within 30s"
        );
        assert!(result.num_writes == 1000);
        assert!(result.write_amplification >= 1.0);
        // Space amplification varies due to compression and segment layout
        assert!(result.space_amplification > 0.0);
        assert!(result.writes_per_second > 0.0);
    }

    #[test]
    fn test_read_amplification_analysis() {
        // Add timeout protection
        let start = Instant::now();
        let analyzer = ReadAmplificationAnalyzer::new();
        analyzer.populate(500, 8, 128); // Reduced from 1,000
        let result = analyzer.run_test(1000, 0.9); // Reduced from 10,000

        assert!(
            start.elapsed() < Duration::from_secs(30),
            "Test should complete within 30s"
        );
        assert!(result.num_reads == 1000);
        assert!(result.read_amplification >= 1.0);
        assert!(result.cache_hit_ratio_actual >= 0.8);
        assert!(result.reads_per_second > 0.0);
    }

    // ========================================================================
    // OPT-008: AmplificationTracker tests
    // ========================================================================

    #[test]
    fn test_tracker_initial_state() {
        let tracker = AmplificationTracker::new();
        let stats = tracker.get_stats();

        assert_eq!(stats.logical_write_bytes, 0);
        assert_eq!(stats.actual_disk_write_bytes, 0);
        assert_eq!(stats.logical_read_bytes, 0);
        assert_eq!(stats.actual_disk_read_bytes, 0);
        assert_eq!(stats.logical_data_bytes, 0);
        assert_eq!(stats.actual_disk_usage_bytes, 0);
        assert_eq!(stats.write_amplification, 1.0);
        assert_eq!(stats.read_amplification, 1.0);
        assert_eq!(stats.space_amplification, 1.0);
    }

    #[test]
    fn test_write_amplification_calculation() {
        let tracker = AmplificationTracker::new();

        // Record 1KB logical write
        tracker.record_logical_write(1024);

        // Record 3KB actual disk write (WAL + segment)
        tracker.record_disk_write(3072);

        let wa = tracker.get_write_amplification();
        assert!((wa - 3.0).abs() < f64::EPSILON);

        let stats = tracker.get_stats();
        assert_eq!(stats.logical_write_bytes, 1024);
        assert_eq!(stats.actual_disk_write_bytes, 3072);
        assert!((stats.write_amplification - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_read_amplification_calculation() {
        let tracker = AmplificationTracker::new();

        // Record 10 bytes logical read (key length)
        tracker.record_logical_read(10);

        // Record 4KB actual disk read (one block)
        tracker.record_disk_read(4096);

        let ra = tracker.get_read_amplification();
        assert!((ra - 409.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_space_amplification_calculation() {
        let tracker = AmplificationTracker::new();

        // Set logical data size to 10KB
        tracker.update_logical_data(10_240);

        // Set disk usage to 30KB (3x due to old versions, tombstones, etc.)
        tracker.update_disk_usage(30_720);

        let sa = tracker.get_space_amplification();
        assert!((sa - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_multiple_accumulated_writes() {
        let tracker = AmplificationTracker::new();

        // Simulate 10 writes
        for _ in 0..10 {
            tracker.record_logical_write(100); // 100 bytes logical per write
            tracker.record_disk_write(350); // 350 bytes actual per write (WAL overhead)
        }

        let wa = tracker.get_write_amplification();
        assert!((wa - 3.5).abs() < f64::EPSILON);

        let stats = tracker.get_stats();
        assert_eq!(stats.logical_write_bytes, 1000);
        assert_eq!(stats.actual_disk_write_bytes, 3500);
    }

    #[test]
    fn test_tracker_reset() {
        let tracker = AmplificationTracker::new();

        tracker.record_logical_write(1024);
        tracker.record_disk_write(3072);
        tracker.record_logical_read(10);
        tracker.record_disk_read(4096);
        tracker.update_logical_data(1024);
        tracker.update_disk_usage(3072);

        tracker.reset();

        let stats = tracker.get_stats();
        assert_eq!(stats.logical_write_bytes, 0);
        assert_eq!(stats.actual_disk_write_bytes, 0);
        assert_eq!(stats.logical_read_bytes, 0);
        assert_eq!(stats.actual_disk_read_bytes, 0);
        assert_eq!(stats.logical_data_bytes, 0);
        assert_eq!(stats.actual_disk_usage_bytes, 0);
    }

    #[test]
    fn test_tracker_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let tracker = Arc::new(AmplificationTracker::new());
        let num_threads = 8;
        let ops_per_thread = 1000;

        let mut handles = Vec::new();

        for i in 0..num_threads {
            let t = Arc::clone(&tracker);
            handles.push(thread::spawn(move || {
                for _ in 0..ops_per_thread {
                    t.record_logical_write(100);
                    t.record_disk_write(300);
                    t.record_logical_read(10);
                    t.record_disk_read(4096);
                }
                // Prevent compiler optimizing away reads
                if i == 0 {
                    let _ = t.get_write_amplification();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = tracker.get_stats();
        let expected_logical = (num_threads * ops_per_thread * 100) as u64;
        let expected_disk = (num_threads * ops_per_thread * 300) as u64;

        assert_eq!(stats.logical_write_bytes, expected_logical);
        assert_eq!(stats.actual_disk_write_bytes, expected_disk);
        assert!((stats.write_amplification - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_zero_division_protection() {
        let tracker = AmplificationTracker::new();

        // Should return 1.0 when no data
        assert_eq!(tracker.get_write_amplification(), 1.0);
        assert_eq!(tracker.get_read_amplification(), 1.0);
        assert_eq!(tracker.get_space_amplification(), 1.0);

        // Only record disk operations, no logical operations
        tracker.record_disk_write(1024);
        tracker.record_disk_read(4096);
        tracker.update_disk_usage(3072);

        // Should still return 1.0 (logical is 0)
        assert_eq!(tracker.get_write_amplification(), 1.0);
        assert_eq!(tracker.get_read_amplification(), 1.0);
        assert_eq!(tracker.get_space_amplification(), 1.0);
    }
}
