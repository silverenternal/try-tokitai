//! Bearer token 鉴权中间件
//!
//! 仅在 `--api-key` 提供时启用。所有请求都要求：
//! `Authorization: Bearer <token>`。

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::server::error::ApiError;

/// Bearer token 校验中间件
pub async fn bearer_auth(
    State(expected): State<String>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match token {
        Some(actual) if actual == expected => next.run(request).await,
        Some(_) => ApiError::Unauthorized("Bearer token 不正确".to_string()).into_response(),
        None => ApiError::Unauthorized("缺少 Authorization: Bearer <token> 头".to_string())
            .into_response(),
    }
}
