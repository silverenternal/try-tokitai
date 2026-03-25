//! AI Assistant Library
//! 
//! 基于 Tokitai 的自进化 AI 助手框架
//! 
//! ## 核心模块
//! 
//! - `autonomy`: 自主进化系统（HybridGapDetector, Prompt Engineering）
//! - `context`: 上下文存储管理
//! - `tool_matrix`: 工具矩阵和服务注册表
//! - `orchestrator`: 编排调度系统
//! - `tools`: 工具集合

// 内部模块（不公开）
mod config;
mod command_resolver;
mod path_resolver;
mod sandbox;
mod tools;
mod context;
mod autonomy;
mod observability;
mod dialogue;
mod prompt_engineering;
mod tool_matrix;
mod orchestrator;
mod integration;
mod provider_config;
mod external_process;
mod assistant_common;
mod cli_assistant;
mod autonomous_assistant;
mod experiments;
pub mod llm;
pub mod mcp;
pub mod tool_market;
pub mod tui;

// 重新导出常用类型
pub use autonomy::hybrid_gap_detector::{HybridGapDetector, HybridConfig, HybridToolGap};
pub use autonomy::gap_detector::{TaskExecutionRecord, ToolGap, GapType};
pub use assistant_common::AssistantConfig;
pub use cli_assistant::CliAssistant;
pub use autonomous_assistant::AutonomousAssistant;

// 重新导出 orchestrator 公共类型（用于集成测试）
pub use orchestrator::{
    Workflow, WorkflowEngine, Stage, Step, StepStatus, StageStatus, WorkflowStatus,
    DeclarativeWorkflow, DeclarativeWorkflowStep, RetryConfig,
    ErrorHandler, ErrorStrategy, AgentRole, Orchestrator,
};
