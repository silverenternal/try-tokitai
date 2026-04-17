//! 数据工具监控指标
//!
//! 对齐项目中的 SelectorMetrics 模式，提供完整的监控链路

#![allow(dead_code)]

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据工具监控指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataToolMetrics {
    /// 总调用次数
    pub total_calls: u64,
    /// 成功调用次数
    pub successful_calls: u64,
    /// 失败调用次数
    pub failed_calls: u64,
    /// 重试次数
    pub retry_count: u64,
    /// 总耗时 (毫秒)
    pub total_duration_ms: u64,
    /// 最近一次调用时间戳
    pub last_call_time: Option<u64>,
    /// 最近一次错误信息
    pub last_error: Option<String>,
}

impl DataToolMetrics {
    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.successful_calls as f64 / self.total_calls as f64
    }

    /// 计算平均耗时 (毫秒)
    pub fn avg_duration_ms(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.total_duration_ms as f64 / self.total_calls as f64
    }

    /// 计算失败率
    pub fn failure_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.failed_calls as f64 / self.total_calls as f64
    }

    /// 计算重试率
    pub fn retry_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.retry_count as f64 / self.total_calls as f64
    }
}

/// 操作类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataToolOperation {
    FormatJson,
    MinifyJson,
    ValidateJson,
    QueryJson,
    ExtractKeys,
    MergeJson,
    JsonToCsv,
}

impl DataToolOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataToolOperation::FormatJson => "format_json",
            DataToolOperation::MinifyJson => "minify_json",
            DataToolOperation::ValidateJson => "validate_json",
            DataToolOperation::QueryJson => "query_json",
            DataToolOperation::ExtractKeys => "extract_keys",
            DataToolOperation::MergeJson => "merge_json",
            DataToolOperation::JsonToCsv => "json_to_csv",
        }
    }
}

/// 指标收集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// 按操作类型统计的指标
    metrics: Arc<RwLock<HashMap<DataToolOperation, DataToolMetrics>>>,
    /// 总体指标
    total: Arc<RwLock<DataToolMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            total: Arc::new(RwLock::new(DataToolMetrics::default())),
        }
    }

    /// 记录调用开始
    pub fn start_call(&self, operation: DataToolOperation) -> CallTimer {
        CallTimer::new(self.clone(), operation)
    }

    /// 记录成功调用
    pub fn record_success(&self, operation: DataToolOperation, duration_ms: u64) {
        // 更新操作指标
        {
            let mut metrics = self.metrics.write();
            let entry = metrics.entry(operation).or_default();
            entry.total_calls += 1;
            entry.successful_calls += 1;
            entry.total_duration_ms += duration_ms;
            entry.last_call_time = Some(current_timestamp());
        }

        // 更新总体指标
        {
            let mut total = self.total.write();
            total.total_calls += 1;
            total.successful_calls += 1;
            total.total_duration_ms += duration_ms;
            total.last_call_time = Some(current_timestamp());
        }
    }

    /// 记录失败调用
    pub fn record_failure(&self, operation: DataToolOperation, duration_ms: u64, error: &str) {
        // 更新操作指标
        {
            let mut metrics = self.metrics.write();
            let entry = metrics.entry(operation).or_default();
            entry.total_calls += 1;
            entry.failed_calls += 1;
            entry.total_duration_ms += duration_ms;
            entry.last_call_time = Some(current_timestamp());
            entry.last_error = Some(error.to_string());
        }

        // 更新总体指标
        {
            let mut total = self.total.write();
            total.total_calls += 1;
            total.failed_calls += 1;
            total.total_duration_ms += duration_ms;
            total.last_call_time = Some(current_timestamp());
            total.last_error = Some(error.to_string());
        }
    }

    /// 记录重试
    pub fn record_retry(&self, operation: DataToolOperation) {
        let mut metrics = self.metrics.write();
        let entry = metrics.entry(operation).or_default();
        entry.retry_count += 1;

        let mut total = self.total.write();
        total.retry_count += 1;
    }

    /// 获取操作指标
    pub fn get_metrics(&self, operation: DataToolOperation) -> Option<DataToolMetrics> {
        self.metrics.read().get(&operation).cloned()
    }

    /// 获取所有操作指标
    pub fn get_all_metrics(&self) -> HashMap<DataToolOperation, DataToolMetrics> {
        self.metrics.read().clone()
    }

    /// 获取总体指标
    pub fn get_total_metrics(&self) -> DataToolMetrics {
        self.total.read().clone()
    }

    /// 重置所有指标
    pub fn reset(&self) {
        self.metrics.write().clear();
        *self.total.write() = DataToolMetrics::default();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 调用计时器 - RAII 模式
pub struct CallTimer {
    collector: MetricsCollector,
    operation: DataToolOperation,
    start_time: Instant,
}

impl CallTimer {
    fn new(collector: MetricsCollector, operation: DataToolOperation) -> Self {
        Self {
            collector,
            operation,
            start_time: Instant::now(),
        }
    }

    /// 记录成功
    pub fn success(self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        self.collector.record_success(self.operation, duration_ms);
    }

    /// 记录失败
    pub fn failure(self, error: &str) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        self.collector
            .record_failure(self.operation, duration_ms, error);
    }
}

/// 获取当前时间戳（秒）
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // 记录成功调用
        collector.record_success(DataToolOperation::FormatJson, 10);
        collector.record_success(DataToolOperation::FormatJson, 20);

        // 记录失败调用
        collector.record_failure(DataToolOperation::FormatJson, 5, "test error");

        let metrics = collector
            .get_metrics(DataToolOperation::FormatJson)
            .unwrap();
        assert_eq!(metrics.total_calls, 3);
        assert_eq!(metrics.successful_calls, 2);
        assert_eq!(metrics.failed_calls, 1);
        assert_eq!(metrics.total_duration_ms, 35);
        assert!(metrics.last_error.is_some());
    }

    #[test]
    fn test_call_timer() {
        let collector = MetricsCollector::new();

        {
            let timer = collector.start_call(DataToolOperation::MinifyJson);
            timer.success();
        }

        {
            let timer = collector.start_call(DataToolOperation::MinifyJson);
            timer.failure("test error");
        }

        let metrics = collector
            .get_metrics(DataToolOperation::MinifyJson)
            .unwrap();
        assert_eq!(metrics.total_calls, 2);
        assert_eq!(metrics.successful_calls, 1);
        assert_eq!(metrics.failed_calls, 1);
    }

    #[test]
    fn test_metrics_calculations() {
        let mut metrics = DataToolMetrics {
            total_calls: 100,
            successful_calls: 90,
            failed_calls: 10,
            retry_count: 5,
            total_duration_ms: 1000,
            ..Default::default()
        };

        assert!((metrics.success_rate() - 0.9).abs() < 0.001);
        assert!((metrics.failure_rate() - 0.1).abs() < 0.001);
        assert!((metrics.avg_duration_ms() - 10.0).abs() < 0.001);
        assert!((metrics.retry_rate() - 0.05).abs() < 0.001);

        // 测试零调用情况
        metrics = DataToolMetrics::default();
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.failure_rate(), 0.0);
        assert_eq!(metrics.avg_duration_ms(), 0.0);
    }

    #[test]
    fn test_total_metrics() {
        let collector = MetricsCollector::new();

        collector.record_success(DataToolOperation::FormatJson, 10);
        collector.record_success(DataToolOperation::QueryJson, 20);
        collector.record_failure(DataToolOperation::MergeJson, 5, "error");

        let total = collector.get_total_metrics();
        assert_eq!(total.total_calls, 3);
        assert_eq!(total.successful_calls, 2);
        assert_eq!(total.failed_calls, 1);
    }
}
