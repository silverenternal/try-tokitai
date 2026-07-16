//! 统一的 IO 工具错误处理模块
//!
//! 使用 thiserror 定义结构化错误类型，支持：
//! - 错误分类（路径验证、IO、解析等）
//! - 错误上下文（路径、操作类型等）
//! - AI 友好的错误消息和建议

use serde_json::{json, Value};
use thiserror::Error;

/// IO 工具统一错误类型
#[derive(Error, Debug, Clone)]
pub enum IoToolError {
    /// 路径验证失败
    #[error("路径验证失败：{message}")]
    PathValidation {
        message: String,
        path: String,
        suggestion: String,
    },

    /// 文件不存在
    #[error("文件不存在：{path}")]
    FileNotFound { path: String, suggestion: String },

    /// 目录不存在
    #[error("目录不存在：{path}")]
    DirNotFound { path: String, suggestion: String },

    /// 不是文件
    #[error("路径不是文件：{path}")]
    NotAFile { path: String, suggestion: String },

    /// 不是目录
    #[error("路径不是目录：{path}")]
    NotADirectory { path: String, suggestion: String },

    /// IO 操作失败
    #[error("IO 操作失败：{message}")]
    IoError {
        message: String,
        path: Option<String>,
        operation: String,
        suggestion: String,
    },

    /// 目录创建失败
    #[error("创建目录失败：{message}")]
    DirCreationFailed {
        path: String,
        message: String,
        suggestion: String,
    },

    /// 文件已存在
    #[error("文件/目录已存在：{path}")]
    AlreadyExists { path: String, suggestion: String },

    /// 无效的编辑模式
    #[error("无效的编辑模式：{mode}")]
    InvalidEditMode {
        mode: String,
        valid_modes: Vec<String>,
        suggestion: String,
    },

    /// 文本未找到（用于 replace 操作）
    #[error("未找到要替换的文本")]
    TextNotFound {
        search_text: String,
        closest_line: Option<usize>,
        closest_col: Option<usize>,
        context: Option<String>,
        suggestion: String,
    },

    /// 缺少必需参数
    #[error("缺少必需参数：{param_name}")]
    MissingParameter {
        param_name: String,
        message: String,
        suggestion: String,
    },

    /// 模式太长
    #[error("搜索模式过长：{length} > {max_length}")]
    PatternTooLong {
        length: usize,
        max_length: usize,
        suggestion: String,
    },

    /// 无效的正则表达式
    #[error("无效的正则表达式：{message}")]
    InvalidRegex {
        pattern: String,
        message: String,
        suggestion: String,
    },

    /// PDF 加载失败
    #[error("PDF 加载失败：{message}")]
    PdfLoadFailed {
        path: String,
        message: String,
        file_size: Option<u64>,
        suggestion: String,
    },

    /// 无效的文件类型
    #[error("无效的文件类型")]
    InvalidFileType {
        path: String,
        expected_extension: String,
        actual_extension: Option<String>,
        suggestion: String,
    },

    /// 路径过长
    #[error("路径过长：{length} > {max_length}")]
    PathTooLong {
        length: usize,
        max_length: usize,
        suggestion: String,
    },

    /// 无效的 JSON
    #[error("JSON 解析失败：{message}")]
    InvalidJson {
        input: String,
        message: String,
        suggestion: String,
    },

    /// 符号链接循环
    #[error("检测到符号链接循环")]
    SymlinkLoop { path: String, suggestion: String },

    /// 权限不足
    #[error("权限不足：{message}")]
    PermissionDenied {
        path: Option<String>,
        message: String,
        suggestion: String,
    },

    /// 内部错误（不应该发生）
    #[error("内部错误：{message}")]
    Internal { message: String, suggestion: String },
}

impl IoToolError {
    /// 转换为 AI 友好的 JSON 响应
    pub fn to_value(&self) -> Value {
        let (code, context) = match self {
            IoToolError::PathValidation {
                path,
                message,
                suggestion,
            } => (
                "path_validation_failed",
                json!({
                    "path": path,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::FileNotFound { path, suggestion } => (
                "file_not_found",
                json!({
                    "path": path,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::DirNotFound { path, suggestion } => (
                "dir_not_found",
                json!({
                    "path": path,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::NotAFile { path, suggestion } => (
                "not_a_file",
                json!({
                    "path": path,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::NotADirectory { path, suggestion } => (
                "not_a_directory",
                json!({
                    "path": path,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::IoError {
                message,
                path,
                operation,
                suggestion,
            } => (
                "io_error",
                json!({
                    "path": path,
                    "operation": operation,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::DirCreationFailed {
                path,
                message,
                suggestion,
            } => (
                "directory_creation_failed",
                json!({
                    "path": path,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::AlreadyExists { path, suggestion } => (
                "already_exists",
                json!({
                    "path": path,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::InvalidEditMode {
                mode,
                valid_modes,
                suggestion,
            } => (
                "invalid_edit_mode",
                json!({
                    "provided_mode": mode,
                    "valid_modes": valid_modes,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::TextNotFound {
                search_text,
                closest_line,
                closest_col,
                context,
                suggestion,
            } => (
                "text_not_found",
                json!({
                    "search_text": search_text,
                    "closest_line": closest_line,
                    "closest_col": closest_col,
                    "context": context,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::MissingParameter {
                param_name,
                message,
                suggestion,
            } => (
                "missing_parameter",
                json!({
                    "parameter": param_name,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::PatternTooLong {
                length,
                max_length,
                suggestion,
            } => (
                "pattern_too_long",
                json!({
                    "pattern_length": length,
                    "max_length": max_length,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::InvalidRegex {
                pattern,
                message,
                suggestion,
            } => (
                "invalid_regex",
                json!({
                    "pattern": pattern,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::PdfLoadFailed {
                path,
                message,
                file_size,
                suggestion,
            } => (
                "pdf_load_failed",
                json!({
                    "path": path,
                    "message": message,
                    "file_size": file_size,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::InvalidFileType {
                path,
                expected_extension,
                actual_extension,
                suggestion,
            } => (
                "invalid_file_type",
                json!({
                    "path": path,
                    "expected_extension": expected_extension,
                    "actual_extension": actual_extension,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::PathTooLong {
                length,
                max_length,
                suggestion,
            } => (
                "path_too_long",
                json!({
                    "path_length": length,
                    "max_length": max_length,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::InvalidJson {
                input,
                message,
                suggestion,
            } => (
                "invalid_json",
                json!({
                    "input": input,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::SymlinkLoop { path, suggestion } => (
                "symlink_loop",
                json!({
                    "path": path,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::PermissionDenied {
                path,
                message,
                suggestion,
            } => (
                "permission_denied",
                json!({
                    "path": path,
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
            IoToolError::Internal {
                message,
                suggestion,
            } => (
                "internal_error",
                json!({
                    "message": message,
                    "suggestion": suggestion
                }),
            ),
        };

        json!({
            "error": {
                "code": code,
                "message": self.to_string(),
                "context": context
            }
        })
    }

    /// 便捷方法：创建成功响应
    pub fn success_response(operation: &str, data: Value) -> Value {
        json!({
            "status": "success",
            "operation": operation,
            "data": data
        })
    }
}

/// 结果类型别名
pub type IoResult<T> = Result<T, IoToolError>;

/// 允许使用 ? 操作符将 IoToolError 转换为 Value
impl From<IoToolError> for Value {
    fn from(err: IoToolError) -> Self {
        err.to_value()
    }
}

/// 转换为 serde_json::Result 的扩展方法
#[allow(dead_code)]
pub trait ToIoResult<T> {
    fn to_io_result(self, operation: &str, path: Option<&str>) -> IoResult<T>;
}

impl<T> ToIoResult<T> for std::io::Result<T> {
    fn to_io_result(self, operation: &str, path: Option<&str>) -> IoResult<T> {
        self.map_err(|e| IoToolError::IoError {
            message: e.to_string(),
            path: path.map(|p| p.to_string()),
            operation: operation.to_string(),
            suggestion: "请检查文件权限或文件是否被其他进程占用".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found_to_value() {
        let err = IoToolError::FileNotFound {
            path: "/nonexistent.txt".to_string(),
            suggestion: "请检查路径是否正确".to_string(),
        };
        let value = err.to_value();
        assert_eq!(value["error"]["code"], "file_not_found");
        assert_eq!(value["error"]["context"]["path"], "/nonexistent.txt");
    }

    #[test]
    fn test_success_response() {
        let response =
            IoToolError::success_response("read_file", json!({"content": "hello", "size": 5}));
        assert_eq!(response["status"], "success");
        assert_eq!(response["operation"], "read_file");
    }
}
