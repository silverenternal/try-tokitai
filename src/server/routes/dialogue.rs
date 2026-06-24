//! `/v1/dialogue` 端点
//!
//! 把 `DialogueStateMachine` 暴露成 REST。

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dialogue::state_machine::DialogueState;
use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct DialogueStateResp {
    pub state: String,
    pub context: Value,
}

#[derive(Deserialize)]
pub struct TransitionReq {
    pub to: String,
}

#[derive(Deserialize)]
pub struct GoalReq {
    pub goal: String,
}

#[derive(Deserialize)]
pub struct PlanReq {
    pub plan: String,
}

fn parse_state(name: &str) -> Result<DialogueState, ApiError> {
    Ok(match name {
        "Idle" => DialogueState::Idle,
        "Clarifying" => DialogueState::Clarifying,
        "Planning" => DialogueState::Planning,
        "Executing" => DialogueState::Executing,
        "Reviewing" => DialogueState::Reviewing,
        "Completed" => DialogueState::Completed,
        "Error" => DialogueState::Error,
        "WaitingForConfirmation" => DialogueState::WaitingForConfirmation,
        other => {
            return Err(ApiError::BadRequest(format!(
                "未知的 DialogueState：{}",
                other
            )))
        }
    })
}

async fn get_state(State(state): State<AppState>) -> Json<DialogueStateResp> {
    let dialogue = state.dialogue.lock();
    let st = format!("{:?}", dialogue.current_state());
    let context = serde_json::to_value(dialogue.context()).unwrap_or(Value::Null);
    Json(DialogueStateResp { state: st, context })
}

async fn transition(
    State(state): State<AppState>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<Value>, ApiError> {
    let target = parse_state(&req.to)?;
    let mut dialogue = state.dialogue.lock();
    dialogue
        .transition(target, None)
        .map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn set_goal(
    State(state): State<AppState>,
    Json(req): Json<GoalReq>,
) -> Result<Json<Value>, ApiError> {
    let mut dialogue = state.dialogue.lock();
    dialogue
        .set_goal(req.goal)
        .map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn set_plan(
    State(state): State<AppState>,
    Json(req): Json<PlanReq>,
) -> Result<Json<Value>, ApiError> {
    let mut dialogue = state.dialogue.lock();
    dialogue
        .set_plan(req.plan)
        .map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn history(State(state): State<AppState>) -> Json<Value> {
    let dialogue = state.dialogue.lock();
    let history: Vec<_> = dialogue
        .get_history()
        .iter()
        .map(|t| {
            serde_json::json!({
                "from": format!("{:?}", t.from),
                "to": format!("{:?}", t.to),
                "reason": t.reason,
                "timestamp": t.timestamp,
            })
        })
        .collect();
    Json(serde_json::json!({"transitions": history, "count": history.len()}))
}

async fn reset(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let mut dialogue = state.dialogue.lock();
    dialogue
        .reset()
        .map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

/// 路由辅助：被 `routes/clients.rs` 等模块复用
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dialogue/state", get(get_state))
        .route("/dialogue/transition", post(transition))
        .route("/dialogue/goal", post(set_goal))
        .route("/dialogue/plan", post(set_plan))
        .route("/dialogue/history", get(history))
        .route("/dialogue/reset", post(reset))
}