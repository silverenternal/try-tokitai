//! 系统工具模块
//!
//! 提供系统相关的工具函数，分为以下几个子模块：
//!
//! ## 模块划分
//! - `process_manager`: 进程管理（查询、监控）
//! - `system_monitor`: 系统监控（资源、信息）
//! - `system_commands`: 命令执行（安全白名单机制）
//! - `code_analyzer`: 代码分析（行数统计、函数查找）
//! - `backend`: 平台后端抽象（内部使用）
//! - `error`: 错误类型定义
//! - `config`: 统一配置管理
//!
//! ## 架构改进
//! - 职责分离：进程管理、系统监控、命令执行分别独立
//! - 平台抽象：统一 Linux/macOS 接口，消除重复代码
//! - 类型安全：使用枚举错误类型替代 String
//! - JSON 输出：所有工具支持统一的 JSON 格式便于 LLM 解析
//! - 安全增强：白名单机制、敏感信息过滤、TOCTOU 修复
//! - 配置管理：统一常量配置，便于调整

pub mod backend;
pub mod code_analyzer;
pub mod config;
pub mod error;
pub mod process_manager;
pub mod system_commands;
pub mod system_monitor;

// 重新导出主要工具类型
pub use code_analyzer::CodeAnalyzer;
pub use process_manager::ProcessManager;
pub use system_commands::SystemCommands;

// 为向后兼容，保留旧的工具箱名称（内部使用新实现）
/// @deprecated 使用 ProcessManager 替代
pub type ProcessTools = ProcessManager;

/// @deprecated 使用 SystemCommands 替代
pub type SystemTools = SystemCommands;

/// @deprecated 使用 CodeAnalyzer 替代
pub type CodeTools = CodeAnalyzer;
