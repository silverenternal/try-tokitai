//! 健康检查与版本信息端点

use axum::routing::get;
use axum::Router;
use serde::Serialize;

use crate::server::state::AppState;

/// 健康检查响应
#[derive(Serialize)]
pub struct HealthResp {
    pub status: &'static str,
    pub service: &'static str,
}

/// 版本响应
#[derive(Serialize)]
pub struct VersionResp {
    pub name: &'static str,
    pub version: &'static str,
    pub rust_version: &'static str,
}

/// ping 响应
#[derive(Serialize)]
pub struct PingResp {
    pub pong: bool,
    pub timestamp: i64,
}

/// `GET /healthz` — 探针用健康检查
async fn healthz() -> axum::Json<HealthResp> {
    axum::Json(HealthResp {
        status: "ok",
        service: "ai-assistant-server",
    })
}

/// `GET /v1/ping` — 极简存活探测
async fn ping() -> axum::Json<PingResp> {
    axum::Json(PingResp {
        pong: true,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// `GET /v1/version` — 服务版本
async fn version() -> axum::Json<VersionResp> {
    axum::Json(VersionResp {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
    })
}

/// 本模块子路由（在 v1 命名空间下挂载）
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ping", get(ping))
        .route("/version", get(version))
}