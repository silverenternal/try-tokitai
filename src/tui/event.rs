//! Application-level events for TUI ↔ LLM communication
//!
//! Events flow from the async LLM streaming task to the synchronous TUI render loop
//! via a tokio mpsc unbounded channel.

use crate::llm::Usage;
use serde_json::Value;

/// Events emitted from background tasks to the TUI main loop
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A streaming token chunk from the LLM
    StreamChunk {
        /// Text content delta
        content: String,
        /// Accumulated tool calls (if any), updated with each delta
        tool_calls: Option<Vec<Value>>,
        /// Finish reason from the API ("stop", "tool_calls", "length", etc.)
        finish_reason: Option<String>,
        /// Token usage (only present in final chunk from non-streaming)
        usage: Option<Usage>,
    },
    /// The LLM stream has completed
    StreamComplete,
    /// An error occurred during streaming
    StreamError(String),
    /// A tool execution result
    ToolResult {
        /// Tool call ID
        call_id: String,
        /// Result text from tool execution
        result: String,
        /// Whether the tool executed successfully
        success: bool,
    },
}
