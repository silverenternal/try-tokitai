/// 维基百科搜索工具
/// 
/// 使用维基百科 API 进行搜索，无需 API key
/// 支持多语言（中文、英文等）

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;
use tokitai::tool;

/// 维基百科搜索响应结构
#[derive(Debug, Deserialize)]
struct WikipediaResponse {
    query: WikipediaQuery,
}

#[derive(Debug, Deserialize)]
struct WikipediaQuery {
    search: Vec<WikipediaResult>,
}

#[derive(Debug, Deserialize)]
struct WikipediaResult {
    title: String,
    snippet: String,
    #[serde(default)]
    url: Option<String>,
}

/// 搜索结果结构（与 web_search.rs 兼容）
#[derive(Debug, serde::Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub engine: String,
}

/// 搜索响应结构
#[derive(Debug, serde::Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub results: Vec<SearchResult>,
}

/// 维基百科搜索工具
pub struct WikipediaTools {
    client: ureq::Agent,
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
    ///
    /// # 示例
    /// ```
    /// search_wikipedia(query="Rust 编程语言", limit=Some(5))
    /// ```
    #[tool(default_limit = "5")]
    pub fn search_wikipedia(&self, query: String, limit: Option<usize>) -> Result<String> {
        let limit = limit.unwrap_or(5).min(20);
        
        tracing::info!("📚 搜索维基百科：{} (limit={})", query, limit);
        
        let encoded = urlencoding::encode(&query);
        
        // 使用中文维基百科 API
        let url = format!(
            "https://zh.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
            encoded, limit
        );
        
        let response = self.client.get(&url)
            .call()
            .context("维基百科 API 请求失败，请检查网络连接")?;
        
        if response.status() != 200 {
            anyhow::bail!("维基百科 API 返回错误状态：{}", response.status());
        }
        
        let json: WikipediaResponse = response
            .into_json()
            .context("解析维基百科响应失败")?;
        
        // 格式化结果
        let results: Vec<SearchResult> = json.query.search
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
        
        let response_obj = SearchResponse {
            query: query.clone(),
            total,
            results,
        };

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
        
        Ok(serde_json::to_string_pretty(&response_obj)?)
    }
    
    /// 搜索英文维基百科
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 返回结果数量（默认 5，最大 20）
    #[tool(default_limit = "5")]
    pub fn search_wikipedia_en(&self, query: String, limit: Option<usize>) -> Result<String> {
        let limit = limit.unwrap_or(5).min(20);
        
        tracing::info!("📚 搜索英文维基百科：{} (limit={})", query, limit);
        
        let encoded = urlencoding::encode(&query);
        
        // 使用英文维基百科 API
        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json",
            encoded, limit
        );
        
        let response = self.client.get(&url)
            .call()
            .context("维基百科 API 请求失败")?;
        
        if response.status() != 200 {
            anyhow::bail!("维基百科 API 返回错误状态：{}", response.status());
        }
        
        let json: WikipediaResponse = response.into_json()?;
        
        let results: Vec<SearchResult> = json.query.search
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
        
        let response_obj = SearchResponse {
            query: query.clone(),
            total: results.len(),
            results,
        };
        
        Ok(serde_json::to_string_pretty(&response_obj)?)
    }
}

impl WikipediaTools {
    pub fn new() -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .user_agent("Tokitai/1.0 (Wikipedia Search)")
            .build();
        
        Self { client }
    }
}

impl Default for WikipediaTools {
    fn default() -> Self {
        Self::new()
    }
}

/// 清理维基百科摘要中的 HTML 标签和多余空格
fn clean_snippet(snippet: &str) -> String {
    // 移除 HTML 标签
    let without_html = snippet.replace("<span class=\"searchmatch\">", "")
                              .replace("</span>", "");
    
    // 清理多余空格
    without_html.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_clean_snippet() {
        let input = r#"<span class="searchmatch">Rust</span> is a programming language"#;
        let result = clean_snippet(input);
        assert_eq!(result, "Rust is a programming language");
    }
    
    #[test]
    fn test_wikipedia_tools_creation() {
        let tools = WikipediaTools::new();
        // 仅测试创建，不进行网络请求
        assert!(true);
    }
    
    #[test]
    fn test_search_response_serialization() {
        let response = SearchResponse {
            query: "test".to_string(),
            total: 1,
            results: vec![SearchResult {
                title: "Test".to_string(),
                url: "https://example.com".to_string(),
                snippet: "A test".to_string(),
                engine: "wikipedia".to_string(),
            }],
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("example.com"));
    }
}
