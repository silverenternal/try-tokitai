//! `/v1/tool-market` 端点：发布 / 搜索 / 安装 / 列出工具

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct SearchReq {
    pub query: String,
}

#[derive(Deserialize)]
pub struct ToolReq {
    pub tool: String,
}

#[derive(Serialize)]
pub struct ListResp {
    pub tools: Vec<String>,
}

async fn list(State(state): State<AppState>) -> Result<Json<ListResp>, ApiError> {
    let guard = state.tool_market.lock().await;
    let market = guard
        .as_ref()
        .ok_or_else(|| ApiError::Conflict("tool_market 未初始化".to_string()))?;
    let tools = market
        .list()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ListResp { tools }))
}

async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchReq>,
) -> Result<Json<Value>, ApiError> {
    let guard = state.tool_market.lock().await;
    let market = guard
        .as_ref()
        .ok_or_else(|| ApiError::Conflict("tool_market 未初始化".to_string()))?;
    market
        .search(&req.query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({"ok": true, "query": req.query})))
}

async fn install(
    State(state): State<AppState>,
    Json(req): Json<ToolReq>,
) -> Result<Json<Value>, ApiError> {
    let guard = state.tool_market.lock().await;
    let market = guard
        .as_ref()
        .ok_or_else(|| ApiError::Conflict("tool_market 未初始化".to_string()))?;
    market
        .install(&req.tool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({"ok": true, "tool": req.tool})))
}

async fn publish(
    State(state): State<AppState>,
    Json(req): Json<ToolReq>,
) -> Result<Json<Value>, ApiError> {
    let guard = state.tool_market.lock().await;
    let market = guard
        .as_ref()
        .ok_or_else(|| ApiError::Conflict("tool_market 未初始化".to_string()))?;
    market
        .publish(&req.tool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({"ok": true, "tool": req.tool})))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tool-market/list", get(list))
        .route("/tool-market/search", post(search))
        .route("/tool-market/install", post(install))
        .route("/tool-market/publish", post(publish))
}
