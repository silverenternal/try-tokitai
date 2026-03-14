//! 自主迭代循环模块
//!
//! 实现 AI 自主的任务分解、规划、执行和审查能力
//!
//! # 架构说明
//!
//! ```text
//! autonomy/
//! ├── task_decomposer.rs    # 任务分解引擎（DAG 依赖分析）
//! ├── iteration_tracker.rs  # 迭代状态追踪器（事件溯源）
//! ├── git_workflow.rs       # 自主 Git 工作流
//! └── agents/
//!     ├── mod.rs            # Agent 系统导出
//!     ├── planner.rs        # 规划 Agent
//!     ├── executor.rs       # 执行 Agent
//!     └── reviewer.rs       # 审查 Agent
//! ```
//!
//! # 设计原则
//! - 纯文件存储，零数据库依赖
//! - 事件溯源，支持回放
//! - 状态机驱动，支持暂停/恢复

pub mod task_decomposer;
pub mod iteration_tracker;
pub mod git_workflow;
pub mod agents;

pub use task_decomposer::{Task, TaskStatus, TaskGraph, TaskDecomposer};
pub use iteration_tracker::{IterationState, IterationTracker, IterationEvent, IterationSession};
pub use git_workflow::GitWorkflow;
pub use agents::{PlannerAgent, ExecutorAgent, ReviewerAgent, AgentCoordinator};
