//! HTTP 客户端工具集
//!
//! 提供统一的 HTTP 请求功能，支持 SSRF 防护、请求监控和配置化管理

use reqwest::blocking::{Client, Response};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokitai::tool;

use super::{
    error::{NetworkResult, HttpError},
    ssrf_protection::{self, RuntimeSsrfConfig},
    request_monitor::{RequestLog, RequestMonitor},
};
use crate::tool_matrix::matrix::{ServiceHealth, ServiceLifecycle, ServiceStats};

// ============================================================================
// 配置结构
// ============================================================================

/// HTTP 客户端配置
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// 连接超时（秒）
    pub connect_timeout_secs: u64,
    /// 连接池最大空闲连接数
    pub pool_max_idle_per_host: usize,
    /// 空闲连接超时（秒）
    pub pool_idle_timeout_secs: u64,
    /// TCP Keep-Alive（秒）
    pub tcp_keepalive_secs: u64,
    /// 最大重定向次数
    pub max_redirects: usize,
    /// User-Agent
    pub user_agent: String,
    /// SSRF 防护配置
    pub ssrf_config: RuntimeSsrfConfig,
    /// 是否启用请求监控
    pub enable_monitoring: bool,
    /// 最大响应体大小（字节）
    pub max_response_size: usize,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            connect_timeout_secs: 10,
            pool_max_idle_per_host: 10,
            pool_idle_timeout_secs: 90,
            tcp_keepalive_secs: 30,
            max_redirects: 5,
            user_agent: "Tokitai AI Assistant/1.0".to_string(),
            ssrf_config: RuntimeSsrfConfig::default(),
            enable_monitoring: true,
            max_response_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

// ============================================================================
// HTTP 客户端工具集
// ============================================================================

/// HTTP 客户端工具集
/// 提供类似 curl 的功能，支持发送 HTTP 请求
pub struct HttpClientTools {
    config: HttpClientConfig,
    client: Client,
    monitor: Arc<RequestMonitor>,
}

impl HttpClientTools {
    /// 创建新的 HTTP 客户端工具实例（使用默认配置）
    pub fn new() -> Self {
        Self::with_config(HttpClientConfig::default())
    }

    /// 创建带自定义配置的 HTTP 客户端工具实例
    pub fn with_config(config: HttpClientConfig) -> Self {
        let client = Self::build_client(&config);
        let monitor = Arc::new(RequestMonitor::new());

        Self {
            config,
            client,
            monitor,
        }
    }

    /// 创建带自定义监控器的实例
    #[allow(dead_code)]
    pub fn with_monitor(config: HttpClientConfig, monitor: Arc<RequestMonitor>) -> Self {
        let client = Self::build_client(&config);

        Self {
            config,
            client,
            monitor,
        }
    }

    /// 构建 reqwest Client
    fn build_client(config: &HttpClientConfig) -> Client {
        let ssrf_config = config.ssrf_config.clone();
        let max_redirects = config.max_redirects;

        Client::builder()
            // 连接池配置
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs(config.pool_idle_timeout_secs))
            .tcp_keepalive(Duration::from_secs(config.tcp_keepalive_secs))
            // 超时配置
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            // 重定向配置 - 自定义策略防止 SSRF 绕过
            .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                if attempt.previous().len() >= max_redirects {
                    tracing::warn!("达到最大重定向次数限制");
                    return attempt.stop();
                }

                let next_url = attempt.url().as_str();
                match ssrf_protection::validate_url_with_config(next_url, &ssrf_config) {
                    Ok(_) => attempt.follow(),
                    Err(e) => {
                        tracing::warn!("阻止不安全重定向 URL: {} - {}", next_url, e);
                        attempt.stop()
                    }
                }
            }))
            // User-Agent
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 HTTP 客户端失败")
    }

    /// 包装请求并记录监控信息
    fn request_with_monitor<F, T>(
        &self,
        method: &str,
        url: &str,
        f: F,
    ) -> NetworkResult<(T, u64)>
    where
        F: FnOnce() -> NetworkResult<(T, u64)>,
    {
        if !self.config.enable_monitoring {
            return f();
        }

        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();

        match &result {
            Ok((_, bytes)) => {
                self.monitor.record(RequestLog {
                    url: url.to_string(),
                    method: method.to_string(),
                    status: 200,
                    duration_ms: duration.as_millis(),
                    bytes: *bytes,
                    timestamp: chrono::Utc::now(),
                });
            }
            Err(_) => {
                self.monitor.record(RequestLog {
                    url: url.to_string(),
                    method: method.to_string(),
                    status: 500,
                    duration_ms: duration.as_millis(),
                    bytes: 0,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        result
    }

    /// 验证 URL（SSRF 防护）
    fn validate_url(&self, url: &str) -> NetworkResult<()> {
        // 验证 URL 长度
        const MAX_URL_LENGTH: usize = 4096;
        if url.len() > MAX_URL_LENGTH {
            return Err(HttpError::WithContext {
                context: format!("URL 过长 ({} > {} 字符)", url.len(), MAX_URL_LENGTH),
            }.into());
        }

        ssrf_protection::validate_url_with_config(url, &self.config.ssrf_config)
    }

    /// 检查响应 IP 安全性
    fn check_response_ip(&self, response: &Response) -> NetworkResult<()> {
        if let Some(remote_addr) = response.remote_addr() {
            ssrf_protection::check_ip_safety_with_config(
                &remote_addr.ip(),
                &self.config.ssrf_config
            )?;
        }
        Ok(())
    }

    /// 限制响应体大小
    fn check_response_size(&self, size: usize) -> NetworkResult<()> {
        if size > self.config.max_response_size {
            return Err(HttpError::ResponseTooLarge {
                size,
                max: self.config.max_response_size,
            }.into());
        }
        Ok(())
    }
}

impl Default for HttpClientTools {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ServiceLifecycle 实现
// ============================================================================

impl ServiceLifecycle for HttpClientTools {
    fn service_name(&self) -> &str {
        "http_client"
    }

    fn init(&mut self) -> Result<(), String> {
        tracing::info!(
            "HTTP 客户端服务初始化完成（超时={}s, 连接池={}）",
            self.config.timeout_secs,
            self.config.pool_max_idle_per_host
        );
        Ok(())
    }

    fn health(&self) -> ServiceHealth {
        let stats = self.monitor.get_stats();

        if stats.total_requests > 0 {
            let error_rate = stats.failed_requests as f32 / stats.total_requests as f32;
            if error_rate < 0.01 {
                ServiceHealth::Healthy
            } else if error_rate < 0.1 {
                ServiceHealth::Degraded
            } else {
                ServiceHealth::Unhealthy
            }
        } else {
            ServiceHealth::Healthy
        }
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.monitor.clear_stats();
        tracing::info!("HTTP 客户端服务已关闭");
        Ok(())
    }

    fn stats(&self) -> ServiceStats {
        let monitor_stats = self.monitor.get_stats();
        ServiceStats {
            total_requests: monitor_stats.total_requests,
            success_count: monitor_stats.successful_requests,
            failure_count: monitor_stats.failed_requests,
            avg_latency_ms: monitor_stats.avg_response_time_ms,
            p99_latency_ms: 0,
            last_called_at: None,
            recent_latencies: vec![],
        }
    }
}

// ============================================================================
// Tool 实现
// ============================================================================

// 注意：HttpClientTools 的 tool 方法使用 &self，不需要可变借用
// update_ssrf_config 使用 &mut self，不在 #[tool] impl 块中
#[tool]
impl HttpClientTools {
    /// 发送 HTTP GET 请求
    ///
    /// # 参数
    /// - `url`: 请求 URL
    /// - `headers`: 可选的自定义请求头（JSON 对象）
    /// - `timeout`: 可选的自定义超时时间（秒），最大 300 秒
    ///
    /// # 返回
    /// 返回 JSON 格式：`{ status, headers, body, url }`
    #[tool(default_headers = "null", default_timeout = "null")]
    pub fn http_get(
        &self,
        url: String,
        headers: Option<serde_json::Value>,
        timeout: Option<u64>,
    ) -> NetworkResult<serde_json::Value> {
        self.validate_url(&url)?;

        let url_clone = url.clone();

        self.request_with_monitor("GET", &url, || {
            let mut req = self.client.get(&url);

            // 添加自定义 headers
            if let Some(headers_val) = headers {
                if let Some(headers_obj) = headers_val.as_object() {
                    for (key, value) in headers_obj {
                        if let Some(value_str) = value.as_str() {
                            req = req.header(key, value_str);
                        }
                    }
                }
            }

            // 应用自定义超时
            if let Some(timeout_secs) = timeout {
                let effective_timeout = timeout_secs.min(300);
                req = req.timeout(Duration::from_secs(effective_timeout));
            }

            let response = req.send()?;

            // 检查 IP 安全性
            self.check_response_ip(&response)?;

            let status = response.status().as_u16();
            let headers_map: std::collections::HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            let headers_json = serde_json::to_value(&headers_map)?;

            let body = response.text()?;
            let bytes = body.len() as u64;

            // 检查响应大小
            self.check_response_size(body.len())?;

            Ok((
                json!({
                    "status": status,
                    "headers": headers_json,
                    "body": body,
                    "url": url_clone
                }),
                bytes,
            ))
        })
        .map(|(data, _)| data)
    }

    /// 发送 HTTP POST 请求
    ///
    /// # 参数
    /// - `url`: 请求 URL
    /// - `body`: 可选的请求体
    /// - `content_type`: 可选的 Content-Type（默认 application/json）
    /// - `headers`: 可选的自定义请求头
    /// - `timeout`: 可选的自定义超时时间（秒）
    ///
    /// # 返回
    /// 返回 JSON 格式：`{ status, headers, body, url }`
    #[tool(default_body = "null", default_content_type = "null", default_headers = "null", default_timeout = "null")]
    pub fn http_post(
        &self,
        url: String,
        body: Option<String>,
        content_type: Option<String>,
        headers: Option<serde_json::Value>,
        timeout: Option<u64>,
    ) -> NetworkResult<serde_json::Value> {
        self.validate_url(&url)?;

        let mut req = self.client.post(&url);

        // 设置 Content-Type
        let content_type = content_type.unwrap_or_else(|| "application/json".to_string());
        req = req.header("Content-Type", &content_type);

        // 添加自定义 headers
        if let Some(headers_val) = headers {
            if let Some(headers_obj) = headers_val.as_object() {
                for (key, value) in headers_obj {
                    if let Some(value_str) = value.as_str() {
                        req = req.header(key, value_str);
                    }
                }
            }
        }

        // 添加请求体
        if let Some(body_str) = body {
            req = req.body(body_str);
        }

        // 应用自定义超时
        if let Some(timeout_secs) = timeout {
            req = req.timeout(Duration::from_secs(timeout_secs.min(300)));
        }

        self.request_with_monitor("POST", &url, || {
            let response = req.send()?;
            self.check_response_ip(&response)?;

            let status = response.status().as_u16();
            let headers_map: std::collections::HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            let headers_json = serde_json::to_value(&headers_map)?;

            let body = response.text()?;
            self.check_response_size(body.len())?;

            Ok((
                json!({
                    "status": status,
                    "headers": headers_json,
                    "body": body,
                    "url": url
                }),
                body.len() as u64,
            ))
        })
        .map(|(data, _)| data)
    }

    /// 检查 URL 是否可访问
    ///
    /// # 参数
    /// - `url`: 要检查的 URL
    /// - `timeout`: 可选的超时时间（秒）
    ///
    /// # 返回
    /// 返回 JSON 格式：`{ accessible, status/response_time_ms/error, url }`
    #[tool(default_timeout = "null")]
    pub fn check_url(&self, url: String, timeout: Option<u64>) -> NetworkResult<serde_json::Value> {
        self.validate_url(&url)?;

        let mut req = self.client.head(&url);
        if let Some(timeout_secs) = timeout {
            req = req.timeout(Duration::from_secs(timeout_secs.min(60)));
        }

        let start = std::time::Instant::now();
        let response = req.send();
        let elapsed = start.elapsed();

        match response {
            Ok(resp) => {
                // 检查 IP 安全性
                if let Some(remote_addr) = resp.remote_addr() {
                    if let Err(e) = ssrf_protection::check_ip_safety_with_config(
                        &remote_addr.ip(),
                        &self.config.ssrf_config
                    ) {
                        return Ok(json!({
                            "accessible": false,
                            "error": e.to_string(),
                            "url": url
                        }));
                    }
                }

                let status = resp.status().as_u16();
                Ok(json!({
                    "accessible": true,
                    "status": status,
                    "response_time_ms": elapsed.as_millis() as u64,
                    "url": url
                }))
            }
            Err(e) => Ok(json!({
                "accessible": false,
                "error": e.to_string(),
                "url": url
            })),
        }
    }

    /// 下载文件到本地
    ///
    /// # 参数
    /// - `url`: 文件 URL
    /// - `save_path`: 保存路径
    /// - `timeout`: 可选的超时时间（秒），最大 600 秒
    ///
    /// # 返回
    /// 返回下载结果信息
    #[tool(default_timeout = "null")]
    pub fn download_file(
        &self,
        url: String,
        save_path: String,
        timeout: Option<u64>,
    ) -> NetworkResult<String> {
        self.validate_url(&url)?;
        ssrf_protection::validate_save_path_with_config(
            &save_path,
            &self.config.ssrf_config
        )?;

        let mut req = self.client.get(&url);
        if let Some(timeout_secs) = timeout {
            req = req.timeout(Duration::from_secs(timeout_secs.min(600)));
        }

        let response = req.send()?;
        self.check_response_ip(&response)?;

        let status = response.status();
        if !status.is_success() {
            return Err(HttpError::StatusCode {
                status: status.as_u16(),
                message: "下载失败".to_string(),
            }.into());
        }

        let bytes = response.bytes()?;

        // 限制下载文件大小
        const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50MB
        if bytes.len() > MAX_FILE_SIZE {
            return Err(HttpError::ResponseTooLarge {
                size: bytes.len(),
                max: MAX_FILE_SIZE,
            }.into());
        }

        std::fs::write(&save_path, &bytes)?;

        Ok(format!(
            "✅ 成功下载文件\nURL: {}\n保存路径：{}\n文件大小：{} bytes",
            url, save_path, bytes.len()
        ))
    }

    /// 获取请求统计信息
    pub fn get_stats(&self) -> NetworkResult<serde_json::Value> {
        let stats = self.monitor.get_stats();
        Ok(json!({
            "total_requests": stats.total_requests,
            "successful_requests": stats.successful_requests,
            "failed_requests": stats.failed_requests,
            "total_bytes": stats.total_bytes,
            "avg_response_time_ms": stats.avg_response_time_ms,
        }))
    }

    /// 清空请求统计
    pub fn clear_stats(&self) -> NetworkResult<String> {
        self.monitor.clear_stats();
        Ok("✅ 统计信息已清空".to_string())
    }
}

// 单独的 impl 块用于需要 &mut self 的方法
impl HttpClientTools {
    /// 更新 SSRF 配置（热更新）
    #[allow(dead_code)]
    pub fn update_ssrf_config(&mut self, config: RuntimeSsrfConfig) {
        self.config.ssrf_config = config;
        // 重建客户端以应用新配置
        self.client = Self::build_client(&self.config);
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.connect_timeout_secs, 10);
        assert_eq!(config.max_response_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_http_client_creation() {
        let client = HttpClientTools::new();
        assert!(client.config.enable_monitoring);
    }

    #[test]
    fn test_http_client_with_custom_config() {
        let config = HttpClientConfig {
            timeout_secs: 60,
            enable_monitoring: false,
            ..Default::default()
        };
        let client = HttpClientTools::with_config(config);
        assert!(!client.config.enable_monitoring);
        assert_eq!(client.config.timeout_secs, 60);
    }

    #[test]
    fn test_service_lifecycle() {
        let mut client = HttpClientTools::new();

        // 测试 init
        assert!(client.init().is_ok());

        // 测试 health（应该是 Healthy）
        let health = client.health();
        assert!(matches!(health, ServiceHealth::Healthy));

        // 测试 stats
        let stats = client.stats();
        assert_eq!(stats.total_requests, 0);

        // 测试 shutdown
        assert!(client.shutdown().is_ok());
    }

    #[test]
    fn test_ssrf_config_update() {
        let mut client = HttpClientTools::new();
        let new_config = RuntimeSsrfConfig::new();
        
        // 测试热更新
        client.update_ssrf_config(new_config);
    }
}
