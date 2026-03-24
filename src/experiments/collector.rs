//! 实验数据收集模块
//!
//! 用于收集 HybridGapDetector 的性能指标和实验数据
//!
//! ## 使用示例
//! ```rust,ignore
//! let mut collector = ExperimentCollector::new("hybrid_gap_detector_test")?;
//! 
//! // 记录检测延迟
//! collector.record_metric("detection_latency_ms", 1234.5);
//! 
//! // 记录 API 调用
//! collector.record_api_call("causal_analysis", 0.015);
//! 
//! // 保存实验结果
//! collector.save_results()?;
//! ```

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// 实验配置
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// 实验名称
    pub name: String,
    /// 实验描述
    pub description: String,
    /// Git 提交哈希
    pub git_commit: String,
    /// 配置参数
    pub configuration: HashMap<String, serde_json::Value>,
}

/// 实验指标
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentMetrics {
    /// 检测延迟（毫秒）
    pub detection_latency_ms: Option<f64>,
    /// 每周期 API 调用次数
    pub api_calls_per_cycle: Option<u32>,
    /// API 成本（美元）
    pub api_cost_usd: Option<f64>,
    /// 缓存命中率
    pub cache_hit_rate: Option<f64>,
    /// 检测到的缺口数量
    pub gaps_detected: Option<u32>,
    /// 统计缺口数量
    pub statistical_gaps: Option<u32>,
    /// 因果缺口数量
    pub causal_gaps: Option<u32>,
    /// 任务完成率
    pub task_completion_rate: Option<f64>,
    /// 用户满意度
    pub avg_satisfaction: Option<f64>,
}

/// 实验结果
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// 实验名称
    pub experiment_name: String,
    /// 时间戳
    pub timestamp: String,
    /// Git 提交哈希
    pub git_commit: String,
    /// 指标
    pub metrics: ExperimentMetrics,
    /// 配置
    pub configuration: HashMap<String, serde_json::Value>,
    /// 备注
    pub notes: String,
}

/// 实验数据收集器
#[allow(dead_code)]
pub struct ExperimentCollector {
    /// 实验配置
    config: ExperimentConfig,
    /// 收集的指标
    metrics: ExperimentMetrics,
    /// API 调用记录
    api_calls: Vec<ApiCallRecord>,
    /// 数据目录
    data_dir: PathBuf,
}

/// API 调用记录
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallRecord {
    /// 调用类型
    pub call_type: String,
    /// 成本（美元）
    pub cost_usd: f64,
    /// 时间戳
    pub timestamp: u64,
}

#[allow(dead_code)]
impl ExperimentCollector {
    /// 创建新的收集器
    pub fn new(experiment_name: &str) -> Result<Self> {
        let data_dir = PathBuf::from("experiments/data");
        fs::create_dir_all(&data_dir)
            .with_context(|| "创建实验数据目录失败")?;

        // 获取 Git 提交哈希
        let git_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Self {
            config: ExperimentConfig {
                name: experiment_name.to_string(),
                description: String::new(),
                git_commit,
                configuration: HashMap::new(),
            },
            metrics: ExperimentMetrics::default(),
            api_calls: Vec::new(),
            data_dir,
        })
    }

    /// 记录通用指标
    pub fn record_metric(&mut self, name: &str, value: f64) {
        // 根据指标名称更新对应的字段
        match name {
            "detection_latency_ms" => self.metrics.detection_latency_ms = Some(value),
            "cache_hit_rate" => self.metrics.cache_hit_rate = Some(value),
            "task_completion_rate" => self.metrics.task_completion_rate = Some(value),
            "avg_satisfaction" => self.metrics.avg_satisfaction = Some(value),
            _ => {} // 忽略未知指标
        }
    }

    /// 记录 API 调用
    pub fn record_api_call(&mut self, call_type: &str, cost_usd: f64) {
        self.api_calls.push(ApiCallRecord {
            call_type: call_type.to_string(),
            cost_usd,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
    }

    /// 更新检测延迟
    pub fn update_detection_latency(&mut self, latency_ms: f64) {
        self.metrics.detection_latency_ms = Some(latency_ms);
    }

    /// 更新缺口统计
    pub fn update_gaps_detected(&mut self, total: u32, statistical: u32, causal: u32) {
        self.metrics.gaps_detected = Some(total);
        self.metrics.statistical_gaps = Some(statistical);
        self.metrics.causal_gaps = Some(causal);
    }

    /// 计算总 API 成本
    pub fn total_api_cost(&self) -> f64 {
        self.api_calls.iter().map(|call| call.cost_usd).sum()
    }

    /// 设置配置参数
    pub fn set_config<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)
            .with_context(|| format!("序列化配置失败：{}", key))?;
        self.config.configuration.insert(key.to_string(), json_value);
        Ok(())
    }

    /// 设置实验描述
    pub fn set_description(&mut self, description: &str) {
        self.config.description = description.to_string();
    }

    /// 添加备注
    pub fn add_notes(&mut self, notes: &str) {
        self.config.configuration
            .insert("notes".to_string(), serde_json::json!(notes));
    }

    /// 保存实验结果
    pub fn save_results(&self) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("{}_{}.json", self.config.name, timestamp);
        let filepath = self.data_dir.join(&filename);

        let result = ExperimentResult {
            experiment_name: self.config.name.clone(),
            timestamp,
            git_commit: self.config.git_commit.clone(),
            metrics: self.metrics.clone(),
            configuration: self.config.configuration.clone(),
            notes: String::new(),
        };

        let file = File::create(&filepath)
            .with_context(|| format!("创建结果文件失败：{:?}", filepath))?;
        let writer = BufWriter::new(file);
        
        serde_json::to_writer_pretty(writer, &result)
            .with_context(|| "序列化实验结果失败")?;

        Ok(filepath)
    }

    /// 加载历史实验结果
    pub fn load_historical_results(experiment_name: &str) -> Result<Vec<ExperimentResult>> {
        let data_dir = PathBuf::from("experiments/data");
        let mut results = Vec::new();

        if !data_dir.exists() {
            return Ok(results);
        }

        for entry in fs::read_dir(&data_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if filename.starts_with(experiment_name) {
                        let content = fs::read_to_string(&path)?;
                        if let Ok(result) = serde_json::from_str::<ExperimentResult>(&content) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        // 按时间戳排序
        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        Ok(results)
    }

    /// 生成实验报告
    pub fn generate_report(results: &[ExperimentResult]) -> ExperimentReport {
        let mut report = ExperimentReport::default();
        
        if results.is_empty() {
            return report;
        }

        report.total_experiments = results.len();
        
        // 计算平均指标
        let mut latency_sum = 0.0;
        let mut latency_count = 0;
        let mut cost_sum = 0.0;
        
        for result in results {
            if let Some(latency) = result.metrics.detection_latency_ms {
                latency_sum += latency;
                latency_count += 1;
            }
            if let Some(cost) = result.metrics.api_cost_usd {
                cost_sum += cost;
            }
        }

        if latency_count > 0 {
            report.avg_detection_latency_ms = Some(latency_sum / latency_count as f64);
        }
        
        report.total_api_cost_usd = Some(cost_sum);
        report.experiment_names = results.iter().map(|r| r.experiment_name.clone()).collect();
        
        report
    }
}

/// 实验报告
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperimentReport {
    /// 实验总数
    pub total_experiments: usize,
    /// 平均检测延迟
    pub avg_detection_latency_ms: Option<f64>,
    /// 总 API 成本
    pub total_api_cost_usd: Option<f64>,
    /// 实验名称列表
    pub experiment_names: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_collector() {
        let collector = ExperimentCollector::new("test_experiment");
        assert!(collector.is_ok());
    }

    #[test]
    fn test_record_metrics() {
        let mut collector = ExperimentCollector::new("test_metrics").unwrap();
        
        collector.record_metric("detection_latency_ms", 1234.5);
        collector.record_metric("cache_hit_rate", 0.75);
        
        assert_eq!(collector.metrics.detection_latency_ms, Some(1234.5));
        assert_eq!(collector.metrics.cache_hit_rate, Some(0.75));
    }

    #[test]
    fn test_record_api_calls() {
        let mut collector = ExperimentCollector::new("test_api").unwrap();
        
        collector.record_api_call("causal_analysis", 0.015);
        collector.record_api_call("causal_analysis", 0.015);
        collector.record_api_call("causal_analysis", 0.015);
        
        assert_eq!(collector.api_calls.len(), 3);
        assert!((collector.total_api_cost() - 0.045).abs() < 0.001);
    }

    #[test]
    fn test_save_and_load_results() {
        let mut collector = ExperimentCollector::new("test_save").unwrap();
        collector.update_detection_latency(500.0);
        collector.update_gaps_detected(10, 8, 2);
        collector.set_config("statistical_threshold", 0.5).unwrap();
        
        let filepath = collector.save_results().unwrap();
        assert!(filepath.exists());
        
        // 清理测试文件
        let _ = fs::remove_file(filepath);
    }

    #[test]
    fn test_generate_report() {
        let results = vec![
            ExperimentResult {
                experiment_name: "test1".to_string(),
                timestamp: "20260320_120000".to_string(),
                git_commit: "abc123".to_string(),
                metrics: ExperimentMetrics {
                    detection_latency_ms: Some(500.0),
                    api_cost_usd: Some(0.05),
                    ..Default::default()
                },
                configuration: HashMap::new(),
                notes: String::new(),
            },
            ExperimentResult {
                experiment_name: "test2".to_string(),
                timestamp: "20260320_130000".to_string(),
                git_commit: "abc123".to_string(),
                metrics: ExperimentMetrics {
                    detection_latency_ms: Some(600.0),
                    api_cost_usd: Some(0.06),
                    ..Default::default()
                },
                configuration: HashMap::new(),
                notes: String::new(),
            },
        ];

        let report = ExperimentCollector::generate_report(&results);
        
        assert_eq!(report.total_experiments, 2);
        assert_eq!(report.avg_detection_latency_ms, Some(550.0));
        assert_eq!(report.total_api_cost_usd, Some(0.11));
    }
}
