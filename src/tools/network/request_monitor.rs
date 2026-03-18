//! 请求监控模块
//!
//! 提供请求统计、日志记录和背压控制

use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::VecDeque;
use chrono::{DateTime, Utc};

// ============================================================================
// 数据结构
// ============================================================================

/// 请求统计信息
#[derive(Default, Clone, Debug)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_bytes: u64,
    pub avg_response_time_ms: f64,
    /// P50 响应时间 (ms)
    pub p50_latency_ms: f64,
    /// P95 响应时间 (ms)
    pub p95_latency_ms: f64,
    /// P99 响应时间 (ms)
    pub p99_latency_ms: f64,
}

/// 请求日志记录
#[derive(Clone, Debug)]
pub struct RequestLog {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub duration_ms: u128,
    pub bytes: u64,
    pub timestamp: DateTime<Utc>,
}

/// 背压配置
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// 最大日志队列长度
    pub max_queue_size: usize,
    /// 队列使用率警告阈值 (0.0 - 1.0)
    pub warning_threshold: f32,
    /// 队列使用率拒绝阈值 (0.0 - 1.0)
    pub reject_threshold: f32,
    /// 是否启用背压
    pub enabled: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            warning_threshold: 0.8,
            reject_threshold: 0.95,
            enabled: true,
        }
    }
}

/// 背压状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureStatus {
    Normal,
    Warning,
    Critical,
}

impl BackpressureStatus {
    pub fn is_accepting(&self) -> bool {
        match self {
            BackpressureStatus::Normal | BackpressureStatus::Warning => true,
            BackpressureStatus::Critical => false,
        }
    }
}

// ============================================================================
// 请求监控器
// ============================================================================

/// 请求监控器 - 统一的请求日志和统计
pub struct RequestMonitor {
    stats: Arc<RwLock<RequestStats>>,
    logs: Arc<RwLock<VecDeque<RequestLog>>>,
    latencies: Arc<RwLock<VecDeque<f64>>>,
    config: BackpressureConfig,
    current_status: Arc<RwLock<BackpressureStatus>>,
}

impl RequestMonitor {
    pub fn new() -> Self {
        Self::with_config(BackpressureConfig::default())
    }

    pub fn with_config(config: BackpressureConfig) -> Self {
        Self {
            stats: Arc::new(RwLock::new(RequestStats::default())),
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_queue_size))),
            latencies: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            config,
            current_status: Arc::new(RwLock::new(BackpressureStatus::Normal)),
        }
    }

    /// 记录请求结果
    pub fn record(&self, log: RequestLog) {
        // 检查背压状态
        if self.config.enabled {
            let status = *self.current_status.read();
            if !status.is_accepting() {
                tracing::warn!("背压保护：拒绝记录请求（队列已满）");
                return;
            }
        }

        let duration_ms = log.duration_ms as f64;
        let status = log.status;
        let bytes = log.bytes;

        let mut stats = self.stats.write();
        stats.total_requests += 1;

        if status >= 200 && status < 300 {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        stats.total_bytes += bytes;

        // 更新平均响应时间
        let total = stats.successful_requests + stats.failed_requests;
        stats.avg_response_time_ms =
            (stats.avg_response_time_ms * (total - 1) as f64 + duration_ms)
                / total as f64;

        // 记录日志（保留最近 max_logs 条）
        let mut logs = self.logs.write();

        // 背压检查 - 如果队列已满，移除最旧的条目
        if self.config.enabled && logs.len() >= self.config.max_queue_size {
            logs.pop_front();
        }

        logs.push_back(log);

        // 更新背压状态（在添加新日志后）
        if self.config.enabled {
            self.update_backpressure_status(logs.len());
        }

        // 记录延迟用于百分位数计算
        let mut latencies = self.latencies.write();
        latencies.push_back(duration_ms);
        if latencies.len() > 1000 {
            latencies.pop_front();
        }
        
        // 更新百分位数
        self.update_percentile_stats(&mut stats, &latencies);
    }

    /// 更新百分位数统计
    fn update_percentile_stats(&self, stats: &mut RequestStats, latencies: &VecDeque<f64>) {
        if latencies.is_empty() {
            return;
        }

        let mut sorted: Vec<f64> = latencies.iter().cloned().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = sorted.len();
        stats.p50_latency_ms = sorted[len * 50 / 100];
        stats.p95_latency_ms = sorted[len * 95 / 100];
        stats.p99_latency_ms = sorted[len * 99 / 100];
    }

    /// 更新背压状态
    fn update_backpressure_status(&self, queue_len: usize) {
        let ratio = queue_len as f32 / self.config.max_queue_size as f32;
        
        let new_status = if ratio >= self.config.reject_threshold {
            BackpressureStatus::Critical
        } else if ratio >= self.config.warning_threshold {
            BackpressureStatus::Warning
        } else {
            BackpressureStatus::Normal
        };

        let mut status = self.current_status.write();
        if *status != new_status {
            tracing::info!("背压状态变更：{:?} -> {:?}", *status, new_status);
            *status = new_status;
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> RequestStats {
        self.stats.read().clone()
    }

    /// 获取最近请求日志
    pub fn get_recent_logs(&self, limit: usize) -> Vec<RequestLog> {
        self.logs.read().iter().rev().take(limit).cloned().collect()
    }

    /// 获取失败率
    pub fn get_failure_rate(&self) -> f64 {
        let stats = self.stats.read();
        if stats.total_requests == 0 {
            0.0
        } else {
            stats.failed_requests as f64 / stats.total_requests as f64
        }
    }

    /// 获取背压状态
    pub fn get_backpressure_status(&self) -> BackpressureStatus {
        *self.current_status.read()
    }

    /// 检查是否可以接受新请求
    pub fn can_accept_request(&self) -> bool {
        if !self.config.enabled {
            return true;
        }
        
        let status = self.get_backpressure_status();
        status.is_accepting()
    }

    /// 获取队列使用率
    pub fn get_queue_usage(&self) -> f32 {
        let logs = self.logs.read();
        logs.len() as f32 / self.config.max_queue_size as f32
    }

    /// 清空统计
    pub fn clear_stats(&self) {
        let mut stats = self.stats.write();
        stats.total_requests = 0;
        stats.successful_requests = 0;
        stats.failed_requests = 0;
        stats.total_bytes = 0;
        stats.avg_response_time_ms = 0.0;
        stats.p50_latency_ms = 0.0;
        stats.p95_latency_ms = 0.0;
        stats.p99_latency_ms = 0.0;
    }

    /// 清空日志
    pub fn clear_logs(&self) {
        self.logs.write().clear();
        self.latencies.write().clear();
        *self.current_status.write() = BackpressureStatus::Normal;
    }

    /// 获取延迟直方图
    pub fn get_latency_histogram(&self, buckets: usize) -> Vec<(f64, usize)> {
        let latencies = self.latencies.read();
        if latencies.is_empty() {
            return vec![];
        }

        let min = latencies.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = latencies.iter().cloned().fold(0.0, f64::max);
        
        if min == max {
            return vec![(min, latencies.len())];
        }

        let bucket_size = (max - min) / buckets as f64;
        let mut histogram = vec![0; buckets];

        for &latency in latencies.iter() {
            let bucket_idx = ((latency - min) / bucket_size).min((buckets - 1) as f64) as usize;
            histogram[bucket_idx] += 1;
        }

        (0..buckets)
            .map(|i| (min + i as f64 * bucket_size, histogram[i]))
            .collect()
    }
}

impl Default for RequestMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_log(status: u16, duration_ms: u128) -> RequestLog {
        RequestLog {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            status,
            duration_ms,
            bytes: 1024,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_request_monitor_record() {
        let monitor = RequestMonitor::new();

        monitor.record(create_test_log(200, 100));

        let stats = monitor.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.total_bytes, 1024);
    }

    #[test]
    fn test_request_monitor_failure_rate() {
        let monitor = RequestMonitor::new();

        monitor.record(create_test_log(200, 100));
        monitor.record(create_test_log(500, 200));

        let failure_rate = monitor.get_failure_rate();
        assert!((failure_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_request_monitor_recent_logs() {
        let monitor = RequestMonitor::new();

        for i in 0..10 {
            monitor.record(create_test_log(200, 100 + i as u128 * 10));
        }

        let logs = monitor.get_recent_logs(5);
        assert_eq!(logs.len(), 5);
        // 最新的应该在前面
        assert_eq!(logs[0].duration_ms, 190);
    }

    #[test]
    fn test_backpressure_status() {
        let config = BackpressureConfig {
            max_queue_size: 10,
            warning_threshold: 0.8,
            reject_threshold: 0.95,
            enabled: true,
        };

        let monitor = RequestMonitor::with_config(config);

        // 初始状态应该是 Normal
        assert_eq!(monitor.get_backpressure_status(), BackpressureStatus::Normal);

        // 填充队列到 80%
        for _ in 0..8 {
            monitor.record(create_test_log(200, 100));
        }
        assert_eq!(monitor.get_backpressure_status(), BackpressureStatus::Warning);

        // 填充队列到 95%
        for _ in 0..2 {
            monitor.record(create_test_log(200, 100));
        }
        assert_eq!(monitor.get_backpressure_status(), BackpressureStatus::Critical);

        // 在 Critical 状态下，新请求应该被拒绝
        assert!(!monitor.can_accept_request());
    }

    #[test]
    fn test_queue_usage() {
        let config = BackpressureConfig {
            max_queue_size: 100,
            ..Default::default()
        };

        let monitor = RequestMonitor::with_config(config);

        assert_eq!(monitor.get_queue_usage(), 0.0);

        for _ in 0..50 {
            monitor.record(create_test_log(200, 100));
        }

        assert!((monitor.get_queue_usage() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_percentile_stats() {
        let monitor = RequestMonitor::new();

        // 记录 100 个请求，延迟从 1 到 100
        for i in 1..=100 {
            monitor.record(create_test_log(200, i as u128));
        }

        let stats = monitor.get_stats();
        
        // P50 应该在 50 左右
        assert!(stats.p50_latency_ms >= 49.0 && stats.p50_latency_ms <= 51.0);
        // P95 应该在 95 左右
        assert!(stats.p95_latency_ms >= 94.0 && stats.p95_latency_ms <= 96.0);
        // P99 应该在 99 左右
        assert!(stats.p99_latency_ms >= 98.0 && stats.p99_latency_ms <= 100.0);
    }

    #[test]
    fn test_latency_histogram() {
        let monitor = RequestMonitor::new();

        // 记录一些不同延迟的请求
        for i in 1..=10 {
            monitor.record(create_test_log(200, i as u128 * 10));
        }

        let histogram = monitor.get_latency_histogram(5);
        assert_eq!(histogram.len(), 5);
        
        // 所有桶的计数总和应该等于请求数
        let total: usize = histogram.iter().map(|(_, count)| count).sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn test_clear_stats() {
        let monitor = RequestMonitor::new();

        monitor.record(create_test_log(200, 100));
        monitor.clear_stats();

        let stats = monitor.get_stats();
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_clear_logs() {
        let monitor = RequestMonitor::new();

        for _ in 0..10 {
            monitor.record(create_test_log(200, 100));
        }

        monitor.clear_logs();

        let logs = monitor.get_recent_logs(100);
        assert_eq!(logs.len(), 0);
        assert_eq!(monitor.get_backpressure_status(), BackpressureStatus::Normal);
    }
}
