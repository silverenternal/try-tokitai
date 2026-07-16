//! 搜索模块公共数据结构
//!
//! 定义搜索相关的通用数据模型

use serde::{Deserialize, Serialize};

// ============================================================================
// 通用搜索结果结构
// ============================================================================

/// 通用搜索结果
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchResult {
    /// 结果标题
    pub title: String,
    /// 结果 URL
    pub url: String,
    /// 摘要/片段
    pub snippet: String,
    /// 来源引擎
    #[serde(default)]
    pub engine: String,
}

#[allow(dead_code)]
impl SearchResult {
    pub fn new(title: String, url: String, snippet: String, engine: String) -> Self {
        Self {
            title,
            url,
            snippet,
            engine,
        }
    }

    /// 创建不带引擎信息的结果
    pub fn simple(title: String, url: String, snippet: String) -> Self {
        Self {
            title,
            url,
            snippet,
            engine: String::new(),
        }
    }
}

/// 搜索响应包装器
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    /// 原始查询
    pub query: String,
    /// 结果总数
    pub total: usize,
    /// 搜索结果列表
    pub results: Vec<SearchResult>,
}

impl SearchResponse {
    pub fn new(query: String, results: Vec<SearchResult>) -> Self {
        let total = results.len();
        Self {
            query,
            total,
            results,
        }
    }

    #[allow(dead_code)]
    pub fn empty(query: String) -> Self {
        Self {
            query,
            total: 0,
            results: Vec::new(),
        }
    }

    pub fn with_message(query: String, message: &str) -> Self {
        Self {
            query,
            total: 0,
            results: vec![SearchResult {
                title: "无结果".to_string(),
                url: String::new(),
                snippet: message.to_string(),
                engine: "system".to_string(),
            }],
        }
    }
}

// ============================================================================
// 图片搜索结果结构
// ============================================================================

/// 图片搜索结果
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ImageSearchResult {
    /// 图片标题
    pub title: String,
    /// 图片来源页面 URL
    pub url: String,
    /// 图片直接 URL
    pub img_src: String,
    /// 缩略图 URL
    pub thumbnail: Option<String>,
    /// 来源网站
    pub source: String,
    /// 搜索引擎
    pub engine: String,
}

#[allow(dead_code)]
impl ImageSearchResult {
    pub fn new(
        title: String,
        url: String,
        img_src: String,
        source: String,
        engine: String,
    ) -> Self {
        Self {
            title,
            url,
            img_src,
            thumbnail: None,
            source,
            engine,
        }
    }
}

/// 图片搜索响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageSearchResponse {
    /// 原始查询
    pub query: String,
    /// 结果总数
    pub total: usize,
    /// 图片结果列表
    pub results: Vec<ImageSearchResult>,
}

impl ImageSearchResponse {
    pub fn new(query: String, results: Vec<ImageSearchResult>) -> Self {
        let total = results.len();
        Self {
            query,
            total,
            results,
        }
    }
}

// ============================================================================
// 搜索配置和请求
// ============================================================================

/// 搜索请求
#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    /// 搜索关键词
    #[allow(dead_code)]
    pub query: String,
    /// 返回结果数量
    #[allow(dead_code)]
    pub limit: usize,
    /// 搜索引擎（可选）
    pub engine: Option<String>,
    /// 时间范围（可选）
    pub time_range: Option<TimeRange>,
    /// 语言（可选）
    #[allow(dead_code)]
    pub language: Option<String>,
}

#[allow(dead_code)]
impl SearchRequest {
    pub fn new(query: String, limit: usize) -> Self {
        Self {
            query,
            limit,
            ..Default::default()
        }
    }

    pub fn with_engine(mut self, engine: String) -> Self {
        self.engine = Some(engine);
        self
    }

    pub fn with_time_range(mut self, time_range: TimeRange) -> Self {
        self.time_range = Some(time_range);
        self
    }
}

/// 时间范围
#[derive(Debug, Clone, Copy, Default)]
pub enum TimeRange {
    #[default]
    Any,
    #[allow(dead_code)]
    PastHour,
    #[allow(dead_code)]
    PastDay,
    #[allow(dead_code)]
    PastWeek,
    #[allow(dead_code)]
    PastMonth,
    #[allow(dead_code)]
    PastYear,
}

#[allow(dead_code)]
impl TimeRange {
    #[allow(clippy::trivially_copy_pass_by_ref, clippy::wrong_self_convention)]
    pub fn to_param(&self) -> &'static str {
        match self {
            TimeRange::Any => "",
            TimeRange::PastHour => "qdr:h",
            TimeRange::PastDay => "qdr:d",
            TimeRange::PastWeek => "qdr:w",
            TimeRange::PastMonth => "qdr:m",
            TimeRange::PastYear => "qdr:y",
        }
    }
}

// ============================================================================
// 搜索引擎配置
// ============================================================================

/// 搜索引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchEngineType {
    #[allow(dead_code)]
    DuckDuckGo,
    #[allow(dead_code)]
    Searxng,
    #[allow(dead_code)]
    Wikipedia,
    #[allow(dead_code)]
    WikipediaEn,
    #[allow(dead_code)]
    Arxiv,
    #[allow(dead_code)]
    Google,
    #[allow(dead_code)]
    Bing,
}

impl std::fmt::Display for SearchEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchEngineType::DuckDuckGo => write!(f, "DuckDuckGo"),
            SearchEngineType::Searxng => write!(f, "SearXNG"),
            SearchEngineType::Wikipedia => write!(f, "Wikipedia"),
            SearchEngineType::WikipediaEn => write!(f, "Wikipedia EN"),
            SearchEngineType::Arxiv => write!(f, "arXiv"),
            SearchEngineType::Google => write!(f, "Google"),
            SearchEngineType::Bing => write!(f, "Bing"),
        }
    }
}

#[allow(dead_code)]
impl SearchEngineType {
    /// 获取引擎的默认 API URL
    pub fn default_url(&self) -> Option<&'static str> {
        match self {
            SearchEngineType::DuckDuckGo => Some("https://html.duckduckgo.com/html/"),
            SearchEngineType::Wikipedia => Some("https://zh.wikipedia.org/w/api.php"),
            SearchEngineType::WikipediaEn => Some("https://en.wikipedia.org/w/api.php"),
            SearchEngineType::Arxiv => Some("https://export.arxiv.org/api/query"),
            _ => None,
        }
    }
}

// ============================================================================
// 搜索引擎健康状态
// ============================================================================

/// 引擎健康状态
#[derive(Debug, Clone)]
pub struct EngineHealth {
    /// 引擎名称
    pub name: String,
    /// 是否健康
    pub is_healthy: bool,
    /// 成功次数
    pub success_count: u32,
    /// 失败次数
    pub fail_count: u32,
    /// 成功率
    pub success_rate: f32,
    /// 平均响应时间 (ms)
    pub avg_response_time_ms: f64,
    /// 最后检查时间
    #[allow(dead_code)]
    pub last_check: std::time::Instant,
}

#[allow(dead_code)]
impl EngineHealth {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_healthy: true,
            success_count: 0,
            fail_count: 0,
            success_rate: 1.0,
            avg_response_time_ms: 0.0,
            last_check: std::time::Instant::now(),
        }
    }

    /// 计算健康分数 (0.0 - 1.0)
    pub fn health_score(&self) -> f32 {
        if !self.is_healthy {
            return 0.0;
        }
        let total = self.success_count + self.fail_count;
        if total == 0 {
            return 1.0;
        }
        self.success_count as f32 / total as f32
    }
}

// ============================================================================
// 搜索统计
// ============================================================================

/// 搜索统计信息
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// 总搜索次数
    pub total_searches: u64,
    /// 成功次数
    pub successful_searches: u64,
    /// 失败次数
    pub failed_searches: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 平均响应时间 (ms)
    pub avg_response_time_ms: f64,
}

impl SearchStats {
    pub fn success_rate(&self) -> f32 {
        let total = self.successful_searches + self.failed_searches;
        if total == 0 {
            1.0
        } else {
            self.successful_searches as f32 / total as f32
        }
    }

    pub fn cache_hit_rate(&self) -> f32 {
        if self.total_searches == 0 {
            0.0
        } else {
            self.cache_hits as f32 / self.total_searches as f32
        }
    }
}
