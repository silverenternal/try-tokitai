//! `/v1/sessions` 端点
//!
//! 提供内存版对话会话管理：CRUD + 追加消息。Session 数据存在
//! `SharedStores::sessions` 內（重启会丢失；持久化留给 Commit 6 的 docs 阶段）。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::server::error::ApiError;
use crate::server::state::AppState;
use crate::server::stores::ConversationSession;

#[derive(Serialize)]
pub struct SessionListResp {
    pub count: usize,
    pub sessions: Vec<ConversationSession>,
}

#[derive(Deserialize)]
pub struct CreateSessionReq {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct AppendMessageReq {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct AppendMessageResp {
    pub ok: bool,
    pub message: Value,
    pub message_count: usize,
}

async fn list(State(state): State<AppState>) -> Json<SessionListResp> {
    let store = state.stores.sessions.lock();
    Json(SessionListResp {
        count: store.list().len(),
        sessions: store.list().into_iter().cloned().collect(),
    })
}

async fn create(
    State(state): State<AppState>,
    req: Option<Json<CreateSessionReq>>,
) -> (StatusCode, Json<ConversationSession>) {
    let name = req.and_then(|Json(r)| r.name);
    let mut store = state.stores.sessions.lock();
    let session = store.create(name);
    (StatusCode::CREATED, Json(session))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConversationSession>, ApiError> {
    let store = state.stores.sessions.lock();
    store
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("session {} 不存在", id)))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let mut store = state.stores.sessions.lock();
    if store.delete(&id) {
        Ok(Json(serde_json::json!({"ok": true})))
    } else {
        Err(ApiError::NotFound(format!("session {} 不存在", id)))
    }
}

async fn append_message(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<AppendMessageReq>,
) -> Result<Json<AppendMessageResp>, ApiError> {
    let mut store = state.stores.sessions.lock();
    let session = store
        .get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("session {} 不存在", id)))?;
    let message = serde_json::json!({
        "role": req.role,
        "content": req.content,
        "name": req.name,
    });
    session.messages.push(message.clone());
    session.touch();
    let count = session.messages.len();
    Ok(Json(AppendMessageResp {
        ok: true,
        message,
        message_count: count,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(list))
        .route("/sessions", post(create))
        .route("/sessions/:id", get(get_one).delete(delete_one))
        .route("/sessions/:id/messages", post(append_message))
}
