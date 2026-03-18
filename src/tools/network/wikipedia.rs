//! 维基百科搜索工具
//!
//! 使用维基百科 API 进行搜索，无需 API key
//! 支持多语言（中文、英文等）

use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;
use tokitai::tool;

use super::error::{NetworkResult, SearchError};
use super::search::types::{SearchResult, SearchResponse};

// ============================================================================
// 配置结构
// ============================================================================

/// 维基百科工具配置
#[derive(Debug, Clone)]
pub struct WikipediaConfig {
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// 连接超时（秒）
    pub connect_timeout_secs: u64,
    /// 默认搜索结果数量
    pub default_limit: usize,
    /// 最大搜索结果数量
    pub max_limit: usize,
    /// User-Agent
    pub user_agent: String,
}

impl Default for WikipediaConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            connect_timeout_secs: 5,
            default_limit: 5,
            max_limit: 20,
            user_agent: "Tokitai AI Assistant/1.0".to_string(),
        }
    }
}

// ============================================================================
// 数据结构
// ============================================================================

/// 维基百科搜索响应结构
#[derive(Debug, Deserialize)]
struct WikipediaApiQuery {
    search: Vec<WikipediaApiResult>,
}

#[derive(Debug, Deserialize)]
struct WikipediaApiResult {
    title: String,
    snippet: String,
    #[serde(default)]
    url: Option<String>,
}

// ============================================================================
// 维基百科工具集
// ============================================================================

/// 维基百科搜索工具
pub struct WikipediaTools {
    config: WikipediaConfig,
    client: Client,
}

#[tool]
impl WikipediaTools {
    /// 搜索维基百科（中文）
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    ///
    /// # 返回
    /// 返回 JSON 格式的搜索结果列表，包含标题、URL、摘要
    #[tool(default_limit = "null")]
    pub fn search_wikipedia(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> NetworkResult<String> {
        let limit = limit.unwrap_or(self.config.default_limit).min(self.config.max_limit);

        tracing::info!("📚 搜索维基百科：{} (limit={})", query, limit);

        let encoded = urlencoding::encode(&query);

        // 使用中文维基百科 API
        let url = format!(
            "https://zh.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
            encoded, limit
        );

        let response = self.client.get(&url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: "维基百科 API 返回错误状态".to_string(),
            }.into());
        }

        let json: WikipediaApiQuery = response.json()?;

        // 格式化结果
        let results: Vec<SearchResult> = json
            .search
            .into_iter()
            .map(|r| {
                let wiki_url = r.url.unwrap_or_else(|| {
                    format!("https://zh.wikipedia.org/wiki/{}", r.title)
                });
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
            let response = SearchResponse::with_message(query, "未找到相关结果");
            return Ok(serde_json::to_string_pretty(&response)?);
        }

        tracing::info!("✅ 维基百科搜索成功，找到 {} 条结果", total);

        let response = SearchResponse::new(query, results);
        Ok(serde_json::to_string_pretty(&response)?)
    }

    /// 搜索英文维基百科
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    #[tool(default_limit = "null")]
    pub fn search_wikipedia_en(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> NetworkResult<String> {
        let limit = limit.unwrap_or(self.config.default_limit).min(self.config.max_limit);

        tracing::info!("📚 搜索英文维基百科：{} (limit={})", query, limit);

        let encoded = urlencoding::encode(&query);

        // 使用英文维基百科 API
        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
            encoded, limit
        );

        let response = self.client.get(&url).send()?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(SearchError::ApiError {
                status,
                message: "维基百科 API 返回错误状态".to_string(),
            }.into());
        }

        let json: WikipediaApiQuery = response.json()?;

        let results: Vec<SearchResult> = json
            .search
            .into_iter()
            .map(|r| {
                let wiki_url = r.url.unwrap_or_else(|| {
                    format!("https://en.wikipedia.org/wiki/{}", r.title)
                });
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

    /// 获取维基百科页面内容
    ///
    /// # 参数
    /// - `title`: 页面标题
    ///
    /// # 返回
    /// 返回页面内容（纯文本）
    pub fn get_page_content(&self, title: String) -> NetworkResult<String> {
        let encoded = urlencoding::encode(&title);

        let url = format!(
            "https://zh.wikipedia.org/w/api.php?action=query&prop=extracts&exintro=true&explaintext=true&format=json&titles={}",
            encoded
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(SearchError::ApiError {
                status: response.status().as_u16(),
                message: "获取页面失败".to_string(),
            }.into());
        }

        // 解析响应获取内容
        let json: serde_json::Value = response.json()?;

        // 提取页面内容
        if let Some(pages) = json.get("query").and_then(|q| q.get("pages")) {
            if let Some(page) = pages.as_object().and_then(|o| o.values().next()) {
                if let Some(extract) = page.get("extract").and_then(|e| e.as_str()) {
                    if extract.is_empty() {
                        return Ok("页面无内容".to_string());
                    }
                    return Ok(extract.to_string());
                }
            }
        }

        Err(SearchError::NoResults.into())
    }
}

impl WikipediaTools {
    pub fn new() -> Self {
        Self::with_config(WikipediaConfig::default())
    }

    pub fn with_config(config: WikipediaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .expect("创建 HTTP 客户端失败");

        Self { config, client }
    }
}

impl Default for WikipediaTools {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 清理维基百科摘要中的 HTML 标签和多余空格
fn clean_snippet(snippet: &str) -> String {
    let without_html = snippet
        .replace("<span class=\"searchmatch\">", "")
        .replace("</span>", "");

    without_html.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wikipedia_config_default() {
        let config = WikipediaConfig::default();
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.default_limit, 5);
        assert_eq!(config.max_limit, 20);
    }

    #[test]
    fn test_clean_snippet() {
        let input = r#"<span class="searchmatch">Rust</span> is a programming language"#;
        let result = clean_snippet(input);
        assert_eq!(result, "Rust is a programming language");
    }

    #[test]
    fn test_wikipedia_tools_creation() {
        let tools = WikipediaTools::new();
        assert_eq!(tools.config.timeout_secs, 10);
    }

    #[test]
    fn test_search_response_serialization() {
        let response = SearchResponse::new(
            "test".to_string(),
            vec![SearchResult {
                title: "Test".to_string(),
                url: "https://example.com".to_string(),
                snippet: "A test".to_string(),
                engine: "wikipedia".to_string(),
            }],
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("example.com"));
    }
}
