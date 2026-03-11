use anyhow::{Context, Result, bail};
use moka::sync::Cache;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokitai::tool;
use urlencoding::encode;

/// 搜索错误类型
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("网络请求失败：{0}")]
    Network(#[from] ureq::Error),

    #[error("搜索 API 返回错误：{status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("未找到搜索结果")]
    NoResults,

    #[error("搜索超时")]
    Timeout(#[from] std::io::Error),
}

/// 搜索结果结构（JSON 格式）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    #[serde(default)]
    pub engine: String,
}

/// 搜索响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub results: Vec<SearchResult>,
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

/// 网络搜索工具集 - 支持多引擎搜索
pub struct WebSearchTools {
    client: ureq::Agent,
    max_retries: u32,
    cache: Cache<String, String>,
    /// SearXNG 实例列表（按优先级排序）
    searxng_instances: Vec<String>,
}

/// 获取推荐的 SearXNG 公共实例列表
fn get_searxng_instances() -> Vec<String> {
    vec![
        // 如果用户配置了 SEARXNG_URL，优先使用
        std::env::var("SEARXNG_URL").unwrap_or_default(),
        // 公共实例（按可靠性排序）
        "https://searx.be".to_string(),
        "https://search.ononoki.org".to_string(),
        "https://s.x.com".to_string(),
        "https://searx.org".to_string(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect()
}

#[tool]
impl WebSearchTools {
    /// 使用 DuckDuckGo 搜索网页内容
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    ///
    /// # 返回
    /// 返回 JSON 格式的搜索结果列表
    #[tool(default_limit = "null")]
    pub fn search_web(&self, query: String, limit: Option<usize>) -> Result<String> {
        let limit = limit.unwrap_or(5).min(20);
        
        tracing::info!("🔍 搜索网页：{} (limit={})", query, limit);
        
        // 检查缓存
        if let Some(cached) = self.cache.get(&query) {
            tracing::debug!("✅ 使用缓存结果");
            return Ok(cached);
        }
        
        // 策略 1：优先尝试 SearXNG 实例（更可靠）
        for (i, searxng_url) in self.searxng_instances.iter().enumerate() {
            tracing::debug!("尝试 SearXNG 实例 [{}/{}]: {}", i + 1, self.searxng_instances.len(), searxng_url);
            match self.search_with_searxng(searxng_url, &query, limit) {
                Ok(results) => {
                    tracing::info!("✅ SearXNG 搜索成功 [{}]", searxng_url);
                    // 存入缓存
                    self.cache.insert(query.clone(), results.clone());
                    return Ok(results);
                }
                Err(e) => {
                    tracing::warn!("SearXNG [{}] 失败：{}", searxng_url, e);
                    // 继续尝试下一个实例
                }
            }
        }
        
        // 策略 2：回退到 DuckDuckGo
        tracing::info!("所有 SearXNG 实例不可用，回退到 DuckDuckGo");
        match self.search_with_duckduckgo(&query, limit) {
            Ok(result) => {
                // 存入缓存
                self.cache.insert(query, result.clone());
                Ok(result)
            }
            Err(e) => {
                // 所有搜索引擎都失败
                tracing::error!("所有搜索引擎都失败：{}", e);
                Err(anyhow::anyhow!(
                    "搜索失败：{}。建议：1) 检查网络连接 2) 稍后重试",
                    e
                ))
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
    pub fn fetch_url(&self, url: String) -> Result<String> {
        tracing::info!("📄 获取网页：{}", url);
        
        let response = self.client.get(&url)
            .call()
            .context("获取网页失败，请检查网络连接")?;

        let body = response
            .into_string()
            .context("读取网页内容失败")?;

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
    pub fn search_arxiv(&self, query: String, limit: Option<usize>) -> Result<String> {
        let limit = limit.unwrap_or(5).min(20);
        
        tracing::info!("📚 搜索 arXiv 论文：{} (limit={})", query, limit);
        
        let base_url = "https://export.arxiv.org/api/query";
        let encoded_query = encode(&query);
        let url = format!(
            "{}?search_query=all:{}&start=0&max_results={}&sortBy=relevance&sortOrder=descending",
            base_url, encoded_query, limit
        );

        let response = self.client.get(&url)
            .call()
            .context("arXiv 搜索请求失败")?;

        if response.status() != 200 {
            bail!("arXiv API 返回错误状态：{}", response.status());
        }

        let body = response.into_string()?;
        let results = parse_arxiv_results(&body, limit)?;
        
        let response_obj = SearchResponse {
            query,
            total: results.len(),
            results,
        };

        Ok(serde_json::to_string_pretty(&response_obj)?)
    }

    /// 搜索新闻（使用 SearXNG news 引擎）
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `days`: 搜索最近 N 天的新闻（默认 7 天，必填）
    #[tool]
    pub fn search_news(&self, query: String, days: u32) -> Result<String> {
        let days = if days == 0 { 7 } else { days };
        
        tracing::info!("📰 搜索新闻：{} (最近{}天)", query, days);
        
        // 策略 1：优先尝试 SearXNG 实例
        for (i, searxng_url) in self.searxng_instances.iter().enumerate() {
            tracing::debug!("尝试 SearXNG 新闻实例 [{}/{}]: {}", i + 1, self.searxng_instances.len(), searxng_url);
            
            let encoded_query = encode(&query);
            let url = format!(
                "{}/search?q={}&format=json&engines=google_news,bing_news&categories=news",
                searxng_url, encoded_query
            );
            
            match self.search_with_searxng_news(searxng_url, &url) {
                Ok(results) => {
                    tracing::info!("✅ SearXNG 新闻搜索成功 [{}]", searxng_url);
                    let response_obj = SearchResponse {
                        query: query.clone(),
                        total: results.len(),
                        results,
                    };
                    return Ok(serde_json::to_string_pretty(&response_obj)?);
                }
                Err(e) => {
                    tracing::warn!("SearXNG 新闻实例 [{}] 失败：{}", searxng_url, e);
                    // 继续尝试下一个实例
                }
            }
        }
        
        // 策略 2：回退到普通网页搜索
        tracing::warn!("所有 SearXNG 新闻实例不可用，使用普通网页搜索");
        self.search_web(query, Some(days as usize))
    }
}

impl WebSearchTools {
    pub fn new() -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (compatible; AI Assistant/1.0)")
            .build();

        let cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(3600))  // 1 小时过期
            .build();

        Self {
            client,
            max_retries: 3,
            cache,
            searxng_instances: get_searxng_instances(),
        }
    }

    /// 使用 SearXNG 搜索新闻
    fn search_with_searxng_news(&self, _base_url: &str, url: &str) -> Result<Vec<SearchResult>> {
        let response = self.client.get(url)
            .timeout(Duration::from_secs(10))
            .call()
            .context("SearXNG 新闻请求失败")?;

        if response.status() != 200 {
            bail!("SearXNG 新闻 API 返回错误状态：{}", response.status());
        }

        let searxng_resp: SearxngResponse = response.into_json()?;
        let results: Vec<SearchResult> = searxng_resp.results
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

    /// 使用 SearXNG 搜索
    fn search_with_searxng(&self, base_url: &str, query: &str, limit: usize) -> Result<String> {
        let encoded_query = encode(query);
        let url = format!(
            "{}/search?q={}&format=json&engines=google,bing,duckduckgo&categories=general",
            base_url, encoded_query
        );

        let response = self.client.get(&url)
            .call()
            .context("SearXNG 请求失败")?;

        if response.status() != 200 {
            bail!("SearXNG 返回错误状态：{}", response.status());
        }

        let searxng_resp: SearxngResponse = response.into_json()?;
        let results: Vec<SearchResult> = searxng_resp.results
            .into_iter()
            .take(limit)
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.content,
                engine: r.engine,
            })
            .collect();

        let response_obj = SearchResponse {
            query: query.to_string(),
            total: results.len(),
            results,
        };

        Ok(serde_json::to_string_pretty(&response_obj)?)
    }

    /// 使用 DuckDuckGo 搜索（带重试）
    fn search_with_duckduckgo(&self, query: &str, limit: usize) -> Result<String> {
        let encoded_query = encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        tracing::debug!("DuckDuckGo 搜索 URL: {}", url);

        // 指数退避重试
        let mut last_error = None;
        for attempt in 1..=self.max_retries {
            match self.search_ddg_inner(&url, limit) {
                Ok(results) => {
                    tracing::info!("✅ DuckDuckGo 搜索成功，找到 {} 条结果", results.len());
                    let response_obj = SearchResponse {
                        query: query.to_string(),
                        total: results.len(),
                        results,
                    };
                    return Ok(serde_json::to_string_pretty(&response_obj)?);
                }
                Err(e) => {
                    tracing::warn!("⚠️ 搜索失败 (尝试 {}/{}): {}", attempt, self.max_retries, e);
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        let delay = Duration::from_millis(attempt as u64 * 300);
                        tracing::debug!("{}ms 后重试", delay.as_millis());
                        std::thread::sleep(delay);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("搜索失败")))
    }

    fn search_ddg_inner(&self, url: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let response = self.client.get(url)
            .call()
            .context("搜索请求失败，请检查网络连接")?;

        let status = response.status();
        tracing::debug!("DuckDuckGo 响应状态：{}", status);

        if status != 200 {
            // 503 表示速率限制
            if status == 503 {
                bail!(SearchError::ApiError {
                    status: 503,
                    message: "DuckDuckGo 服务不可用，可能触发了速率限制".to_string(),
                });
            }
            bail!("搜索 API 返回错误状态：{}", status);
        }

        let body = response.into_string()?;
        
        // 检查是否返回了错误页面
        if body.contains("503 Service Unavailable") || body.contains("Please try again") {
            return Err(SearchError::ApiError {
                status: 503,
                message: "DuckDuckGo 返回错误页面，可能触发了反爬虫机制".to_string(),
            }.into());
        }
        
        let results = parse_duckduckgo_results(&body, limit)
            .context("解析搜索结果失败")?;

        if results.is_empty() {
            bail!(SearchError::NoResults);
        }

        Ok(results)
    }
}

impl Default for WebSearchTools {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 DuckDuckGo HTML 搜索结果
fn parse_duckduckgo_results(html: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let document = Html::parse_document(html);
    let mut results = Vec::new();

    // 检查是否返回了错误页面
    if html.contains("503 Service Unavailable") {
        return Err(SearchError::ApiError {
            status: 503,
            message: "DuckDuckGo 服务不可用，可能触发了速率限制".to_string(),
        }.into());
    }

    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__a").unwrap();
    let url_selector = Selector::parse(".result__url").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

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
            .unwrap_or_else(|| String::new());

        results.push(SearchResult {
            title: trim_whitespace(&title),
            url: trim_whitespace(&url),
            snippet: trim_whitespace(&snippet),
            engine: "duckduckgo".to_string(),
        });
    }

    Ok(results)
}

/// 解析 arXiv XML 结果
fn parse_arxiv_results(xml: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();
    
    // 简单解析 arXiv Atom feed
    // 查找 entry 标签
    let entries: Vec<&str> = xml.split("<entry>").skip(1).collect();
    
    for entry in entries.into_iter().take(limit) {
        let title = extract_xml_tag(entry, "title")
            .unwrap_or_else(|| "无标题".to_string());
        let id = extract_xml_tag(entry, "id")
            .unwrap_or_else(|| "未知 URL".to_string());
        let summary = extract_xml_tag(entry, "summary")
            .unwrap_or_else(|| String::new());
        
        // 提取 arXiv ID
        let arxiv_id = id.split("/abs/").last().unwrap_or("unknown");
        
        results.push(SearchResult {
            title: trim_whitespace(&title),
            url: id.clone(),
            snippet: format!("[ARXIV_ID: {}] {}", arxiv_id, trim_whitespace(&summary)),
            engine: "arxiv".to_string(),
        });
    }
    
    Ok(results)
}

/// 从 XML 中提取标签内容
fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    if let Some(start) = content.find(&open_tag) {
        if let Some(end) = content.find(&close_tag) {
            let value = &content[start + open_tag.len()..end];
            return Some(value.trim().to_string());
        }
    }
    None
}

/// 从 HTML 中提取纯文本内容
fn extract_text_from_html(html: &str) -> String {
    let document = Html::parse_document(html);

    // 移除 script 和 style 标签
    let remove_selector = Selector::parse("script, style, noscript").unwrap();
    let mut to_remove = Vec::new();
    for element in document.select(&remove_selector) {
        to_remove.push(element.id());
    }

    // 优先提取 main 内容区域
    let content_selector = Selector::parse("main, article, .content, .post, .entry").unwrap();
    let body_selector = Selector::parse("body").unwrap();

    let content = document
        .select(&content_selector)
        .next()
        .or_else(|| document.select(&body_selector).next())
        .unwrap_or(document.root_element());

    let text = content
        .text()
        .collect::<Vec<_>>()
        .join("\n");

    clean_text(&text)
}

/// 清理文本中的多余空白
fn clean_text(text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 去除首尾空白并压缩中间空白
fn trim_whitespace(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_whitespace("  hello   world  "), "hello world");
        assert_eq!(trim_whitespace("single"), "single");
        assert_eq!(trim_whitespace(""), "");
    }

    #[test]
    fn test_clean_text() {
        let input = "line1\n  \nline2\n\n\nline3";
        let result = clean_text(input);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
    }

    #[test]
    fn test_extract_xml_tag() {
        let xml = "<entry><title>Test Title</title><summary>Test summary</summary></entry>";
        assert_eq!(extract_xml_tag(xml, "title"), Some("Test Title".to_string()));
        assert_eq!(extract_xml_tag(xml, "summary"), Some("Test summary".to_string()));
        assert_eq!(extract_xml_tag(xml, "nonexistent"), None);
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            snippet: "A test snippet".to_string(),
            engine: "test".to_string(),
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("example.com"));
    }
}
