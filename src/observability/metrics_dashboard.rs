//! 性能指标仪表盘
//!
//! 实时监控系统性能和 AI 行为指标
//!
//! ## 指标分类
//! - Latency: 用户输入到首 token、工具调用延迟、迭代周期时间
//! - Throughput: 每分钟请求数、每请求工具数、每小时迭代数
//! - Quality: 任务完成率、迭代成功率、用户满意度

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;
use anyhow::Result;

/// 时间序列数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// 时间戳
    pub timestamp: u64,
    /// 值
    pub value: f64,
}

/// 延迟指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// 用户输入到首 token（ms）
    pub input_to_first_token: VecDeque<TimeSeriesPoint>,
    /// 工具调用延迟（ms）
    pub tool_call_latency: VecDeque<TimeSeriesPoint>,
    /// 迭代周期时间（秒）
    pub iteration_cycle_time: VecDeque<TimeSeriesPoint>,
}

/// 吞吐量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// 每分钟请求数
    pub requests_per_minute: VecDeque<TimeSeriesPoint>,
    /// 每请求工具数
    pub tools_per_request: VecDeque<TimeSeriesPoint>,
    /// 每小时迭代数
    pub iterations_per_hour: VecDeque<TimeSeriesPoint>,
}

/// 质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// 任务完成率（0.0-1.0）
    pub task_completion_rate: VecDeque<TimeSeriesPoint>,
    /// 迭代成功率（0.0-1.0）
    pub iteration_success_rate: VecDeque<TimeSeriesPoint>,
    /// 用户满意度（1-5）
    pub user_satisfaction_score: VecDeque<TimeSeriesPoint>,
}

/// 性能指标仪表盘
pub struct MetricsDashboard {
    /// 数据目录
    data_dir: PathBuf,
    /// 延迟指标
    latency: LatencyMetrics,
    /// 吞吐量指标
    throughput: ThroughputMetrics,
    /// 质量指标
    quality: QualityMetrics,
    /// 配置
    config: DashboardConfig,
}

/// 仪表盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// 数据点保留数量
    pub max_data_points: usize,
    /// 采样间隔（秒）
    pub sampling_interval_seconds: u64,
    /// 是否自动保存
    pub auto_save_enabled: bool,
    /// 自动保存间隔（秒）
    pub auto_save_interval_seconds: u64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            max_data_points: 1000,
            sampling_interval_seconds: 1,
            auto_save_enabled: true,
            auto_save_interval_seconds: 60,
        }
    }
}

impl MetricsDashboard {
    /// 创建新的仪表盘
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        
        Ok(Self {
            data_dir,
            latency: LatencyMetrics {
                input_to_first_token: VecDeque::new(),
                tool_call_latency: VecDeque::new(),
                iteration_cycle_time: VecDeque::new(),
            },
            throughput: ThroughputMetrics {
                requests_per_minute: VecDeque::new(),
                tools_per_request: VecDeque::new(),
                iterations_per_hour: VecDeque::new(),
            },
            quality: QualityMetrics {
                task_completion_rate: VecDeque::new(),
                iteration_success_rate: VecDeque::new(),
                user_satisfaction_score: VecDeque::new(),
            },
            config: DashboardConfig::default(),
        })
    }

    /// 记录延迟指标
    pub fn record_latency(
        &mut self,
        input_to_first_token_ms: Option<f64>,
        tool_call_latency_ms: Option<f64>,
        iteration_cycle_time_s: Option<f64>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if let Some(value) = input_to_first_token_ms {
            self.latency.input_to_first_token.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        if let Some(value) = tool_call_latency_ms {
            self.latency.tool_call_latency.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        if let Some(value) = iteration_cycle_time_s {
            self.latency.iteration_cycle_time.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        
        self.trim_data_points();
    }

    /// 记录吞吐量指标
    pub fn record_throughput(
        &mut self,
        requests_per_minute: Option<f64>,
        tools_per_request: Option<f64>,
        iterations_per_hour: Option<f64>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if let Some(value) = requests_per_minute {
            self.throughput.requests_per_minute.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        if let Some(value) = tools_per_request {
            self.throughput.tools_per_request.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        if let Some(value) = iterations_per_hour {
            self.throughput.iterations_per_hour.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        
        self.trim_data_points();
    }

    /// 记录质量指标
    pub fn record_quality(
        &mut self,
        task_completion_rate: Option<f64>,
        iteration_success_rate: Option<f64>,
        user_satisfaction: Option<f64>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if let Some(value) = task_completion_rate {
            self.quality.task_completion_rate.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        if let Some(value) = iteration_success_rate {
            self.quality.iteration_success_rate.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        if let Some(value) = user_satisfaction {
            self.quality.user_satisfaction_score.push_back(TimeSeriesPoint { timestamp: now, value });
        }
        
        self.trim_data_points();
    }

    /// 裁剪数据点
    fn trim_data_points(&mut self) {
        while self.latency.input_to_first_token.len() > self.config.max_data_points {
            self.latency.input_to_first_token.pop_front();
        }
        while self.latency.tool_call_latency.len() > self.config.max_data_points {
            self.latency.tool_call_latency.pop_front();
        }
        while self.latency.iteration_cycle_time.len() > self.config.max_data_points {
            self.latency.iteration_cycle_time.pop_front();
        }
        while self.throughput.requests_per_minute.len() > self.config.max_data_points {
            self.throughput.requests_per_minute.pop_front();
        }
        while self.throughput.tools_per_request.len() > self.config.max_data_points {
            self.throughput.tools_per_request.pop_front();
        }
        while self.throughput.iterations_per_hour.len() > self.config.max_data_points {
            self.throughput.iterations_per_hour.pop_front();
        }
        while self.quality.task_completion_rate.len() > self.config.max_data_points {
            self.quality.task_completion_rate.pop_front();
        }
        while self.quality.iteration_success_rate.len() > self.config.max_data_points {
            self.quality.iteration_success_rate.pop_front();
        }
        while self.quality.user_satisfaction_score.len() > self.config.max_data_points {
            self.quality.user_satisfaction_score.pop_front();
        }
    }

    /// 获取最新指标摘要
    pub fn get_summary(&self) -> MetricsSummary {
        MetricsSummary {
            avg_input_to_first_token_ms: self.average(&self.latency.input_to_first_token),
            avg_tool_call_latency_ms: self.average(&self.latency.tool_call_latency),
            avg_iteration_cycle_time_s: self.average(&self.latency.iteration_cycle_time),
            avg_requests_per_minute: self.average(&self.throughput.requests_per_minute),
            avg_tools_per_request: self.average(&self.throughput.tools_per_request),
            avg_iterations_per_hour: self.average(&self.throughput.iterations_per_hour),
            avg_task_completion_rate: self.average(&self.quality.task_completion_rate),
            avg_iteration_success_rate: self.average(&self.quality.iteration_success_rate),
            avg_user_satisfaction: self.average(&self.quality.user_satisfaction_score),
        }
    }

    /// 获取延迟指标
    pub fn get_latency_metrics(&self) -> &LatencyMetrics {
        &self.latency
    }

    /// 获取吞吐量指标
    pub fn get_throughput_metrics(&self) -> &ThroughputMetrics {
        &self.throughput
    }

    /// 获取质量指标
    pub fn get_quality_metrics(&self) -> &QualityMetrics {
        &self.quality
    }

    /// 保存指标到文件
    pub fn save_metrics(&self) -> Result<()> {
        let file_path = self.data_dir.join("metrics.json");
        let data = MetricsData {
            latency: self.latency.clone(),
            throughput: self.throughput.clone(),
            quality: self.quality.clone(),
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(file_path, json)?;
        
        Ok(())
    }

    /// 加载指标
    pub fn load_metrics(&mut self) -> Result<()> {
        let file_path = self.data_dir.join("metrics.json");
        if file_path.exists() {
            let json = std::fs::read_to_string(file_path)?;
            let data: MetricsData = serde_json::from_str(&json)?;
            self.latency = data.latency;
            self.throughput = data.throughput;
            self.quality = data.quality;
        }
        Ok(())
    }

    /// 清空所有指标
    pub fn clear(&mut self) {
        self.latency.input_to_first_token.clear();
        self.latency.tool_call_latency.clear();
        self.latency.iteration_cycle_time.clear();
        self.throughput.requests_per_minute.clear();
        self.throughput.tools_per_request.clear();
        self.throughput.iterations_per_hour.clear();
        self.quality.task_completion_rate.clear();
        self.quality.iteration_success_rate.clear();
        self.quality.user_satisfaction_score.clear();
    }

    /// 添加数据点
    fn push_point(&mut self, deque: &mut VecDeque<TimeSeriesPoint>, timestamp: u64, value: f64) {
        deque.push_back(TimeSeriesPoint { timestamp, value });
        while deque.len() > self.config.max_data_points {
            deque.pop_front();
        }
    }

    /// 计算平均值
    fn average(&self, deque: &VecDeque<TimeSeriesPoint>) -> f64 {
        if deque.is_empty() {
            return 0.0;
        }
        let sum: f64 = deque.iter().map(|p| p.value).sum();
        sum / deque.len() as f64
    }
}

/// 指标数据（用于序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsData {
    latency: LatencyMetrics,
    throughput: ThroughputMetrics,
    quality: QualityMetrics,
    saved_at: u64,
}

/// 指标摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub avg_input_to_first_token_ms: f64,
    pub avg_tool_call_latency_ms: f64,
    pub avg_iteration_cycle_time_s: f64,
    pub avg_requests_per_minute: f64,
    pub avg_tools_per_request: f64,
    pub avg_iterations_per_hour: f64,
    pub avg_task_completion_rate: f64,
    pub avg_iteration_success_rate: f64,
    pub avg_user_satisfaction: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dashboard_creation() {
        let temp_dir = TempDir::new().unwrap();
        let dashboard = MetricsDashboard::new(temp_dir.path()).unwrap();
        assert!(dashboard.get_summary().avg_input_to_first_token_ms == 0.0);
    }

    #[test]
    fn test_record_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let mut dashboard = MetricsDashboard::new(temp_dir.path()).unwrap();
        
        dashboard.record_latency(Some(100.0), Some(50.0), Some(10.0));
        dashboard.record_quality(Some(0.9), Some(0.8), Some(4.5));
        
        let summary = dashboard.get_summary();
        assert_eq!(summary.avg_input_to_first_token_ms, 100.0);
        assert_eq!(summary.avg_tool_call_latency_ms, 50.0);
        assert_eq!(summary.avg_user_satisfaction, 4.5);
    }
}
