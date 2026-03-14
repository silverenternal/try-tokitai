use anyhow::Result;
use moka::sync::Cache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use scraper::{Html, Selector};

/// 搜索错误类型
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("网络请求失败：{0}")]
    Network(String),

    #[error("搜索 API 返回错误：{status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("未找到搜索结果")]
    NoResults,

    #[error("搜索超时")]
    Timeout(#[from] std::io::Error),
}

/// 搜索结果结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    #[serde(default)]
    pub engine: String,
}

/// 搜索引擎健康状态
struct EngineHealth {
    last_check: std::time::Instant,
    success_count: u32,
    fail_count: u32,
    is_healthy: bool,
}

impl EngineHealth {
    fn new() -> Self {
        Self {
            last_check: std::time::Instant::now(),
            success_count: 0,
            fail_count: 0,
            is_healthy: true,
        }
    }

    fn health_score(&self) -> f32 {
        if !self.is_healthy {
            return 0.0;
        }
        let total = self.success_count + self.fail_count;
        if total == 0 {
            return 1.0;
        }
        1.0 + (self.success_count as f32 / (total as f32 + 1.0))
    }
}

/// 搜索引擎 trait
pub trait SearchEngine: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError>;
    fn health_check(&self) -> bool;
}

/// SearXNG 引擎实现
pub struct SearxngEngine {
    url: String,
    client: ureq::Agent,
}

impl SearxngEngine {
    pub fn new(url: &str) -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (compatible; AI Assistant/1.0)")
            .build();

        Self {
            url: url.to_string(),
            client,
        }
    }
}

impl SearchEngine for SearxngEngine {
    fn name(&self) -> &str {
        "SearXNG"
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "{}/search?q={}&format=json&engines=bing,duckduckgo",
            self.url, encoded_query
        );

        let response = self
            .client
            .get(&url)
            .call()
            .map_err(|e| SearchError::Network(e.to_string()))?;

        if response.status() != 200 {
            return Err(SearchError::ApiError {
                status: response.status(),
                message: "SearXNG API 返回错误状态".to_string(),
            });
        }

        let searxng_resp: SearxngResponse = response
            .into_json()
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let results: Vec<SearchResult> = searxng_resp
            .results
            .into_iter()
            .take(limit)
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.content,
                engine: r.engine,
            })
            .collect();

        if results.is_empty() {
            return Err(SearchError::NoResults);
        }

        Ok(results)
    }

    fn health_check(&self) -> bool {
        let url = format!("{}/healthz", self.url);
        self.client.get(&url).call().is_ok()
    }
}

/// DuckDuckGo 引擎实现
pub struct DuckDuckGoEngine {
    client: ureq::Agent,
}

impl DuckDuckGoEngine {
    pub fn new() -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (compatible; AI Assistant/1.0)")
            .build();

        Self { client }
    }
}

impl Default for DuckDuckGoEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine for DuckDuckGoEngine {
    fn name(&self) -> &str {
        "DuckDuckGo"
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let response = self
            .client
            .get(&url)
            .call()
            .map_err(|e| SearchError::Network(e.to_string()))?;

        if response.status() != 200 {
            return Err(SearchError::ApiError {
                status: response.status(),
                message: "DuckDuckGo 返回错误状态".to_string(),
            });
        }

        let body = response
            .into_string()
            .map_err(|e| SearchError::Network(e.to_string()))?;

        // 检查是否返回了错误页面
        if body.contains("503 Service Unavailable") {
            return Err(SearchError::ApiError {
                status: 503,
                message: "DuckDuckGo 服务不可用".to_string(),
            });
        }

        let results = parse_duckduckgo_results(&body, limit)
            .ok_or(SearchError::NoResults)?;

        if results.is_empty() {
            return Err(SearchError::NoResults);
        }

        Ok(results)
    }

    fn health_check(&self) -> bool {
        self.client.get("https://duckduckgo.com").call().is_ok()
    }
}

/// SearXNG 响应结构
#[derive(Debug, Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Debug, Deserialize)]
struct SearxngResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    engine: String,
}

/// 解析 DuckDuckGo HTML 结果
fn parse_duckduckgo_results(html: &str, limit: usize) -> Option<Vec<SearchResult>> {
    let document = Html::parse_document(html);
    let mut results = Vec::new();

    let result_selector = Selector::parse(".result").ok()?;
    let title_selector = Selector::parse(".result__a").ok()?;
    let url_selector = Selector::parse(".result__url").ok()?;
    let snippet_selector = Selector::parse(".result__snippet").ok()?;

    for result in document.select(&result_selector).take(limit) {
        let title = result
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| "无标题".to_string());

        let url = result
            .select(&url_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| "未知 URL".to_string());

        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(String::new);

        results.push(SearchResult {
            title: trim_whitespace(&title),
            url: trim_whitespace(&url),
            snippet: trim_whitespace(&snippet),
            engine: "duckduckgo".to_string(),
        });
    }

    Some(results)
}

fn trim_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 搜索引擎管理器
pub struct SearchEngineManager {
    engines: Vec<Arc<dyn SearchEngine>>,
    health_status: Arc<RwLock<Vec<EngineHealth>>>,
    cache: Cache<String, Vec<SearchResult>>,
}

impl SearchEngineManager {
    pub fn new() -> Self {
        let mut engines: Vec<Arc<dyn SearchEngine>> = Vec::new();

        // 从环境变量读取自定义 SearXNG 实例
        if let Ok(searxng_url) = std::env::var("SEARXNG_URL") {
            engines.push(Arc::new(SearxngEngine::new(&searxng_url)));
        }

        // 添加公共 SearXNG 实例
        engines.push(Arc::new(SearxngEngine::new("https://searx.be")));
        engines.push(Arc::new(SearxngEngine::new("https://search.ononoki.org")));

        // DuckDuckGo 作为备选
        engines.push(Arc::new(DuckDuckGoEngine::new()));

        let health_status = Arc::new(RwLock::new(
            engines.iter().map(|_| EngineHealth::new()).collect(),
        ));

        Self {
            engines,
            health_status,
            cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(3600))
                .build(),
        }
    }

    /// 智能搜索：按健康度排序引擎
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        // 检查缓存
        if let Some(cached) = self.cache.get(&query.to_string()) {
            tracing::debug!("使用缓存结果");
            return Ok(cached);
        }

        // 按健康度排序引擎
        let mut engine_indices: Vec<(usize, f32)> = self
            .health_status
            .read()
            .iter()
            .enumerate()
            .map(|(i, h)| (i, h.health_score()))
            .collect();

        engine_indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // 依次尝试引擎
        let mut last_error = None;
        for (idx, _score) in engine_indices {
            let engine = &self.engines[idx];

            if !engine.health_check() {
                tracing::debug!("跳过不健康引擎：{}", engine.name());
                continue;
            }

            tracing::debug!("尝试引擎：{}", engine.name());

            match engine.search(query, limit) {
                Ok(results) => {
                    // 更新健康状态
                    self.health_status.write()[idx].success_count += 1;

                    // 存入缓存
                    self.cache.insert(query.to_string(), results.clone());

                    return Ok(results);
                }
                Err(e) => {
                    tracing::warn!("引擎 {} 失败：{}", engine.name(), e);
                    self.health_status.write()[idx].fail_count += 1;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(SearchError::NoResults))
    }

    /// 获取引擎健康状态
    /// TODO: Phase 5 集成到 /health 命令
    #[allow(dead_code)]
    pub fn get_health_status(&self) -> Vec<(String, bool)> {
        let health = self.health_status.read();
        self.engines
            .iter()
            .zip(health.iter())
            .map(|(engine, h)| (engine.name().to_string(), h.is_healthy))
            .collect()
    }

    /// 清空缓存
    /// TODO: Phase 5 集成到 /optimize 命令
    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        self.cache.invalidate_all();
    }

    /// 获取缓存大小
    /// TODO: Phase 5 集成到 /stats 命令
    #[allow(dead_code)]
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl Default for SearchEngineManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_health_score() {
        let mut health = EngineHealth::new();
        assert_eq!(health.health_score(), 1.0);

        health.success_count = 10;
        health.fail_count = 0;
        assert!(health.health_score() > 1.0);

        health.is_healthy = false;
        assert_eq!(health.health_score(), 0.0);
    }

    #[test]
    fn test_search_engine_manager_creation() {
        let manager = SearchEngineManager::new();
        assert!(!manager.engines.is_empty());
    }

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_whitespace("  hello   world  "), "hello world");
        assert_eq!(trim_whitespace("single"), "single");
        assert_eq!(trim_whitespace(""), "");
    }
}
