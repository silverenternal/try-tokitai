//! `/v1/orchestrator` 端点

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::orchestrator::orchestrator::OrchestratorCommand;
use crate::orchestrator::AgentRole;
use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct OrchestratorStateResp {
    pub current_role: String,
    pub context_tokens: usize,
    pub context_messages: usize,
    pub in_workflow: bool,
    pub current_workflow_id: Option<String>,
    pub verbose: bool,
}

#[derive(Deserialize)]
pub struct CommandReq {
    pub command: String,
    #[serde(default)]
    pub arg: Option<String>,
}

#[derive(Serialize)]
pub struct CommandResp {
    pub kind: String,
    pub data: Value,
}

#[derive(Serialize)]
pub struct ContextResp {
    pub messages: usize,
    pub tokens: usize,
}

#[derive(Deserialize)]
pub struct RoleReq {
    pub role: String,
}

fn state_snapshot(state: &AppState) -> OrchestratorStateResp {
    let orchestrator = state.orchestrator.lock();
    let st = orchestrator.get_state();
    OrchestratorStateResp {
        current_role: st.current_role,
        context_tokens: st.context_tokens,
        context_messages: st.context_messages,
        in_workflow: st.in_workflow,
        current_workflow_id: st.current_workflow_id,
        verbose: orchestrator.config.verbose,
    }
}

async fn get_state(State(state): State<AppState>) -> Json<OrchestratorStateResp> {
    Json(state_snapshot(&state))
}

async fn run_command(
    State(state): State<AppState>,
    Json(req): Json<CommandReq>,
) -> Result<Json<CommandResp>, ApiError> {
    let command = match req.command.as_str() {
        "SwitchRole" => {
            let role = req.arg.unwrap_or_else(|| "general".to_string());
            OrchestratorCommand::SwitchRole(AgentRole::from_str(&role))
        }
        "OptimizeContext" => OrchestratorCommand::OptimizeContext,
        "ShowContext" => OrchestratorCommand::ShowContext,
        "ShowRoles" => OrchestratorCommand::ShowRoles,
        "Workflow" => OrchestratorCommand::Workflow(req.arg.unwrap_or_default()),
        "ShowHelp" => OrchestratorCommand::ShowHelp,
        "HealthCheck" => OrchestratorCommand::HealthCheck,
        "Stats" => OrchestratorCommand::Stats,
        "OptimizeCache" => OrchestratorCommand::OptimizeCache,
        "Toolbox" => OrchestratorCommand::Toolbox,
        "SwitchProvider" => OrchestratorCommand::SwitchProvider,
        "ShowProviders" => OrchestratorCommand::ShowProviders,
        other => {
            return Err(ApiError::BadRequest(format!(
                "未知 orchestrator 命令：{}",
                other
            )))
        }
    };

    let result = state.orchestrator.lock().execute_command(command);
    Ok(Json(CommandResp {
        kind: format!("{:?}", result),
        data: serde_json::json!({"result": format!("{:?}", result)}),
    }))
}

async fn get_context(State(state): State<AppState>) -> Json<ContextResp> {
    let orchestrator = state.orchestrator.lock();
    let messages = orchestrator.get_context_messages();
    Json(ContextResp {
        messages: messages.len(),
        tokens: orchestrator.get_state().context_tokens,
    })
}

async fn clear_context(State(state): State<AppState>) -> Json<Value> {
    state.orchestrator.lock().clear_context();
    Json(serde_json::json!({"ok": true}))
}

async fn set_role(
    State(state): State<AppState>,
    Json(req): Json<RoleReq>,
) -> Result<Json<Value>, ApiError> {
    let role = AgentRole::from_str(&req.role);
    let mut orchestrator = state.orchestrator.lock();
    orchestrator.execute_command(OrchestratorCommand::SwitchRole(role));
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn show_roles(State(state): State<AppState>) -> Json<Value> {
    let mut orchestrator = state.orchestrator.lock();
    let result = orchestrator.execute_command(OrchestratorCommand::ShowRoles);
    Json(serde_json::json!({"result": format!("{:?}", result)}))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let orchestrator = state.orchestrator.lock();
    let llm = state.llm.lock();
    Json(serde_json::json!({
        "orchestrator": orchestrator.get_state(),
        "provider_count": llm.list_providers().len(),
        "current_provider": llm.current_provider_type().map(|p| p.as_str()),
    }))
}

async fn stats(State(state): State<AppState>) -> Json<Value> {
    let mut orchestrator = state.orchestrator.lock();
    let result = orchestrator.execute_command(OrchestratorCommand::Stats);
    Json(serde_json::json!({"result": format!("{:?}", result)}))
}

async fn clear_cache(State(state): State<AppState>) -> Json<Value> {
    let mut orchestrator = state.orchestrator.lock();
    let result = orchestrator.execute_command(OrchestratorCommand::OptimizeCache);
    Json(serde_json::json!({"result": format!("{:?}", result)}))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orchestrator/state", get(get_state))
        .route("/orchestrator/command", post(run_command))
        .route("/orchestrator/context", get(get_context))
        .route("/orchestrator/context/clear", post(clear_context))
        .route("/orchestrator/role", post(set_role))
        .route("/orchestrator/roles", get(show_roles))
        .route("/orchestrator/health", get(health))
        .route("/orchestrator/stats", get(stats))
        .route("/orchestrator/cache/clear", post(clear_cache))
}
