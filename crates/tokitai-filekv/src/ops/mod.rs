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

pub mod feature_flag;
#[cfg(test)]
pub mod feature_flag_tests;
#[cfg(feature = "metrics")]
pub mod metrics;
#[cfg(feature = "async-io")]
pub mod async_io;
pub mod audit_log;
pub mod timeout_control;
pub mod memory_tracker;
pub mod preallocator;
pub mod amplification;

// Re-exports for convenience
pub use feature_flag::{FeatureFlag, FeatureFlagController, FeatureState, FeatureStateChange, FeatureFlagStats, FeatureReport};
#[cfg(feature = "metrics")]
pub use metrics::FileKVMetrics;
#[cfg(feature = "async-io")]
pub use async_io::{AsyncIoConfig, AsyncIoStats, AsyncWriter, AsyncWriteOp, AsyncWriteResult};
pub use audit_log::{AuditLogConfig, AuditLogger, AuditEntry, AuditOperation, AuditLogStats};
pub use timeout_control::{TimeoutConfig, TimeoutStats};
pub use memory_tracker::{MemoryTracker, MemoryUsage};
pub use preallocator::{AdaptivePreallocator, AdaptivePreallocatorConfig, PreallocatorStats, SharedAdaptivePreallocator};
pub use amplification::{WriteAmplificationAnalyzer, AmplificationReport};
