//! 路由模块索引
//!
//! 每个子文件实现一类资源端点；通过 `Router::merge` 挂到顶层。

pub mod chat;
pub mod dialogue;
pub mod health;
pub mod orchestrator;
pub mod providers;
pub mod sessions;
pub mod tools;
pub mod workflows;
