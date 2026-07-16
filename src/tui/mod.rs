//! TUI (Terminal User Interface) module
//!
//! Claude Code-style single-panel chat interface with streaming LLM support.
//!
//! ## Features
//! - Full-screen chat panel with virtual scrolling
//! - Rich text input with history, cursor, editing
//! - Async LLM streaming via tokio channels
//! - Inline tool call visualization
//! - Slash command system
//! - Permission dialog for tool calls
//! - Status bar with model/token info

pub mod agent_loader;
pub mod app;
pub mod commands;
pub mod components;
pub mod event;
pub mod layout;
pub mod model_config;
pub mod privacy_guard;
pub mod research_pipeline;
pub mod research_workspace;
pub mod scientist_tools;
pub mod session;
pub mod streaming;

pub use app::{run_tui, AppMode, ThinkingLevel, TuiApp};
pub use commands::{CommandRegistry, CommandResult};
pub use components::{
    ChatPanel, ConfigField, ConfigScreen, ConfigScreenState, InputBar, InputBarState, MessageBlock,
    PendingToolCall, PermissionAction, PermissionDialog, SecurityLevelChoice, StatusBar,
    StatusBarState, ThinkingBlock, ToolCallBlock, ToolCallStatus, ToolResultBlock,
};
pub use event::AppEvent;
pub use layout::TuiLayout;
pub use session::{SessionManager, SessionMeta};
pub use streaming::{build_conversation, is_tool_call_finish, start_llm_stream};
