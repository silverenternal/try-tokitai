//! 统一 API 错误模型
//!
//! 所有 handler 通过 `Result<T, ApiError>` 返回错误；
//! `IntoResponse` 将其映射为对应的 HTTP 状态码 + JSON 错误信封。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

/// API 错误统一类型
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// 400 客户端请求格式错误
    #[error("请求参数无效：{0}")]
    BadRequest(String),
    /// 401 未鉴权或 token 错误
    #[error("未授权：{0}")]
    Unauthorized(String),
    /// 404 资源不存在
    #[error("未找到：{0}")]
    NotFound(String),
    /// 409 当前状态不允许该操作
    #[error("冲突：{0}")]
    Conflict(String),
    /// 422 业务校验失败 / 工具调用失败
    #[error("工具执行失败：{0}")]
    ToolError(String),
    /// 502 上游 LLM 调用失败
    #[error("LLM 调用失败：{0}")]
    LlmError(String),
    /// 500 内部错误
    #[error("内部错误：{0}")]
    Internal(String),
}

/// 错误响应 JSON 信封
#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
    request_id: String,
}

impl ApiError {
    fn parts(&self) -> (StatusCode, &'static str, String) {
        match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, "BadRequest", m.clone()),
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, "Unauthorized", m.clone()),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, "NotFound", m.clone()),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, "Conflict", m.clone()),
            ApiError::ToolError(m) => (StatusCode::UNPROCESSABLE_ENTITY, "ToolError", m.clone()),
            ApiError::LlmError(m) => (StatusCode::BAD_GATEWAY, "LlmError", m.clone()),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal", m.clone()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = self.parts();
        let body = ErrorBody {
            error: ErrorDetail {
                code,
                message,
                request_id: Uuid::new_v4().to_string(),
            },
        };
        (status, Json(body)).into_response()
    }
}

/// 从 anyhow::Error 转换为 ApiError（默认归类为 Internal）
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// 从 serde_json::Error 转换为 ApiError
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("JSON 解析失败：{}", err))
    }
}

/// 便捷宏：让 handler 直接 `bail!` 到 ApiError
#[macro_export]
macro_rules! bail_api {
    ($variant:ident, $($arg:tt)*) => {
        return Err($crate::server::error::ApiError::$variant(format!($($arg)*)))
    };
}