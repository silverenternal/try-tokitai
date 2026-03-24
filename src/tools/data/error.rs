//! 统一数据工具错误类型
//!
//! 按可恢复性分类，支持 AI 决策重试策略

use std::fmt;
use serde::{Serialize, Deserialize};

/// 错误可恢复性分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorRecoverability {
    /// 可重试错误（临时性故障）
    Retryable,
    /// 需要修正输入后重试
    Fixable,
    /// 不可恢复错误（配置/资源超限）
    Fatal,
}

/// 数据操作统一错误类型
#[derive(Debug, Serialize, Deserialize)]
pub enum DataToolError {
    /// JSON 解析错误（输入格式问题）
    JsonParse {
        message: String,
        recoverability: ErrorRecoverability,
    },

    /// 资源超限（长度/深度/数量）
    ResourceExceeded {
        resource_type: String,
        current: usize,
        max: usize,
        recoverability: ErrorRecoverability,
    },

    /// 类型错误（输入类型不符合预期）
    InvalidType {
        expected: String,
        actual: String,
        recoverability: ErrorRecoverability,
    },

    /// 路径错误（JSON 路径不存在）
    PathNotFound {
        path: String,
        recoverability: ErrorRecoverability,
    },

    /// IO 错误
    Io {
        message: String,
        recoverability: ErrorRecoverability,
    },

    /// 内部错误（unexpected）
    Internal {
        message: String,
        recoverability: ErrorRecoverability,
    },
}

impl DataToolError {
    /// 获取错误可恢复性
    pub fn recoverability(&self) -> ErrorRecoverability {
        match self {
            DataToolError::JsonParse { recoverability, .. } => *recoverability,
            DataToolError::ResourceExceeded { recoverability, .. } => *recoverability,
            DataToolError::InvalidType { recoverability, .. } => *recoverability,
            DataToolError::PathNotFound { recoverability, .. } => *recoverability,
            DataToolError::Io { recoverability, .. } => *recoverability,
            DataToolError::Internal { recoverability, .. } => *recoverability,
        }
    }

    /// 判断是否可重试
    pub fn is_retryable(&self) -> bool {
        self.recoverability() == ErrorRecoverability::Retryable
    }

    /// 判断是否需要修正输入
    pub fn is_fixable(&self) -> bool {
        self.recoverability() == ErrorRecoverability::Fixable
    }

    /// 创建 JSON 解析错误
    pub fn json_parse(msg: impl Into<String>) -> Self {
        Self::JsonParse {
            message: msg.into(),
            recoverability: ErrorRecoverability::Fixable,
        }
    }

    /// 创建资源超限错误
    pub fn resource_exceeded(
        resource_type: impl Into<String>,
        current: usize,
        max: usize,
    ) -> Self {
        Self::ResourceExceeded {
            resource_type: resource_type.into(),
            current,
            max,
            recoverability: ErrorRecoverability::Fatal,
        }
    }

    /// 创建类型错误
    pub fn invalid_type(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::InvalidType {
            expected: expected.into(),
            actual: actual.into(),
            recoverability: ErrorRecoverability::Fixable,
        }
    }

    /// 创建路径不存在错误
    pub fn path_not_found(path: impl Into<String>) -> Self {
        Self::PathNotFound {
            path: path.into(),
            recoverability: ErrorRecoverability::Fixable,
        }
    }

    /// 创建 IO 错误
    pub fn io(msg: impl Into<String>) -> Self {
        Self::Io {
            message: msg.into(),
            recoverability: ErrorRecoverability::Retryable,
        }
    }

    /// 创建内部错误
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal {
            message: msg.into(),
            recoverability: ErrorRecoverability::Fatal,
        }
    }
}

impl fmt::Display for DataToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataToolError::JsonParse { message, .. } => {
                write!(f, "JSON 解析错误：{}", message)
            }
            DataToolError::ResourceExceeded {
                resource_type,
                current,
                max,
                ..
            } => {
                write!(f, "{} 超限 ({} > {})", resource_type, current, max)
            }
            DataToolError::InvalidType {
                expected, actual, ..
            } => {
                write!(f, "类型错误：期望 {}，实际 {}", expected, actual)
            }
            DataToolError::PathNotFound { path, .. } => {
                write!(f, "路径不存在：{}", path)
            }
            DataToolError::Io { message, .. } => {
                write!(f, "IO 错误：{}", message)
            }
            DataToolError::Internal { message, .. } => {
                write!(f, "内部错误：{}", message)
            }
        }
    }
}

impl std::error::Error for DataToolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl DataToolError {
    /// 转换为 AI 友好的 JSON 响应
    pub fn to_value(&self) -> serde_json::Value {
        use serde_json::json;
        let (code, context) = match self {
            DataToolError::JsonParse { message, .. } => (
                "json_parse_error",
                json!({
                    "message": message
                }),
            ),
            DataToolError::ResourceExceeded {
                resource_type,
                current,
                max,
                ..
            } => (
                "resource_exceeded",
                json!({
                    "resource_type": resource_type,
                    "current": current,
                    "max": max
                }),
            ),
            DataToolError::InvalidType {
                expected, actual, ..
            } => (
                "invalid_type",
                json!({
                    "expected": expected,
                    "actual": actual
                }),
            ),
            DataToolError::PathNotFound { path, .. } => (
                "path_not_found",
                json!({
                    "path": path
                }),
            ),
            DataToolError::Io { message, .. } => (
                "io_error",
                json!({
                    "message": message
                }),
            ),
            DataToolError::Internal { message, .. } => (
                "internal_error",
                json!({
                    "message": message
                }),
            ),
        };

        json!({
            "success": false,
            "error": {
                "code": code,
                "context": context
            }
        })
    }
}

/// 允许使用 ? 操作符将 DataToolError 转换为 Value
impl From<DataToolError> for serde_json::Value {
    fn from(err: DataToolError) -> Self {
        err.to_value()
    }
}

/// 统一结果类型
pub type DataToolResult<T> = Result<T, DataToolError>;

/// 错误辅助信息（用于 tracing 和日志）
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ErrorContext {
    pub operation: Option<String>,
    pub path: Option<String>,
    pub length: Option<usize>,
    pub depth: Option<usize>,
    pub details: Option<String>,
}

#[allow(dead_code)]
impl ErrorContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_length(mut self, length: usize) -> Self {
        self.length = Some(length);
        self
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// 格式化为详细错误信息
    pub fn format(&self, base_error: &str) -> String {
        let mut parts = vec![base_error.to_string()];

        if let Some(op) = &self.operation {
            parts.push(format!("操作：{}", op));
        }

        if let Some(path) = &self.path {
            parts.push(format!("路径：{}", path));
        }

        if let Some(len) = &self.length {
            parts.push(format!("长度：{}", len));
        }

        if let Some(depth) = &self.depth {
            parts.push(format!("深度：{}", depth));
        }

        if let Some(details) = &self.details {
            parts.push(format!("详情：{}", details));
        }

        parts.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DataToolError::json_parse("invalid JSON");
        assert!(format!("{}", err).contains("JSON 解析错误"));

        let err = DataToolError::path_not_found("user.name");
        assert_eq!(format!("{}", err), "路径不存在：user.name");

        let err = DataToolError::resource_exceeded("JSON 长度", 200, 100);
        assert_eq!(format!("{}", err), "JSON 长度 超限 (200 > 100)");
    }

    #[test]
    fn test_error_recoverability() {
        let err = DataToolError::json_parse("invalid");
        assert!(err.is_fixable());
        assert!(!err.is_retryable());

        let err = DataToolError::io("connection reset");
        assert!(err.is_retryable());

        let err = DataToolError::resource_exceeded("length", 200, 100);
        assert!(!err.is_retryable());
        assert!(!err.is_fixable());
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_operation("format_json")
            .with_path("$.user.name")
            .with_length(1024)
            .with_depth(5)
            .with_details("嵌套对象");

        let formatted = ctx.format("解析失败");
        assert!(formatted.contains("操作：format_json"));
        assert!(formatted.contains("路径：$.user.name"));
        assert!(formatted.contains("长度：1024"));
        assert!(formatted.contains("深度：5"));
        assert!(formatted.contains("详情：嵌套对象"));
    }
}
