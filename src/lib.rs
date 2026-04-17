//! AI Assistant Library
//!
//! 基于 Tokitai 的自进化 AI 助手框架
//!
//! ## 核心模块
//!
//! - `autonomy`: 自主进化系统（HybridGapDetector, Prompt Engineering）
//! - `tool_matrix`: 工具矩阵和服务注册表
//! - `orchestrator`: 编排调度系统
//! - `tools`: 工具集合
//!
//! ## 依赖 Crates
//!
//! - `tokitai-context`: Git 风格的平行上下文管理系统（独立 crate）

// 内部模块（不公开）
mod command_resolver;
mod config;
mod path_resolver;
mod sandbox;
mod tools;
// Context is now a separate crate: tokitai-context
pub use tokitai_context as context; // Re-export for backward compatibility
mod assistant_common;
mod autonomous_assistant;
pub mod autonomy;
mod cli_assistant;
mod dialogue;
pub mod experiments;
mod external_process;
mod integration;
pub mod llm;
pub mod mcp;
mod observability;
mod orchestrator;
mod prompt_engineering;
mod provider_config;
pub mod tool_market;
mod tool_matrix;
pub mod tui;

// 重新导出常用类型
pub use assistant_common::AssistantConfig;
pub use autonomous_assistant::AutonomousAssistant;
pub use autonomy::gap_detector::{GapType, TaskExecutionRecord, ToolGap};
pub use autonomy::hybrid_gap_detector::{HybridConfig, HybridGapDetector, HybridToolGap};
pub use cli_assistant::CliAssistant;

// 重新导出 parallel context 类型（用于集成测试和论文实验）
// Now re-exported from tokitai-context crate
pub use tokitai_context::{
    BranchDiff, BranchMetadata, BranchState, ContextBranch, ContextGraph, ContextGraphManager,
    CowConfig, CowManager, CowStats, ForkResult, MergeResult, MergeStrategy, Merger,
    ParallelContextManager, ParallelContextManagerConfig,
};

// 重新导出 orchestrator 公共类型（用于集成测试）
pub use orchestrator::{
    AgentRole, DeclarativeWorkflow, DeclarativeWorkflowStep, ErrorHandler, ErrorStrategy,
    Orchestrator, RetryConfig, Stage, StageStatus, Step, StepStatus, Workflow, WorkflowEngine,
    WorkflowStatus,
};

// 重新导出 experiments 公共类型（用于测试和论文实验）
pub use experiments::collector::DataCollector;
pub use experiments::ExperimentGroup;

// 测试工具模块（仅在测试时可用）
#[cfg(test)]
pub mod test_utils;
