//! LLM streaming bridge
//!
//! Bridges the async LLM streaming world with the synchronous TUI render loop.
//! Spawns a tokio task that calls `provider.chat_stream()` and forwards chunks
//! via an mpsc channel. Returns an `AbortHandle` for cancellation.

use crate::llm::{ChatRequest, LLMProvider};
use crate::tui::event::AppEvent;
use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

/// Start an LLM streaming task.
///
/// Returns an `AbortHandle` that can be used to cancel the stream (e.g., on Ctrl+C).
pub fn start_llm_stream(
    provider: Arc<dyn LLMProvider>,
    request: ChatRequest,
    tx: UnboundedSender<AppEvent>,
    runtime: &tokio::runtime::Runtime,
) -> AbortHandle {
    runtime.spawn(async move {
        let result = stream_llm_response(provider, request, tx.clone()).await;
        match result {
            Ok(()) => {
                let _ = tx.send(AppEvent::StreamComplete);
            }
            Err(e) => {
                // If the channel is closed, the receiver has been dropped — no need to send
                let _ = tx.send(AppEvent::StreamError(e.to_string()));
            }
        }
    }).abort_handle()
}

/// Core streaming logic: calls the provider and forwards AppEvents.
/// Tool call deltas are already accumulated by the provider, so we
/// just forward each chunk as-is.
async fn stream_llm_response(
    provider: Arc<dyn LLMProvider>,
    request: ChatRequest,
    tx: UnboundedSender<AppEvent>,
) -> Result<()> {
    let mut stream = provider.chat_stream(request).await?;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if tx.send(AppEvent::StreamChunk {
                    content: chunk.content,
                    tool_calls: chunk.tool_calls,
                    finish_reason: chunk.finish_reason,
                    usage: chunk.usage,
                }).is_err() {
                    // Receiver dropped — stop streaming
                    break;
                }
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError(e.to_string()));
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Build an API message history from user/friendlier types.
///
/// IMPORTANT: When the LLM makes tool calls, the text and all tool_calls
/// from a single assistant turn must be in ONE message.
pub fn build_conversation(
    messages: &[crate::tui::components::message_block::MessageBlock],
    agent_prompt: Option<&str>,
) -> Vec<crate::llm::Message> {
    use crate::llm::Message;
    use crate::tui::components::message_block::MessageBlock;

    let mut result: Vec<Message> = Vec::new();

    // System message: use agent prompt if available, otherwise default
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let sys_prompt = if let Some(prompt) = agent_prompt {
        prompt.replace("{cwd}", &cwd)
    } else {
        format!(
            "You are a helpful AI assistant with access to tools.\n\
             Current working directory: {}\n\
             When reading or writing files, use paths relative to this directory.",
            cwd
        )
    };
    result.push(Message::system(&sys_prompt));

    for block in messages {
        match block {
            MessageBlock::User { content, .. } => {
                result.push(Message::user(content));
            }
            MessageBlock::Assistant { content } => {
                // Check if previous message is also an assistant message (with tool_calls)
                // If so, merge this content into it as a prefix
                if let Some(last) = result.last_mut() {
                    if last.role == "assistant" && last.tool_calls.is_some() {
                        // Prepend this text to the existing assistant message
                        last.content = format!("{}\n{}", content, last.content);
                        continue;
                    }
                }
                result.push(Message::assistant(content));
            }
            MessageBlock::ToolCall { name, args, call_id, .. } => {
                // Tool calls must merge with the PREVIOUS assistant message
                // (the text before tool calls is in a preceding Assistant block)
                let tc_json = serde_json::json!({
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(args).unwrap_or_default(),
                    }
                });

                if let Some(last) = result.last_mut() {
                    if last.role == "assistant" {
                        // Merge into existing assistant message
                        let mut tcs = last.tool_calls.take().unwrap_or_default();
                        tcs.push(tc_json);
                        last.tool_calls = Some(tcs);
                        // Preserve existing content, set to empty only if not already set
                        if last.content.is_empty() {
                            // This is OK - the assistant text content comes from the
                            // preceding Assistant block which merges above
                        }
                        continue;
                    }
                }

                // No previous assistant message - create standalone
                result.push(Message {
                    role: "assistant".to_string(),
                    content: String::new(),
                    name: None,
                    tool_calls: Some(vec![tc_json]),
                    tool_call_id: None,
                });
            }
            MessageBlock::ToolResult { call_id, result: tool_result, success: _ } => {
                result.push(Message {
                    role: "tool".to_string(),
                    content: tool_result.clone(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(call_id.clone()),
                });
            }
            // Skip streaming, diff, error, system, and thinking blocks in API history
            _ => {}
        }
    }

    result
}

/// Get the current streaming buffer text from messages
pub fn get_streaming_text(messages: &[crate::tui::components::message_block::MessageBlock]) -> String {
    use crate::tui::components::message_block::MessageBlock;
    for block in messages.iter().rev() {
        if let MessageBlock::AssistantStreaming { content } = block {
            return content.clone();
        }
    }
    String::new()
}

/// Check if a finish reason indicates tool calls should be extracted
pub fn is_tool_call_finish(finish_reason: &Option<String>) -> bool {
    matches!(finish_reason.as_deref(), Some("tool_calls") | Some("function_call"))
}
