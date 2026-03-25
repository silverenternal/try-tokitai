use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::collections::HashMap;

/// AI 配置
#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct AiConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Multi-provider configuration (optional)
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// Default provider name
    #[serde(default)]
    pub default_provider: Option<String>,
}

/// Provider configuration
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ProviderConfig {
    /// API URL
    pub api_url: String,
    /// API Key
    pub api_key: Option<String>,
    /// Default model for this provider
    #[serde(default)]
    pub model: String,
    /// Cost per 1K tokens (USD)
    #[serde(default)]
    pub cost_per_1k_tokens: f64,
    /// Quality score (0-10)
    #[serde(default = "default_quality")]
    pub quality_score: f64,
    /// Context window size
    #[serde(default = "default_context_window")]
    pub context_window: usize,
}

fn default_quality() -> f64 {
    5.0
}

fn default_context_window() -> usize {
    4096
}

fn default_model() -> String {
    "qwen3.5:397b".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

/// 工具配置
#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct ToolsConfig {
    #[serde(default)]
    pub enabled: Vec<String>,
}

/// 搜索配置
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SearchConfig {
    #[serde(default)]
    pub searxng_url: Option<String>,
    #[serde(default = "default_engines")]
    pub engines: Vec<String>,
    #[serde(default = "default_cache_capacity")]
    pub cache_capacity: u64,
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
}

fn default_engines() -> Vec<String> {
    vec!["google".to_string(), "bing".to_string(), "duckduckgo".to_string()]
}

fn default_cache_capacity() -> u64 {
    100
}

fn default_cache_ttl_secs() -> u64 {
    3600
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            searxng_url: None,
            engines: default_engines(),
            cache_capacity: default_cache_capacity(),
            cache_ttl_secs: default_cache_ttl_secs(),
        }
    }
}

/// 下载配置
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
#[derive(Default)]
pub struct DownloadConfig {
    /// 默认下载目录
    pub default_dir: Option<String>,
}


/// 用户工具配置（工作目录、下载目录等）
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
#[derive(Default)]
pub struct UserToolsConfig {
    /// 默认工作目录（文件操作、项目模板等）
    pub workspace_dir: Option<String>,
    /// 默认下载目录
    pub download_dir: Option<String>,
}


/// 上下文存储配置
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ContextConfig {
    /// 上下文存储根目录
    pub root_dir: Option<String>,
    /// 短期层最大保留轮数
    #[serde(default = "default_max_short_term_rounds")]
    pub max_short_term_rounds: usize,
    /// 是否启用 mmap
    #[serde(default = "default_true")]
    pub enable_mmap: bool,
    /// 是否启用日志
    #[serde(default = "default_true")]
    pub enable_logging: bool,
    /// 是否启用知识索引
    #[serde(default = "default_true")]
    pub enable_knowledge_index: bool,
    /// 知识库根目录
    pub knowledge_root: Option<String>,
    /// 是否从目录结构自动同步分类
    #[serde(default = "default_true")]
    pub auto_sync_categories: bool,
    /// 是否自动推荐知识
    #[serde(default = "default_true")]
    pub auto_recommend_knowledge: bool,
    /// 推荐阈值
    #[serde(default = "default_recommend_threshold")]
    pub recommend_threshold: f32,
    /// 推荐数量限制
    #[serde(default = "default_recommend_limit")]
    pub recommend_limit: usize,
}

fn default_max_short_term_rounds() -> usize {
    10
}

fn default_recommend_threshold() -> f32 {
    0.5
}

fn default_recommend_limit() -> usize {
    3
}

fn default_true() -> bool {
    true
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            root_dir: None,
            max_short_term_rounds: default_max_short_term_rounds(),
            enable_mmap: true,
            enable_logging: true,
            enable_knowledge_index: true,
            knowledge_root: Some("./docs".to_string()),
            auto_sync_categories: true,
            auto_recommend_knowledge: true,
            recommend_threshold: 0.5,
            recommend_limit: 3,
        }
    }
}

/// 主配置结构
#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct Config {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub download: DownloadConfig,
    #[serde(default)]
    pub user_tools: UserToolsConfig,
    #[serde(default)]
    pub context: ContextConfig,
}

impl Config {
    /// 从配置文件加载
    #[allow(dead_code)]
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = path.unwrap_or_else(|| PathBuf::from("config.toml"));

        if !config_path.exists() {
            tracing::warn!("配置文件不存在：{:?}，使用默认配置", config_path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("读取配置文件失败：{:?}", config_path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败：{:?}", config_path))?;

        tracing::info!("配置文件加载成功：{:?}", config_path);
        Ok(config)
    }

    /// 获取默认工作目录
    /// 优先级：配置 > 当前目录
    #[allow(dead_code)]
    pub fn get_workspace_dir(&self) -> PathBuf {
        self.user_tools.workspace_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// 获取默认下载目录
    /// 优先级：配置 > 系统下载目录 > ./downloads
    #[allow(dead_code)]
    pub fn get_download_dir(&self) -> PathBuf {
        // 1. 使用配置的下载目录
        if let Some(dir) = self.user_tools.download_dir.as_ref() {
            return PathBuf::from(dir);
        }

        // 2. 使用系统下载目录
        if let Some(dir) = dirs::download_dir() {
            return dir;
        }

        // 3. 回退到 ./downloads
        PathBuf::from("./downloads")
    }

    /// 从环境变量加载 AI 配置
    #[allow(dead_code)]
    pub fn load_from_env() -> Self {
        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "qwen3.5:397b".to_string());
        let temperature = std::env::var("AI_TEMPERATURE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.7);
        let max_tokens = std::env::var("AI_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4096);

        Config {
            ai: AiConfig {
                model,
                temperature,
                max_tokens,
                providers: HashMap::new(),
                default_provider: None,
            },
            tools: ToolsConfig::default(),
            search: SearchConfig::default(),
            download: DownloadConfig::default(),
            user_tools: UserToolsConfig::default(),
            context: ContextConfig::default(),
        }
    }
}
