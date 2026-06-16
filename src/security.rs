//! 安全配置与授权模块
//!
//! 集中管理系统所有安全相关配置，包括：
//! - 工具调用授权检查
//! - 风险等级分类
//! - 速率限制
//! - Sandbox 路径控制
//! - MCP 认证
//! - 自主模式限制

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::tool_matrix::matrix::RiskLevel;

// ============================================================================
// SecurityConfig
// ============================================================================

/// 集中安全配置，控制所有执行路径的安全策略
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    // --- 授权 ---
    /// 无需用户确认即可自动批准的最高风险等级
    pub max_auto_approve_risk: RiskLevel,
    /// TUI 模式是否跳过权限对话框
    pub auto_approve_tools: bool,
    /// 自主模式允许的最高风险等级
    pub autonomous_max_risk: RiskLevel,

    // --- Sandbox ---
    /// 文件操作允许的根目录列表
    pub allowed_roots: Vec<PathBuf>,
    /// 是否允许跟踪符号链接
    pub allow_symlinks: bool,
    /// 读取文件的最大大小（字节）
    pub max_file_size: usize,
    /// 路径最大深度
    pub max_path_depth: u32,

    // --- MCP ---
    /// MCP Server API Key（空字符串表示未设置）
    pub mcp_api_key: String,
    /// MCP 是否需要认证
    pub mcp_auth_required: bool,

    // --- 速率限制 ---
    /// 每分钟最大工具调用数
    pub max_tool_calls_per_minute: u32,
    /// 每秒突发限制
    pub tool_call_burst_limit: u32,

    // --- 外部工具 ---
    /// 是否允许自动发现外部工具
    pub auto_discover_external_tools: bool,

    // --- 工具生成 ---
    /// 工具生成允许的输出目录列表
    pub allowed_tool_gen_paths: Vec<PathBuf>,

    // --- 自主模式 ---
    /// 自主模式是否允许 git push
    pub allow_autonomous_git_push: bool,
    /// 自主模式是否允许回滚
    pub allow_autonomous_rollback: bool,
    /// 自主模式是否允许代码审查（cargo fmt/clippy/test）
    pub allow_autonomous_review: bool,

    // --- 运行时 ---
    /// 速率限制器（共享实例）
    #[allow(clippy::type_complexity)]
    pub rate_limiter: Arc<RateLimiter>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            max_auto_approve_risk: RiskLevel::Low,
            auto_approve_tools: false,
            autonomous_max_risk: RiskLevel::Safe,

            allowed_roots: vec![
                current_dir.clone(),
                current_dir.join("sandbox"),
                current_dir.join("downloads"),
                current_dir.join("target"),
            ],
            allow_symlinks: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_path_depth: 100,

            mcp_api_key: String::new(),
            mcp_auth_required: true,

            max_tool_calls_per_minute: 60,
            tool_call_burst_limit: 10,

            auto_discover_external_tools: false,

            // 默认允许项目目录；可通过 TUI 或配置文件收紧
            allowed_tool_gen_paths: vec![],

            allow_autonomous_git_push: false,
            allow_autonomous_rollback: false,
            allow_autonomous_review: false,

            rate_limiter: Arc::new(RateLimiter::new(60, 10)),
        }
    }
}

// ============================================================================
// ExecutionMode
// ============================================================================

/// 工具调用的执行上下文
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// CLI 交互模式
    Cli,
    /// TUI 界面模式
    Tui {
        /// TUI 中是否跳过权限对话框
        auto_approve_tools: bool,
    },
    /// 自主进化模式（无用户交互）
    Autonomous,
    /// MCP Server 模式（外部 API 调用）
    Mcp,
}

// ============================================================================
// AuthDecision
// ============================================================================

/// 授权检查的结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    /// 允许执行
    Allow,
    /// 拒绝执行，附带原因
    Deny(String),
    /// 需要用户确认
    RequiresConfirmation,
}

// ============================================================================
// authorize_tool_call
// ============================================================================

/// 根据工具风险等级和执行模式，返回授权决策
///
/// 这是所有工具执行路径的统一授权入口。
/// 在 `call_tool()` 或 `execute_tools()` 之前调用。
pub fn authorize_tool_call(
    tool_name: &str,
    config: &SecurityConfig,
    mode: ExecutionMode,
) -> AuthDecision {
    let risk = default_tool_risk_map()
        .get(tool_name)
        .cloned()
        .unwrap_or(RiskLevel::Moderate); // 未分类工具默认中等风险

    match mode {
        ExecutionMode::Cli => {
            // CLI 模式没有交互确认 UI，所以超阈值的工具直接拒绝
            if risk > config.max_auto_approve_risk {
                AuthDecision::Deny(format!(
                    "Tool '{}' (risk={}) exceeds CLI auto-approve limit (max={}). \
                     Use TUI mode for interactive confirmation, or raise max_auto_approve_risk.",
                    tool_name,
                    risk_level_display(&risk),
                    risk_level_display(&config.max_auto_approve_risk)
                ))
            } else {
                AuthDecision::Allow
            }
        }
        ExecutionMode::Tui { auto_approve_tools: _ } => {
            // TUI 模式的风险判断和确认流程在 finish_stream_with_tools 中处理，
            // 此函数仅作为最后的硬性安全检查。
            if risk == RiskLevel::Low
                && risk > config.max_auto_approve_risk
            {
                AuthDecision::Deny(format!(
                    "Low-level tool '{}' blocked — exceeds max auto-approve risk ({})",
                    tool_name,
                    risk_level_display(&config.max_auto_approve_risk)
                ))
            } else {
                AuthDecision::Allow
            }
        }
        ExecutionMode::Autonomous => {
            if risk > config.autonomous_max_risk {
                AuthDecision::Deny(format!(
                    "Tool '{}' (risk={}) blocked in autonomous mode (max allowed={})",
                    tool_name,
                    risk_level_display(&risk),
                    risk_level_display(&config.autonomous_max_risk)
                ))
            } else {
                AuthDecision::Allow
            }
        }
        ExecutionMode::Mcp => {
            // MCP: 拒绝 High/Low 风险操作
            if risk == RiskLevel::Low {
                AuthDecision::Deny(format!(
                    "Low-level tool '{}' not available via MCP",
                    tool_name
                ))
            } else {
                AuthDecision::Allow
            }
        }
    }
}

// ============================================================================
// RiskLevel helpers
// ============================================================================

/// 风险等级比较：Safe < Moderate < Dangerous
impl PartialOrd for RiskLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RiskLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let rank = |r: &RiskLevel| match r {
            RiskLevel::Safe => 0,
            RiskLevel::Moderate => 1,
            RiskLevel::Low => 2,
        };
        rank(self).cmp(&rank(other))
    }
}

fn risk_level_display(r: &RiskLevel) -> &'static str {
    match r {
        RiskLevel::Safe => "safe",
        RiskLevel::Moderate => "moderate",
        RiskLevel::Low => "low",
    }
}

// ============================================================================
// default_tool_risk_map
// ============================================================================

/// 返回内置工具的默认风险等级映射
///
/// 用于覆盖 `ToolRegistry` 中硬编码的 "safe" 默认值。
/// 未在此映射中的工具默认为 Moderate（中等风险）。
pub fn default_tool_risk_map() -> &'static HashMap<String, RiskLevel> {
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<String, RiskLevel>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();

        // === Safe: 只读操作，不影响系统状态 ===
        let safe_tools = [
            "read_file", "list_dir", "search_content", "get_current_dir",
            "get_env", "list_env", "git_status", "git_log", "git_diff",
            "git_branch", "git_remote",
            "code_search", "file_search", "find_function", "count_lines",
            "analyze_code", "code_analyze",
            "search_web", "wikipedia_search",
            "json_format", "json_query", "json_validate", "json_merge",
            "data_conversion",
            "list_processes", "process_info", "system_monitor",
            "check_tcp_port",
            "read_pdf_text", "read_pdf",
            "calc", "time_now",
            "get_weather",
            "project_list_templates",
            "observability_metrics", "observability_logs",
            "dialogue_summarize", "dialogue_context",
            "prompt_list", "prompt_get",
        ];
        for name in safe_tools {
            map.insert(name.to_string(), RiskLevel::Safe);
        }

        // === Moderate: 写入/网络操作，但在 sandbox 内 ===
        let moderate_tools = [
            "write_file", "edit_file", "copy_file", "move_file",
            "mkdir", "create_dir",
            "download_file", "download_image",
            "http_get", "http_post", "http_request",
            "run_safe_command",
            "file_compress", "file_decompress",
            "create_project_template",
            "git_init", "git_clone", "git_fetch", "git_pull",
            "git_stash", "git_tag",
            "screenshot", "take_screenshot",
            "pdf_create", "pdf_merge",
            "observability_trace",
        ];
        for name in moderate_tools {
            map.insert(name.to_string(), RiskLevel::Moderate);
        }

        // === Low: 高风险操作 — 命令执行、文件删除、Git 写入等 ===
        let low_risk_tools = [
            "run_command", "shell_exec", "exec",
            "delete_file", "delete_dir", "remove_file",
            "git_push", "git_checkout", "git_commit", "git_add",
            "git_merge", "git_reset", "git_rebase",
            "ssh_exec", "ssh_connect",
            "network_scan", "port_scan",
            "kill_process", "stop_process",
        ];
        for name in low_risk_tools {
            map.insert(name.to_string(), RiskLevel::Low);
        }

        map
    })
}

// ============================================================================
// RateLimiter
// ============================================================================

/// 基于滑动窗口的速率限制器
///
/// 支持 per-tool 和 global 两种维度的限制。
/// 使用时间戳队列实现精确的滑动窗口。
/// 线程安全，可在多线程执行路径中共享。
#[derive(Debug)]
pub struct RateLimiter {
    /// tool_name -> 最近调用时间戳队列
    tool_calls: Mutex<HashMap<String, Vec<Instant>>>,
    /// 全局调用时间戳队列
    global_timestamps: Mutex<Vec<Instant>>,
    /// 每分钟最大调用数（per-tool）
    max_per_minute: u32,
    /// 每秒突发限制
    burst_per_second: u32,
    /// 窗口时长
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_per_minute: u32, burst_per_second: u32) -> Self {
        Self {
            tool_calls: Mutex::new(HashMap::new()),
            global_timestamps: Mutex::new(Vec::new()),
            max_per_minute,
            burst_per_second,
            window: Duration::from_secs(60),
        }
    }

    /// 检查某个工具调用是否允许
    ///
    /// 返回 `Ok(())` 表示允许，`Err(reason)` 表示被限流。
    pub fn check(&self, tool_name: &str) -> Result<(), String> {
        let now = Instant::now();
        let burst_window = Duration::from_secs(1);

        // Per-tool 检查
        {
            let mut calls = self.tool_calls.lock().unwrap();
            let timestamps = calls
                .entry(tool_name.to_string())
                .or_insert_with(Vec::new);

            // 清理过期的时间戳（per-minute 窗口）
            timestamps.retain(|t| now.duration_since(*t) < self.window);

            // 每分钟限制
            if timestamps.len() as u32 >= self.max_per_minute {
                return Err(format!(
                    "Rate limit exceeded for '{}': {} calls per minute (max {})",
                    tool_name,
                    timestamps.len(),
                    self.max_per_minute
                ));
            }

            // 突发检查：统计 1 秒内的时间戳数量
            let recent_burst: Vec<_> = timestamps
                .iter()
                .filter(|t| now.duration_since(**t) < burst_window)
                .collect();
            if recent_burst.len() as u32 >= self.burst_per_second {
                return Err(format!(
                    "Burst limit exceeded for '{}': {} calls/sec (max {})",
                    tool_name,
                    recent_burst.len(),
                    self.burst_per_second
                ));
            }

            timestamps.push(now);
        }

        // Global 检查
        {
            let mut global = self.global_timestamps.lock().unwrap();
            let global_limit = (self.max_per_minute * 5) as usize;

            // 清理过期
            global.retain(|t| now.duration_since(*t) < self.window);

            if global.len() >= global_limit {
                return Err(format!(
                    "Global rate limit exceeded: {} total calls per minute (max {})",
                    global.len(),
                    global_limit
                ));
            }

            global.push(now);
        }

        Ok(())
    }

    /// 仅全局检查（不按工具分桶）
    pub fn check_global(&self) -> Result<(), String> {
        let mut global = self.global_timestamps.lock().unwrap();
        let now = Instant::now();
        let global_limit = (self.max_per_minute * 5) as usize;

        global.retain(|t| now.duration_since(*t) < self.window);

        if global.len() >= global_limit {
            return Err(format!(
                "Global rate limit exceeded: {} total calls per minute (max {})",
                global.len(),
                global_limit
            ));
        }

        global.push(now);
        Ok(())
    }

    /// 重置所有统计
    pub fn reset(&self) {
        self.tool_calls.lock().unwrap().clear();
        self.global_timestamps.lock().unwrap().clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Moderate);
        assert!(RiskLevel::Moderate < RiskLevel::Low);
        assert!(RiskLevel::Safe < RiskLevel::Low);
        assert_eq!(RiskLevel::Safe, RiskLevel::Safe);
    }

    #[test]
    fn test_default_risk_map_has_common_tools() {
        let map = default_tool_risk_map();
        assert_eq!(map.get("read_file"), Some(&RiskLevel::Safe));
        assert_eq!(map.get("write_file"), Some(&RiskLevel::Moderate));
        assert_eq!(map.get("run_command"), Some(&RiskLevel::Low));
        assert_eq!(map.get("git_push"), Some(&RiskLevel::Low));
    }

    #[test]
    fn test_authorize_safe_tool_cli() {
        let config = SecurityConfig::default();
        let decision = authorize_tool_call("read_file", &config, ExecutionMode::Cli);
        assert_eq!(decision, AuthDecision::Allow);
    }

    #[test]
    fn test_authorize_low_risk_tool_cli_denied_when_safe() {
        let mut config = SecurityConfig::default();
        config.max_auto_approve_risk = RiskLevel::Safe;
        let decision = authorize_tool_call("run_command", &config, ExecutionMode::Cli);
        // CLI mode: Low tools exceeding max_auto_approve_risk are Denied (no UI for confirmation)
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_authorize_low_risk_tool_cli_allowed_with_low_default() {
        let config = SecurityConfig::default(); // max_auto_approve_risk = Low (允许所有)
        let decision = authorize_tool_call("run_command", &config, ExecutionMode::Cli);
        assert!(matches!(decision, AuthDecision::Allow));
    }

    #[test]
    fn test_authorize_low_risk_tool_autonomous_blocked() {
        let config = SecurityConfig::default(); // autonomous_max_risk = Safe
        let decision = authorize_tool_call("run_command", &config, ExecutionMode::Autonomous);
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_authorize_safe_tool_autonomous_allowed() {
        let config = SecurityConfig::default();
        let decision = authorize_tool_call("read_file", &config, ExecutionMode::Autonomous);
        assert_eq!(decision, AuthDecision::Allow);
    }

    #[test]
    fn test_authorize_low_risk_tool_mcp_blocked() {
        let config = SecurityConfig::default();
        let decision = authorize_tool_call("git_push", &config, ExecutionMode::Mcp);
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_authorize_moderate_tool_mcp_allowed() {
        let config = SecurityConfig::default();
        let decision = authorize_tool_call("write_file", &config, ExecutionMode::Mcp);
        assert_eq!(decision, AuthDecision::Allow);
    }

    #[test]
    fn test_rate_limiter_allows_normal_usage() {
        let limiter = RateLimiter::new(60, 10);
        for _ in 0..5 {
            assert!(limiter.check("read_file").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_excessive_burst() {
        let limiter = RateLimiter::new(60, 1);
        // 第一次调用通过
        limiter.check("test_tool").unwrap();
        // 第二次调用在 1 秒窗口内，burst=1，应该被拒绝
        let result = limiter.check("test_tool");
        assert!(result.is_err(), "Expected burst limit error, got Ok");
    }

    #[test]
    fn test_rate_limiter_per_tool_isolation() {
        let limiter = RateLimiter::new(60, 1);
        limiter.check("tool_a").unwrap();
        // Different tool should not be affected by tool_a's burst
        let result = limiter.check("tool_b");
        assert!(result.is_ok(), "Different tools should have independent limits");
    }

    #[test]
    fn test_rate_limiter_global_limit() {
        let limiter = RateLimiter::new(3, 10); // max 3 per tool, global = 15
        // Exceed per-tool limit
        limiter.check("test_tool").unwrap();
        limiter.check("test_tool").unwrap();
        limiter.check("test_tool").unwrap();
        let result = limiter.check("test_tool");
        assert!(result.is_err(), "Expected per-tool rate limit");
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(60, 1);
        limiter.check("test_tool").unwrap();
        // Second call should be blocked
        assert!(limiter.check("test_tool").is_err());
        limiter.reset();
        assert!(limiter.check("test_tool").is_ok());
    }

    #[test]
    fn test_unknown_tool_denied_in_cli() {
        let config = SecurityConfig::default(); // max_auto_approve_risk = Moderate
        // Unknown tools default to Moderate, which equals max_auto_approve_risk,
        // so they are Allowed in CLI (user is present and can see what happens)
        let decision = authorize_tool_call("some_new_unknown_tool", &config, ExecutionMode::Cli);
        assert!(matches!(decision, AuthDecision::Allow));
    }

    #[test]
    fn test_unknown_tool_denied_in_autonomous() {
        let config = SecurityConfig::default(); // autonomous_max_risk = Safe
        let decision = authorize_tool_call("some_new_unknown_tool", &config, ExecutionMode::Autonomous);
        // Unknown tools default to Moderate, which > Safe, so Denied in Autonomous
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_tui_auto_approve_respects_risk() {
        let mut config = SecurityConfig::default();
        config.max_auto_approve_risk = RiskLevel::Moderate;
        // Safe tool allowed
        let decision = authorize_tool_call(
            "read_file",
            &config,
            ExecutionMode::Tui { auto_approve_tools: true },
        );
        assert_eq!(decision, AuthDecision::Allow);
        // Low tool exceeding max_auto_approve_risk is Denied (hard safety check)
        let decision = authorize_tool_call(
            "run_command",
            &config,
            ExecutionMode::Tui { auto_approve_tools: true },
        );
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }
}
