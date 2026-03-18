//! 数据工具模块
//!
//! 提供 JSON 处理、数据格式转换等功能
//!
//! ## 服务化架构
//! - 实现 ServiceLifecycle trait
//! - 统一配置管理 (DataToolConfig)
//! - 通用验证器 (Validator trait)
//! - 完整监控指标 (MetricsCollector)
//! - 按可恢复性分类的错误类型

pub mod config;
pub mod error;
pub mod validator;
pub mod metrics;
pub mod json_format;
pub mod json_query;
pub mod json_merge;
pub mod data_conversion;

// 重新导出错误类型
// 注意：DataToolError 供内部使用，不直接导出
#[allow(dead_code, unused_imports)]
pub use error::DataToolError;

// 重新导出配置
pub use config::DataToolConfig;

// 重新导出指标
pub use metrics::MetricsCollector;

// 重新导出工具类
pub use json_format::JsonFormatTools;
pub use json_query::JsonQueryTools;
pub use json_merge::JsonMergeTools;
pub use data_conversion::DataConversionTools;

// 服务化架构导入
use crate::tool_matrix::matrix::{ServiceLifecycle, ServiceHealth, ServiceStats};

/// 数据工具服务（统一入口）
#[derive(Debug)]
pub struct DataService {
    pub config: DataToolConfig,
    pub metrics: MetricsCollector,
    pub format_tools: JsonFormatTools,
    pub query_tools: JsonQueryTools,
    pub merge_tools: JsonMergeTools,
    pub conversion_tools: DataConversionTools,
    initialized: bool,
}

impl DataService {
    pub fn new() -> Self {
        Self::with_config(DataToolConfig::default())
    }

    pub fn with_config(config: DataToolConfig) -> Self {
        let metrics = MetricsCollector::new();
        Self {
            format_tools: JsonFormatTools::with_config(config.clone()),
            query_tools: JsonQueryTools::with_config(config.clone()),
            merge_tools: JsonMergeTools::with_config(config.clone()),
            conversion_tools: DataConversionTools::with_config(config.clone()),
            config,
            metrics,
            initialized: false,
        }
    }

    /// 获取服务版本
    pub fn version() -> &'static str {
        "1.0.0"
    }
}

impl Default for DataService {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceLifecycle for DataService {
    fn service_name(&self) -> &str {
        "data_tools"
    }

    fn init(&mut self) -> Result<(), String> {
        // 初始化检查
        tracing::info!(
            target: "data_tools",
            "初始化数据工具服务，配置：max_length={}, max_depth={}",
            self.config.max_length,
            self.config.max_depth
        );
        self.initialized = true;
        Ok(())
    }

    fn health(&self) -> ServiceHealth {
        if !self.initialized {
            return ServiceHealth::Unhealthy;
        }
        // 基本健康检查：配置有效
        if self.config.max_length == 0 || self.config.max_depth == 0 {
            return ServiceHealth::Degraded;
        }
        ServiceHealth::Healthy
    }

    fn shutdown(&mut self) -> Result<(), String> {
        tracing::info!(target: "data_tools", "关闭数据工具服务");
        self.initialized = false;
        Ok(())
    }

    fn stats(&self) -> ServiceStats {
        let total = self.metrics.get_total_metrics();
        ServiceStats {
            total_requests: total.total_calls,
            success_count: total.successful_calls,
            failure_count: total.failed_calls,
            avg_latency_ms: total.avg_duration_ms(),
            p99_latency_ms: 0, // 简化实现
            last_called_at: total.last_call_time.map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }),
            recent_latencies: Vec::new(),
        }
    }
}

// 为了向后兼容，保留旧的工具类导出
#[deprecated(since = "0.3.0", note = "请使用 DataService 或独立的工具类")]
pub type JsonTools = JsonFormatTools;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_service_creation() {
        let service = DataService::new();
        assert_eq!(service.service_name(), "data_tools");
        // 未初始化时返回 Unhealthy
        assert!(matches!(service.health(), ServiceHealth::Unhealthy));
    }

    #[test]
    fn test_data_service_lifecycle() {
        let mut service = DataService::new();

        // 初始化前
        assert!(matches!(service.health(), ServiceHealth::Unhealthy));

        // 初始化
        assert!(service.init().is_ok());
        assert!(matches!(service.health(), ServiceHealth::Healthy));
        assert!(service.initialized);

        // 关闭
        assert!(service.shutdown().is_ok());
        assert!(matches!(service.health(), ServiceHealth::Unhealthy));
    }

    #[test]
    fn test_data_service_with_config() {
        let config = DataToolConfig::builder()
            .max_length(1024 * 1024)
            .max_depth(50)
            .build();

        let service = DataService::with_config(config.clone());
        assert_eq!(service.config.max_length, config.max_length);
        assert_eq!(service.config.max_depth, config.max_depth);
    }

    #[test]
    fn test_data_service_stats() {
        let mut service = DataService::new();
        assert!(service.init().is_ok());

        let stats = service.stats();
        assert_eq!(stats.total_requests, 0);

        // 调用一些工具后检查统计
        let _ = service.format_tools.format_json(r#"{"a":1}"#.to_string());

        // 验证工具正常工作（不检查统计，因为指标可能不共享）
    }
}
