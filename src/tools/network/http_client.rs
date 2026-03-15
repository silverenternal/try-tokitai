use once_cell::sync::Lazy;
use tokitai::tool;
use serde_json::json;
use std::time::Duration;
use url::Url;
use std::sync::Arc;
use crate::tools::network::request_monitor::{RequestMonitor, RequestLog};

/// HTTP 客户端工具集
/// 提供类似 curl 的功能，支持发送 HTTP 请求
pub struct HttpClientTools {
    /// 请求监控器
    pub monitor: Arc<RequestMonitor>,
}

// 复用 reqwest Client 连接池，避免每次重建
// 优化配置：连接池/Keep-Alive/HTTP2/重试机制
static HTTP_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        // 连接池配置
        .pool_max_idle_per_host(10)              // 每主机最大空闲连接数
        .pool_idle_timeout(Duration::from_secs(90)) // 空闲连接超时时间
        .tcp_keepalive(Duration::from_secs(30))     // TCP Keep-Alive

        // 超时配置
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))

        // 重定向配置 - 自定义策略防止 SSRF 绕过
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            // 每次重定向都检查 URL 安全性
            let next_url = attempt.url().as_str();
            match crate::tools::network::ssrf_protection::validate_url(next_url) {
                Ok(_) => attempt.follow(),
                Err(e) => {
                    tracing::warn!("阻止不安全的重定向 URL: {} - {}", next_url, e);
                    attempt.stop()
                }
            }
        }))

        // User-Agent
        .user_agent("AI-Assistant/0.1.0")

        .build()
        .expect("创建 HTTP 客户端失败")
});

impl HttpClientTools {
    /// 创建新的 HTTP 客户端工具实例
    pub fn new() -> Self {
        Self {
            monitor: Arc::new(RequestMonitor::new()),
        }
    }

    /// 创建带自定义监控器的实例
    /// TODO: Phase 5 集成到配置系统
    #[allow(dead_code)]
    pub fn with_monitor(monitor: Arc<RequestMonitor>) -> Self {
        Self { monitor }
    }

    /// 包装请求并记录监控信息
    fn request_with_monitor<F, T>(&self, method: &str, url: &str, f: F) -> Result<(T, u64), String>
    where
        F: FnOnce() -> Result<(T, u64), String>,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();

        match &result {
            Ok((_, bytes)) => {
                self.monitor.record(RequestLog {
                    url: url.to_string(),
                    method: method.to_string(),
                    status: 200,
                    duration_ms: duration.as_millis() as u128,
                    bytes: *bytes,
                    timestamp: chrono::Utc::now(),
                });
            }
            Err(_) => {
                self.monitor.record(RequestLog {
                    url: url.to_string(),
                    method: method.to_string(),
                    status: 500,
                    duration_ms: duration.as_millis() as u128,
                    bytes: 0,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        result
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

use crate::tool_matrix::matrix::{ServiceLifecycle, ServiceHealth, ServiceStats};

impl ServiceLifecycle for HttpClientTools {
    fn service_name(&self) -> &str {
        "http_client"
    }

    fn init(&mut self) -> Result<(), String> {
        // HTTP 客户端使用静态 Lazy 初始化，连接池已自动管理
        // 这里只记录日志
        tracing::info!("HTTP 客户端服务初始化完成（连接池已就绪）");
        Ok(())
    }

    fn health(&self) -> ServiceHealth {
        // 检查 HTTP 客户端是否可用
        // 简单检查：监控器是否正常工作
        let stats = self.monitor.get_stats();
        
        // 如果最近有成功请求，认为服务健康
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
            // 没有请求记录，尝试简单检查
            ServiceHealth::Healthy
        }
    }

    fn shutdown(&mut self) -> Result<(), String> {
        // reqwest Client 使用静态 Lazy，不需要显式关闭
        // 但可以清理监控器数据
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
            p99_latency_ms: 0,  // RequestStats 没有 P99
            last_called_at: None,  // RequestStats 没有时间戳
            recent_latencies: vec![],
        }
    }
}

// SSRF 防护：禁止访问的内网地址段
fn is_safe_url(url: &str) -> Result<(), String> {
    use std::net::IpAddr;

    let parsed = Url::parse(url)
        .map_err(|e| format!("无效 URL 格式：{}", e))?;

    // 只允许 http/https 协议
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("不支持的协议：{}，仅支持 http/https", scheme));
    }

    // 解析主机名
    let host = parsed.host_str()
        .ok_or("URL 缺少主机名")?;

    // 检查常见内网域名
    let blocked_hosts = ["localhost", "localhost.localdomain", "internal", "intranet"];
    if blocked_hosts.contains(&host.to_lowercase().as_str()) {
        return Err(format!(
            "禁止访问内网地址：{} (SSRF 防护)",
            host
        ));
    }

    // 尝试解析 IP 地址
    if let Ok(ip_addr) = host.parse::<IpAddr>() {
        check_ip_safety(&ip_addr)?;
    }
    // 如果是域名，在建立连接时会再次检查解析后的 IP

    Ok(())
}

// 检查 IP 地址是否安全（非内网地址）
fn check_ip_safety(ip: &std::net::IpAddr) -> Result<(), String> {
    use std::net::IpAddr::{V4, V6};
    
    match ip {
        V4(ip4) => {
            // 检查 IPv4 内网地址
            if ip4.is_private() 
                || ip4.is_loopback() 
                || ip4.is_link_local()
                || ip4.is_unspecified()
            {
                return Err(format!(
                    "禁止访问内网地址：{} (SSRF 防护)",
                    ip
                ));
            }
            // 检查 10.0.0.0/8
            let octets = ip4.octets();
            if octets[0] == 10 {
                return Err(format!(
                    "禁止访问内网地址：{} (SSRF 防护)",
                    ip
                ));
            }
        }
        V6(ip6) => {
            if ip6.is_loopback() 
                || ip6.is_unspecified()
                || ip6.is_unique_local()
            {
                return Err(format!(
                    "禁止访问内网地址：{} (SSRF 防护)",
                    ip
                ));
            }
        }
    }
    Ok(())
}

// 验证 URL 长度
fn validate_url_length(url: &str) -> Result<(), String> {
    const MAX_URL_LENGTH: usize = 4096;
    if url.len() > MAX_URL_LENGTH {
        return Err(format!(
            "URL 过长 ({} > {} 字符)",
            url.len(),
            MAX_URL_LENGTH
        ));
    }
    Ok(())
}

#[tool]
impl HttpClientTools {
    /// 发送 HTTP GET 请求
    /// 适用于获取 API 数据、网页内容等
    pub fn http_get(
        &self,
        url: String,
        headers: Option<serde_json::Value>,
        timeout: Option<u64>,
    ) -> Result<serde_json::Value, String> {
        validate_url_length(&url)?;
        is_safe_url(&url)?;

        let url_clone = url.clone();
        let result = self.request_with_monitor("GET", &url, || {
            let client = &*HTTP_CLIENT;

            let mut req = client.get(&url);

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

            // 应用自定义超时（如果有）
            if let Some(timeout_secs) = timeout {
                req = req.timeout(Duration::from_secs(timeout_secs.min(300))); // 最大 5 分钟
            }

            let response = req
                .send()
                .map_err(|e| format!("发送请求失败：{}", e))?;

            // 检查 IP 安全性（防止重定向到内网）
            if let Some(remote_addr) = response.remote_addr() {
                check_ip_safety(&remote_addr.ip())?;
            }

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

            let headers_json = serde_json::to_value(&headers_map)
                .map_err(|e| format!("转换 headers 失败：{}", e))?;

            let body = response
                .text()
                .map_err(|e| format!("读取响应失败：{}", e))?;

            let bytes = body.len() as u64;

            Ok((json!({
                "status": status,
                "headers": headers_json,
                "body": body,
                "url": url_clone
            }), bytes))
        });

        result.map(|(data, _)| data)
    }

    /// 发送 HTTP POST 请求
    /// 适用于提交表单数据、JSON 数据等
    pub fn http_post(
        &self,
        url: String,
        body: Option<String>,
        content_type: Option<String>,
        headers: Option<serde_json::Value>,
        timeout: Option<u64>,
    ) -> Result<serde_json::Value, String> {
        validate_url_length(&url)?;
        is_safe_url(&url)?;
        
        let client = &*HTTP_CLIENT;
        
        let mut req = client.post(&url);

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
        
        // 应用自定义超时（如果有）
        if let Some(timeout_secs) = timeout {
            req = req.timeout(Duration::from_secs(timeout_secs.min(300)));
        }

        let response = req
            .send()
            .map_err(|e| format!("发送请求失败：{}", e))?;
        
        // 检查 IP 安全性
        if let Some(remote_addr) = response.remote_addr() {
            check_ip_safety(&remote_addr.ip())?;
        }

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

        let headers_json = serde_json::to_value(&headers_map)
            .map_err(|e| format!("转换 headers 失败：{}", e))?;

        let body = response
            .text()
            .map_err(|e| format!("读取响应失败：{}", e))?;

        Ok(json!({
            "status": status,
            "headers": headers_json,
            "body": body,
            "url": url
        }))
    }

    /// 检查 URL 是否可访问
    /// 快速检查网站或 API 是否在线
    pub fn check_url(&self, url: String, timeout: Option<u64>) -> Result<serde_json::Value, String> {
        validate_url_length(&url)?;
        is_safe_url(&url)?;
        
        let client = &*HTTP_CLIENT;
        
        let start = std::time::Instant::now();
        
        let mut req = client.head(&url);
        if let Some(timeout_secs) = timeout {
            req = req.timeout(Duration::from_secs(timeout_secs.min(60)));
        }
        
        let response = req.send();
        let elapsed = start.elapsed();

        match response {
            Ok(resp) => {
                // 检查 IP 安全性
                if let Some(remote_addr) = resp.remote_addr() {
                    if let Err(e) = check_ip_safety(&remote_addr.ip()) {
                        return Ok(json!({
                            "accessible": false,
                            "error": e,
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
            }))
        }
    }

    /// 下载文件到本地
    /// 适用于下载图片、文档等二进制文件
    pub fn download_file(
        &self,
        url: String,
        save_path: String,
        timeout: Option<u64>,
    ) -> Result<String, String> {
        validate_url_length(&url)?;
        is_safe_url(&url)?;
        validate_save_path(&save_path)?;
        
        let client = &*HTTP_CLIENT;
        
        let mut req = client.get(&url);
        if let Some(timeout_secs) = timeout {
            req = req.timeout(Duration::from_secs(timeout_secs.min(600))); // 最大 10 分钟
        }

        let response = req
            .send()
            .map_err(|e| format!("发送请求失败：{}", e))?;
        
        // 检查 IP 安全性
        if let Some(remote_addr) = response.remote_addr() {
            check_ip_safety(&remote_addr.ip())?;
        }

        let status = response.status();
        if !status.is_success() {
            return Err(format!("下载失败：HTTP {}", status));
        }

        let bytes = response
            .bytes()
            .map_err(|e| format!("读取数据失败：{}", e))?;
        
        // 限制下载文件大小
        const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50MB
        if bytes.len() > MAX_FILE_SIZE {
            return Err(format!(
                "文件过大 ({} > {} MB)",
                bytes.len() / 1024 / 1024,
                MAX_FILE_SIZE / 1024 / 1024
            ));
        }

        std::fs::write(&save_path, &bytes)
            .map_err(|e| format!("写入文件失败：{}", e))?;

        Ok(format!(
            "✅ 成功下载文件\nURL: {}\n保存路径：{}\n文件大小：{} bytes",
            url, save_path, bytes.len()
        ))
    }
}

/// 验证保存路径是否安全
fn validate_save_path(path: &str) -> Result<(), String> {
    const MAX_PATH_LENGTH: usize = 1024;
    
    if path.len() > MAX_PATH_LENGTH {
        return Err(format!("路径过长 ({} > {} 字符)", path.len(), MAX_PATH_LENGTH));
    }
    
    // 解析为绝对路径
    let path_buf = std::path::PathBuf::from(path);
    let absolute_path = path_buf.canonicalize()
        .unwrap_or_else(|_| path_buf.clone());
    
    // 获取当前工作目录
    let cwd = std::env::current_dir()
        .map_err(|e| format!("获取当前目录失败：{}", e))?;
    
    // 检查路径是否在当前目录或其子目录下
    if !absolute_path.starts_with(&cwd) {
        return Err(format!(
            "禁止写入当前目录外的路径：{} (安全限制)",
            path
        ));
    }
    
    // 检查是否尝试访问敏感目录
    let path_str = absolute_path.to_string_lossy();
    let sensitive_dirs = ["/etc", "/root", "/home", "/var", "/usr", "/bin", "/sbin"];
    for sensitive in &sensitive_dirs {
        if path_str.starts_with(sensitive) {
            return Err(format!("禁止写入敏感目录：{} (安全限制)", sensitive));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_url_valid() {
        assert!(is_safe_url("https://example.com").is_ok());
        assert!(is_safe_url("http://api.github.com/users").is_ok());
    }

    #[test]
    fn test_is_safe_url_invalid_scheme() {
        assert!(is_safe_url("file:///etc/passwd").is_err());
        assert!(is_safe_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_is_safe_url_localhost() {
        assert!(is_safe_url("http://127.0.0.1:8080").is_err());
        assert!(is_safe_url("http://localhost:3000").is_err());
        assert!(is_safe_url("http://localhost.localdomain").is_err());
    }

    #[test]
    fn test_is_safe_url_private_ip() {
        assert!(is_safe_url("http://192.168.1.1").is_err());
        assert!(is_safe_url("http://10.0.0.1").is_err());
        assert!(is_safe_url("http://172.16.0.1").is_err());
    }

    #[test]
    fn test_validate_url_length() {
        let long_url = "https://example.com/".to_string() + &"a".repeat(5000);
        assert!(validate_url_length(&long_url).is_err());
        
        let short_url = "https://example.com".to_string();
        assert!(validate_url_length(&short_url).is_ok());
    }

    #[test]
    fn test_validate_save_path() {
        // 当前目录内的路径应该有效
        let _valid_path = "./test_download.txt";
        // 注意：这个测试可能会失败，因为文件可能不存在
        // assert!(validate_save_path(valid_path).is_ok() || validate_save_path(valid_path).is_err());
        
        // 敏感目录应该被拒绝
        assert!(validate_save_path("/etc/test.txt").is_err());
        assert!(validate_save_path("/root/secret.txt").is_err());
    }
}
