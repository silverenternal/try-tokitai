//! 工具矩阵服务
//!
//! 基于动态工具箱和 Skills 文件的工具管理系统
//!
//! ## 设计理念
//! - **动态工具箱**：按领域分类的工具集合（如文件操作箱、网络工具箱）
//! - **Skills 文件**：每个工具箱的"说明书"，告诉 AI 如何正确使用工具
//! - **自我进化**：AI 发现工具不足时 → 快速开发新工具 → 注册到工具箱 → 更新 Skills 文件
//!
//! ## 模块结构
//! - `matrix`: 工具矩阵和工具箱结构定义
//! - `registry`: 工具注册表，支持运行时注册
//! - `skills_manager`: Skills 文件管理器
//! - `selector`: 动态工具选择器
//!
//! ## 使用示例
//! ```rust,ignore
//! use crate::tool_matrix::{ToolRegistry, ToolBox, ToolDefinition, SkillsManager, ToolSelector};
//!
//! // 创建注册表
//! let registry = ToolRegistry::new();
//!
//! // 创建工具箱
//! let mut file_box = ToolBox::new("file_ops", "File Operations", "File tools");
//! file_box.add_tool(ToolDefinition::new("read_file", "Read a file", "{}"));
//! registry.create_toolbox(file_box).unwrap();
//!
//! // 创建 Skills 管理器
//! let skills_mgr = SkillsManager::default();
//!
//! // 创建工具选择器
//! let selector = ToolSelector::new(registry);
//! let result = selector.select_tools_by_query("read file", 5);
//! ```

pub mod matrix;
pub mod registry;
pub mod skills_manager;
pub mod selector;

