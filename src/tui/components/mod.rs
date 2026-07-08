//! TUI visual components

pub mod chat_panel;
pub mod config_screen;
pub mod conversation_graph;
pub mod diff_viewer;
pub mod input_bar;
pub mod logo;
pub mod message_block;
pub mod permission_dialog;
pub mod reviewer_panel;
pub mod status_bar;
pub mod thinking_block;
pub mod tool_block;

pub use chat_panel::ChatPanel;
pub use config_screen::{ConfigField, ConfigScreen, ConfigScreenState, SecurityLevelChoice};
pub use conversation_graph::render_graph;
pub use diff_viewer::{DiffLine, FileDiff};
pub use input_bar::{InputBar, InputBarState};
pub use message_block::{MessageBlock, ToolCallStatus};
pub use permission_dialog::{PendingToolCall, PermissionAction, PermissionDialog};
pub use reviewer_panel::ReviewerPanel;
pub use status_bar::{StatusBar, StatusBarState};
pub use thinking_block::ThinkingBlock;
pub use tool_block::{ToolCallBlock, ToolResultBlock};
