//! 可观测性模块
//!
//! 实现全链路追踪、指标采集和回放功能

pub mod tracing;

pub use tracing::{TraceSpan, TracingRecorder, TraceContext};
