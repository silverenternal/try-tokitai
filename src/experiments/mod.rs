//! 实验框架模块
//!
//! 提供基准测试、日志记录和结果分析功能

pub mod logger;
pub mod benchmark;
pub mod analysis;

pub use logger::{ExperimentLogger, TaskExecutionLog, EvolutionCycleLog, ExperimentReport};
pub use benchmark::BenchmarkRunner;
pub use analysis::ResultAnalyzer;
