//! 路由模块索引
//!
//! 每个子文件实现一类资源端点；通过 `Router::merge` 挂到顶层。

pub mod autonomy;
pub mod chat;
pub mod cli;
pub mod context;
pub mod dialogue;
pub mod health;
pub mod mcp;
pub mod orchestrator;
pub mod providers;
pub mod sessions;
pub mod tool_market;
pub mod tools;
pub mod workflows;
