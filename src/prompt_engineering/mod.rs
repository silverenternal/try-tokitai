//! 提示词工程服务
//!
//! 提供多角色提示词模板管理和渲染功能
//!
//! ## 模块结构
//! - `template`: 提示词模板结构定义
//! - `manager`: 提示词模板管理器
//! - `renderer`: 提示词渲染引擎
//!
//! ## 使用示例
//! ```rust,ignore
//! use crate::prompt_engineering::{PromptTemplateManager, PromptTemplate};
//!
//! let manager = PromptTemplateManager::default();
//! let system_prompt = manager.get_system_prompt("Planner", &json!({
//!     "tools": "read_file, write_file, grep"
//! })).unwrap();
//! ```

pub mod manager;
pub mod prompt_tools;
pub mod renderer;
pub mod template;

pub use manager::PromptTemplateManager;
pub use prompt_tools::PromptTools;
