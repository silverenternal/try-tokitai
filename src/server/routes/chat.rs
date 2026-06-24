//! `/v1/chat` 与 `/v1/chat/stream` 端点
//!
//! - `POST /v1/chat`：非流式，调用 LLMManager 当前 provider 的 chat 方法
//! - `POST /v1/chat/stream`：SSE 流式，仅在当前 provider 是 OpenAI 时可用
//!
//! 注意：Chat 在 server 模式下不复刻 CliAssistant::chat_and_handle_tools 的
//! 工具递归调用循环；如需工具调用，请直接用 `/v1/tools/call`。

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::post;
use axum::{Json, Router};
use futures::stream::Stream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::Infallible;

use crate::llm::{ChatRequest, LLMProvider, Message};
use crate::server::error::ApiError;
use crate::server::state::AppState;

/// `POST /v1/chat` 请求体
#[derive(Deserialize)]
pub struct ChatReq {
    /// 对话消息列表，OpenAI 兼容格式：`{role, content, name?}`
    pub messages: Vec<Value>,
    /// 可选模型名（覆盖 LLMManager 当前 model）
    #[serde(default)]
    pub model: Option<String>,
    /// 温度（默认 0.7）
    #[serde(default = "default_temp")]
    pub temperature: f32,
    /// 最大 token 数
    #[serde(default)]
    pub max_tokens: Option<usize>,
    /// top_p
    #[serde(default)]
    pub top_p: Option<f32>,
}

fn default_temp() -> f32 {
    0.7
}

/// `POST /v1/chat` 响应
#[derive(Serialize)]
pub struct ChatResp {
    pub content: String,
    pub model: String,
    pub usage: Option<Value>,
    pub finish_reason: Option<String>,
}

fn current_provider(state: &AppState) -> Result<std::sync::Arc<dyn LLMProvider>, ApiError> {
    let llm = state.llm.lock();
    llm.current_provider()
        .cloned()
        .ok_or_else(|| ApiError::LlmError("未设置当前 LLM provider".to_string()))
}

/// `POST /v1/chat` handler
async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatReq>,
) -> Result<Json<ChatResp>, ApiError> {
    let provider = current_provider(&state)?;

    let messages: Vec<Message> = req
        .messages
        .into_iter()
        .map(message_from_value)
        .collect::<Result<_, _>>()?;

    let model = req
        .model
        .unwrap_or_else(|| provider.default_model().to_string());

    let chat_req = ChatRequest {
        model,
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        top_p: req.top_p,
        stop: None,
        stream: false,
    };

    let resp = provider
        .chat(chat_req)
        .await
        .map_err(|e| ApiError::LlmError(e.to_string()))?;

    Ok(Json(ChatResp {
        content: resp.content,
        model: resp.model,
        usage: resp.usage.map(|u| {
            serde_json::json!({
                "prompt_tokens": u.prompt_tokens,
                "completion_tokens": u.completion_tokens,
                "total_tokens": u.total_tokens,
            })
        }),
        finish_reason: resp.finish_reason,
    }))
}

/// `POST /v1/chat/stream` handler — SSE 流式响应
async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatReq>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let provider = current_provider(&state)?;

    // SSE 流式目前仅在 OpenAIProvider 实现
    if provider.provider_type().as_str() != "openai" {
        return Err(ApiError::Conflict(format!(
            "当前 provider '{}' 不支持流式响应，请使用 /v1/chat",
            provider.provider_type().as_str()
        )));
    }

    let messages: Vec<Message> = req
        .messages
        .into_iter()
        .map(message_from_value)
        .collect::<Result<_, _>>()?;
    let model = req
        .model
        .unwrap_or_else(|| provider.default_model().to_string());
    let chat_req = ChatRequest {
        model,
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        top_p: req.top_p,
        stop: None,
        stream: true,
    };

    let mut stream = provider
        .chat_stream(chat_req)
        .await
        .map_err(|e| ApiError::LlmError(e.to_string()))?;

    let sse = async_stream::stream! {
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(c) => {
                    let payload = serde_json::json!({
                        "delta": c.content,
                        "finish_reason": c.finish_reason,
                    });
                    yield Ok::<_, Infallible>(Event::default().data(payload.to_string()));
                }
                Err(e) => {
                    let payload = serde_json::json!({
                        "error": { "code": "LlmError", "message": e.to_string() },
                    });
                    yield Ok(Event::default().data(payload.to_string()));
                }
            }
        }
        yield Ok(Event::default().data("[DONE]"));
    };

    Ok(Sse::new(sse).keep_alive(KeepAlive::default()))
}

/// 把 OpenAI 兼容的 JSON message 转换为 LLMManager 内部 Message
fn message_from_value(v: Value) -> Result<Message, ApiError> {
    let role = v
        .get("role")
        .and_then(|r| r.as_str())
        .ok_or_else(|| ApiError::BadRequest("消息缺少 role 字段".to_string()))?;
    let content = v
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| ApiError::BadRequest("消息缺少 content 字段".to_string()))?
        .to_string();

    match role {
        "system" => Ok(Message::system(&content)),
        "user" => Ok(Message::user(&content)),
        "assistant" => Ok(Message::assistant(&content)),
        other => Err(ApiError::BadRequest(format!(
            "不支持的消息角色：{}",
            other
        ))),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
}