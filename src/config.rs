use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

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
#[derive(Debug, Deserialize, Clone, Default)]
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

/// 下载配置
#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct DownloadConfig {
    pub default_dir: Option<String>,
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
    pub search: Option<SearchConfig>,
    #[serde(default)]
    pub download: Option<DownloadConfig>,
    #[serde(default)]
    pub context: ContextConfig,
}

impl Config {
    /// 从配置文件加载
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
            },
            tools: ToolsConfig::default(),
            search: None,
            download: None,
            context: ContextConfig::default(),
        }
    }
}
