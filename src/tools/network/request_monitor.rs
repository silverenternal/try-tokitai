use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::VecDeque;
use chrono::{DateTime, Utc};

/// 请求统计信息
#[derive(Default, Clone, Debug)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_bytes: u64,
    pub avg_response_time_ms: f64,
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

/// 请求监控器 - 统一的请求日志和统计
pub struct RequestMonitor {
    stats: Arc<RwLock<RequestStats>>,
    logs: Arc<RwLock<VecDeque<RequestLog>>>,
    max_logs: usize,
}

impl RequestMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(RequestStats::default())),
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_logs: 1000,
        }
    }

    /// 记录请求结果
    pub fn record(&self, log: RequestLog) {
        let mut stats = self.stats.write();
        stats.total_requests += 1;

        if log.status >= 200 && log.status < 300 {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        stats.total_bytes += log.bytes;

        // 计算平均响应时间
        let total = stats.successful_requests + stats.failed_requests;
        stats.avg_response_time_ms =
            (stats.avg_response_time_ms * (total - 1) as f64 + log.duration_ms as f64)
                / total as f64;

        // 记录日志（保留最近 max_logs 条）
        let mut logs = self.logs.write();
        logs.push_back(log);
        if logs.len() > self.max_logs {
            logs.pop_front();
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

    /// 清空统计
    pub fn clear_stats(&self) {
        let mut stats = self.stats.write();
        stats.total_requests = 0;
        stats.successful_requests = 0;
        stats.failed_requests = 0;
        stats.total_bytes = 0;
        stats.avg_response_time_ms = 0.0;
    }

    /// 清空日志
    pub fn clear_logs(&self) {
        self.logs.write().clear();
    }
}

impl Default for RequestMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_monitor_record() {
        let monitor = RequestMonitor::new();

        monitor.record(RequestLog {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            status: 200,
            duration_ms: 100,
            bytes: 1024,
            timestamp: Utc::now(),
        });

        let stats = monitor.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.total_bytes, 1024);
    }

    #[test]
    fn test_request_monitor_failure_rate() {
        let monitor = RequestMonitor::new();

        monitor.record(RequestLog {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            status: 200,
            duration_ms: 100,
            bytes: 1024,
            timestamp: Utc::now(),
        });

        monitor.record(RequestLog {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            status: 500,
            duration_ms: 200,
            bytes: 0,
            timestamp: Utc::now(),
        });

        let failure_rate = monitor.get_failure_rate();
        assert!((failure_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_request_monitor_recent_logs() {
        let monitor = RequestMonitor::new();

        for i in 0..10 {
            monitor.record(RequestLog {
                url: format!("https://example.com/{}", i),
                method: "GET".to_string(),
                status: 200,
                duration_ms: 100,
                bytes: 1024,
                timestamp: Utc::now(),
            });
        }

        let logs = monitor.get_recent_logs(5);
        assert_eq!(logs.len(), 5);
        assert_eq!(logs[0].url, "https://example.com/9");
    }
}
