//! TUI (Terminal User Interface) 模块
//!
//! 使用 ratatui 实现的图形化终端界面
//!
//! ## 功能
//! - 多面板布局（工具列表、对话区、上下文）
//! - 实时状态显示
//! - 快捷键系统

pub mod app;
pub mod components;
pub mod layout;

pub use app::{run_tui, TuiApp};
pub use components::{
    ChatMessage, ChatPanel, ChatState, StatusBar, StatusBarState, ToolItem, ToolListPanel,
    ToolListState,
};
pub use layout::TuiLayout;
