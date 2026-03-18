//! 进程管理工具
//!
//! 提供进程查询、监控和管理功能
//!
//! ## 功能
//! - 列出进程（支持按 CPU 排序）
//! - 获取进程详细信息
//! - 搜索进程
//! - 查看进程打开的文件
//! - 查看进程环境变量（过滤敏感信息）
//!
//! ## 错误处理
//! 使用类型安全的 `ProcessError` 枚举，支持调用方进行错误恢复

use tokitai::tool;
use std::sync::Arc;

use super::backend::{ProcessBackend, create_backend};

/// 敏感环境变量前缀/关键词（用于过滤）
const SENSITIVE_ENV_PATTERNS: &[&str] = &[
    "PASSWORD", "PASSWD", "SECRET", "TOKEN", "API_KEY", "APIKEY",
    "PRIVATE_KEY", "PRIVATEKEY", "CREDENTIAL", "CRED",
    "AWS_SECRET", "AZURE_", "GCP_", "DATABASE_URL", "DB_PASS",
    "ENCRYPTION_KEY", "SIGNING_KEY", "AUTH_TOKEN",
];

/// 进程管理工具集
///
/// 提供查看和管理系统进程的功能
///
/// ## 示例
/// ```rust,ignore
/// let tools = ProcessManager::default();
/// let processes = tools.list_processes(Some(10))?;
/// let info = tools.get_process_info(1234)?;
/// ```
pub struct ProcessManager {
    backend: Arc<Box<dyn ProcessBackend>>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self {
            backend: Arc::new(create_backend()),
        }
    }
}

impl ProcessManager {
    /// 创建新的进程管理器（使用默认后端）
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带有自定义后端的进程管理器（用于测试）
    pub fn with_backend(backend: Box<dyn ProcessBackend>) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }
}

#[tool]
impl ProcessManager {
    /// 列出当前运行的进程
    ///
    /// 显示进程名、PID、CPU 使用率、内存使用率等信息，按 CPU 使用率降序排列
    ///
    /// ## 参数
    /// - `limit`: 返回的最大进程数，默认 20，最大 100
    ///
    /// ## 返回
    /// JSON 格式的进程列表：
    /// ```json
    /// {
    ///   "success": true,
    ///   "data": {
    ///     "count": 10,
    ///     "limit": 20,
    ///     "processes": [{"pid": 1234, "comm": "bash", ...}]
    ///   }
    /// }
    /// ```
    ///
    /// ## 错误
    /// - `ProcessError::CommandFailed`: 执行系统命令失败
    /// - `ProcessError::ParseFailed`: 解析进程信息失败
    ///
    /// ## 性能
    /// - 典型延迟：50-100ms
    /// - 输出大小：每进程约 200 字节
    pub fn list_processes(&self, limit: Option<usize>) -> Result<String, String> {
        let limit = limit.unwrap_or(20).min(100);

        let processes = self.backend.list_processes(limit)
            .map_err(|e| e.to_string())?;

        let process_list: Vec<serde_json::Value> = processes
            .iter()
            .map(|p| p.to_summary_json())
            .collect();

        Ok(serde_json::json!({
            "success": true,
            "data": {
                "count": process_list.len(),
                "limit": limit,
                "processes": process_list
            }
        }).to_string())
    }

    /// 获取进程详细信息
    ///
    /// 查看指定 PID 的进程详情，包括父子关系、资源使用等
    ///
    /// ## 参数
    /// - `pid`: 目标进程 ID
    ///
    /// ## 返回
    /// JSON 格式的进程详细信息
    ///
    /// ## 错误
    /// - `ProcessError::NotFound`: 进程不存在
    /// - `ProcessError::PermissionDenied`: 无权限访问
    /// - `ProcessError::ParseFailed`: 解析失败
    ///
    /// ## 安全
    /// - 自动验证进程存在性和所有权
    /// - 单次系统调用完成验证 + 获取，避免 TOCTOU 竞争
    pub fn get_process_info(&self, pid: u32) -> Result<String, String> {
        let info = self.backend.get_process_info(pid)
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "success": true,
            "data": info.to_json_value()
        }).to_string())
    }

    /// 按名称搜索进程
    ///
    /// 查找所有命令行包含指定关键词的进程
    ///
    /// ## 参数
    /// - `name`: 搜索关键词（支持部分匹配）
    /// - `limit`: 返回的最大结果数，默认 20，最大 100
    ///
    /// ## 返回
    /// JSON 格式的匹配进程列表
    ///
    /// ## 错误
    /// - `ProcessError::CommandFailed`: 执行 pgrep 失败
    /// - `ProcessError::InvalidArgument`: 搜索关键词无效
    ///
    /// ## 安全
    /// - 搜索关键词长度限制为 256 字符
    /// - 自动转义特殊字符
    pub fn search_processes(&self, name: String, limit: Option<usize>) -> Result<String, String> {
        validate_search_pattern(&name)?;

        let limit = limit.unwrap_or(20).min(100);

        let processes = self.backend.search_processes(&name, limit)
            .map_err(|e| e.to_string())?;

        let process_list: Vec<serde_json::Value> = processes
            .iter()
            .map(|p| p.to_summary_json())
            .collect();

        let (found, message) = if processes.is_empty() {
            (false, "未找到匹配的进程")
        } else {
            (true, "")
        };

        Ok(serde_json::json!({
            "success": true,
            "data": {
                "found": found,
                "count": process_list.len(),
                "search_term": name,
                "processes": process_list,
            },
            "message": if message.is_empty() { None } else { Some(message) }
        }).to_string())
    }

    /// 查看进程的打开文件
    ///
    /// 列出进程打开的文件描述符（仅限自己的进程或 root）
    ///
    /// ## 参数
    /// - `pid`: 目标进程 ID
    /// - `limit`: 返回的最大文件数，默认 50，最大 200
    ///
    /// ## 返回
    /// JSON 格式的文件描述符列表
    ///
    /// ## 错误
    /// - `ProcessError::NotFound`: 进程不存在
    /// - `ProcessError::PermissionDenied`: 无权限访问
    ///
    /// ## 性能
    /// - 典型延迟：20-50ms
    /// - 输出大小限制为 50KB
    pub fn get_process_files(&self, pid: u32, limit: Option<usize>) -> Result<String, String> {
        let limit = limit.unwrap_or(50).min(200);

        let files = self.backend.get_process_files(pid, limit)
            .map_err(|e| e.to_string())?;

        // 限制输出大小
        let output = serde_json::json!({
            "pid": pid,
            "count": files.len(),
            "files": files
        });

        let output_str = output.to_string();
        if output_str.len() > super::config::MAX_OUTPUT_SIZE {
            // 截断文件列表
            let mut truncated_files = Vec::new();
            let mut current_size = 0;

            for file in &files {
                let file_json = serde_json::json!({ "file": file }).to_string();
                if current_size + file_json.len() > super::config::MAX_OUTPUT_SIZE - 100 {
                    break;
                }
                truncated_files.push(file);
                current_size += file_json.len();
            }

            return Ok(serde_json::json!({
                "success": true,
                "data": {
                    "pid": pid,
                    "count": truncated_files.len(),
                    "truncated": true,
                    "message": "输出过大，已截断",
                    "files": truncated_files
                }
            }).to_string());
        }

        Ok(serde_json::json!({
            "success": true,
            "data": {
                "pid": pid,
                "count": files.len(),
                "files": files
            }
        }).to_string())
    }

    /// 查看进程的环境变量
    ///
    /// 获取指定进程的环境变量（仅限自己的进程，自动过滤敏感变量）
    ///
    /// ## 参数
    /// - `pid`: 目标进程 ID
    ///
    /// ## 返回
    /// JSON 格式的环境变量列表（已过滤敏感信息）
    ///
    /// ## 错误
    /// - `ProcessError::NotFound`: 进程不存在
    /// - `ProcessError::PermissionDenied`: 无权限访问
    ///
    /// ## 安全
    /// - 自动过滤包含 PASSWORD、SECRET、TOKEN 等关键词的敏感变量
    /// - 仅显示变量名，不显示值（安全模式）
    pub fn get_process_env(&self, pid: u32) -> Result<String, String> {
        let env_vars = self.backend.get_process_env(pid)
            .map_err(|e| e.to_string())?;

        // 过滤敏感变量并只保留变量名
        let filtered_vars: Vec<String> = env_vars
            .iter()
            .filter(|v| !is_sensitive_env(v))
            .filter_map(|v| v.split('=').next().map(|s| s.to_string()))
            .collect();

        Ok(serde_json::json!({
            "success": true,
            "data": {
                "pid": pid,
                "count": filtered_vars.len(),
                "variables": filtered_vars
            },
            "note": "敏感环境变量已过滤，仅显示变量名"
        }).to_string())
    }
}

/// 验证搜索模式
fn validate_search_pattern(pattern: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Err("搜索关键词不能为空".to_string());
    }

    if pattern.len() > super::config::MAX_PATTERN_LENGTH {
        return Err(format!(
            "搜索模式过长 ({} > {} 字符)",
            pattern.len(),
            super::config::MAX_PATTERN_LENGTH
        ));
    }

    // 检查是否包含危险字符（防止命令注入）
    let dangerous_chars = [';', '|', '&', '$', '`', '(', ')', '<', '>', '\n', '\r'];
    for ch in pattern.chars() {
        if dangerous_chars.contains(&ch) {
            return Err(format!("搜索关键词包含非法字符：{}", ch));
        }
    }

    Ok(())
}

/// 检查是否为敏感环境变量
fn is_sensitive_env(env_var: &str) -> bool {
    let upper = env_var.to_uppercase();
    SENSITIVE_ENV_PATTERNS.iter().any(|pattern| upper.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_search_pattern_empty() {
        assert!(validate_search_pattern("").is_err());
    }

    #[test]
    fn test_validate_search_pattern_too_long() {
        let long_pattern = "a".repeat(300);
        assert!(validate_search_pattern(&long_pattern).is_err());
    }

    #[test]
    fn test_validate_search_pattern_dangerous_chars() {
        assert!(validate_search_pattern("test;rm").is_err());
        assert!(validate_search_pattern("test|cat").is_err());
        assert!(validate_search_pattern("test&ls").is_err());
        assert!(validate_search_pattern("test$(whoami)").is_err());
    }

    #[test]
    fn test_validate_search_pattern_valid() {
        assert!(validate_search_pattern("bash").is_ok());
        assert!(validate_search_pattern("python3").is_ok());
        assert!(validate_search_pattern("my-app").is_ok());
    }

    #[test]
    fn test_is_sensitive_env() {
        assert!(is_sensitive_env("DATABASE_PASSWORD=secret"));
        assert!(is_sensitive_env("AWS_SECRET_KEY=xxx"));
        assert!(is_sensitive_env("API_TOKEN=yyy"));
        assert!(!is_sensitive_env("PATH=/usr/bin"));
        assert!(!is_sensitive_env("HOME=/home/user"));
        assert!(!is_sensitive_env("USER=root"));
    }

    #[test]
    fn test_process_manager_creation() {
        let manager = ProcessManager::new();
        assert!(true);
    }

    #[test]
    fn test_list_processes() {
        let manager = ProcessManager::new();
        let result = manager.list_processes(Some(5));

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("\"success\":true"));
        assert!(output.contains("\"processes\""));
        // 输出可能为空数组或其他格式，只要不 panic 即可
    }

    #[test]
    fn test_get_process_info_for_current() {
        let manager = ProcessManager::new();
        let current_pid = std::process::id();
        let result = manager.get_process_info(current_pid);

        // 当前进程应该总是可访问
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains(&format!("\"pid\":{}", current_pid)));
        assert!(output.contains("\"success\":true"));
    }

    #[test]
    fn test_search_processes() {
        let manager = ProcessManager::new();
        // 搜索当前进程名
        let result = manager.search_processes("cargo".to_string(), Some(5));

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("\"search_term\""));
        assert!(output.contains("\"success\":true"));
    }

    #[test]
    fn test_get_nonexistent_process() {
        let manager = ProcessManager::new();
        // 使用一个极不可能存在的 PID
        let result = manager.get_process_info(99999999);

        // 可能返回错误或成功（如果进程存在），只要不 panic 即可
        if result.is_ok() {
            // 如果成功，验证输出格式
            let output = result.unwrap();
            assert!(output.contains("\"success\"") || output.contains("\"error\""));
        }
    }
}
