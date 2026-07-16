//! 系统命令执行工具
//!
//! 提供安全的 shell 命令执行功能
//!
//! ## 安全机制
//! - **白名单机制**：只允许执行白名单中的命令
//! - **参数验证**：严格验证命令参数，防止注入
//! - **输出限制**：限制输出大小，防止 DoS
//! - **确认机制**：危险操作需要显式确认
//!
//! ## 命令分类
//! - **只读命令**（白名单）：ls, cat, grep, find 等
//! - **危险命令**（黑名单）：rm, chmod, sudo 等（完全禁止）
//! - **任意命令**：需要 confirmed=true 且记录日志

use serde_json::json;
use tokitai::tool;

use super::config;
use super::error::CommandError;
use crate::text_encoding::decode_bytes;

/// 系统命令执行工具集
///
/// ## 安全说明
/// - `run_safe_command`: 只允许白名单命令
/// - `run_command`: 可执行任意命令但需要 confirmed=true
///
/// ## 示例
/// ```rust,ignore
/// let tools = SystemCommands::default();
///
/// // 安全命令（白名单）
/// let result = tools.run_safe_command("ls -la".to_string())?;
///
/// // 任意命令（需要确认）
/// let result = tools.run_command("echo hello".to_string(), true)?;
/// ```
pub struct SystemCommands;

impl Default for SystemCommands {
    fn default() -> Self {
        Self
    }
}

#[allow(deprecated)]
#[tool]
impl SystemCommands {
    /// 执行安全的 shell 命令（白名单机制）
    ///
    /// 只能执行预定义白名单中的只读命令，如 ls, cat, grep, find 等
    ///
    /// ## 参数
    /// - `command`: 完整的命令字符串
    ///
    /// ## 白名单命令
    /// 文件操作：ls, cat, head, tail, wc, file, stat, readlink, realpath, basename, dirname
    /// 搜索工具：grep, find, locate
    /// 文本处理：awk, sed, cut, sort, uniq, tr, tee
    /// 系统信息：pwd, whoami, id, uname, hostname, date, time, cal
    /// 其他：echo, printf, true, false, sleep
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "exit_code": 0,
    ///     "stdout": "...",
    ///     "stderr": "..."
    ///   }
    /// }
    /// ```
    ///
    /// ## 错误
    /// - `CommandError::Blacklisted`: 命令在黑名单中
    /// - `CommandError::NotWhitelisted`: 命令不在白名单中
    /// - `CommandError::InvalidArgument`: 命令格式无效
    /// - `CommandError::ExecutionFailed`: 执行失败
    /// - `CommandError::OutputTruncated`: 输出过大被截断
    ///
    /// ## 安全
    /// - 使用白名单而非黑名单
    /// - 禁止 shell 元字符（; | & $ ` 等）
    /// - 限制输出大小
    pub fn run_safe_command(&self, command: String) -> Result<String, String> {
        // 验证命令长度
        if command.len() > config::MAX_COMMAND_LENGTH {
            return Err(CommandError::InvalidArgument(format!(
                "命令过长 ({} > {} 字符)",
                command.len(),
                config::MAX_COMMAND_LENGTH
            ))
            .to_string());
        }

        // 使用 shlex 解析命令（安全的 shell 分词）
        let parts = safe_split_command(&command)?;

        if parts.is_empty() {
            return Err(CommandError::InvalidArgument("空命令".to_string()).to_string());
        }

        let command_name = parts[0].as_str();

        // 检查黑名单（即使伪装也要拦截）
        if is_dangerous_command(command_name) {
            return Err(CommandError::Blacklisted(command_name.to_string()).to_string());
        }

        // 检查白名单
        if !is_whitelisted_command(command_name) {
            return Err(CommandError::NotWhitelisted(command_name.to_string()).to_string());
        }

        // 执行命令（不使用 shell 解释器，直接执行）
        let output = run_whitelisted_command(&parts).map_err(|e| {
            CommandError::ExecutionFailed(format!("执行命令失败：{}", e)).to_string()
        })?;

        let mut stdout = decode_bytes(&output.stdout);
        let stderr = decode_bytes(&output.stderr);

        // 限制输出大小
        if stdout.len() > config::MAX_OUTPUT_SIZE {
            stdout.truncate(config::MAX_OUTPUT_SIZE);
            stdout.push_str("\n... [输出已截断]");
        }

        let mut result = json!({
            "success": output.status.success(),
            "data": {
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": stdout,
                "stderr": stderr,
            }
        });

        // 如果有 stderr 且命令成功，添加警告
        if !stderr.is_empty() && output.status.success() {
            result["data"]["warning"] = json!(stderr);
        }

        Ok(result.to_string())
    }

    /// 执行任意 shell 命令（需要确认）
    ///
    /// 可以执行任意 shell 命令，但必须设置 `confirmed=true`
    ///
    /// ## 参数
    /// - `command`: 完整的命令字符串
    /// - `confirmed`: 必须为 true 才能执行
    ///
    /// ## 返回
    /// JSON 格式：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "exit_code": 0,
    ///     "stdout": "...",
    ///     "stderr": "..."
    ///   }
    /// }
    /// ```
    ///
    /// ## 错误
    /// - `CommandError::ConfirmationRequired`: 需要确认（confirmed=false）
    /// - `CommandError::ExecutionFailed`: 执行失败
    /// - `CommandError::OutputTruncated`: 输出过大被截断
    ///
    /// ## 安全警告
    /// - 此函数可执行危险命令（rm, dd, mkfs 等）
    /// - 调用方应确保 confirmed 参数经过用户明确同意
    /// - 建议记录所有执行的命令到审计日志
    pub fn run_command(&self, command: String, confirmed: bool) -> Result<String, String> {
        if !confirmed {
            return Err(CommandError::ConfirmationRequired.to_string());
        }

        // 验证命令长度
        if command.len() > config::MAX_COMMAND_LENGTH {
            return Err(CommandError::InvalidArgument(format!(
                "命令过长 ({} > {} 字符)",
                command.len(),
                config::MAX_COMMAND_LENGTH
            ))
            .to_string());
        }

        // 根据操作系统选择 shell
        let output = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/c", &command])
                .output()
        } else {
            std::process::Command::new("sh")
                .args(["-c", &command])
                .output()
        }
        .map_err(|e| CommandError::ExecutionFailed(format!("执行命令失败：{}", e)).to_string())?;

        let mut stdout = decode_bytes(&output.stdout);
        let mut stderr = decode_bytes(&output.stderr);

        // 限制输出大小
        if stdout.len() > config::MAX_OUTPUT_SIZE {
            stdout.truncate(config::MAX_OUTPUT_SIZE);
            stdout.push_str("\n... [输出已截断]");
        }
        if stderr.len() > config::MAX_OUTPUT_SIZE {
            stderr.truncate(config::MAX_OUTPUT_SIZE);
            stderr.push_str("\n... [输出已截断]");
        }

        Ok(json!({
            "success": output.status.success(),
            "data": {
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": stdout,
                "stderr": stderr,
            },
            "confirmed": confirmed,
        })
        .to_string())
    }

    /// 获取当前工作目录
    ///
    /// ## 返回
    /// JSON 格式：`{"success": true, "data": {"path": "<current-working-directory>"}}`
    ///
    /// ## 错误
    /// - 获取当前目录失败（罕见）
    pub fn get_current_dir(&self) -> Result<String, String> {
        std::env::current_dir()
            .map(|p| {
                json!({
                    "success": true,
                    "data": {
                        "path": p.to_string_lossy().to_string()
                    }
                })
                .to_string()
            })
            .map_err(|e| format!("获取当前目录失败：{}", e))
    }

    /// 获取环境变量
    ///
    /// ## 参数
    /// - `key`: 环境变量名称
    ///
    /// ## 返回
    /// JSON 格式：`{"success": true, "data": {"value": "..."}}`
    ///
    /// ## 安全
    /// - 敏感变量（PASSWORD, SECRET, TOKEN 等）会被过滤
    pub fn get_env(&self, key: String) -> Result<String, String> {
        // 检查是否为敏感变量
        if is_sensitive_env_key(&key) {
            return Err("⚠️ 安全限制：不能访问敏感环境变量".to_string());
        }

        std::env::var(&key)
            .map(|v| {
                json!({
                    "success": true,
                    "data": {
                        "key": key,
                        "value": v
                    }
                })
                .to_string()
            })
            .map_err(|e| format!("获取环境变量失败：{}", e))
    }

    /// 列出所有环境变量
    ///
    /// ## 返回
    /// JSON 格式的所有非敏感环境变量列表
    ///
    /// ## 安全
    /// - 自动过滤敏感变量
    /// - 显示变量名和值
    pub fn list_env(&self) -> Result<String, String> {
        let vars: Vec<serde_json::Value> = std::env::vars()
            .filter(|(k, _)| !is_sensitive_env_key(k))
            .map(|(k, v)| json!({"key": k, "value": v}))
            .collect();

        Ok(json!({
            "success": true,
            "data": {
                "count": vars.len(),
                "variables": vars
            },
            "note": "敏感环境变量已过滤"
        })
        .to_string())
    }

    /// 以 JSON 格式执行安全命令（已废弃，使用 run_safe_command 替代）
    #[deprecated(since = "1.0.0", note = "使用 run_safe_command 替代，已返回 JSON 格式")]
    #[allow(deprecated)]
    pub fn run_safe_command_json(&self, command: String) -> Result<String, String> {
        self.run_safe_command(command)
    }

    /// 以 JSON 格式执行任意命令（需要确认）（已废弃，使用 run_command 替代）
    #[deprecated(since = "1.0.0", note = "使用 run_command 替代，已返回 JSON 格式")]
    #[allow(deprecated)]
    pub fn run_command_json(&self, command: String, confirmed: bool) -> Result<String, String> {
        self.run_command(command, confirmed)
    }
}

/// 安全的命令分词函数
///
/// 使用简单的状态机解析命令，避免 shell 注入
/// 不支持命令替换、变量展开等危险特性
fn safe_split_command(command: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in command.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => {
                if in_single_quote {
                    current.push(ch);
                } else {
                    escape_next = true;
                }
            }
            '\'' => {
                if in_double_quote {
                    current.push(ch);
                } else {
                    in_single_quote = !in_single_quote;
                }
            }
            '"' => {
                if in_single_quote {
                    current.push(ch);
                } else {
                    in_double_quote = !in_double_quote;
                }
            }
            ' ' | '\t' => {
                if in_single_quote || in_double_quote {
                    current.push(ch);
                } else if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            // 检查危险字符（在引号外）
            ';' | '|' | '&' | '$' | '`' | '<' | '>' | '(' | ')'
                if !in_single_quote && !in_double_quote =>
            {
                return Err(format!("命令包含危险的 shell 元字符：{}", ch));
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    // 检查未闭合的引号
    if in_single_quote {
        return Err("命令包含未闭合的单引号".to_string());
    }
    if in_double_quote {
        return Err("命令包含未闭合的双引号".to_string());
    }

    Ok(parts)
}

/// 检查是否是危险命令（黑名单）
fn run_whitelisted_command(parts: &[String]) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        if parts.first().is_some_and(|command| command == "ls") {
            let mut command = std::process::Command::new("cmd");
            command.args(["/D", "/C", "dir", "/A"]);
            command.args(parts.iter().skip(1).filter(|arg| !arg.starts_with('-')));
            return command.output();
        }
    }

    std::process::Command::new(&parts[0])
        .args(&parts[1..])
        .output()
}

fn is_dangerous_command(command: &str) -> bool {
    // 包含完整路径的命令也要检查
    let command_base = command.rsplit('/').next().unwrap_or(command);
    config::DANGEROUS_COMMANDS.contains(&command_base)
}

/// 检查是否是白名单命令
fn is_whitelisted_command(command: &str) -> bool {
    config::WHITELISTED_COMMANDS.contains(&command)
}

/// 检查是否为敏感环境变量
fn is_sensitive_env_key(key: &str) -> bool {
    let upper = key.to_uppercase();
    config::SENSITIVE_ENV_PATTERNS
        .iter()
        .any(|pattern| upper.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dangerous_command() {
        assert!(is_dangerous_command("rm"));
        assert!(is_dangerous_command("sudo"));
        assert!(is_dangerous_command("/usr/bin/rm"));
        assert!(is_dangerous_command("curl"));
        assert!(!is_dangerous_command("ls"));
        assert!(!is_dangerous_command("cat"));
    }

    #[test]
    fn test_is_whitelisted_command() {
        assert!(is_whitelisted_command("ls"));
        assert!(is_whitelisted_command("cat"));
        assert!(is_whitelisted_command("grep"));
        assert!(!is_whitelisted_command("rm"));
        assert!(!is_whitelisted_command("sudo"));
    }

    #[test]
    fn test_is_sensitive_env_key() {
        assert!(is_sensitive_env_key("DATABASE_PASSWORD"));
        assert!(is_sensitive_env_key("AWS_SECRET_KEY"));
        assert!(is_sensitive_env_key("API_TOKEN"));
        assert!(!is_sensitive_env_key("PATH"));
        assert!(!is_sensitive_env_key("HOME"));
        assert!(!is_sensitive_env_key("USER"));
    }

    #[test]
    fn test_safe_split_command_simple() {
        assert_eq!(safe_split_command("ls -la").unwrap(), vec!["ls", "-la"]);
        assert_eq!(
            safe_split_command("cat file.txt").unwrap(),
            vec!["cat", "file.txt"]
        );
    }

    #[test]
    fn test_safe_split_command_with_quotes() {
        assert_eq!(
            safe_split_command("echo 'hello world'").unwrap(),
            vec!["echo", "hello world"]
        );
        assert_eq!(
            safe_split_command("echo \"hello world\"").unwrap(),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn test_safe_split_command_dangerous() {
        assert!(safe_split_command("ls; rm -rf /").is_err());
        assert!(safe_split_command("cat file | grep test").is_err());
        assert!(safe_split_command("echo $(whoami)").is_err());
        assert!(safe_split_command("ls `pwd`").is_err());
    }

    #[test]
    fn test_run_safe_command_valid() {
        let tools = SystemCommands;
        let result = tools.run_safe_command("ls -la".to_string());
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["data"].is_object());
    }

    #[test]
    fn test_run_safe_command_blacklisted() {
        let tools = SystemCommands;
        let result = tools.run_safe_command("rm -rf /".to_string());
        // 黑名单命令应该返回错误
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("黑名单") || err_msg.contains("禁止"));
    }

    #[test]
    fn test_run_safe_command_not_whitelisted() {
        let tools = SystemCommands;
        let result = tools.run_safe_command("cargo build".to_string());
        // 不在白名单的命令应该返回错误
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("白名单") || err_msg.contains("黑名单") || err_msg.contains("禁止")
        );
    }

    #[test]
    fn test_run_command_requires_confirmation() {
        let tools = SystemCommands;
        let result = tools.run_command("echo hello".to_string(), false);
        // 需要确认的命令应该返回错误
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("确认") || err_msg.contains("安全"));
    }

    #[test]
    fn test_run_command_with_confirmation() {
        let tools = SystemCommands;
        let result = tools.run_command("echo hello".to_string(), true);
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["data"]["stdout"].as_str().unwrap().contains("hello"));
    }

    #[test]
    fn test_get_env_sensitive() {
        let tools = SystemCommands;
        let result = tools.get_env("DATABASE_PASSWORD".to_string());
        // 敏感环境变量应该返回错误
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("敏感") || err_msg.contains("安全"));
    }

    #[test]
    fn test_get_current_dir() {
        let tools = SystemCommands;
        let result = tools.get_current_dir();
        assert!(result.is_ok());
        let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output["success"], true);
        assert!(output["data"]["path"].is_string());
    }
}
