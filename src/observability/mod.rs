//! 可观测性模块
//!
//! 实现全链路追踪、指标采集和回放功能
//!
//! ## 模块结构
//! - `tracing`: 全链路追踪
//! - `observability_tools`: 可观测性工具
//! - `replay`: 迭代回放系统（PEND-004）
//! - `metrics_dashboard`: 性能指标仪表盘（PEND-005）
//! - `tool_timeline`: 工具调用链可视化（PEND-006）

pub mod tracing;
pub mod observability_tools;
pub mod replay;
pub mod metrics_dashboard;
pub mod tool_timeline;

pub use observability_tools::ObservabilityTools;
