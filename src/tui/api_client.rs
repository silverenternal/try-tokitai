//! AI API 客户端 - 整合连接池、流式响应、缓存、线程池

use crate::tui::app::TuiError;
use moka::sync::Cache;
use once_cell::sync::Lazy;
use reqwest::{Client, ClientBuilder};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use threadpool::ThreadPool;
use tracing::{debug, info};

/// ========== 配置常量 ==========

/// HTTP 连接池最大空闲连接数
const HTTP_POOL_MAX_IDLE_PER_HOST: usize = 10;

/// HTTP 连接池空闲超时时间（秒）
const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 90;

/// HTTP TCP keepalive 时间（秒）
const HTTP_TCP_KEEPALIVE_SECS: u64 = 30;

/// HTTP 请求超时时间（秒）
const HTTP_REQUEST_TIMEOUT_SECS: u64 = 120;

/// HTTP 连接超时时间（秒）
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 10;

/// 线程池工作线程数
const API_THREAD_POOL_SIZE: usize = 4;

/// 缓存最大条目数
const CACHE_MAX_CAPACITY: u64 = 100;

/// 缓存存活时间（秒）
const CACHE_TTL_SECS: u64 = 3600;

/// ========== 全局单例 ==========

/// 全局 HTTP 连接池（单例）
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    ClientBuilder::new()
        .pool_max_idle_per_host(HTTP_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(HTTP_POOL_IDLE_TIMEOUT_SECS))
        .tcp_keepalive(Duration::from_secs(HTTP_TCP_KEEPALIVE_SECS))
        .timeout(Duration::from_secs(HTTP_REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .user_agent("Tokitai AI Assistant/0.2.0")
        .build()
        .expect("Failed to create HTTP client")
});

/// 全局线程池（工作线程数由配置常量定义）
static API_THREAD_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPool::with_name("api-worker".to_string(), API_THREAD_POOL_SIZE)
});

/// 响应缓存（最大条目数和过期时间由配置常量定义）
static RESPONSE_CACHE: Lazy<Cache<String, String>> = Lazy::new(|| {
    Cache::builder()
        .max_capacity(CACHE_MAX_CAPACITY)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});

/// 请求计数器（用于统计）
static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);

/// API 客户端配置
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub api_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_url: std::env::var("AI_API_URL")
                .unwrap_or_else(|_| "https://ollama.com/v1/chat/completions".to_string()),
            api_key: std::env::var("AI_API_KEY").ok(),
            model: std::env::var("AI_MODEL")
                .unwrap_or_else(|_| "qwen3.5:397b".to_string()),
        }
    }
}

/// 流式响应事件
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 接收到文本块
    Text(String),
    /// 完成
    Done,
    /// 错误
    Error(String),
}

/// API 客户端
pub struct ApiClient {
    config: ApiConfig,
}

impl ApiClient {
    pub fn new(config: ApiConfig) -> Self {
        Self { config }
    }

    /// 获取统计信息
    pub fn get_stats() -> (u64, u64) {
        (
            REQUEST_COUNT.load(Ordering::Relaxed),
            CACHE_HITS.load(Ordering::Relaxed),
        )
    }

    /// 同步调用（非流式，带缓存）
    pub fn chat_sync(&self, message: &str) -> Result<String, TuiError> {
        let start = std::time::Instant::now();
        REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);

        // 1. 检查缓存
        let cache_key = normalize_query(message);
        if let Some(cached) = RESPONSE_CACHE.get(&cache_key) {
            CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            debug!("缓存命中：{}", message);
            return Ok(cached);
        }

        // 2. 调用 API（使用 tokio runtime 包装异步调用）
        let rt = tokio::runtime::Handle::current()
            .block_on(async { self.chat_internal(message).await });

        // 3. 存入缓存
        if let Ok(ref content) = rt {
            RESPONSE_CACHE.insert(cache_key, content.clone());
        }

        let elapsed = start.elapsed();
        info!("API 请求完成：{:?}, 缓存：{}", elapsed, RESPONSE_CACHE.entry_count());

        rt
    }

    /// 异步内部调用
    async fn chat_internal(&self, message: &str) -> Result<String, TuiError> {
        let request_body = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "user",
                    "content": message
                }
            ]
        });

        let mut request = HTTP_CLIENT
            .post(&self.config.api_url)
            .json(&request_body);

        if let Some(key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| TuiError::ApiRequest(format!("请求失败：{}", e)))?;

        // 检查状态码
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            // 尝试解析错误信息
            if let Ok(error_json) = serde_json::from_str::<Value>(&error_text) {
                if let Some(err_msg) = error_json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    if err_msg.to_lowercase().contains("auth")
                        || err_msg.to_lowercase().contains("key")
                        || err_msg.to_lowercase().contains("token")
                    {
                        return Err(TuiError::AuthFailed);
                    }
                }
            }

            return Err(TuiError::ApiRequest(format!("HTTP {}: {}", status, error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| TuiError::ApiRequest(format!("解析响应失败：{}", e)))?;

        // 解析响应
        if let Some(choices) = response_json.get("choices").and_then(|v| v.as_array()) {
            if let Some(first) = choices.first() {
                if let Some(content) = first
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    return Ok(content.to_string());
                }
            }
        }

        Err(TuiError::InvalidResponse)
    }

    /// 流式调用（打字机效果）
    pub fn chat_stream(
        &self,
        message: &str,
        tx: std::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<(), TuiError> {
        REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);

        // 检查缓存
        let cache_key = normalize_query(message);
        if let Some(cached) = RESPONSE_CACHE.get(&cache_key) {
            CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            // 缓存命中，逐字发送模拟流式效果
            for chunk in cached.chars().collect::<Vec<_>>().chunks(50) {
                if tx.send(StreamEvent::Text(chunk.iter().collect())).is_err() {
                    debug!("通道已关闭，停止发送缓存内容");
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if tx.send(StreamEvent::Done).is_err() {
                debug!("通道已关闭，无法发送 Done 事件");
            }
            return Ok(());
        }

        // 在线程池中执行异步流式请求
        let config = self.config.clone();
        let message = message.to_string();
        let tx_clone = tx.clone();  // 克隆一个用于错误上报

        API_THREAD_POOL.execute(move || {
            // 创建 tokio runtime
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(format!("创建 runtime 失败：{}", e)));
                    return;
                }
            };

            let result = rt.block_on(async {
                Self::chat_stream_internal(&config, &message, tx).await
            });

            if let Err(e) = result {
                let _ = tx_clone.send(StreamEvent::Error(e.to_string()));
            }
        });

        Ok(())
    }

    /// 内部流式请求
    async fn chat_stream_internal(
        config: &ApiConfig,
        message: &str,
        tx: std::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<(), TuiError> {
        let request_body = json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": message
                }
            ],
            "stream": true
        });

        let mut request = HTTP_CLIENT
            .post(&config.api_url)
            .json(&request_body);

        if let Some(key) = &config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| TuiError::ApiRequest(format!("请求失败：{}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if error_text.to_lowercase().contains("auth")
                || error_text.to_lowercase().contains("key")
            {
                return Err(TuiError::AuthFailed);
            }
            return Err(TuiError::ApiRequest(format!("HTTP 错误：{}", error_text)));
        }

        // 读取 SSE 流
        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        let mut full_content = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| TuiError::ApiRequest(format!("读取流失败：{}", e)))?;
            let text = String::from_utf8_lossy(&chunk);

            // 解析 SSE 格式：data: {...}
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = line[6..].trim();
                    if data == "[DONE]" {
                        let _ = tx.send(StreamEvent::Done);
                        return Ok(());
                    }

                    // 尝试解析 JSON
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(content) = json
                            .get("choices")
                            .and_then(|c| c.as_array())
                            .and_then(|c| c.first())
                            .and_then(|c| c.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            if !content.is_empty() {
                                full_content.push_str(content);
                                if tx.send(StreamEvent::Text(content.to_string())).is_err() {
                                    debug!("通道已关闭，停止发送流式内容");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }

        // 存入缓存
        if !full_content.is_empty() {
            RESPONSE_CACHE.insert(normalize_query(message), full_content);
        }

        let _ = tx.send(StreamEvent::Done);
        Ok(())
    }

    /// 清空缓存
    pub fn clear_cache() {
        RESPONSE_CACHE.invalidate_all();
        info!("缓存已清空");
    }

    /// 获取缓存大小
    pub fn cache_size() -> u64 {
        RESPONSE_CACHE.entry_count()
    }
}

/// 归一化查询（用于缓存键）- 只 trim 空白，保留大小写
fn normalize_query(query: &str) -> String {
    query.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_query() {
        assert_eq!(normalize_query("Hello"), "Hello");
        assert_eq!(normalize_query("  TEST  "), "TEST");
        assert_eq!(normalize_query("Rust 语言"), "Rust 语言");
        assert_eq!(normalize_query("rust"), "rust");
        assert_ne!(normalize_query("Rust"), "rust"); // 保留大小写
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert!(!config.api_url.is_empty());
        assert!(!config.model.is_empty());
    }
}
