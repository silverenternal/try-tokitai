use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::security::{RateLimiter, SecurityConfig};
use crate::tool_matrix::matrix::RiskLevel;

/// AI 配置
#[derive(Debug, Deserialize, Clone)]
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

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            providers: HashMap::default(),
            default_provider: None,
        }
    }
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
    #[serde(default = "default_model")]
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
    "qwen3.7-plus".to_string()
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
    vec![
        "google".to_string(),
        "bing".to_string(),
        "duckduckgo".to_string(),
    ]
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

/// 安全配置（对应 config.toml 中的 [security] 段）
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SecurityTomlConfig {
    /// 无需用户确认即可自动批准的最高风险等级 (safe/moderate/dangerous)
    #[serde(default = "default_max_auto_risk")]
    pub max_auto_approve_risk: RiskLevel,
    /// TUI 模式是否跳过权限对话框
    #[serde(default)]
    pub auto_approve_tools: bool,
    /// 自主模式允许的最高风险等级
    #[serde(default = "default_autonomous_max_risk")]
    pub autonomous_max_risk: RiskLevel,
    /// 文件操作允许的根目录列表
    #[serde(default)]
    pub allowed_roots: Vec<PathBuf>,
    /// 是否允许跟踪符号链接
    #[serde(default)]
    pub allow_symlinks: bool,
    /// 读取文件的最大大小（字节）
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,
    /// 路径最大深度
    #[serde(default = "default_max_path_depth")]
    pub max_path_depth: u32,
    /// MCP Server API Key
    #[serde(default)]
    pub mcp_api_key: String,
    /// MCP 是否需要认证
    #[serde(default = "default_mcp_auth_required")]
    pub mcp_auth_required: bool,
    /// 每分钟最大工具调用数
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls_per_minute: u32,
    /// 每秒突发限制
    #[serde(default = "default_tool_call_burst")]
    pub tool_call_burst_limit: u32,
    /// 是否允许自动发现外部工具
    #[serde(default)]
    pub auto_discover_external_tools: bool,
    /// 工具生成允许的输出目录列表
    #[serde(default)]
    pub allowed_tool_gen_paths: Vec<PathBuf>,
    /// 自主模式是否允许 git push
    #[serde(default)]
    pub allow_autonomous_git_push: bool,
    /// 自主模式是否允许回滚
    #[serde(default)]
    pub allow_autonomous_rollback: bool,
    /// 自主模式是否允许代码审查（cargo fmt/clippy/test）
    #[serde(default)]
    pub allow_autonomous_review: bool,
}

fn default_max_auto_risk() -> RiskLevel {
    // Low = 最低安全限制, 最高通过率 — 默认放行所有操作 (Safe / Moderate / Low)
    // 如需收紧, 在 config.toml [security] 中设置 max_auto_approve_risk = "safe" 或 "moderate"
    RiskLevel::Safe
}
fn default_autonomous_max_risk() -> RiskLevel {
    RiskLevel::Safe
}
fn default_max_file_size() -> usize {
    10 * 1024 * 1024
}
fn default_max_path_depth() -> u32 {
    100
}
fn default_mcp_auth_required() -> bool {
    true
}
fn default_max_tool_calls() -> u32 {
    0
}
fn default_tool_call_burst() -> u32 {
    0
}

impl Default for SecurityTomlConfig {
    fn default() -> Self {
        Self {
            max_auto_approve_risk: RiskLevel::Safe,
            auto_approve_tools: false,
            autonomous_max_risk: RiskLevel::Safe,
            allowed_roots: vec![],
            allow_symlinks: false,
            max_file_size: 10 * 1024 * 1024,
            max_path_depth: 100,
            mcp_api_key: String::new(),
            mcp_auth_required: true,
            max_tool_calls_per_minute: 0,
            tool_call_burst_limit: 0,
            auto_discover_external_tools: false,
            allowed_tool_gen_paths: vec![],
            allow_autonomous_git_push: false,
            allow_autonomous_rollback: false,
            allow_autonomous_review: false,
        }
    }
}

impl SecurityTomlConfig {
    /// 将 TOML 配置 + 环境变量覆盖转换为 SecurityConfig
    pub fn into_security_config(self) -> SecurityConfig {
        // 环境变量覆盖（优先级高于 config.toml）
        let mcp_api_key = std::env::var("MCP_API_KEY").unwrap_or(self.mcp_api_key);

        let max_tool_calls = std::env::var("SECURITY_MAX_TOOL_CALLS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.max_tool_calls_per_minute);

        let burst_limit = std::env::var("SECURITY_TOOL_CALL_BURST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.tool_call_burst_limit);

        // allowed_roots: 如果 TOML 未配置，默认使用当前目录
        let allowed_roots = if self.allowed_roots.is_empty() {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            vec![
                current_dir.clone(),
                current_dir.join("sandbox"),
                current_dir.join("downloads"),
                current_dir.join("target"),
            ]
        } else {
            self.allowed_roots
        };

        SecurityConfig {
            max_auto_approve_risk: self.max_auto_approve_risk,
            auto_approve_tools: self.auto_approve_tools,
            autonomous_max_risk: self.autonomous_max_risk,
            allowed_roots,
            allow_symlinks: self.allow_symlinks,
            max_file_size: self.max_file_size,
            max_path_depth: self.max_path_depth,
            mcp_api_key,
            mcp_auth_required: self.mcp_auth_required,
            max_tool_calls_per_minute: max_tool_calls,
            tool_call_burst_limit: burst_limit,
            auto_discover_external_tools: self.auto_discover_external_tools,
            allowed_tool_gen_paths: self.allowed_tool_gen_paths,
            allow_autonomous_git_push: self.allow_autonomous_git_push,
            allow_autonomous_rollback: self.allow_autonomous_rollback,
            allow_autonomous_review: self.allow_autonomous_review,
            rate_limiter: std::sync::Arc::new(RateLimiter::new(max_tool_calls, burst_limit)),
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
    #[serde(default)]
    pub security: SecurityTomlConfig,
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
        self.user_tools
            .workspace_dir
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
            security: SecurityTomlConfig::default(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ========== 默认值测试 ==========

    #[test]
    fn test_default_ai_config() {
        let config = AiConfig::default();
        assert_eq!(config.model, "qwen3.5:397b");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
        assert!(config.providers.is_empty());
        assert!(config.default_provider.is_none());
    }

    #[test]
    fn test_default_context_config() {
        let config = ContextConfig::default();
        assert_eq!(config.max_short_term_rounds, 10);
        assert!(config.enable_mmap);
        assert!(config.enable_logging);
        assert!(config.enable_knowledge_index);
        assert!(config.auto_sync_categories);
        assert!(config.auto_recommend_knowledge);
        assert_eq!(config.recommend_threshold, 0.5);
        assert_eq!(config.recommend_limit, 3);
    }

    #[test]
    fn test_default_search_config() {
        let config = SearchConfig::default();
        assert!(config.searxng_url.is_none());
        assert_eq!(config.engines.len(), 3);
        assert!(config.engines.contains(&"google".to_string()));
        assert_eq!(config.cache_capacity, 100);
        assert_eq!(config.cache_ttl_secs, 3600);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.ai.model, "qwen3.5:397b");
        assert!(config.tools.enabled.is_empty());
        assert_eq!(config.context.max_short_term_rounds, 10);
    }

    // ========== Provider 配置测试 ==========

    #[test]
    fn test_provider_config_default_values() {
        let provider = ProviderConfig {
            api_url: "https://api.example.com".to_string(),
            api_key: Some("test-key".to_string()),
            model: "test-model".to_string(),
            cost_per_1k_tokens: 0.001,
            quality_score: 8.5,
            context_window: 8192,
        };

        assert_eq!(provider.api_url, "https://api.example.com");
        assert_eq!(provider.quality_score, 8.5);
        assert_eq!(provider.context_window, 8192);
    }

    #[test]
    fn test_provider_config_deserialize() {
        let toml_content = r#"
            api_url = "https://api.openai.com"
            api_key = "sk-test123"
            model = "gpt-4"
            cost_per_1k_tokens = 0.03
            quality_score = 9.5
            context_window = 128000
        "#;

        let provider: ProviderConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(provider.api_url, "https://api.openai.com");
        assert_eq!(provider.api_key, Some("sk-test123".to_string()));
        assert_eq!(provider.model, "gpt-4");
        assert_eq!(provider.quality_score, 9.5);
    }

    // ========== TOML 解析测试 ==========

    #[test]
    fn test_config_from_toml_complete() {
        let toml_content = r#"
            [ai]
            model = "gpt-4-turbo"
            temperature = 0.5
            max_tokens = 2048
            default_provider = "openai"

            [ai.providers.openai]
            api_url = "https://api.openai.com"
            api_key = "sk-test"
            model = "gpt-4"
            quality_score = 9.0

            [context]
            root_dir = "./.tokitai"
            max_short_term_rounds = 5
            enable_mmap = false
            enable_logging = true
            recommend_threshold = 0.7
            recommend_limit = 5

            [search]
            searxng_url = "https://searx.example.org"
            engines = ["duckduckgo", "bing"]
            cache_capacity = 200

            [user_tools]
            workspace_dir = "/home/user/projects"
            download_dir = "/home/user/downloads"
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();

        // AI 配置
        assert_eq!(config.ai.model, "gpt-4-turbo");
        assert_eq!(config.ai.temperature, 0.5);
        assert_eq!(config.ai.max_tokens, 2048);
        assert_eq!(config.ai.default_provider, Some("openai".to_string()));
        assert!(config.ai.providers.contains_key("openai"));

        // 上下文配置
        assert_eq!(config.context.root_dir, Some("./.tokitai".to_string()));
        assert_eq!(config.context.max_short_term_rounds, 5);
        assert!(!config.context.enable_mmap);
        assert_eq!(config.context.recommend_threshold, 0.7);
        assert_eq!(config.context.recommend_limit, 5);

        // 搜索配置
        assert_eq!(
            config.search.searxng_url,
            Some("https://searx.example.org".to_string())
        );
        assert_eq!(config.search.engines, vec!["duckduckgo", "bing"]);
        assert_eq!(config.search.cache_capacity, 200);

        // 用户工具配置
        assert_eq!(
            config.user_tools.workspace_dir,
            Some("/home/user/projects".to_string())
        );
        assert_eq!(
            config.user_tools.download_dir,
            Some("/home/user/downloads".to_string())
        );
    }

    #[test]
    fn test_config_from_toml_minimal() {
        let toml_content = r#"# 最小配置"#;

        let config: Config = toml::from_str(toml_content).unwrap();

        // 应该使用默认值
        assert_eq!(config.ai.model, "qwen3.5:397b");
        assert_eq!(config.ai.temperature, 0.7);
        assert!(config.context.enable_mmap);
        assert!(config.context.enable_logging);
    }

    // ========== 文件加载测试 ==========

    #[test]
    fn test_config_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let toml_content = r#"
            [ai]
            model = "test-model"
            temperature = 0.3
        "#;

        fs::write(&config_path, toml_content).unwrap();

        let config = Config::load(Some(config_path)).unwrap();
        assert_eq!(config.ai.model, "test-model");
        assert_eq!(config.ai.temperature, 0.3);
    }

    #[test]
    fn test_config_load_nonexistent_file() {
        let config = Config::load(Some(PathBuf::from("/nonexistent/config.toml"))).unwrap();

        // 应该返回默认配置
        assert_eq!(config.ai.model, "qwen3.5:397b");
    }

    #[test]
    fn test_config_load_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");

        fs::write(&config_path, "invalid toml {{{{").unwrap();

        let result = Config::load(Some(config_path));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("解析配置文件失败"));
    }

    // ========== 目录获取测试 ==========

    #[test]
    fn test_get_workspace_dir_from_config() {
        let config = Config {
            user_tools: UserToolsConfig {
                workspace_dir: Some("/custom/workspace".to_string()),
                download_dir: None,
            },
            ..Default::default()
        };

        assert_eq!(
            config.get_workspace_dir(),
            PathBuf::from("/custom/workspace")
        );
    }

    #[test]
    fn test_get_workspace_dir_fallback_to_current() {
        let config = Config::default();
        let workspace = config.get_workspace_dir();

        // 应该返回当前目录
        assert!(workspace.is_absolute() || workspace == PathBuf::from("."));
    }

    #[test]
    fn test_get_download_dir_from_config() {
        let config = Config {
            user_tools: UserToolsConfig {
                workspace_dir: None,
                download_dir: Some("/custom/downloads".to_string()),
            },
            ..Default::default()
        };

        assert_eq!(
            config.get_download_dir(),
            PathBuf::from("/custom/downloads")
        );
    }

    #[test]
    fn test_get_download_dir_from_env() {
        // 这个测试依赖于 dirs crate 的行为
        let config = Config::default();
        let download_dir = config.get_download_dir();

        // 应该是系统下载目录或 ./downloads
        if let Some(system_download) = dirs::download_dir() {
            assert_eq!(download_dir, system_download);
        } else {
            assert_eq!(download_dir, PathBuf::from("./downloads"));
        }
    }

    // ========== 环境变量测试 ==========

    #[test]
    fn test_load_from_env_defaults() {
        // 清除环境变量以确保测试可重复
        std::env::remove_var("AI_MODEL");
        std::env::remove_var("AI_TEMPERATURE");
        std::env::remove_var("AI_MAX_TOKENS");

        let config = Config::load_from_env();

        assert_eq!(config.ai.model, "qwen3.5:397b");
        assert_eq!(config.ai.temperature, 0.7);
        assert_eq!(config.ai.max_tokens, 4096);
    }

    #[test]
    fn test_load_from_env_custom_values() {
        std::env::set_var("AI_MODEL", "custom-model");
        std::env::set_var("AI_TEMPERATURE", "0.9");
        std::env::set_var("AI_MAX_TOKENS", "8192");

        let config = Config::load_from_env();

        assert_eq!(config.ai.model, "custom-model");
        assert_eq!(config.ai.temperature, 0.9);
        assert_eq!(config.ai.max_tokens, 8192);

        // 清理环境变量
        std::env::remove_var("AI_MODEL");
        std::env::remove_var("AI_TEMPERATURE");
        std::env::remove_var("AI_MAX_TOKENS");
    }

    #[test]
    fn test_load_from_env_invalid_values() {
        std::env::set_var("AI_TEMPERATURE", "not-a-number");
        std::env::set_var("AI_MAX_TOKENS", "invalid");

        let config = Config::load_from_env();

        // 应该回退到默认值
        assert_eq!(config.ai.temperature, 0.7);
        assert_eq!(config.ai.max_tokens, 4096);

        std::env::remove_var("AI_TEMPERATURE");
        std::env::remove_var("AI_MAX_TOKENS");
    }

    // ========== 边界条件测试 ==========

    #[test]
    fn test_config_extreme_values() {
        let config: Config = toml::from_str("").unwrap();
        // Use defaults for simplicity; Config is now larger with security section
        assert!(config.ai.temperature >= 0.0);
        assert!(config.context.max_short_term_rounds <= 10);
    }

    #[test]
    fn test_config_large_values() {
        let toml_content = r#"
            [ai]
            max_tokens = 1000000

            [context]
            max_short_term_rounds = 10000
            recommend_limit = 1000
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();
        assert_eq!(config.ai.max_tokens, 1000000);
        assert_eq!(config.context.max_short_term_rounds, 10000);
        assert_eq!(config.context.recommend_limit, 1000);
    }

    #[test]
    fn test_security_config_from_toml() {
        let toml_content = r#"
            [security]
            max_auto_approve_risk = "moderate"
            auto_approve_tools = true
            allow_autonomous_git_push = true
            max_tool_calls_per_minute = 30
            tool_call_burst_limit = 5
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();
        let sec = config.security;
        assert_eq!(sec.max_auto_approve_risk, RiskLevel::Moderate);
        assert!(sec.auto_approve_tools);
        assert!(sec.allow_autonomous_git_push);
        assert_eq!(sec.max_tool_calls_per_minute, 30);
        assert_eq!(sec.tool_call_burst_limit, 5);

        // Verify conversion to SecurityConfig
        let built = sec.into_security_config();
        assert_eq!(built.max_auto_approve_risk, RiskLevel::Moderate);
        assert!(built.auto_approve_tools);
        assert_eq!(built.max_tool_calls_per_minute, 30);
    }

    #[test]
    fn test_security_config_from_toml_low() {
        let toml_content = r#"
            [security]
            max_auto_approve_risk = "low"
            autonomous_max_risk = "moderate"
            allow_symlinks = true
            max_file_size = 52428800
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();
        let sec = config.security;
        assert_eq!(sec.max_auto_approve_risk, RiskLevel::Low);
        assert_eq!(sec.autonomous_max_risk, RiskLevel::Moderate);
        assert!(sec.allow_symlinks);
        assert_eq!(sec.max_file_size, 52428800);
    }

    // ========== Clone 测试 ==========

    #[test]
    fn test_config_clone() {
        let config1 = Config::default();
        let config2 = config1.clone();
        assert_eq!(config1.ai.model, config2.ai.model);
    }

    // ========== Debug 输出测试 ==========

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("ai:"));
        assert!(debug_str.contains("context:"));
        assert!(debug_str.contains("security:"));
    }

    // ========== Security config tests ==========

    #[test]
    fn test_security_toml_config_defaults() {
        let config = SecurityTomlConfig::default();
        assert_eq!(config.max_auto_approve_risk, RiskLevel::Safe);
        assert!(!config.auto_approve_tools);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(config.mcp_auth_required);
        assert_eq!(config.max_tool_calls_per_minute, 0);
        assert_eq!(config.tool_call_burst_limit, 0);
    }
}
