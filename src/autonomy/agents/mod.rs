//! Agent 系统模块
//!
//! 实现 Planner-Executor-Reviewer 三 Agent 协作架构
//!
//! # 架构说明
//!
//! ```text
//! agents/
//! ├── mod.rs          # 模块导出
//! ├── planner.rs      # 规划 Agent - 制定执行计划
//! ├── executor.rs     # 执行 Agent - 按计划执行任务
//! └── reviewer.rs     # 审查 Agent - 代码审查和质量把关
//! ```
//!
//! # 设计原则
//! - 每个 Agent 职责单一
//! - 通过共享工作区通信
//! - 纯文件存储状态

pub mod planner;
pub mod executor;
pub mod reviewer;
pub mod coordinator;

pub use planner::PlannerAgent;
pub use executor::ExecutorAgent;
pub use reviewer::ReviewerAgent;
pub use coordinator::AgentCoordinator;
