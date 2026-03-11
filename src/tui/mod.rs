//! TUI 应用模块
//!
//! 架构说明：
//! - app.rs: App 状态 + 业务逻辑
//! - ui.rs: 纯渲染逻辑（无状态）
//! - event.rs: 事件处理
//! - api_client.rs: API 客户端（连接池 + 流式 + 缓存）
//! - assistant.rs: AI 助手（整合 tokitai 工具调用）

pub mod app;
pub mod event;
pub mod ui;
pub mod api_client;
pub mod assistant;

pub use ui::run_tui;
