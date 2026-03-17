//! 编排器模块
//!
//! 协调提示词工程和工具矩阵，实现角色切换和上下文优化
//!
//! ## 模块结构
//! - `role_switcher`: 角色切换器
//! - `context_optimizer`: 上下文优化器
//! - `workflow`: 工作流程引擎（声明式工作流定义）
//! - `workflow_loader`: TOML 工作流加载器
//! - `orchestrator`: 统一编排器入口
//! - `intervention_protocol`: 用户干预协议（PEND-003）

pub mod context_optimizer;
pub mod role_switcher;
pub mod workflow;
pub mod workflow_loader;
pub mod orchestrator;
pub mod intervention_protocol;

pub use orchestrator::Orchestrator;
pub use role_switcher::{AgentRole, RoleSwitcher, RoleSwitchResult};
pub use context_optimizer::{ContextMessage, ContextOptimizer, MessageType, OptimizerConfig};
#[allow(unused_imports)]
pub use workflow::{Workflow, WorkflowEngine, templates};
