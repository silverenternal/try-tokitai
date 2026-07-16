//! 安全配置与授权模块
//!
//! 统一管理工具调用授权、风险等级、速率限制、Sandbox 路径控制、
//! MCP 认证以及自主模式限制。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::tool_matrix::matrix::RiskLevel;

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub max_auto_approve_risk: RiskLevel,
    pub auto_approve_tools: bool,
    pub autonomous_max_risk: RiskLevel,
    pub allowed_roots: Vec<PathBuf>,
    pub allow_symlinks: bool,
    pub max_file_size: usize,
    pub max_path_depth: u32,
    pub mcp_api_key: String,
    pub mcp_auth_required: bool,
    pub max_tool_calls_per_minute: u32,
    pub tool_call_burst_limit: u32,
    pub auto_discover_external_tools: bool,
    pub allowed_tool_gen_paths: Vec<PathBuf>,
    pub allow_autonomous_git_push: bool,
    pub allow_autonomous_rollback: bool,
    pub allow_autonomous_review: bool,
    #[allow(clippy::type_complexity)]
    pub rate_limiter: Arc<RateLimiter>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            max_auto_approve_risk: RiskLevel::Safe,
            auto_approve_tools: false,
            autonomous_max_risk: RiskLevel::Safe,
            allowed_roots: vec![
                current_dir.clone(),
                current_dir.join("sandbox"),
                current_dir.join("downloads"),
                current_dir.join("target"),
            ],
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
            rate_limiter: Arc::new(RateLimiter::new(0, 0)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Cli,
    Tui { auto_approve_tools: bool },
    Autonomous,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    Allow,
    Deny(String),
    RequiresConfirmation,
}

pub fn authorize_tool_call(
    tool_name: &str,
    config: &SecurityConfig,
    mode: ExecutionMode,
) -> AuthDecision {
    let risk = default_tool_risk_map()
        .get(tool_name)
        .cloned()
        .unwrap_or(RiskLevel::Low);

    match mode {
        ExecutionMode::Cli => {
            if risk > config.max_auto_approve_risk {
                AuthDecision::Deny(format!(
                    "Tool '{}' (risk={}) exceeds CLI auto-approve limit (max={}). Use TUI mode for interactive confirmation, or raise max_auto_approve_risk.",
                    tool_name,
                    risk_level_display(&risk),
                    risk_level_display(&config.max_auto_approve_risk)
                ))
            } else {
                AuthDecision::Allow
            }
        }
        ExecutionMode::Tui { .. } => {
            if risk == RiskLevel::Low && risk > config.max_auto_approve_risk {
                AuthDecision::Deny(format!(
                    "Low-level tool '{}' blocked - exceeds max auto-approve risk ({})",
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

pub fn default_tool_risk_map() -> &'static HashMap<String, RiskLevel> {
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<String, RiskLevel>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();

        let safe_tools = [
            "read_file",
            "read_file_head",
            "read_file_range",
            "list_dir",
            "inspect_path",
            "grep",
            "search_content",
            "get_current_dir",
            "get_env",
            "list_env",
            "git_status",
            "git_log",
            "git_diff",
            "git_diff_file",
            "git_branch",
            "git_remote",
            "code_search",
            "file_search",
            "find_function",
            "count_lines",
            "analyze_code",
            "code_analyze",
            "search_web",
            "wikipedia_search",
            "json_format",
            "json_query",
            "json_validate",
            "json_merge",
            "data_conversion",
            "list_processes",
            "process_info",
            "system_monitor",
            "check_tcp_port",
            "read_pdf_text",
            "read_pdf",
            "calc",
            "time_now",
            "get_weather",
            "project_list_templates",
            "observability_metrics",
            "observability_logs",
            "dialogue_summarize",
            "dialogue_context",
            "prompt_list",
            "prompt_get",
            "grep",
            "find_files",
            "count_file_types",
            "find_large_files",
            "tree_dir",
            "get_file_info",
            "diagnostics",
            "symbol_search",
            "references_search",
            "fetch_url",
            "search_arxiv",
            "search_paper",
            "fetch_paper",
            "fetch_papers",
            "inspect_dataset",
            "search_public_datasets",
            "fetch_public_dataset_manifest",
            "search_github_repositories",
            "search_github_code",
            "search_github_datasets",
            "sympy_simplify",
            "sympy_solve",
            "sympy_integrate",
            "sympy_diff",
            "sympy_matrix",
            "terminal_create",
            "terminal_read",
        ];
        for name in safe_tools {
            map.insert(name.to_string(), RiskLevel::Safe);
        }

        let moderate_tools = [
            "write_file",
            "edit_file",
            "search_and_replace_multi",
            "apply_patch",
            "copy_file",
            "move_file",
            "mkdir",
            "create_dir",
            "rename_path",
            "download_file",
            "download_image",
            "generate_image",
            "http_get",
            "http_post",
            "http_request",
            "run_safe_command",
            "file_compress",
            "file_decompress",
            "create_project_template",
            "git_init",
            "git_clone",
            "git_fetch",
            "git_pull",
            "git_stash",
            "git_tag",
            "screenshot",
            "take_screenshot",
            "pdf_create",
            "pdf_merge",
            "observability_trace",
            "run_python",
            "run_python_file",
            "run_r",
            "run_julia",
            "format_file",
            "test_target",
            "terminal_run",
            "terminal_run_structured",
            "browser_computer",
        ];
        for name in moderate_tools {
            map.insert(name.to_string(), RiskLevel::Moderate);
        }

        let low_risk_tools = [
            "run_command",
            "shell_exec",
            "exec",
            "delete_file",
            "delete_dir",
            "remove_file",
            "git_push",
            "git_checkout",
            "git_commit",
            "git_add",
            "git_merge",
            "git_reset",
            "git_rebase",
            "ssh_exec",
            "ssh_connect",
            "network_scan",
            "port_scan",
            "kill_process",
            "stop_process",
        ];
        for name in low_risk_tools {
            map.insert(name.to_string(), RiskLevel::Low);
        }

        map
    })
}

#[derive(Debug)]
pub struct RateLimiter {
    tool_calls: Mutex<HashMap<String, Vec<Instant>>>,
    global_timestamps: Mutex<Vec<Instant>>,
    max_per_minute: u32,
    burst_per_second: u32,
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

    pub fn check(&self, tool_name: &str) -> Result<(), String> {
        let now = Instant::now();
        let burst_window = Duration::from_secs(1);

        {
            let mut calls = self.tool_calls.lock().unwrap();
            let timestamps = calls.entry(tool_name.to_string()).or_insert_with(Vec::new);
            timestamps.retain(|t| now.duration_since(*t) < self.window);

            if self.max_per_minute > 0 && timestamps.len() as u32 >= self.max_per_minute {
                return Err(format!(
                    "Rate limit exceeded for '{}': {} calls per minute (max {})",
                    tool_name,
                    timestamps.len(),
                    self.max_per_minute
                ));
            }

            let recent_burst: Vec<_> = if self.burst_per_second == 0 {
                Vec::new()
            } else {
                timestamps
                    .iter()
                    .filter(|t| now.duration_since(**t) < burst_window)
                    .collect()
            };
            if self.burst_per_second > 0 && recent_burst.len() as u32 >= self.burst_per_second {
                return Err(format!(
                    "Burst limit exceeded for '{}': {} calls/sec (max {})",
                    tool_name,
                    recent_burst.len(),
                    self.burst_per_second
                ));
            }

            timestamps.push(now);
        }

        {
            let mut global = self.global_timestamps.lock().unwrap();
            let global_limit = (self.max_per_minute.saturating_mul(5)) as usize;
            global.retain(|t| now.duration_since(*t) < self.window);

            if global_limit > 0 && global.len() >= global_limit {
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

    pub fn check_global(&self) -> Result<(), String> {
        let mut global = self.global_timestamps.lock().unwrap();
        let now = Instant::now();
        let global_limit = (self.max_per_minute.saturating_mul(5)) as usize;

        global.retain(|t| now.duration_since(*t) < self.window);
        if global_limit > 0 && global.len() >= global_limit {
            return Err(format!(
                "Global rate limit exceeded: {} total calls per minute (max {})",
                global.len(),
                global_limit
            ));
        }

        global.push(now);
        Ok(())
    }

    pub fn reset(&self) {
        self.tool_calls.lock().unwrap().clear();
        self.global_timestamps.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Moderate);
        assert!(RiskLevel::Moderate < RiskLevel::Low);
        assert!(RiskLevel::Safe < RiskLevel::Low);
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
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_authorize_low_risk_tool_cli_allowed_with_low_default() {
        let mut config = SecurityConfig::default();
        config.max_auto_approve_risk = RiskLevel::Low;
        let decision = authorize_tool_call("run_command", &config, ExecutionMode::Cli);
        assert!(matches!(decision, AuthDecision::Allow));
    }

    #[test]
    fn test_authorize_low_risk_tool_autonomous_blocked() {
        let config = SecurityConfig::default();
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
        limiter.check("test_tool").unwrap();
        let result = limiter.check("test_tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limiter_per_tool_isolation() {
        let limiter = RateLimiter::new(60, 1);
        limiter.check("tool_a").unwrap();
        let result = limiter.check("tool_b");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(60, 1);
        limiter.check("test_tool").unwrap();
        assert!(limiter.check("test_tool").is_err());
        limiter.reset();
        assert!(limiter.check("test_tool").is_ok());
    }

    #[test]
    fn test_unknown_tool_denied_in_cli_by_default() {
        let config = SecurityConfig::default();
        let decision = authorize_tool_call("some_new_unknown_tool", &config, ExecutionMode::Cli);
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_unknown_tool_allowed_in_cli_when_explicitly_permitted() {
        let mut config = SecurityConfig::default();
        config.max_auto_approve_risk = RiskLevel::Low;
        let decision = authorize_tool_call("some_new_unknown_tool", &config, ExecutionMode::Cli);
        assert!(matches!(decision, AuthDecision::Allow));
    }

    #[test]
    fn test_unknown_tool_denied_in_autonomous() {
        let config = SecurityConfig::default();
        let decision =
            authorize_tool_call("some_new_unknown_tool", &config, ExecutionMode::Autonomous);
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn test_tui_auto_approve_respects_risk() {
        let mut config = SecurityConfig::default();
        config.max_auto_approve_risk = RiskLevel::Moderate;
        let decision = authorize_tool_call(
            "read_file",
            &config,
            ExecutionMode::Tui {
                auto_approve_tools: true,
            },
        );
        assert_eq!(decision, AuthDecision::Allow);

        let decision = authorize_tool_call(
            "run_command",
            &config,
            ExecutionMode::Tui {
                auto_approve_tools: true,
            },
        );
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }
}
