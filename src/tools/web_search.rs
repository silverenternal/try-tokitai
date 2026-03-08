use tokitai::tool;

/// 网络搜索工具集
pub struct WebSearchTools;

#[tool]
impl WebSearchTools {
    /// 使用搜索引擎搜索内容
    pub fn search_web(&self, query: String) -> Result<String, String> {
        // 使用 DuckDuckGo HTML 搜索（无需 API key）
        let encoded_query = urlencoding::encode(&query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);
        
        // 简单的 HTTP 请求（实际项目中应该使用异步 reqwest）
        let response = ureq::get(&url)
            .call()
            .map_err(|e| format!("搜索请求失败：{}", e))?;
        
        let body = response.into_string()
            .map_err(|e| format!("读取响应失败：{}", e))?;
        
        // 解析搜索结果（简化版本）
        let results = parse_search_results(&body, 5);
        Ok(results)
    }

    /// 获取网页内容
    pub fn fetch_url(&self, url: String) -> Result<String, String> {
        let response = ureq::get(&url)
            .call()
            .map_err(|e| format!("获取网页失败：{}", e))?;
        
        let body = response.into_string()
            .map_err(|e| format!("读取网页失败：{}", e))?;
        
        // 简单的 HTML 清理
        let text = strip_html_tags(&body);
        Ok(text.chars().take(5000).collect())
    }
}

fn parse_search_results(html: &str, limit: usize) -> String {
    let mut results = Vec::new();
    let mut count = 0;
    
    for line in html.lines() {
        if line.contains("<a ") && count < limit {
            if let Some(title) = extract_title(line) {
                if let Some(url) = extract_url(line) {
                    results.push(format!("[{}] {}", title, url));
                    count += 1;
                }
            }
        }
    }
    
    if results.is_empty() {
        "未找到搜索结果".to_string()
    } else {
        results.join("\n")
    }
}

fn extract_title(line: &str) -> Option<String> {
    // 简化提取逻辑
    Some(line.chars().take(100).collect())
}

fn extract_url(_line: &str) -> Option<String> {
    Some("https://example.com".to_string())
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    
    result
}

// 添加 ureq 依赖用于同步 HTTP 请求
// 在 Cargo.toml 中需要添加：ureq = "2.9"
