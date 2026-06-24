//! `/v1/tools` 与 `/v1/tools/call` 端点
//!
//! - `GET /v1/tools` 返回所有可用工具的元数据
//! - `POST /v1/tools/call` 调用指定工具，body = `{ "name": "...", "arguments": {...} }`

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::server::error::ApiError;
use crate::server::state::AppState;
use crate::tools::{
    CodeTools, DownloadTools, FileOperations, GitOperations, HttpClientTools, JsonFormatTools,
    SearchTools, SystemTools,
};

/// `GET /v1/tools` 响应
#[derive(Serialize)]
pub struct ListToolsResp {
    pub count: usize,
    pub tools: Vec<Value>,
}

/// `POST /v1/tools/call` 请求
#[derive(Deserialize)]
pub struct CallToolReq {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// `POST /v1/tools/call` 响应
#[derive(Serialize)]
pub struct CallToolResp {
    pub name: String,
    pub result: String,
}

/// `GET /v1/tools` — 列出所有可用工具（OpenAI function 格式）
async fn list_tools(State(_state): State<AppState>) -> Json<ListToolsResp> {
    // 直接使用关联函数 `T::tool_definitions()`
    let mut tools: Vec<Value> = Vec::new();

    macro_rules! collect_defs {
        ($ty:ty) => {{
            for def in <$ty as tokitai::ToolProvider>::tool_definitions() {
                tools.push(serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": serde_json::from_str::<Value>(&def.input_schema)
                            .unwrap_or(Value::Null),
                    }
                }));
            }
        }};
    }

    collect_defs!(FileOperations);
    collect_defs!(SystemTools);
    collect_defs!(CodeTools);
    collect_defs!(SearchTools);
    collect_defs!(DownloadTools);
    collect_defs!(GitOperations);
    collect_defs!(HttpClientTools);
    collect_defs!(JsonFormatTools);

    Json(ListToolsResp {
        count: tools.len(),
        tools,
    })
}

/// `POST /v1/tools/call` — 调用工具
async fn call_tool(
    State(state): State<AppState>,
    Json(req): Json<CallToolReq>,
) -> Result<Json<CallToolResp>, ApiError> {
    let result = state
        .tool_set
        .call_tool(&req.name, &req.arguments)
        .map_err(|e| ApiError::ToolError(e.to_string()))?;
    Ok(Json(CallToolResp {
        name: req.name,
        result,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tools", get(list_tools))
        .route("/tools/call", post(call_tool))
}