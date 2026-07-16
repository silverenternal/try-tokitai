//! External Process Wrapper (EPW) Module
//!
//! This module provides a unified interface for wrapping external processes,
//! HTTP services, and scripts as tokitai tools that can be discovered and
//! invoked by AI agents.
//!
//! ## Overview
//! The External Process Wrapper system extends the tokitai tool ecosystem
//! beyond Rust code, allowing AI agents to:
//! - Call local CLI tools (git, docker, npm, etc.)
//! - Invoke remote HTTP services (REST APIs, webhooks)
//! - Execute script files (.sh, .py, .js)
//!
//! ## Architecture
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         AI Dispatch Layer               │
//! │  (SelfImprovementLoop, Orchestrator)    │
//! ├─────────────────────────────────────────┤
//! │         Tool Matrix Layer               │
//! │  (ToolMatrix, ToolSelector, Registry)   │
//! ├─────────────────────────────────────────┤
//! │      EPW Wrapper Layer (This Module)    │
//! │  ┌──────────┬──────────┬──────────┐    │
//! │  │ Process  │   HTTP   │  Script  │    │
//! │  │ Wrapper  │  Wrapper │  Wrapper │    │
//! │  └──────────┴──────────┴──────────┘    │
//! ├─────────────────────────────────────────┤
//! │      External Execution Layer           │
//! │  (CLI Tools, HTTP APIs, Scripts)        │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Core Components
//! - [`metadata`]: Data structures for tool metadata and configuration
//! - [`wrapper`]: Core `ExternalTool` trait definition
//! - [`process_wrapper`]: Implementation for local executables
//! - [`http_wrapper`]: Implementation for HTTP services
//! - [`script_wrapper`]: Implementation for script files
//! - [`discovery`]: Auto-discovery of external tools
//! - [`registry`]: External tool registry
//! - [`orchestration`]: Tool composition and workflow orchestration
//!
//! ## Quick Start
//!
//! ### Wrapping a CLI Tool
//! ```rust,ignore
//! use crate::external_process::metadata::{ExternalToolMetadata, ExternalToolType, ProcessConfig};
//! use crate::external_process::process_wrapper::ProcessWrapper;
//!
//! // Create process configuration
//! let config = ProcessConfig::new("git")
//!     .with_args(vec!["commit".to_string(), "-m".to_string(), "{{message}}".to_string()]);
//!
//! // Create metadata with input schema
//! let metadata = ExternalToolMetadata::new(
//!     "git_commit",
//!     "Commit changes to Git repository",
//!     ExternalToolType::process(config),
//!     serde_json::json!({
//!         "type": "object",
//!         "properties": {
//!             "message": {"type": "string", "description": "Commit message"}
//!         },
//!         "required": ["message"]
//!     }),
//!     "version_control",
//!     "ai_agent",
//! );
//!
//! // Create wrapper
//! let wrapper = ProcessWrapper::new(metadata);
//!
//! // Execute
//! let result = wrapper.execute(serde_json::json!({
//!     "message": "Initial commit"
//! })).await?;
//! ```
//!
//! ### Using the Builder Pattern
//! ```rust,ignore
//! use crate::external_process::process_wrapper::ProcessWrapperBuilder;
//! use crate::external_process::metadata::{RiskLevel, schema_helpers};
//!
//! let wrapper = ProcessWrapperBuilder::new("git_commit", "git")
//!     .description("Commit changes to Git repository")
//!     .args(vec!["commit".to_string(), "-m".to_string(), "{{message}}".to_string()])
//!     .input_schema(schema_helpers::create_string_params_schema(vec![
//!         ("message", "Commit message", true),
//!     ]))
//!     .domain("version_control")
//!     .tag("git")
//!     .tag("commit")
//!     .risk_level(RiskLevel::Medium)
//!     .build();
//! ```
//!
//! ### Registering to Tool Matrix
//! ```rust,ignore
//! use crate::external_process::ExternalTool;
//! use crate::tool_matrix::registry::ToolRegistry;
//!
//! let wrapper = create_git_commit_wrapper();
//! let tool_def = wrapper.to_tool_definition();
//!
//! let registry = ToolRegistry::new();
//! registry.register_tool(tool_def)?;
//! ```
//!
//! ## Implementation Status
//! - [x] Core trait definition (`ExternalTool`)
//! - [x] Metadata structures
//! - [x] Process wrapper implementation
//! - [ ] HTTP wrapper implementation
//! - [ ] Script wrapper implementation
//! - [ ] Auto-discovery system
//! - [ ] External tool registry
//! - [ ] Self-improvement loop integration
//!
//! ## Safety and Security
//! External tools can pose security risks. This module provides:
//! - **Risk Levels**: Classify tools by risk (Low, Medium, High, Critical)
//! - **Timeout Handling**: Prevent hanging processes
//! - **Environment Isolation**: Control environment variables
//! - **Working Directory**: Restrict execution context
//!
//! Always validate and sanitize inputs before passing to external tools.

pub mod discovery;
pub mod http_wrapper;
pub mod metadata;
pub mod orchestration;
pub mod process_wrapper;
pub mod registry;
pub mod script_wrapper;
pub mod wrapper;

// Re-export main types for convenience

#[allow(unused_imports)]
pub use discovery::ExternalToolDiscovery;
#[allow(unused_imports)]
pub use registry::ExternalToolRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_process::metadata::{
        ExternalToolMetadata, ExternalToolType, ProcessConfig, RiskLevel,
    };
    use crate::external_process::wrapper::ExternalTool;
    use serde_json::json;

    #[tokio::test]
    async fn test_external_tool_trait_process() {
        use metadata::schema_helpers;
        use process_wrapper::ProcessWrapperBuilder;

        let wrapper = ProcessWrapperBuilder::new("echo_test", "echo")
            .description("Echo test")
            .args(vec!["{{message}}".to_string()])
            .input_schema(schema_helpers::create_string_params_schema(vec![(
                "message",
                "Message to echo",
                true,
            )]))
            .domain("test")
            .build();

        // Test metadata access
        assert_eq!(wrapper.name(), "echo_test");
        assert_eq!(wrapper.domain(), "test");
        assert!(wrapper.is_enabled());

        // Test execution
        let result = wrapper.execute(json!({"message": "Hello"})).await.unwrap();
        assert!(result.success);
        assert!(result.stdout.unwrap().contains("Hello"));

        // Test validation
        let valid_input = json!({"message": "test"});
        assert!(wrapper.validate_input(&valid_input).is_ok());

        let invalid_input = json!({});
        assert!(wrapper.validate_input(&invalid_input).is_err());

        // Test tool definition conversion
        let tool_def = wrapper.to_tool_definition();
        assert_eq!(tool_def.name, "echo_test");
        assert!(!tool_def.description.is_empty());
    }

    #[test]
    fn test_risk_level_default() {
        let risk = RiskLevel::default();
        assert_eq!(risk, RiskLevel::Medium);
    }

    #[test]
    fn test_metadata_creation() {
        let config = ProcessConfig::new("test");
        let metadata = ExternalToolMetadata::new(
            "test_tool",
            "Test tool description",
            ExternalToolType::process(config),
            json!({"type": "object"}),
            "test",
            "test_user",
        );

        assert_eq!(metadata.name, "test_tool");
        assert_eq!(metadata.description, "Test tool description");
        assert!(metadata.enabled);
        assert_eq!(metadata.risk_level, RiskLevel::Medium);
    }
}
