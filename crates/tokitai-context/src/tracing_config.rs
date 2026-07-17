//! Tracing Event Classification System
//!
//! This module provides structured tracing targets and log levels for production observability.
//! All tracing events are categorized by component and severity for easy filtering.
//!
//! # Usage
//!
//! ```rust,no_run
//! use tokitai_context::tracing_config::init_tracing;
//!
//! // Initialize with default filters
//! init_tracing(None)?;
//!
//! // Or customize filters
//! let env_filter = "tokitai::storage=debug,tokitai::merge=info,warn";
//! init_tracing(Some(env_filter))?;
//! ```
//!
//! # Tracing Targets
//!
//! - `tokitai::storage` - FileKV operations (put, get, delete)
//! - `tokitai::merge` - Merge operations (compaction, segment merge)
//! - `tokitai::cache` - Cache operations (block cache, bloom filter)
//! - `tokitai::wal` - Write-Ahead Log operations
//! - `tokitai::index` - Index operations (sparse index, hash index)
//! - `tokitai::branch` - Branch management operations
//! - `tokitai::facade` - High-level API calls
//! - `tokitai::error` - Error events (always logged at error level)

use std::path::Path;
use tracing_subscriber::{
    fmt,
    prelude::*,
    EnvFilter,
    filter::Targets,
};
use anyhow::{Context, Result};

/// Tracing event categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingTarget {
    /// FileKV storage operations
    Storage,
    /// Merge and compaction operations
    Merge,
    /// Cache operations (block cache, bloom filter, ARC)
    Cache,
    /// Write-Ahead Log operations
    Wal,
    /// Index operations (sparse index, hash index)
    Index,
    /// Branch management
    Branch,
    /// High-level facade API calls
    Facade,
    /// Error events
    Error,
    /// Performance metrics
    Metrics,
    /// General application events
    General,
}

impl TracingTarget {
    /// Get the tracing target string
    pub fn as_str(&self) -> &'static str {
        match self {
            TracingTarget::Storage => "tokitai::storage",
            TracingTarget::Merge => "tokitai::merge",
            TracingTarget::Cache => "tokitai::cache",
            TracingTarget::Wal => "tokitai::wal",
            TracingTarget::Index => "tokitai::index",
            TracingTarget::Branch => "tokitai::branch",
            TracingTarget::Facade => "tokitai::facade",
            TracingTarget::Error => "tokitai::error",
            TracingTarget::Metrics => "tokitai::metrics",
            TracingTarget::General => "tokitai::general",
        }
    }

    /// Get default log level for this target
    pub fn default_level(&self) -> tracing::Level {
        match self {
            TracingTarget::Storage => tracing::Level::INFO,
            TracingTarget::Merge => tracing::Level::INFO,
            TracingTarget::Cache => tracing::Level::WARN,
            TracingTarget::Wal => tracing::Level::INFO,
            TracingTarget::Index => tracing::Level::INFO,
            TracingTarget::Branch => tracing::Level::INFO,
            TracingTarget::Facade => tracing::Level::INFO,
            TracingTarget::Error => tracing::Level::ERROR,
            TracingTarget::Metrics => tracing::Level::INFO,
            TracingTarget::General => tracing::Level::INFO,
        }
    }
}

/// Initialize tracing subscriber with optional environment filter
///
/// # Arguments
///
/// * `env_filter` - Optional filter string (e.g., "tokitai::storage=debug,tokitai::merge=info")
///   If None, uses default levels from TracingTarget
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err` if subscriber initialization fails
pub fn init_tracing<P: AsRef<Path>>(env_filter: Option<&str>) -> Result<()> {
    let log_dir = env_filter
        .map(|_| Path::new("./logs"))
        .unwrap_or_else(|| Path::new("./logs"));

    // Create log directory
    std::fs::create_dir_all(log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    // Build filter
    let filter = if let Some(filter_str) = env_filter {
        EnvFilter::try_new(filter_str)
            .with_context(|| format!("Invalid env filter: {}", filter_str))?
    } else {
        // Use default levels
        let mut filter = EnvFilter::new("");
        for target in &[
            TracingTarget::Storage,
            TracingTarget::Merge,
            TracingTarget::Cache,
            TracingTarget::Wal,
            TracingTarget::Index,
            TracingTarget::Branch,
            TracingTarget::Facade,
            TracingTarget::Error,
            TracingTarget::Metrics,
            TracingTarget::General,
        ] {
            filter = filter.add_directive(
                format!("{}={}", target.as_str(), target.default_level())
                    .parse()
                    .with_context(|| "Failed to parse directive")?,
            );
        }
        filter
    };

    // Create formatting layer for stdout
    let fmt_layer = fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_level(true)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stdout);

    // Create file layer for persistent logs
    let file_appender = tracing_appender::rolling::daily(log_dir, "tokitai.log");
    let file_layer = fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_level(true)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(file_appender);

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(file_layer)
        .try_init()
        .with_context(|| "Failed to initialize tracing subscriber")?;

    tracing::info!(target: "tokitai::general", "Tracing initialized");

    Ok(())
}

/// Initialize minimal tracing for production (stdout only, no file)
pub fn init_tracing_minimal() -> Result<()> {
    let filter = EnvFilter::new("tokitai::storage=warn,tokitai::error=error,tokitai::facade=info");

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_level(true))
        .try_init()
        .with_context(|| "Failed to initialize minimal tracing")?;

    Ok(())
}

/// Initialize tracing with JSON output for log aggregation systems
pub fn init_tracing_json<P: AsRef<Path>>(log_dir: P) -> Result<()> {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_appender = tracing_appender::rolling::daily(log_dir, "tokitai.json");

    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(file_appender);

    tracing_subscriber::registry()
        .with(filter)
        .with(json_layer)
        .try_init()
        .with_context(|| "Failed to initialize JSON tracing")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_target_strings() {
        assert_eq!(TracingTarget::Storage.as_str(), "tokitai::storage");
        assert_eq!(TracingTarget::Merge.as_str(), "tokitai::merge");
        assert_eq!(TracingTarget::Cache.as_str(), "tokitai::cache");
        assert_eq!(TracingTarget::Wal.as_str(), "tokitai::wal");
        assert_eq!(TracingTarget::Index.as_str(), "tokitai::index");
        assert_eq!(TracingTarget::Branch.as_str(), "tokitai::branch");
        assert_eq!(TracingTarget::Facade.as_str(), "tokitai::facade");
        assert_eq!(TracingTarget::Error.as_str(), "tokitai::error");
        assert_eq!(TracingTarget::Metrics.as_str(), "tokitai::metrics");
        assert_eq!(TracingTarget::General.as_str(), "tokitai::general");
    }

    #[test]
    fn test_default_levels() {
        // Error target should always be ERROR level
        assert_eq!(TracingTarget::Error.default_level(), tracing::Level::ERROR);
        
        // Cache warnings should be visible by default
        assert_eq!(TracingTarget::Cache.default_level(), tracing::Level::WARN);
        
        // Other targets default to INFO
        assert_eq!(TracingTarget::Storage.default_level(), tracing::Level::INFO);
        assert_eq!(TracingTarget::Merge.default_level(), tracing::Level::INFO);
    }
}
