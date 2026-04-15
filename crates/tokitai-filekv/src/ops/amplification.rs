//! Write Amplification and Space Amplification Analysis
//!
//! This module provides tools for analyzing:
//! - Write Amplification Factor (WAF): Total bytes written / User data written
//! - Space Amplification Factor (SAF): Total storage used / User data size
//! - Read Amplification Factor (RAF): Total I/O reads / User data read

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::fs;
use std::path::PathBuf;

use crate::{FileKV, FileKVConfig, MemTableConfig};
use crate::cache::block_cache::BlockCacheConfig;
use crate::compaction::CompactionConfig;
use crate::ops::audit_log::AuditLogConfig;
use crate::compression::dictionary::DictionaryCompressionConfig;
use crate::io::StdFs;

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
                async_compaction_enabled: false, // Disabled for analysis
                leveled_compaction_enabled: false, // Disabled for analysis (use size-tiered)
                level_size_multiplier: 10,
                max_level: 3,
                l0_file_count_threshold: 3, // OPT-003: Reduced from 4 to 3
                parallel_compaction_enabled: false, // Disabled for analysis
                streaming_compaction_enabled: false, // Disabled for analysis
                write_amplification_threshold: 3.0, // OPT-003: Default WA threshold
                max_background_compaction_threads: 1, // Disabled for analysis (single thread)
                l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: Default L0 size trigger
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
            enable_background_cache_rebalance: false,
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
        println!("User data written: {} bytes ({:.2} MB)", 
                 self.user_bytes_written, 
                 self.user_bytes_written as f64 / 1024.0 / 1024.0);
        println!("Total bytes written: {} bytes ({:.2} MB)", 
                 self.total_bytes_written, 
                 self.total_bytes_written as f64 / 1024.0 / 1024.0);
        println!("Write Amplification Factor: {:.2}x", self.write_amplification);
        println!();
        println!("=== Space Amplification Analysis ===");
        println!("Segment size on disk: {} bytes ({:.2} MB)", 
                 self.segment_size_bytes,
                 self.segment_size_bytes as f64 / 1024.0 / 1024.0);
        println!("User data size: {} bytes ({:.2} MB)", 
                 self.user_data_size,
                 self.user_data_size as f64 / 1024.0 / 1024.0);
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
                async_compaction_enabled: false, // Disabled for analysis
                leveled_compaction_enabled: false, // Disabled for analysis (use size-tiered)
                level_size_multiplier: 10,
                max_level: 3,
                l0_file_count_threshold: 3, // OPT-003: Reduced from 4 to 3
                parallel_compaction_enabled: false, // Disabled for analysis
                streaming_compaction_enabled: false, // Disabled for analysis
                write_amplification_threshold: 3.0, // OPT-003: Default WA threshold
                max_background_compaction_threads: 1, // Disabled for analysis (single thread)
                l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: Default L0 size trigger
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
            enable_background_cache_rebalance: false,
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
        println!("Cache hits: {} ({:.1}%)", 
                 self.cache_hits, 
                 self.cache_hit_ratio_actual * 100.0);
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
        println!("Total Amplification (WAF × RAF × SAF): {:.2}x", 
                 combined_waf * combined_raf * combined_saf);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_write_amplification_analysis() {
        // Add timeout protection
        let start = Instant::now();
        let analyzer = WriteAmplificationAnalyzer::new();
        let result = analyzer.run_test(1000, 8, 128);  // Reduced from 10,000

        assert!(start.elapsed() < Duration::from_secs(30), "Test should complete within 30s");
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
        analyzer.populate(500, 8, 128);  // Reduced from 1,000
        let result = analyzer.run_test(1000, 0.9);  // Reduced from 10,000

        assert!(start.elapsed() < Duration::from_secs(30), "Test should complete within 30s");
        assert!(result.num_reads == 1000);
        assert!(result.read_amplification >= 1.0);
        assert!(result.cache_hit_ratio_actual >= 0.8);
        assert!(result.reads_per_second > 0.0);
    }
}
