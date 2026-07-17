//! 性能基准测试
//!
//! 提供完整的性能基准测试套件，用于评估平行上下文操作的性能
//!
//! ## 测试项目
//! - Fork 操作延迟
//! - Merge 操作吞吐量
//! - 冲突检测性能
//! - 缓存命中率
//! - 存储效率

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use tempfile::TempDir;
use serde::{Deserialize, Serialize};

use crate::{
    ParallelContextManager, ParallelContextManagerConfig,
    branch::ContextBranch,
    cow::{CowManager, BranchCloner, CowConfig},
    bloom_conflict::BloomConflictDetector,
    three_way_merge::ThreeWayMerger,
};

/// 基准测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// 每个测试的分支数量
    pub num_branches: usize,
    /// 每个分支的文件数量
    pub files_per_branch: usize,
    /// 文件大小（字节）
    pub file_size: usize,
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 是否启用 COW
    pub enable_cow: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_branches: 10,
            files_per_branch: 100,
            file_size: 1024, // 1KB
            enable_cache: true,
            enable_cow: true,
        }
    }
}

/// 基准测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// 测试名称
    pub name: String,
    /// 平均延迟
    pub avg_latency_ms: f64,
    /// 最小延迟
    pub min_latency_ms: f64,
    /// 最大延迟
    pub max_latency_ms: f64,
    /// 标准差
    pub std_dev_ms: f64,
    /// 吞吐量（操作/秒）
    pub throughput_ops: f64,
    /// 总测试时间
    pub total_duration_ms: f64,
    /// 迭代次数
    pub iterations: usize,
}

impl BenchmarkResult {
    /// 创建基准测试结果
    pub fn new(name: &str, durations: &[Duration]) -> Self {
        let mut durations_ms: Vec<f64> = durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .collect();

        // P2-005 FIX: Use unwrap_or for partial_cmp to handle NaN safely
        durations_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let avg = durations_ms.iter().sum::<f64>() / durations_ms.len() as f64;
        let min = durations_ms.first().copied().unwrap_or(0.0);
        let max = durations_ms.last().copied().unwrap_or(0.0);

        // 计算标准差
        let variance = durations_ms
            .iter()
            .map(|d| (d - avg).powi(2))
            .sum::<f64>()
            / durations_ms.len() as f64;
        let std_dev = variance.sqrt();

        let total_duration_ms = durations_ms.iter().sum();
        let throughput = if avg > 0.0 {
            1000.0 / avg
        } else {
            0.0
        };

        Self {
            name: name.to_string(),
            avg_latency_ms: avg,
            min_latency_ms: min,
            max_latency_ms: max,
            std_dev_ms: std_dev,
            throughput_ops: throughput,
            total_duration_ms,
            iterations: durations_ms.len(),
        }
    }
}

impl std::fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Benchmark: {}", self.name)?;
        writeln!(f, "  Iterations: {}", self.iterations)?;
        writeln!(f, "  Avg latency: {:.3} ms", self.avg_latency_ms)?;
        writeln!(f, "  Min latency: {:.3} ms", self.min_latency_ms)?;
        writeln!(f, "  Max latency: {:.3} ms", self.max_latency_ms)?;
        writeln!(f, "  Std dev: {:.3} ms", self.std_dev_ms)?;
        writeln!(f, "  Throughput: {:.2} ops/sec", self.throughput_ops)?;
        writeln!(f, "  Total duration: {:.2} ms", self.total_duration_ms)?;
        Ok(())
    }
}

/// 基准测试套件
pub struct BenchmarkSuite {
    config: BenchmarkConfig,
    temp_dir: TempDir,
    results: Vec<BenchmarkResult>,
}

impl BenchmarkSuite {
    /// 创建基准测试套件
    pub fn new(config: BenchmarkConfig) -> Result<Self> {
        let temp_dir = TempDir::new()
            .context("Failed to create temp directory for benchmarks")?;

        Ok(Self {
            config,
            temp_dir,
            results: Vec::new(),
        })
    }

    /// 运行所有基准测试
    pub fn run_all(&mut self) -> Result<Vec<BenchmarkResult>> {
        println!("🚀 Starting Parallel Context Benchmark Suite\n");

        self.run_fork_benchmark()?;
        self.run_merge_benchmark()?;
        self.run_conflict_detection_benchmark()?;
        self.run_cow_benchmark()?;

        println!("\n✅ All benchmarks completed\n");

        Ok(self.results.clone())
    }

    /// Fork 操作基准测试
    pub fn run_fork_benchmark(&mut self) -> Result<BenchmarkResult> {
        println!("📊 Running Fork Benchmark...");

        let context_root = self.temp_dir.path().join("fork_test");
        let mut manager = ParallelContextManager::from_context_root(&context_root)?;

        // 创建主分支并添加文件
        let main_branch = manager.get_branch("main").unwrap().clone();
        self.create_test_files(&main_branch.short_term_dir)?;

        let mut durations = Vec::new();
        let iterations = self.config.num_branches;

        for i in 0..iterations {
            let start = Instant::now();

            let branch_name = format!("feature-{}", i);
            manager.create_branch(&branch_name, "main")?;

            let duration = start.elapsed();
            durations.push(duration);
        }

        let result = BenchmarkResult::new("Fork Operation", &durations);
        println!("  {}\n", result);

        self.results.push(result.clone());
        Ok(result)
    }

    /// Merge 操作基准测试
    pub fn run_merge_benchmark(&mut self) -> Result<BenchmarkResult> {
        println!("📊 Running Merge Benchmark...");

        let context_root = self.temp_dir.path().join("merge_test");
        let mut manager = ParallelContextManager::from_context_root(&context_root)?;

        // 创建多个分支
        let mut branch_ids = Vec::new();
        for i in 0..self.config.num_branches / 2 {
            let branch_name = format!("merge-feature-{}", i);
            let branch = manager.create_branch(&branch_name, "main")?;
            branch_ids.push(branch.branch_id.clone());

            // 添加一些文件
            self.create_test_files(&branch.short_term_dir)?;
        }

        let mut durations = Vec::new();

        // 合并回 main
        for branch_id in branch_ids {
            let start = Instant::now();

            manager.merge(&branch_id, "main", None)?;

            let duration = start.elapsed();
            durations.push(duration);
        }

        let result = BenchmarkResult::new("Merge Operation", &durations);
        println!("  {}\n", result);

        self.results.push(result.clone());
        Ok(result)
    }

    /// 冲突检测基准测试
    pub fn run_conflict_detection_benchmark(&mut self) -> Result<BenchmarkResult> {
        println!("📊 Running Conflict Detection Benchmark...");

        let temp_dir = TempDir::new()?;

        // 创建源分支
        let source_dir = temp_dir.path().join("source");
        let source_branch = ContextBranch::new(
            "source",
            "source",
            "main",
            source_dir.clone(),
        )?;

        // 创建目标分支
        let target_dir = temp_dir.path().join("target");
        let target_branch = ContextBranch::new(
            "target",
            "target",
            "main",
            target_dir.clone(),
        )?;

        // 添加测试文件（部分有冲突）
        self.create_conflicting_files(&source_branch.short_term_dir, &target_branch.short_term_dir)?;

        let mut durations = Vec::new();
        let iterations = 20;

        for _ in 0..iterations {
            let start = Instant::now();

            let detector = BloomConflictDetector::new(
                &source_dir,
                &target_dir,
                "short-term",
            )?;

            let _conflicts = detector.detect_conflicts();

            let duration = start.elapsed();
            durations.push(duration);
        }

        let result = BenchmarkResult::new("Conflict Detection (Bloom)", &durations);
        println!("  {}\n", result);

        self.results.push(result.clone());
        Ok(result)
    }

    /// COW 基准测试
    pub fn run_cow_benchmark(&mut self) -> Result<BenchmarkResult> {
        println!("📊 Running Copy-on-Write Benchmark...");

        let temp_dir = TempDir::new()?;

        // 创建源分支
        let source_dir = temp_dir.path().join("cow_source");
        let source_branch = ContextBranch::new(
            "cow_source",
            "cow_source",
            "main",
            source_dir.clone(),
        )?;

        // 添加大量文件
        self.create_test_files(&source_branch.short_term_dir)?;

        // 创建 COW 管理器
        let cow_manager = CowManager::with_defaults();
        let cloner = BranchCloner::new(std::sync::Arc::new(cow_manager));

        let mut durations = Vec::new();
        let iterations = 10;

        for i in 0..iterations {
            let target_dir = temp_dir.path().join(format!("cow_target_{}", i));
            let _target_branch = ContextBranch::new(
                &format!("cow_target_{}", i),
                &format!("cow_target_{}", i),
                "main",
                target_dir.clone(),
            )?;

            let start = Instant::now();

            let _result = cloner.fork_with_layers(
                &source_dir,
                &target_dir,
                &["short-term", "long-term"],
            );

            let duration = start.elapsed();
            durations.push(duration);
        }

        let result = BenchmarkResult::new("COW Fork", &durations);
        println!("  {}\n", result);

        self.results.push(result.clone());
        Ok(result)
    }

    /// 创建测试文件
    fn create_test_files(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;

        for i in 0..self.config.files_per_branch {
            let file_path = dir.join(format!("file_{}.txt", i));
            let content = format!("Test content for file {}\n", i);
            
            // 填充到指定大小
            let mut content_bytes = content.into_bytes();
            while content_bytes.len() < self.config.file_size {
                content_bytes.push(b' ');
            }

            std::fs::write(&file_path, content_bytes)?;
        }

        Ok(())
    }

    /// 创建有冲突的测试文件
    fn create_conflicting_files(
        &self,
        source_dir: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        std::fs::create_dir_all(source_dir)?;
        std::fs::create_dir_all(target_dir)?;

        for i in 0..self.config.files_per_branch {
            let file_path = source_dir.join(format!("file_{}.txt", i));
            
            if i % 3 == 0 {
                // 1/3 的文件有冲突
                std::fs::write(&file_path, format!("Source content {}", i))?;
                
                let target_path = target_dir.join(format!("file_{}.txt", i));
                std::fs::write(&target_path, format!("Target content {}", i))?;
            } else {
                // 2/3 的文件相同
                let content = format!("Same content {}", i);
                std::fs::write(&file_path, &content)?;
                std::fs::write(target_dir.join(format!("file_{}.txt", i)), &content)?;
            }
        }

        Ok(())
    }

    /// 获取所有结果
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// 生成汇总报告
    pub fn summary_report(&self) -> String {
        let mut report = String::new();
        report.push_str("═══════════════════════════════════════════════════════\n");
        report.push_str("         PARALLEL CONTEXT BENCHMARK SUMMARY\n");
        report.push_str("═══════════════════════════════════════════════════════\n\n");

        for result in &self.results {
            report.push_str(&format!("{}", result));
            report.push('\n');
        }

        report.push_str("═══════════════════════════════════════════════════════\n");
        report.push_str("                    KEY METRICS\n");
        report.push_str("═══════════════════════════════════════════════════════\n\n");

        if let Some(fork_result) = self.results.iter().find(|r| r.name == "Fork Operation") {
            report.push_str(&format!(
                "✓ Fork Latency: {:.3} ms (target: <10 ms) - {}\n",
                fork_result.avg_latency_ms,
                if fork_result.avg_latency_ms < 10.0 { "✅ PASS" } else { "❌ FAIL" }
            ));
        }

        if let Some(merge_result) = self.results.iter().find(|r| r.name == "Merge Operation") {
            report.push_str(&format!(
                "✓ Merge Latency: {:.3} ms (target: <100 ms) - {}\n",
                merge_result.avg_latency_ms,
                if merge_result.avg_latency_ms < 100.0 { "✅ PASS" } else { "❌ FAIL" }
            ));
        }

        report.push_str("\n═══════════════════════════════════════════════════════\n");

        report
    }
}

/// 运行基准测试并输出结果
pub fn run_benchmarks() -> Result<()> {
    let config = BenchmarkConfig::default();
    let mut suite = BenchmarkSuite::new(config)?;

    suite.run_all()?;

    println!("{}", suite.summary_report());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_creation() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
        ];

        let result = BenchmarkResult::new("Test", &durations);

        assert_eq!(result.name, "Test");
        assert!((result.avg_latency_ms - 20.0).abs() < 0.1);
        assert!((result.min_latency_ms - 10.0).abs() < 0.1);
        assert!((result.max_latency_ms - 30.0).abs() < 0.1);
        assert!(result.iterations == 3);
    }

    #[test]
    fn test_benchmark_suite_creation() {
        let config = BenchmarkConfig {
            num_branches: 3,
            files_per_branch: 5,
            file_size: 100,
            enable_cache: true,
            enable_cow: true,
        };

        let mut suite = BenchmarkSuite::new(config).unwrap();
        let results = suite.run_all().unwrap();

        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.iterations > 0));
        assert!(results.iter().all(|r| r.avg_latency_ms > 0.0));
    }
}
