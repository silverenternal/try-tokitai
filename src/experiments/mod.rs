//! 实验数据收集模块
//!
//! 用于收集和记录 HybridGapDetector 和 Prompt Engineering 自进化系统的性能指标和实验数据
//!
//! ## 使用示例
//! ```rust,ignore
//! use crate::experiments::{ExperimentLogger, TaskExecutionLog};
//!
//! let logger = ExperimentLogger::new(
//!     &std::path::PathBuf::from("experiments/logs/ours-full"),
//!     "Ours-Full",
//!     "exp_001"
//! ).unwrap();
//! ```

pub mod collector;
pub mod logger;

#[allow(unused_imports)]
pub use logger::{ExperimentLogger, TaskExecutionLog, SelfEvolutionLog, ExperimentSummary};
