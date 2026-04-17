//! 搜索引擎配置和管理
//!
//! 提供统一的搜索引擎配置和实例管理

use std::sync::Arc;
use std::time::Duration;

use super::search_error::SearchError;
use super::types::{EngineHealth, SearchEngineType, SearchStats};
use crate::tools::network::NetworkResult;

// ============================================================================
// 搜索配置
// ============================================================================

/// 搜索工具配置
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// 连接超时（秒）
    pub connect_timeout_secs: u64,
    /// 最大重试次数
    #[allow(dead_code)]
    pub max_retries: u32,
    /// 缓存容量（条目数）
    pub cache_capacity: u64,
    /// 缓存 TTL（秒）
    pub cache_ttl_secs: u64,
    /// 默认搜索结果数量
    pub default_limit: usize,
    /// 最大搜索结果数量
    pub max_limit: usize,
    /// SearXNG 实例列表
    pub searxng_instances: Vec<String>,
    /// User-Agent
    pub user_agent: String,
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 是否启用健康检查
    #[allow(dead_code)]
    pub enable_health_check: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            connect_timeout_secs: 5,
            max_retries: 3,
            cache_capacity: 100,
            cache_ttl_secs: 3600, // 1 小时
            default_limit: 5,
            max_limit: 20,
            searxng_instances: vec![
                "https://searx.be".to_string(),
                "https://search.ononoki.org".to_string(),
            ],
            user_agent: "Mozilla/5.0 (compatible; Tokitai AI Assistant/1.0)".to_string(),
            enable_cache: true,
            enable_health_check: true,
        }
    }
}

#[allow(dead_code)]
impl SearchConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // 从环境变量读取 SearXNG 实例
        if let Ok(searxng_url) = std::env::var("SEARXNG_URL") {
            config.searxng_instances.insert(0, searxng_url);
        }

        // 从环境变量读取超时
        if let Ok(timeout) = std::env::var("SEARCH_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.timeout_secs = secs;
            }
        }

        // 从环境变量读取缓存配置
        if let Ok(capacity) = std::env::var("SEARCH_CACHE_CAPACITY") {
            if let Ok(cap) = capacity.parse() {
                config.cache_capacity = cap;
            }
        }

        config
    }

    /// 验证配置
    #[allow(dead_code)]
    pub fn validate(&self) -> NetworkResult<()> {
        if self.timeout_secs == 0 {
            return Err(SearchError::InvalidQuery("超时时间必须大于 0".to_string()).into());
        }
        if self.default_limit == 0 {
            return Err(SearchError::InvalidQuery("默认结果数量必须大于 0".to_string()).into());
        }
        if self.max_limit > 100 {
            return Err(SearchError::InvalidQuery("最大结果数量不能超过 100".to_string()).into());
        }
        Ok(())
    }
}

// ============================================================================
// 搜索引擎 Trait
// ============================================================================

use super::types::SearchResult;

/// 搜索引擎接口
pub trait SearchEngine: Send + Sync {
    /// 引擎名称
    fn name(&self) -> &str;

    /// 引擎类型
    #[allow(dead_code)]
    fn engine_type(&self) -> SearchEngineType;

    /// 执行搜索
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError>;

    /// 健康检查
    fn health_check(&self) -> bool;

    /// 获取健康状态
    fn get_health(&self) -> EngineHealth;

    /// 更新统计信息
    fn record_success(&self, response_time_ms: f64);
    fn record_failure(&self);
}

// ============================================================================
// 搜索引擎管理器
// ============================================================================

/// 搜索引擎管理器 - 智能路由和缓存
pub struct SearchEngineManager {
    #[allow(dead_code)]
    config: SearchConfig,
    engines: Vec<Arc<dyn SearchEngine>>,
    stats: parking_lot::RwLock<SearchStats>,
}

impl SearchEngineManager {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            config: config.clone(),
            engines: Self::create_engines(&config),
            stats: parking_lot::RwLock::new(SearchStats::default()),
        }
    }

    /// 创建所有搜索引擎实例
    fn create_engines(config: &SearchConfig) -> Vec<Arc<dyn SearchEngine>> {
        let engines: Vec<Arc<dyn SearchEngine>> = Vec::new();

        // 从环境变量读取自定义 SearXNG 实例
        if let Ok(searxng_url) = std::env::var("SEARXNG_URL") {
            // 这里会创建具体的引擎实例
            // 暂时用占位符，后续实现具体引擎
            tracing::info!("配置自定义 SearXNG 实例：{}", searxng_url);
        }

        // DuckDuckGo 作为默认引擎
        // 后续会实现具体的引擎
        tracing::debug!("初始化搜索引擎列表");

        engines
    }

    /// 获取所有可用的引擎
    #[allow(dead_code)]
    pub fn get_available_engines(&self) -> Vec<&dyn SearchEngine> {
        self.engines
            .iter()
            .filter(|e| e.health_check())
            .map(|e| e.as_ref())
            .collect()
    }

    /// 获取引擎健康状态列表
    pub fn get_health_status(&self) -> Vec<EngineHealth> {
        self.engines.iter().map(|e| e.get_health()).collect()
    }

    /// 按健康度排序引擎
    pub fn get_sorted_engines(&self) -> Vec<(Arc<dyn SearchEngine>, f32)> {
        let mut engines: Vec<_> = self
            .engines
            .iter()
            .map(|e| (e.clone(), e.get_health().health_score()))
            .collect();

        // 按健康度降序排序
        engines.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        engines
    }

    /// 记录搜索统计
    #[allow(dead_code)]
    pub fn record_search(&self, success: bool, response_time_ms: f64, cache_hit: bool) {
        let mut stats = self.stats.write();
        stats.total_searches += 1;

        if success {
            stats.successful_searches += 1;
        } else {
            stats.failed_searches += 1;
        }

        if cache_hit {
            stats.cache_hits += 1;
        }

        // 更新平均响应时间
        let total = stats.total_searches;
        stats.avg_response_time_ms =
            (stats.avg_response_time_ms * (total - 1) as f64 + response_time_ms) / total as f64;
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> SearchStats {
        self.stats.read().clone()
    }

    /// 清空统计
    #[allow(dead_code)]
    pub fn clear_stats(&self) {
        let mut stats = self.stats.write();
        *stats = SearchStats::default();
    }
}

// ============================================================================
// 缓存配置
// ============================================================================

/// 缓存配置
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 最大容量
    pub max_capacity: u64,
    /// 条目 TTL
    pub time_to_live: Duration,
    /// 是否启用预加载
    pub preload_on_miss: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 100,
            time_to_live: Duration::from_secs(3600),
            preload_on_miss: false,
        }
    }
}

#[allow(dead_code)]
impl CacheConfig {
    pub fn new(max_capacity: u64, ttl_secs: u64) -> Self {
        Self {
            max_capacity,
            time_to_live: Duration::from_secs(ttl_secs),
            ..Default::default()
        }
    }
}
