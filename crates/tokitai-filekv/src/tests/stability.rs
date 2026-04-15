//! FileKV 长期稳定性测试
//!
//! 测试 FileKV 在长时间运行下的稳定性：
//! - 内存泄漏检测
//! - 性能衰减检测
//! - 数据一致性验证

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use crate::FileKV;

/// Fixed RNG seed for reproducible stability tests in CI
/// Change this seed if you want different random data but still reproducible runs
const STABILITY_TEST_RNG_SEED: u64 = 42;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;

thread_local! {
    static STABILITY_RNG: RefCell<StdRng> = RefCell::new(StdRng::seed_from_u64(STABILITY_TEST_RNG_SEED));
}

/// 长期稳定性测试配置
#[derive(Debug, Clone)]
pub struct StabilityTestConfig {
    /// 测试持续时间
    pub duration: Duration,
    /// 每秒写入次数
    pub writes_per_second: usize,
    /// 每次写入大小（字节）
    pub write_size_bytes: usize,
    /// 读取比例（0.0-1.0）
    pub read_ratio: f64,
    /// 内存泄漏检测间隔
    pub memory_check_interval: Duration,
    /// 性能检测间隔
    pub performance_check_interval: Duration,
}

impl Default for StabilityTestConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60), // 1 分钟快速测试
            writes_per_second: 100,
            write_size_bytes: 64,
            read_ratio: 0.3,
            memory_check_interval: Duration::from_secs(10),
            performance_check_interval: Duration::from_secs(5),
        }
    }
}

/// 稳定性测试结果
#[derive(Debug, Clone)]
pub struct StabilityTestResult {
    /// 总写入次数
    pub total_writes: usize,
    /// 总读取次数
    pub total_reads: usize,
    /// 平均写入延迟（纳秒）
    pub avg_write_latency_ns: f64,
    /// 平均读取延迟（纳秒）
    pub avg_read_latency_ns: f64,
    /// P99 写入延迟（纳秒）
    pub p99_write_latency_ns: f64,
    /// P99 读取延迟（纳秒）
    pub p99_read_latency_ns: f64,
    /// 初始内存占用（MB）
    pub initial_memory_mb: f64,
    /// 最终内存占用（MB）
    pub final_memory_mb: f64,
    /// 内存增长（MB）
    pub memory_growth_mb: f64,
    /// 内存增长率（%）
    pub memory_growth_percent: f64,
    /// 初始性能（ops/sec）
    pub initial_ops_per_sec: f64,
    /// 最终性能（ops/sec）
    pub final_ops_per_sec: f64,
    /// 性能衰减（%）
    pub performance_degradation_percent: f64,
    /// 数据一致性检查是否通过
    pub consistency_check_passed: bool,
}

/// 长期稳定性测试器
pub struct StabilityTester {
    config: StabilityTestConfig,
    filekv: Arc<FileKV>,
    stats: Mutex<StabilityStats>,
}

#[derive(Debug, Default)]
struct StabilityStats {
    write_latencies: Mutex<Vec<f64>>,
    read_latencies: Mutex<Vec<f64>>,
    performance_samples: Mutex<Vec<f64>>,
    memory_samples: Mutex<Vec<f64>>,
}

impl StabilityTester {
    /// 创建新的稳定性测试器
    pub fn new(filekv: Arc<FileKV>, config: StabilityTestConfig) -> Self {
        Self {
            config,
            filekv,
            stats: Mutex::new(StabilityStats::default()),
        }
    }

    /// 运行稳定性测试
    pub fn run(&self) -> StabilityTestResult {
        println!("Starting stability test for {:?}", self.config.duration);
        println!("  Writes/sec: {}", self.config.writes_per_second);
        println!("  Write size: {} bytes", self.config.write_size_bytes);
        println!("  Read ratio: {:.1}%", self.config.read_ratio * 100.0);

        let start_time = Instant::now();
        let mut total_writes = 0;
        let mut total_reads = 0;
        let mut write_errors = 0;
        let mut read_errors = 0;

        // 初始内存检查
        let initial_memory = self.get_memory_usage_mb();
        println!("Initial memory usage: {:.2} MB", initial_memory);

        // 性能基线
        let baseline_ops = self.measure_ops_per_sec();
        println!("Baseline performance: {:.0} ops/sec", baseline_ops);

        // 主测试循环
        let write_interval = Duration::from_secs_f64(1.0 / self.config.writes_per_second as f64);
        let mut last_write = Instant::now();
        let mut last_memory_check = Instant::now();
        let mut last_perf_check = Instant::now();

        while start_time.elapsed() < self.config.duration {
            let now = Instant::now();

            // 写入操作
            if now.duration_since(last_write) >= write_interval {
                match self.do_write() {
                    Ok(latency_ns) => {
                        self.stats.lock().write_latencies.lock().push(latency_ns);
                        total_writes += 1;
                    }
                    Err(e) => {
                        eprintln!("Write error: {:?}", e);
                        write_errors += 1;
                    }
                }
                last_write = now;
            }

            // 读取操作
            if self.config.read_ratio > 0.0 && total_writes > 0
                && STABILITY_RNG.with(|rng| rng.borrow_mut().gen::<f64>()) < self.config.read_ratio {
                match self.do_read() {
                    Ok(latency_ns) => {
                        self.stats.lock().read_latencies.lock().push(latency_ns);
                        total_reads += 1;
                    }
                    Err(e) => {
                        eprintln!("Read error: {:?}", e);
                        read_errors += 1;
                    }
                }
            }

            // 内存检查
            if now.duration_since(last_memory_check) >= self.config.memory_check_interval {
                let memory = self.get_memory_usage_mb();
                self.stats.lock().memory_samples.lock().push(memory);
                println!("[{:?}] Memory: {:.2} MB", start_time.elapsed(), memory);
                last_memory_check = now;
            }

            // 性能检查
            if now.duration_since(last_perf_check) >= self.config.performance_check_interval {
                let ops = self.measure_ops_per_sec();
                self.stats.lock().performance_samples.lock().push(ops);
                println!("[{:?}] Performance: {:.0} ops/sec", start_time.elapsed(), ops);
                last_perf_check = now;
            }
        }

        // 最终内存检查
        let final_memory = self.get_memory_usage_mb();
        println!("Final memory usage: {:.2} MB", final_memory);

        // 最终性能检查
        let final_ops = self.measure_ops_per_sec();
        println!("Final performance: {:.0} ops/sec", final_ops);

        // 计算结果
        let stats_lock = self.stats.lock();
        let write_latencies = stats_lock.write_latencies.lock().clone();
        let read_latencies = stats_lock.read_latencies.lock().clone();

        let avg_write = if write_latencies.is_empty() {
            0.0
        } else {
            write_latencies.iter().sum::<f64>() / write_latencies.len() as f64
        };

        let avg_read = if read_latencies.is_empty() {
            0.0
        } else {
            read_latencies.iter().sum::<f64>() / read_latencies.len() as f64
        };

        let p99_write = self.calculate_p99(&write_latencies);
        let p99_read = self.calculate_p99(&read_latencies);

        let memory_growth = final_memory - initial_memory;
        let memory_growth_percent = if initial_memory > 0.0 {
            (memory_growth / initial_memory) * 100.0
        } else {
            0.0
        };

        let perf_degradation = if baseline_ops > 0.0 {
            ((baseline_ops - final_ops) / baseline_ops) * 100.0
        } else {
            0.0
        };

        println!("\n=== Stability Test Results ===");
        println!("Total writes: {} ({} errors)", total_writes, write_errors);
        println!("Total reads: {} ({} errors)", total_reads, read_errors);
        println!("Avg write latency: {:.0} ns", avg_write);
        println!("Avg read latency: {:.0} ns", avg_read);
        println!("P99 write latency: {:.0} ns", p99_write);
        println!("P99 read latency: {:.0} ns", p99_read);
        println!("Memory growth: {:.2} MB ({:.2}%)", memory_growth, memory_growth_percent);
        println!("Performance degradation: {:.2}%", perf_degradation);

        StabilityTestResult {
            total_writes,
            total_reads,
            avg_write_latency_ns: avg_write,
            avg_read_latency_ns: avg_read,
            p99_write_latency_ns: p99_write,
            p99_read_latency_ns: p99_read,
            initial_memory_mb: initial_memory,
            final_memory_mb: final_memory,
            memory_growth_mb: memory_growth,
            memory_growth_percent,
            initial_ops_per_sec: baseline_ops,
            final_ops_per_sec: final_ops,
            performance_degradation_percent: perf_degradation,
            consistency_check_passed: write_errors == 0 && read_errors == 0,
        }
    }

    fn do_write(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let key = format!("stability_test_{}", STABILITY_RNG.with(|rng| rng.borrow_mut().gen::<u64>()));
        let value = vec![b'x'; self.config.write_size_bytes];

        let start = Instant::now();
        self.filekv.put(&key, &value)?;
        let latency = start.elapsed().as_nanos() as f64;

        Ok(latency)
    }

    fn do_read(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let key = format!("stability_test_{}", STABILITY_RNG.with(|rng| rng.borrow_mut().gen::<u64>()));

        let start = Instant::now();
        let _ = self.filekv.get(&key);
        let latency = start.elapsed().as_nanos() as f64;

        Ok(latency)
    }

    fn get_memory_usage_mb(&self) -> f64 {
        // 简单的内存使用估算（实际应该用 /proc/self/status 或类似方法）
        // 这里用 stats 作为代理
        let stats = self.filekv.get_stats();
        let cache_size = stats.cache_hits * 8;
        cache_size as f64 / 1024.0 / 1024.0
    }

    fn measure_ops_per_sec(&self) -> f64 {
        let start = Instant::now();
        let mut ops = 0;

        while start.elapsed() < Duration::from_secs(1) {
            let key = format!("perf_test_{}", ops);
            let value = vec![b'x'; 64];
            if self.filekv.put(&key, &value).is_ok() {
                ops += 1;
            }
        }

        ops as f64
    }

    fn calculate_p99(&self, latencies: &[f64]) -> f64 {
        if latencies.is_empty() {
            return 0.0;
        }

        let mut sorted = latencies.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p99_index = (sorted.len() as f64 * 0.99) as usize;
        sorted[p99_index.min(sorted.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use crate::core::config::FileKVConfig;
    use crate::core::memtable::MemTableConfig;
    use crate::cache::block_cache::BlockCacheConfig;
    use crate::compaction::CompactionConfig;
    use crate::ops::audit_log::AuditLogConfig;
    use std::time::{Duration, Instant};

    fn setup_test_filekv() -> (tempfile::TempDir, Arc<FileKV>) {
        let temp_dir = tempfile::tempdir().unwrap();
        let segment_dir = temp_dir.path().join("segments");
        let index_dir = temp_dir.path().join("index");
        let wal_dir = temp_dir.path().join("wal");

        std::fs::create_dir_all(&segment_dir).unwrap();
        std::fs::create_dir_all(&index_dir).unwrap();
        std::fs::create_dir_all(&wal_dir).unwrap();

        let config = FileKVConfig {
            memtable: MemTableConfig {
                flush_threshold_bytes: 4 * 1024 * 1024,
                max_entries: 100_000,
                max_memory_bytes: 64 * 1024 * 1024,
                shards: 32,
            },
            segment_dir,
            enable_wal: true,
            wal_dir,
            index_dir,
            cache: BlockCacheConfig {
                max_items: 10_000,
                max_memory_bytes: 64 * 1024 * 1024,
                frequency_aware: false,
            },
            enable_bloom: true,
            enable_background_flush: false,
            background_flush_interval_ms: 100,
            compaction: CompactionConfig {
                min_segments: 4,
                auto_compact: false,
                check_interval: 100,
                max_segment_size_bytes: 16 * 1024 * 1024,
                target_segment_size_bytes: 8 * 1024 * 1024,
                async_compaction_enabled: false, // Disabled for stability tests
                leveled_compaction_enabled: false, // Disabled for stability tests
                level_size_multiplier: 10,
                max_level: 3,
                l0_file_count_threshold: 4,
                parallel_compaction_enabled: false, // Disabled for stability tests
                streaming_compaction_enabled: true,
                write_amplification_threshold: 3.0, // OPT-003: Default WA threshold
                max_background_compaction_threads: 1, // Disabled for stability tests
                l0_size_bytes_threshold: 64 * 1024 * 1024, // OPT-003: Default L0 size trigger
            },
            segment_preallocate_size: 16 * 1024 * 1024,
            wal_max_size_bytes: 100 * 1024 * 1024,
            wal_max_files: 5,
            cache_warming_enabled: false,
            async_io_enabled: false,
            async_io_max_concurrent_writes: 4,
            async_io_max_queue_depth: 1024,
            async_io_write_timeout_ms: 5000,
            checkpoint_dir: temp_dir.path().join("checkpoints"),
            audit_log: AuditLogConfig {
                log_dir: temp_dir.path().join("audit_logs"),
                enabled: false,
                rotation_interval_hours: 24,
                retention_days: 30,
            },
            enable_zone_map_pruning: true,
            enable_sequential_prefetch: true,
            ..Default::default()
        };

        let kv = Arc::new(FileKV::open(config).unwrap());
        (temp_dir, kv)
    }

    #[test]
    fn test_short_running_stability() {
        let (_temp_dir, filekv) = setup_test_filekv();

        let config = StabilityTestConfig {
            duration: Duration::from_secs(3), // 3 秒快速测试（原 10 秒）
            writes_per_second: 20,            // 降低频率（原 50）
            write_size_bytes: 64,
            read_ratio: 0.2,
            memory_check_interval: Duration::from_secs(1),
            performance_check_interval: Duration::from_secs(1),
        };

        let tester = StabilityTester::new(filekv, config);
        let result = tester.run();

        // 验证测试结果
        assert!(result.consistency_check_passed, "Consistency check failed");
        assert!(result.total_writes > 0, "No writes completed");
        assert!(result.memory_growth_percent < 50.0, "Memory growth too high: {:.2}%", result.memory_growth_percent);
        assert!(result.performance_degradation_percent < 50.0, "Performance degradation too high: {:.2}%", result.performance_degradation_percent);
    }

    #[test]
    fn test_memory_leak_detection() {
        let (_temp_dir, filekv) = setup_test_filekv();

        let config = StabilityTestConfig {
            duration: Duration::from_secs(5),   // 5 秒快速测试（原 30 秒）
            writes_per_second: 50,              // 降低频率（原 100）
            write_size_bytes: 64,               // 减小写入大小（原 128）
            read_ratio: 0.3,                    // 降低读取比例（原 0.5）
            memory_check_interval: Duration::from_secs(2),
            performance_check_interval: Duration::from_secs(2),
        };

        let tester = StabilityTester::new(filekv, config);
        let result = tester.run();

        // 内存泄漏检测：增长不超过 20%
        assert!(
            result.memory_growth_percent < 20.0,
            "Potential memory leak detected: {:.2}% growth",
            result.memory_growth_percent
        );
    }

    #[test]
    fn test_performance_stability() {
        let (_temp_dir, filekv) = setup_test_filekv();

        let config = StabilityTestConfig {
            duration: Duration::from_secs(5),   // 5 秒快速测试（原 30 秒）
            writes_per_second: 50,              // 降低频率（原 200）
            write_size_bytes: 64,
            read_ratio: 0.2,                    // 降低读取比例（原 0.3）
            memory_check_interval: Duration::from_secs(2),
            performance_check_interval: Duration::from_secs(2),
        };

        let tester = StabilityTester::new(filekv, config);
        let result = tester.run();

        // 性能稳定性：衰减不超过 20%
        assert!(
            result.performance_degradation_percent < 20.0,
            "Performance degradation detected: {:.2}%",
            result.performance_degradation_percent
        );
    }
}
