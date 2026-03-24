//! 系统工具错误类型定义
//!
//! 提供类型安全的错误处理，支持调用方进行错误恢复

use thiserror::Error;

/// 进程相关错误
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("进程 {0} 不存在")]
    NotFound(u32),

    #[error("无权限访问进程 {0}: {1}")]
    PermissionDenied(u32, String),

    #[allow(dead_code)]
    #[error("无效的 PID: {0}")]
    InvalidPid(String),

    #[error("命令执行失败：{0}")]
    CommandFailed(String),

    #[error("解析进程信息失败：{0}")]
    ParseFailed(String),

    #[allow(dead_code)]
    #[error("不支持的操作系统：{0}")]
    UnsupportedOS(String),

    #[allow(dead_code)]
    #[error("输出过大被截断：{0}")]
    OutputTruncated(String),

    #[error("IO 错误：{0}")]
    IoError(#[from] std::io::Error),
}

/// 系统命令执行错误
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("命令 '{0}' 在黑名单中，禁止执行")]
    Blacklisted(String),

    #[error("命令 '{0}' 不在白名单中")]
    NotWhitelisted(String),

    #[error("执行失败：{0}")]
    ExecutionFailed(String),

    #[error("参数验证失败：{0}")]
    InvalidArgument(String),

    #[error("需要确认才能执行危险操作")]
    ConfirmationRequired,

    #[allow(dead_code)]
    #[error("输出过大被截断：{0}")]
    OutputTruncated(String),

    #[allow(dead_code)]
    #[error("命令解析失败：{0}")]
    ParseFailed(String),
}

/// 代码分析错误
#[derive(Debug, Error)]
pub enum CodeAnalysisError {
    #[error("读取文件失败：{0}")]
    FileReadFailed(String),

    #[error("文件不存在：{0}")]
    FileNotFound(String),

    #[allow(dead_code)]
    #[error("解析失败：{0}")]
    ParseFailed(String),

    #[allow(dead_code)]
    #[error("不支持的文件类型：{0}")]
    UnsupportedFileType(String),
}

impl From<std::io::Error> for CodeAnalysisError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            CodeAnalysisError::FileNotFound(format!("文件不存在：{}", err))
        } else {
            CodeAnalysisError::FileReadFailed(format!("IO 错误：{}", err))
        }
    }
}

/// 系统信息错误
#[derive(Debug, Error)]
pub enum SystemInfoError {
    #[error("获取系统信息失败：{0}")]
    InfoFetchFailed(String),

    #[allow(dead_code)]
    #[error("解析系统信息失败：{0}")]
    ParseFailed(String),

    #[allow(dead_code)]
    #[error("不支持的操作系统：{0}")]
    UnsupportedOS(String),
}

/// 通用工具错误转换 trait
pub trait ToErrorString {
    fn to_error_string(&self) -> String;
}

impl ToErrorString for std::io::Error {
    fn to_error_string(&self) -> String {
        format!("IO 错误：{}", self)
    }
}

impl ToErrorString for std::string::FromUtf8Error {
    fn to_error_string(&self) -> String {
        format!("UTF-8 解析错误：{}", self)
    }
}

/// 工具错误结果类型别名
#[allow(dead_code)]
pub type ToolResult<T> = Result<T, ToolError>;

/// 通用工具错误枚举
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ToolError {
    #[error("进程错误：{0}")]
    Process(#[from] ProcessError),

    #[error("命令错误：{0}")]
    Command(#[from] CommandError),

    #[error("代码分析错误：{0}")]
    CodeAnalysis(#[from] CodeAnalysisError),

    #[error("系统信息错误：{0}")]
    SystemInfo(#[from] SystemInfoError),

    #[error("工具调用失败：{0}")]
    ToolCallFailed(String),

    #[error("参数验证失败：{0}")]
    ValidationError(String),
}

impl ToolError {
    /// 转换为 JSON 格式
    #[allow(dead_code)]
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "error": true,
            "error_type": match self {
                ToolError::Process(_) => "ProcessError",
                ToolError::Command(_) => "CommandError",
                ToolError::CodeAnalysis(_) => "CodeAnalysisError",
                ToolError::SystemInfo(_) => "SystemInfoError",
                ToolError::ToolCallFailed(_) => "ToolCallError",
                ToolError::ValidationError(_) => "ValidationError",
            },
            "message": self.to_string(),
        }).to_string()
    }

    /// 创建验证错误
    #[allow(dead_code)]
    pub fn validation(msg: impl Into<String>) -> Self {
        ToolError::ValidationError(msg.into())
    }

    /// 创建工具调用失败错误
    #[allow(dead_code)]
    pub fn tool_call(msg: impl Into<String>) -> Self {
        ToolError::ToolCallFailed(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_error_display() {
        let err = ProcessError::NotFound(1234);
        assert_eq!(err.to_string(), "进程 1234 不存在");

        let err = ProcessError::PermissionDenied(1234, "测试".to_string());
        assert!(err.to_string().contains("无权限访问进程 1234"));
    }

    #[test]
    fn test_command_error_display() {
        let err = CommandError::Blacklisted("rm".to_string());
        assert_eq!(err.to_string(), "命令 'rm' 在黑名单中，禁止执行");

        let err = CommandError::NotWhitelisted("cargo".to_string());
        assert_eq!(err.to_string(), "命令 'cargo' 不在白名单中");
    }

    #[test]
    fn test_code_analysis_error_display() {
        let err = CodeAnalysisError::FileNotFound("/test/path".to_string());
        assert_eq!(err.to_string(), "文件不存在：/test/path");
    }

    #[test]
    fn test_tool_error_from_variants() {
        let process_err = ProcessError::NotFound(1234);
        let tool_err: ToolError = process_err.into();
        assert!(matches!(tool_err, ToolError::Process(_)));

        let command_err = CommandError::Blacklisted("rm".to_string());
        let tool_err: ToolError = command_err.into();
        assert!(matches!(tool_err, ToolError::Command(_)));
    }

    #[test]
    fn test_tool_error_to_json() {
        let err = ToolError::validation("测试验证错误");
        let json = err.to_json();
        assert!(json.contains("\"error\":true"));
        assert!(json.contains("ValidationError"));
        assert!(json.contains("测试验证错误"));
    }

    #[test]
    fn test_tool_error_constructors() {
        let err = ToolError::validation("参数错误");
        assert!(matches!(err, ToolError::ValidationError(_)));

        let err = ToolError::tool_call("调用失败");
        assert!(matches!(err, ToolError::ToolCallFailed(_)));
    }
}
