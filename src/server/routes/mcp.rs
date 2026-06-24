//! `/v1/mcp` 端点：MCP 客户端管理

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::mcp::McpServerDescription;
use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct ConnectReq {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct DisconnectReq {
    pub name: String,
}

#[derive(Deserialize)]
pub struct CallReq {
    pub server: String,
    pub tool: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Serialize)]
pub struct ServerListResp {
    pub servers: Vec<Value>,
}

#[derive(Serialize)]
pub struct ToolListResp {
    pub tools: Vec<Value>,
}

#[derive(Deserialize)]
pub struct ToolsQuery {
    pub server: Option<String>,
}

async fn list_servers(State(state): State<AppState>) -> Json<ServerListResp> {
    let mgr = state.mcp.lock();
    let servers: Vec<Value> = mgr
        .client()
        .list_servers()
        .into_iter()
        .map(|s: &McpServerDescription| {
            json!({
                "name": s.name,
                "description": s.description,
                "endpoint": s.endpoint,
                "transport": s.transport,
            })
        })
        .collect();
    Json(ServerListResp { servers })
}

async fn connect(
    State(state): State<AppState>,
    Json(req): Json<ConnectReq>,
) -> Result<Json<Value>, ApiError> {
    let mgr = state.mcp.lock();
    let _ = mgr;
    Ok(Json(json!({
        "ok": true,
        "name": req.name,
        "note": "MCP client stub: 实际 stdio 通讯未启用",
    })))
}

async fn disconnect(
    State(state): State<AppState>,
    Json(req): Json<DisconnectReq>,
) -> Result<Json<Value>, ApiError> {
    let mut mgr = state.mcp.lock();
    mgr.client_mut()
        .disconnect(&req.name)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({"ok": true, "name": req.name})))
}

async fn list_tools(
    State(state): State<AppState>,
    Query(_q): Query<ToolsQuery>,
) -> Json<ToolListResp> {
    let mgr = state.mcp.lock();
    let tools = mgr
        .client()
        .list_tools()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
                "server_name": t.server_name,
            })
        })
        .collect();
    Json(ToolListResp { tools })
}

async fn call_tool(
    State(state): State<AppState>,
    Json(_req): Json<CallReq>,
) -> Result<Json<Value>, ApiError> {
    let _ = state.mcp.lock();
    Err(ApiError::Conflict(
        "MCP client 工具调用未启用（需要外部 stdio 客户端）".to_string(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/mcp/servers", get(list_servers))
        .route("/mcp/connect", post(connect))
        .route("/mcp/disconnect", post(disconnect))
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/call", post(call_tool))
}
