//! `/v1/autonomy` 端点：自主进化循环
//!
//! `AutonomousAssistant::run_autonomous_evolution` 永不返回，
//! 所以通过 `tokio::spawn` + `CancellationToken` 后台执行；
//! REST 端点负责启动 / 状态 / 停止 / 读取 iteration 历史。

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_lot::Mutex as PlMutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::autonomous_assistant::AutonomousAssistant;
use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Default)]
pub struct AutonomyHandle {
    pub task: Option<tokio::task::JoinHandle<()>>,
    pub cancel: Option<CancellationToken>,
    pub status: String,
    pub project_root: Option<PathBuf>,
}

#[derive(Clone, Default)]
pub struct AutonomyStore(pub Arc<PlMutex<AutonomyHandle>>);

#[derive(Deserialize)]
pub struct StartReq {
    pub project_root: Option<String>,
}

#[derive(Serialize)]
pub struct StatusResp {
    pub running: bool,
    pub status: String,
    pub project_root: Option<String>,
}

async fn start(
    State(state): State<AppState>,
    Json(req): Json<StartReq>,
) -> Result<Json<Value>, ApiError> {
    let mut handle = state.autonomy.0.lock();
    if handle.task.is_some() {
        return Err(ApiError::Conflict("autonomy 已在运行".to_string()));
    }

    let project_root = req
        .project_root
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // 复制 config（避免与 main 共享同一份）
    let cfg = state.config.as_ref().clone();
    let assistant = AutonomousAssistant::new(cfg, project_root.clone())
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let task = tokio::task::spawn_blocking(move || {
        // 在 cancellation 检查间隔里跑；如果实现忽略 cancel，只能强 kill
        let _ = cancel_clone; // 当前实现不可中断，记录以便未来扩展
        if let Err(e) = assistant.run_autonomous_evolution() {
            eprintln!("autonomy error: {}", e);
        }
    });

    handle.task = Some(task);
    handle.cancel = Some(cancel);
    handle.status = "running".to_string();
    handle.project_root = Some(project_root.clone());

    Ok(Json(json!({
        "ok": true,
        "project_root": project_root,
        "status": "running",
    })))
}

async fn stop(State(state): State<AppState>) -> Json<Value> {
    let mut handle = state.autonomy.0.lock();
    if let Some(cancel) = handle.cancel.take() {
        cancel.cancel();
    }
    if let Some(task) = handle.task.take() {
        task.abort();
    }
    handle.status = "stopped".to_string();
    Json(json!({"ok": true, "status": "stopped"}))
}

async fn status(State(state): State<AppState>) -> Json<StatusResp> {
    let handle = state.autonomy.0.lock();
    let running = handle.task.is_some();
    Json(StatusResp {
        running,
        status: handle.status.clone(),
        project_root: handle
            .project_root
            .as_ref()
            .map(|p| p.display().to_string()),
    })
}

async fn gaps(State(state): State<AppState>) -> Json<Value> {
    use crate::autonomy::gap_detector::ToolGapDetector;
    let root = state
        .autonomy
        .0
        .lock()
        .project_root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let data_dir = root.join(".tokitai/autonomy/gaps");
    let payload = match ToolGapDetector::new(data_dir.clone()) {
        Ok(detector) => {
            let gaps: Vec<_> = detector
                .get_gaps()
                .iter()
                .map(|g| {
                    json!({
                        "id": g.id,
                        "gap_type": format!("{:?}", g.gap_type),
                        "description": g.description,
                        "suggested_tool_name": g.suggested_tool_name,
                        "priority": g.priority,
                    })
                })
                .collect();
            json!({"data_dir": data_dir, "gaps": gaps})
        }
        Err(e) => json!({"error": e.to_string()}),
    };
    Json(payload)
}

async fn iterations(State(state): State<AppState>) -> Json<Value> {
    let root = state
        .autonomy
        .0
        .lock()
        .project_root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let iter_dir = root.join(".tokitai/autonomy/iterations");
    let mut entries: Vec<Value> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&iter_dir) {
        for entry in read_dir.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(value) = serde_json::from_str::<Value>(&content) {
                        entries.push(value);
                    }
                }
            }
        }
    }
    Json(json!({"iterations": entries, "count": entries.len()}))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/autonomy/start", post(start))
        .route("/autonomy/stop", post(stop))
        .route("/autonomy/status", get(status))
        .route("/autonomy/gaps", get(gaps))
        .route("/autonomy/iterations", get(iterations))
}
