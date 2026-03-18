//! VCS（版本控制系统）工具模块
//!
//! 提供 Git 操作的 AI 微服务封装
//!
//! ## 模块结构
//! - [`git_ops`]: Git 操作工具集
//!
//! ## 使用示例
//! ```rust,ignore
//! use crate::tools::vcs::GitOperations;
//!
//! let git = GitOperations;
//! let status = git.git_status(None)?;
//! ```

pub mod git_ops;

pub use git_ops::GitOperations;
