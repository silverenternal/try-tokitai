//! `/v1/cli` 端点：CLI 桥接
//!
//! 透过 `ServerToolSet` 提供一次性 chat-and-tool 入口，并暴露
//! orchestrator 命令的字符串形式（`/v1/cli/slash`）。

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct RunReq {
    /// 一次性用户输入
    pub input: String,
    /// 可选 session id（当前实现仅做 echo）
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SlashReq {
    pub command: String,
    #[serde(default)]
    pub arg: Option<String>,
}

#[derive(Serialize)]
pub struct RunResp {
    pub echoed: String,
    pub tool_invocation: Option<ToolInvocation>,
}

#[derive(Serialize)]
pub struct ToolInvocation {
    pub tool: String,
    pub result: String,
}

#[derive(Serialize)]
pub struct SlashResp {
    pub kind: String,
    pub data: Value,
}

/// `POST /v1/cli/run` — 简单执行一次工具调用
///
/// 行为：若 `input` 形如 `tool_name arg1=val1 arg2=val2`，则解析为
/// `{name, arguments: {arg1: val1, ...}}` 并委托 `ServerToolSet::call_tool`；
/// 否则将 input 原样回显。
async fn run(
    State(state): State<AppState>,
    Json(req): Json<RunReq>,
) -> Result<Json<RunResp>, ApiError> {
    let trimmed = req.input.trim();
    let mut iter = trimmed.splitn(2, char::is_whitespace);
    let head = iter.next().unwrap_or("").to_string();
    let tail = iter.next().unwrap_or("").trim().to_string();

    let (tool, args) = if head.is_empty() {
        (None, Value::Null)
    } else if tail.is_empty() {
        (Some(head), Value::Object(Default::default()))
    } else {
        let mut obj = serde_json::Map::new();
        for pair in tail.split_whitespace() {
            if let Some((k, v)) = pair.split_once('=') {
                obj.insert(k.to_string(), Value::String(v.to_string()));
            } else {
                obj.insert(pair.to_string(), Value::Bool(true));
            }
        }
        (Some(head), Value::Object(obj))
    };

    let invocation = if let Some(name) = tool {
        let result = state
            .tool_set
            .call_tool(&name, &args)
            .map_err(|e| ApiError::ToolError(e.to_string()))?;
        Some(ToolInvocation { tool: name, result })
    } else {
        None
    };

    Ok(Json(RunResp {
        echoed: req.input,
        tool_invocation: invocation,
    }))
}

/// `POST /v1/cli/slash` — 把字符串命令转换成 OrchestratorCommand
async fn slash(
    State(state): State<AppState>,
    Json(req): Json<SlashReq>,
) -> Result<Json<SlashResp>, ApiError> {
    use crate::orchestrator::orchestrator::OrchestratorCommand;
    use crate::orchestrator::AgentRole;

    let mut orchestrator = state.orchestrator.lock();
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
            )));
        }
    };
    let result = orchestrator.execute_command(command);
    Ok(Json(SlashResp {
        kind: format!("{:?}", result),
        data: json!({"result": format!("{:?}", result)}),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/cli/run", post(run))
        .route("/cli/slash", post(slash))
}
