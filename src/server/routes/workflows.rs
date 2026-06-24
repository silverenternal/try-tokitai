//! `/v1/workflows` 端点
//!
//! 暴露内置工作流模板（`code_review`, `task_decomposition`），并允许通过
//! `WorkflowStore` 跟踪创建后到执行 / 暂停 / 取消的状态。

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;

use crate::orchestrator::workflow::{
    templates, Workflow, WorkflowEngine, WorkflowStatus,
};
use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct WorkflowListResp {
    pub count: usize,
    pub workflows: Vec<WorkflowSummary>,
    pub templates: Vec<TemplateSummary>,
}

#[derive(Serialize)]
pub struct WorkflowSummary {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct TemplateSummary {
    pub id: &'static str,
    pub name: String,
}

fn summarize(id: uuid::Uuid, engine: &WorkflowEngine) -> WorkflowSummary {
    let w = engine.get_workflow();
    WorkflowSummary {
        id,
        name: w.name.clone(),
        description: w.description.clone(),
        status: format!("{:?}", engine.get_status()),
    }
}

async fn list(State(state): State<AppState>) -> Json<WorkflowListResp> {
    let store = state.stores.workflows.lock();
    let workflows: Vec<WorkflowSummary> = store
        .list()
        .into_iter()
        .map(|(id, _)| {
            let engine = store.get(&id).expect("id from list");
            summarize(id, engine)
        })
        .collect();
    let templates = vec![
        TemplateSummary {
            id: "code_review",
            name: "代码审查".to_string(),
        },
        TemplateSummary {
            id: "task_decomposition",
            name: "任务分解".to_string(),
        },
    ];
    Json(WorkflowListResp {
        count: workflows.len(),
        workflows,
        templates,
    })
}

fn build_template(id: &str) -> Result<Workflow, ApiError> {
    match id {
        "code_review" => Ok(templates::create_code_review_workflow()),
        "task_decomposition" => Ok(templates::create_task_decomposition_workflow()),
        other => Err(ApiError::NotFound(format!(
            "未知工作流模板：{}",
            other
        ))),
    }
}

#[derive(serde::Deserialize)]
pub struct CreateWorkflowReq {
    pub template: String,
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkflowReq>,
) -> Result<Json<WorkflowSummary>, ApiError> {
    let workflow = build_template(&req.template)?;
    let engine = WorkflowEngine::new(workflow);
    let mut store = state.stores.workflows.lock();
    let id = store.register(engine);
    let engine_ref = store.get(&id).expect("just registered");
    Ok(Json(summarize(id, engine_ref)))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Value>, ApiError> {
    let store = state.stores.workflows.lock();
    let engine = store
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("workflow {} 不存在", id)))?;
    let w = engine.get_workflow();
    let payload = serde_json::json!({
        "id": id,
        "name": w.name,
        "description": w.description,
        "status": format!("{:?}", engine.get_status()),
    });
    Ok(Json(payload))
}

async fn status(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Value>, ApiError> {
    let store = state.stores.workflows.lock();
    let status = store
        .status_of(&id)
        .ok_or_else(|| ApiError::NotFound(format!("workflow {} 不存在", id)))?;
    Ok(Json(serde_json::json!({"status": format!("{:?}", status)})))
}

async fn execute(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Value>, ApiError> {
    let mut store = state.stores.workflows.lock();
    let engine = store
        .get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("workflow {} 不存在", id)))?;
    if matches!(engine.get_status(), WorkflowStatus::Running) {
        return Err(ApiError::Conflict("workflow 已在运行".to_string()));
    }
    let result = engine
        .execute()
        .map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "workflow_id": result.workflow_id,
        "status": format!("{:?}", result.status),
        "total_duration_ms": result.total_duration_ms,
    })))
}

async fn pause(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Value>, ApiError> {
    let mut store = state.stores.workflows.lock();
    let engine = store
        .get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("workflow {} 不存在", id)))?;
    engine.pause();
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Value>, ApiError> {
    let mut store = state.stores.workflows.lock();
    let engine = store
        .get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("workflow {} 不存在", id)))?;
    engine.cancel();
    Ok(Json(serde_json::json!({"ok": true})))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workflows", get(list))
        .route("/workflows", post(create))
        .route("/workflows/:id", get(get_one))
        .route("/workflows/:id/execute", post(execute))
        .route("/workflows/:id/status", get(status))
        .route("/workflows/:id/pause", post(pause))
        .route("/workflows/:id/cancel", post(cancel))
}