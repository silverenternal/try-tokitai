//! 领域特定错误类型
//!
//! 设计原则:
//! 1. 明确的错误分类：便于 AI 理解和恢复
//! 2. 丰富的错误上下文：提供修复建议
//! 3. 可序列化：支持错误信息传递给 AI

use thiserror::Error;
use serde::{Serialize, Deserialize};

/// 张量操作错误类型
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum TensorError {
    /// 形状不匹配错误
    #[error("Shape mismatch: {message}")]
    ShapeMismatch {
        message: String,
    },

    /// 无效维度错误
    #[error("Invalid dimension {dim}: {message}")]
    InvalidDimension {
        dim: i32,
        message: String,
    },

    /// 数据类型不支持错误
    #[error("Unsupported dtype: {message}")]
    UnsupportedDType {
        message: String,
    },

    /// 广播错误
    #[error("Broadcast failed: {message}")]
    BroadcastError {
        message: String,
    },

    /// 索引越界错误
    #[error("Index out of bounds: {message}")]
    IndexOutOfBounds {
        message: String,
    },

    /// 除零错误
    #[error("Division by zero: {message}")]
    DivisionByZero {
        message: String,
    },

    /// 数值溢出错误
    #[error("Numeric overflow: {message}")]
    NumericOverflow {
        message: String,
    },

    /// 设备错误（如 CUDA 不可用）
    #[error("Device error: {message}")]
    DeviceError {
        message: String,
    },

    /// 内存不足错误
    #[error("Out of memory: {message}")]
    OutOfMemory {
        message: String,
    },

    /// 通用错误（用于未分类的错误）
    #[error("Tensor operation failed: {message}")]
    Other {
        message: String,
    },
}

impl TensorError {
    /// 创建形状不匹配错误
    pub fn shape_mismatch(msg: impl Into<String>) -> Self {
        Self::ShapeMismatch { message: msg.into() }
    }

    /// 创建无效维度错误
    pub fn invalid_dim(dim: i32, msg: impl Into<String>) -> Self {
        Self::InvalidDimension { dim, message: msg.into() }
    }

    /// 创建广播错误
    pub fn broadcast_error(msg: impl Into<String>) -> Self {
        Self::BroadcastError { message: msg.into() }
    }

    /// 创建索引越界错误
    pub fn index_out_of_bounds(msg: impl Into<String>) -> Self {
        Self::IndexOutOfBounds { message: msg.into() }
    }

    /// 创建通用错误
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other { message: msg.into() }
    }

    /// 获取错误修复建议（AI 友好）
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            TensorError::ShapeMismatch { .. } => {
                Some("Check tensor shapes before operation. Use reshape() or view() to make shapes compatible.")
            }
            TensorError::InvalidDimension { .. } => {
                Some("Ensure dimension index is within valid range [0, rank).")
            }
            TensorError::BroadcastError { .. } => {
                Some("Review broadcasting rules: dimensions must be equal or one of them must be 1.")
            }
            TensorError::DivisionByZero { .. } => {
                Some("Add a small epsilon to divisor or check for zero values before division.")
            }
            TensorError::IndexOutOfBounds { .. } => {
                Some("Verify indices are within valid range for the tensor dimensions.")
            }
            _ => None,
        }
    }

    /// 获取错误类别（用于 AI 分类处理）
    pub fn category(&self) -> ErrorCategory {
        match self {
            TensorError::ShapeMismatch { .. } => ErrorCategory::Shape,
            TensorError::InvalidDimension { .. } => ErrorCategory::Dimension,
            TensorError::BroadcastError { .. } => ErrorCategory::Broadcast,
            TensorError::DivisionByZero { .. } => ErrorCategory::Numeric,
            TensorError::NumericOverflow { .. } => ErrorCategory::Numeric,
            TensorError::IndexOutOfBounds { .. } => ErrorCategory::Index,
            TensorError::DeviceError { .. } => ErrorCategory::Device,
            TensorError::OutOfMemory { .. } => ErrorCategory::Memory,
            _ => ErrorCategory::Other,
        }
    }
}

/// 错误类别（用于 AI 分类处理）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    Shape,
    Dimension,
    Broadcast,
    Numeric,
    Index,
    Device,
    Memory,
    Other,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Shape => write!(f, "shape"),
            ErrorCategory::Dimension => write!(f, "dimension"),
            ErrorCategory::Broadcast => write!(f, "broadcast"),
            ErrorCategory::Numeric => write!(f, "numeric"),
            ErrorCategory::Index => write!(f, "index"),
            ErrorCategory::Device => write!(f, "device"),
            ErrorCategory::Memory => write!(f, "memory"),
            ErrorCategory::Other => write!(f, "other"),
        }
    }
}

/// 结果类型别名
pub type TensorResult<T> = Result<T, TensorError>;

// ========== 从 anyhow 转换 ==========

impl From<anyhow::Error> for TensorError {
    fn from(err: anyhow::Error) -> Self {
        TensorError::Other {
            message: err.to_string(),
        }
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TensorError::shape_mismatch("expected [2,3], got [3,2]");
        assert!(err.to_string().contains("Shape mismatch"));
    }

    #[test]
    fn test_error_suggestion() {
        let err = TensorError::shape_mismatch("test");
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn test_error_category() {
        let err = TensorError::shape_mismatch("test");
        assert_eq!(err.category(), ErrorCategory::Shape);

        let err = TensorError::broadcast_error("test");
        assert_eq!(err.category(), ErrorCategory::Broadcast);
    }

    #[test]
    fn test_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("test error");
        let tensor_err: TensorError = anyhow_err.into();
        assert!(matches!(tensor_err, TensorError::Other { .. }));
    }
}
