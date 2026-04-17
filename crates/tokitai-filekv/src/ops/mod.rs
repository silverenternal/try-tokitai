//! Operations and observability components
//!
//! This module contains operational features:
//! - Feature flags for runtime feature control
//! - Prometheus metrics export
//! - Async I/O support
//! - Audit logging
//! - Timeout control
//! - Memory tracking
//! - Adaptive preallocation
//! - Write amplification analysis

pub mod amplification;
#[cfg(feature = "async-io")]
pub mod async_io;
pub mod audit_log;
pub mod feature_flag;
#[cfg(test)]
pub mod feature_flag_tests;
pub mod memory_tracker;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod perf_tracker;
pub mod preallocator;
pub mod timeout_control;

// Re-exports for convenience
pub use amplification::{AmplificationReport, AmplificationStats, AmplificationTracker, WriteAmplificationAnalyzer};
#[cfg(feature = "async-io")]
pub use async_io::{AsyncIoConfig, AsyncIoStats, AsyncWriteOp, AsyncWriteResult, AsyncWriter};
pub use audit_log::{AuditEntry, AuditLogConfig, AuditLogStats, AuditLogger, AuditOperation};
pub use feature_flag::{
    FeatureFlag, FeatureFlagController, FeatureFlagStats, FeatureReport, FeatureState, FeatureStateChange,
};
pub use memory_tracker::{MemoryTracker, MemoryUsage};
#[cfg(feature = "metrics")]
pub use metrics::FileKVMetrics;
pub use perf_tracker::{format_ns, ModuleTiming, PerfSnapshot, PerfTimer, PerfTracker};
pub use preallocator::{
    AdaptivePreallocator, AdaptivePreallocatorConfig, PreallocatorStats, SharedAdaptivePreallocator,
};
pub use timeout_control::{TimeoutConfig, TimeoutStats};
