//! 统一搜索模块
//!
//! 提供多引擎搜索功能，支持 DuckDuckGo、SearXNG、维基百科等
//! 统一错误处理、缓存管理和健康检查
//!
//! # 模块结构
//! 详细功能请参考子模块：
//! - `types`: 数据类型定义
//! - `config`: 配置和引擎管理
//! - `search_error`: 搜索专用错误类型

use moka::sync::Cache;
use std::time::Duration;
use tokitai::tool;

use crate::tools::network::{ssrf_protection, NetworkResult};

// 子模块
pub mod config;
pub mod search_error;
pub mod types;

pub use config::*;
pub use search_error::SearchError;
pub use types::*;

// ============================================================================
// 搜索引擎实现 - DuckDuckGo
// ============================================================================

/// DuckDuckGo 引擎实现
#[allow(dead_code)]
pub struct DuckDuckGoEngine {
    client: reqwest::blocking::Client,
    health: parking_lot::RwLock<EngineHealth>,
}

#[allow(dead_code)]
impl DuckDuckGoEngine {
    pub fn new(config: &SearchConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 DuckDuckGo HTTP 客户端失败");

        Self {
            client,
            health: parking_lot::RwLock::new(EngineHealth::new("DuckDuckGo".to_string())),
        }
    }
}

impl SearchEngine for DuckDuckGoEngine {
    fn name(&self) -> &str {
        "DuckDuckGo"
    }

    fn engine_type(&self) -> SearchEngineType {
        SearchEngineType::DuckDuckGo
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let response = self.client.get(&url).send().map_err(SearchError::Network)?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: "DuckDuckGo 返回错误状态".to_string(),
            });
        }

        let body = response.text().map_err(SearchError::Network)?;

        if body.contains("503 Service Unavailable") {
            return Err(SearchError::ApiError {
                status: 503,
                message: "DuckDuckGo 服务不可用".to_string(),
            });
        }

        let results = parse_duckduckgo_results(&body, limit).ok_or(SearchError::NoResults)?;

        if results.is_empty() {
            return Err(SearchError::NoResults);
        }

        Ok(results)
    }

    fn health_check(&self) -> bool {
        self.client.get("https://duckduckgo.com").send().is_ok()
    }

    fn get_health(&self) -> EngineHealth {
        self.health.read().clone()
    }

    fn record_success(&self, response_time_ms: f64) {
        let mut health = self.health.write();
        health.success_count += 1;
        health.success_rate =
            health.success_count as f32 / (health.success_count + health.fail_count) as f32;
        health.avg_response_time_ms = response_time_ms;
        health.last_check = std::time::Instant::now();
    }

    fn record_failure(&self) {
        let mut health = self.health.write();
        health.fail_count += 1;
        health.success_rate =
            health.success_count as f32 / (health.success_count + health.fail_count) as f32;
        health.last_check = std::time::Instant::now();
    }
}

// ============================================================================
// 搜索引擎实现 - SearXNG
// ============================================================================

/// SearXNG 引擎实现
#[allow(dead_code)]
pub struct SearxngEngine {
    url: String,
    client: reqwest::blocking::Client,
    health: parking_lot::RwLock<EngineHealth>,
}

#[allow(dead_code)]
impl SearxngEngine {
    pub fn new(url: &str, config: &SearchConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 SearXNG HTTP 客户端失败");

        Self {
            url: url.to_string(),
            client,
            health: parking_lot::RwLock::new(EngineHealth::new(format!("SearXNG({})", url))),
        }
    }
}

impl SearchEngine for SearxngEngine {
    fn name(&self) -> &str {
        "SearXNG"
    }

    fn engine_type(&self) -> SearchEngineType {
        SearchEngineType::Searxng
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "{}/search?q={}&format=json&engines=bing,duckduckgo",
            self.url, encoded_query
        );

        let response = self.client.get(&url).send().map_err(SearchError::Network)?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: "SearXNG API 返回错误状态".to_string(),
            });
        }

        let searxng_resp: SearxngResponse = response.json().map_err(SearchError::Network)?;

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
        self.client.get(&url).send().is_ok()
    }

    fn get_health(&self) -> EngineHealth {
        self.health.read().clone()
    }

    fn record_success(&self, response_time_ms: f64) {
        let mut health = self.health.write();
        health.success_count += 1;
        health.success_rate =
            health.success_count as f32 / (health.success_count + health.fail_count) as f32;
        health.avg_response_time_ms = response_time_ms;
        health.last_check = std::time::Instant::now();
    }

    fn record_failure(&self) {
        let mut health = self.health.write();
        health.fail_count += 1;
        health.success_rate =
            health.success_count as f32 / (health.success_count + health.fail_count) as f32;
        health.last_check = std::time::Instant::now();
    }
}

// ============================================================================
// SearXNG 响应结构
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Debug, serde::Deserialize)]
struct SearxngResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    engine: String,
}

// ============================================================================
// 搜索工具集
// ============================================================================

/// 网络搜索工具集 - 支持多引擎搜索
pub struct SearchTools {
    config: SearchConfig,
    client: reqwest::blocking::Client,
    engine_manager: SearchEngineManager,
    cache: Cache<String, String>,
}

#[tool]
impl SearchTools {
    /// 搜索网页内容（多引擎智能路由）
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    ///
    /// # 返回
    /// 返回 JSON 格式的搜索结果列表
    #[tool(default_limit = "null")]
    pub fn search_web(&self, query: String, limit: Option<usize>) -> NetworkResult<String> {
        let limit = limit
            .unwrap_or(self.config.default_limit)
            .min(self.config.max_limit);

        tracing::info!("🔍 搜索网页：{} (limit={})", query, limit);

        // 检查缓存
        if let Some(cached) = self.cache.get(&query) {
            tracing::debug!("使用缓存搜索结果");
            return Ok(cached);
        }

        let start = std::time::Instant::now();

        // 使用引擎管理器搜索
        let results = self.search_with_engines(&query, limit);

        let elapsed = start.elapsed();

        match results {
            Ok(results) => {
                tracing::info!(
                    "✅ 搜索成功，找到 {} 条结果 (耗时 {:.0}ms)",
                    results.len(),
                    elapsed.as_millis()
                );

                let response = SearchResponse::new(query.clone(), results);

                let json = serde_json::to_string_pretty(&response)?;

                // 存入缓存
                if self.config.enable_cache {
                    self.cache.insert(query, json.clone());
                }

                Ok(json)
            }
            Err(e) => {
                tracing::error!("搜索失败：{}", e);
                Err(e)
            }
        }
    }

    /// 获取指定 URL 的网页内容
    ///
    /// # 参数
    /// - `url`: 要获取的网页 URL
    ///
    /// # 返回
    /// 返回清理后的文本内容（最多 5000 字符）
    pub fn fetch_url(&self, url: String) -> NetworkResult<String> {
        tracing::info!("📄 获取网页：{}", url);

        // SSRF 验证
        ssrf_protection::validate_url(&url)?;

        let response = self.client.get(&url).send()?;

        let body = response.text()?;
        let text = extract_text_from_html(&body);
        Ok(text.chars().take(5000).collect())
    }

    /// 搜索 arXiv 论文
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    ///
    /// # 返回
    /// 返回 JSON 格式的论文列表
    #[tool(default_limit = "null")]
    pub fn search_arxiv(&self, query: String, limit: Option<usize>) -> NetworkResult<String> {
        let limit = limit
            .unwrap_or(self.config.default_limit)
            .min(self.config.max_limit);

        tracing::info!("📚 搜索 arXiv 论文：{} (limit={})", query, limit);

        let base_url = "https://export.arxiv.org/api/query";
        let encoded_query = urlencoding::encode(&query);
        let url = format!(
            "{}?search_query=all:{}&start=0&max_results={}&sortBy=relevance&sortOrder=descending",
            base_url, encoded_query, limit
        );

        let response = self.client.get(&url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: format!("arXiv API 返回错误状态：{}", status),
            }
            .into());
        }

        let body = response.text()?;
        let results = parse_arxiv_results(&body, limit)?;

        let response = SearchResponse::new(query, results);

        Ok(serde_json::to_string_pretty(&response)?)
    }

    /// 搜索图片（使用 SearXNG 图片搜索引擎）
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 10，最大 50）
    ///
    /// # 返回
    /// 返回 JSON 格式的图片列表，包含图片 URL、缩略图、来源等信息
    #[tool(default_limit = "null")]
    pub fn search_images(&self, query: String, limit: Option<usize>) -> NetworkResult<String> {
        let limit = limit.unwrap_or(10).min(50);

        tracing::info!("🖼️ 搜索图片：{} (limit={})", query, limit);

        // 收集所有要尝试的 SearXNG 实例
        let mut instances_to_try = Vec::new();

        // 优先使用环境变量
        if let Ok(searxng_url) = std::env::var("SEARXNG_URL") {
            instances_to_try.push(searxng_url);
        }

        // 添加配置的实例
        instances_to_try.extend(self.config.searxng_instances.clone());

        // 依次尝试每个实例
        for searxng_url in instances_to_try {
            match self.search_with_searxng_images(&searxng_url, &query, limit) {
                Ok(results) => {
                    tracing::info!("✅ SearXNG 图片搜索成功 [{}]", searxng_url);
                    let response = ImageSearchResponse::new(query.clone(), results);
                    return Ok(serde_json::to_string_pretty(&response)?);
                }
                Err(e) => {
                    tracing::warn!("SearXNG 图片实例 [{}] 失败：{}", searxng_url, e);
                }
            }
        }

        tracing::error!("所有 SearXNG 图片实例不可用");
        Err(SearchError::EngineUnavailable {
            engine: "SearXNG Images".to_string(),
        }
        .into())
    }

    /// 搜索新闻（使用 SearXNG news 引擎）
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `days`: 搜索最近 N 天的新闻（默认 7 天）
    #[tool]
    pub fn search_news(&self, query: String, days: u32) -> NetworkResult<String> {
        let days = if days == 0 { 7 } else { days };

        tracing::info!("📰 搜索新闻：{} (最近{}天)", query, days);

        // 收集所有要尝试的 SearXNG 实例
        let mut instances_to_try = Vec::new();

        if let Ok(searxng_url) = std::env::var("SEARXNG_URL") {
            instances_to_try.push(searxng_url);
        }
        instances_to_try.extend(self.config.searxng_instances.clone());

        // 依次尝试每个实例
        for searxng_url in instances_to_try {
            let encoded_query = urlencoding::encode(&query);
            let url = format!(
                "{}/search?q={}&format=json&engines=bing_news&categories=news",
                searxng_url, encoded_query
            );

            match self.search_with_searxng_news(&searxng_url, &url) {
                Ok(results) => {
                    tracing::info!("✅ SearXNG 新闻搜索成功 [{}]", searxng_url);
                    let response = SearchResponse::new(query.clone(), results);
                    return Ok(serde_json::to_string_pretty(&response)?);
                }
                Err(e) => {
                    tracing::warn!("SearXNG 新闻实例 [{}] 失败：{}", searxng_url, e);
                }
            }
        }

        // 所有 SearXNG 实例都失败，回退到普通网页搜索
        tracing::warn!("所有 SearXNG 新闻实例不可用，使用普通网页搜索");
        self.search_web(query, Some(days.min(5) as usize))
    }

    /// 搜索维基百科（中文）
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    #[tool(default_limit = "5")]
    pub fn search_wikipedia(&self, query: String, limit: Option<usize>) -> NetworkResult<String> {
        let limit = limit.unwrap_or(5).min(20);

        tracing::info!("📚 搜索维基百科：{} (limit={})", query, limit);

        let encoded = urlencoding::encode(&query);

        let url = format!(
            "https://zh.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
            encoded, limit
        );

        let response = self.client.get(&url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: format!("维基百科 API 返回错误状态：{}", status),
            }
            .into());
        }

        let json: WikipediaResponse = response.json()?;

        let results: Vec<SearchResult> = json
            .query
            .search
            .into_iter()
            .map(|r| {
                let wiki_url = r
                    .url
                    .unwrap_or_else(|| format!("https://zh.wikipedia.org/wiki/{}", r.title));
                SearchResult {
                    title: r.title,
                    url: wiki_url,
                    snippet: clean_snippet(&r.snippet),
                    engine: "wikipedia".to_string(),
                }
            })
            .collect();

        let total = results.len();

        if total == 0 {
            tracing::warn!("未找到维基百科结果");
            return Ok(serde_json::to_string_pretty(&serde_json::json!({
                "query": query,
                "total": 0,
                "results": [],
                "message": "未找到相关结果"
            }))?);
        }

        tracing::info!("✅ 维基百科搜索成功，找到 {} 条结果", total);

        let response = SearchResponse::new(query, results);
        Ok(serde_json::to_string_pretty(&response)?)
    }

    /// 搜索英文维基百科
    #[tool(default_limit = "5")]
    pub fn search_wikipedia_en(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> NetworkResult<String> {
        let limit = limit.unwrap_or(5).min(20);

        tracing::info!("📚 搜索英文维基百科：{} (limit={})", query, limit);

        let encoded = urlencoding::encode(&query);

        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
            encoded, limit
        );

        let response = self.client.get(&url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: format!("维基百科 API 返回错误状态：{}", status),
            }
            .into());
        }

        let json: WikipediaResponse = response.json()?;

        let results: Vec<SearchResult> = json
            .query
            .search
            .into_iter()
            .map(|r| {
                let wiki_url = r
                    .url
                    .unwrap_or_else(|| format!("https://en.wikipedia.org/wiki/{}", r.title));
                SearchResult {
                    title: r.title,
                    url: wiki_url,
                    snippet: clean_snippet(&r.snippet),
                    engine: "wikipedia-en".to_string(),
                }
            })
            .collect();

        let response = SearchResponse::new(query, results);
        Ok(serde_json::to_string_pretty(&response)?)
    }

    /// 获取搜索统计信息
    pub fn get_stats(&self) -> NetworkResult<serde_json::Value> {
        let stats = self.engine_manager.get_stats();
        Ok(serde_json::json!({
            "total_searches": stats.total_searches,
            "successful_searches": stats.successful_searches,
            "failed_searches": stats.failed_searches,
            "cache_hits": stats.cache_hits,
            "avg_response_time_ms": stats.avg_response_time_ms,
            "success_rate": stats.success_rate(),
            "cache_hit_rate": stats.cache_hit_rate(),
        }))
    }

    /// 清空搜索缓存
    pub fn clear_cache(&self) -> NetworkResult<String> {
        self.cache.invalidate_all();
        Ok("✅ 搜索缓存已清空".to_string())
    }

    /// 获取引擎健康状态
    pub fn get_engine_health(&self) -> NetworkResult<serde_json::Value> {
        let health = self.engine_manager.get_health_status();
        Ok(serde_json::json!({
            "engines": health.iter().map(|h| {
                serde_json::json!({
                    "name": h.name,
                    "is_healthy": h.is_healthy,
                    "success_rate": h.success_rate,
                    "avg_response_time_ms": h.avg_response_time_ms,
                })
            }).collect::<Vec<_>>(),
        }))
    }
}

impl SearchTools {
    pub fn new() -> Self {
        Self::with_config(SearchConfig::default())
    }

    pub fn with_config(config: SearchConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 HTTP 客户端失败");

        let engine_manager = SearchEngineManager::new(config.clone());

        let cache = Cache::builder()
            .max_capacity(config.cache_capacity)
            .time_to_live(Duration::from_secs(config.cache_ttl_secs))
            .build();

        Self {
            config,
            client,
            engine_manager,
            cache,
        }
    }

    /// 使用搜索引擎管理器搜索
    fn search_with_engines(&self, query: &str, limit: usize) -> NetworkResult<Vec<SearchResult>> {
        let start = std::time::Instant::now();

        // 按健康度排序引擎
        let sorted_engines = self.engine_manager.get_sorted_engines();

        let mut last_error = None;

        // 仅尝试前 2 个最健康的引擎
        for (engine, _score) in sorted_engines.into_iter().take(2) {
            // 跳过健康度低的引擎
            if _score < 0.5 {
                tracing::debug!("跳过健康度低的引擎 (score={})", _score);
                continue;
            }

            tracing::debug!("尝试引擎：{} (score={})", engine.name(), _score);

            match engine.search(query, limit) {
                Ok(results) => {
                    engine.record_success(start.elapsed().as_millis() as f64);
                    return Ok(results);
                }
                Err(e) => {
                    tracing::warn!("引擎 {} 失败：{}", engine.name(), e);
                    engine.record_failure();
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(SearchError::NoResults).into())
    }

    /// 使用 SearXNG 搜索新闻
    fn search_with_searxng_news(
        &self,
        _base_url: &str,
        url: &str,
    ) -> NetworkResult<Vec<SearchResult>> {
        let response = self.client.get(url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: "SearXNG 新闻 API 返回错误状态".to_string(),
            }
            .into());
        }

        let searxng_resp: SearxngResponse = response.json()?;

        let results: Vec<SearchResult> = searxng_resp
            .results
            .into_iter()
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.content,
                engine: r.engine,
            })
            .collect();

        Ok(results)
    }

    /// 使用 SearXNG 搜索图片
    fn search_with_searxng_images(
        &self,
        searxng_url: &str,
        query: &str,
        limit: usize,
    ) -> NetworkResult<Vec<ImageSearchResult>> {
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "{}/search?q={}&format=json&categories=images",
            searxng_url, encoded_query
        );

        let response = self.client.get(&url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: "SearXNG 图片 API 返回错误状态".to_string(),
            }
            .into());
        }

        let searxng_resp: SearxngImageResponse = response.json()?;

        let results: Vec<ImageSearchResult> = searxng_resp
            .results
            .into_iter()
            .take(limit)
            .map(|r| ImageSearchResult {
                title: r.title,
                url: r.url,
                img_src: r.img_src,
                thumbnail: r.thumbnail,
                source: r.source,
                engine: r.engine,
            })
            .collect();

        if results.is_empty() {
            return Err(SearchError::NoResults.into());
        }

        Ok(results)
    }
}

impl Default for SearchTools {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 图片搜索响应结构
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct SearxngImageResponse {
    results: Vec<SearxngImageResult>,
}

#[derive(Debug, serde::Deserialize)]
struct SearxngImageResult {
    title: String,
    url: String,
    img_src: String,
    thumbnail: Option<String>,
    #[serde(default)]
    source: String,
    #[serde(default)]
    engine: String,
}

// ============================================================================
// 维基百科响应结构
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct WikipediaResponse {
    query: WikipediaQuery,
}

#[derive(Debug, serde::Deserialize)]
struct WikipediaQuery {
    search: Vec<WikipediaSearchResult>,
}

#[derive(Debug, serde::Deserialize)]
struct WikipediaSearchResult {
    title: String,
    snippet: String,
    #[serde(default)]
    url: Option<String>,
}

// ============================================================================
// 工具函数
// ============================================================================

/// 解析 DuckDuckGo HTML 结果
#[allow(dead_code)]
fn parse_duckduckgo_results(html: &str, limit: usize) -> Option<Vec<SearchResult>> {
    let document = scraper::Html::parse_document(html);
    let result_selector = scraper::Selector::parse(".result").ok()?;
    let title_selector = scraper::Selector::parse(".result__title").ok()?;
    let snippet_selector = scraper::Selector::parse(".result__snippet").ok()?;
    let url_selector = scraper::Selector::parse(".result__url").ok()?;

    let mut results = Vec::new();

    for element in document.select(&result_selector).take(limit) {
        if let Some(title) = element.select(&title_selector).next() {
            let title_text = title.text().collect::<String>().trim().to_string();

            let snippet = element
                .select(&snippet_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let url = element
                .select(&url_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if !title_text.is_empty() && !url.is_empty() {
                results.push(SearchResult {
                    title: title_text,
                    url,
                    snippet,
                    engine: "duckduckgo".to_string(),
                });
            }
        }
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

/// 解析 arXiv 结果
fn parse_arxiv_results(xml: &str, limit: usize) -> NetworkResult<Vec<SearchResult>> {
    let mut results = Vec::new();

    let mut current_entry = String::new();
    let mut in_entry = false;

    for line in xml.lines() {
        let line = line.trim();

        if line.contains("<entry>") {
            in_entry = true;
            current_entry.clear();
        }

        if in_entry {
            current_entry.push_str(line);
        }

        if line.contains("</entry>") {
            in_entry = false;

            if let Some(title) = extract_xml_tag(&current_entry, "title") {
                if let Some(id) = extract_xml_tag(&current_entry, "id") {
                    let arxiv_id = id
                        .split("/abs/")
                        .last()
                        .unwrap_or(&id)
                        .split("/pdf/")
                        .last()
                        .unwrap_or(&id);

                    let summary = extract_xml_tag(&current_entry, "summary")
                        .map(|s| s.split_whitespace().take(30).collect::<Vec<_>>().join(" "))
                        .unwrap_or_else(|| "无摘要".to_string());

                    let url = format!("https://arxiv.org/abs/{}", arxiv_id);
                    let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);

                    results.push(SearchResult {
                        title: trim_whitespace(&title),
                        url,
                        snippet: format!("[PDF: {}] {}", pdf_url, trim_whitespace(&summary)),
                        engine: "arxiv".to_string(),
                    });
                }
            }
        }
    }

    if results.is_empty() {
        Err(SearchError::NoResults.into())
    } else {
        Ok(results.into_iter().take(limit).collect())
    }
}

/// 提取 XML 标签内容
fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);

    if let Some(start) = content.find(&open_tag) {
        let start = start + open_tag.len();
        if let Some(end) = content[start..].find(&close_tag) {
            return Some(content[start..start + end].to_string());
        }
    }
    None
}

/// 从 HTML 提取文本
fn extract_text_from_html(html: &str) -> String {
    let document = scraper::Html::parse_document(html);
    document.root_element().text().collect()
}

/// 清理维基百科摘要中的 HTML 标签
fn clean_snippet(snippet: &str) -> String {
    let without_html = snippet
        .replace("<span class=\"searchmatch\">", "")
        .replace("</span>", "");

    trim_whitespace(&without_html)
}

/// 去除首尾空白并压缩中间空白
fn trim_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.default_limit, 5);
        assert_eq!(config.max_limit, 20);
        assert!(config.enable_cache);
    }

    #[test]
    fn test_search_tools_creation() {
        let tools = SearchTools::new();
        assert_eq!(tools.config.timeout_secs, 10);
    }

    #[test]
    fn test_search_response_serialization() {
        let response = SearchResponse::new(
            "test".to_string(),
            vec![SearchResult::simple(
                "Test".to_string(),
                "https://example.com".to_string(),
                "A test".to_string(),
            )],
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("example.com"));
    }

    #[test]
    fn test_image_search_result_serialization() {
        let result = ImageSearchResult::new(
            "Test Image".to_string(),
            "https://example.com".to_string(),
            "https://example.com/image.jpg".to_string(),
            "Example".to_string(),
            "test-engine".to_string(),
        );

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test Image"));
        assert!(json.contains("image.jpg"));
    }

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_whitespace("  hello   world  "), "hello world");
        assert_eq!(trim_whitespace(""), "");
    }

    #[test]
    fn test_clean_snippet() {
        let input = r#"<span class="searchmatch">Rust</span> is a programming language"#;
        let result = clean_snippet(input);
        assert_eq!(result, "Rust is a programming language");
    }
}
