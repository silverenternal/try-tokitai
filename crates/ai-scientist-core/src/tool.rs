//! Tool trait and supporting types
//!
//! Defines the `Tool` trait that all AI Scientist tools implement.
//! This is complementary to the `tokitai` `#[tool]` macro — it provides
//! the runtime trait for dynamic dispatch, while tokitai handles
//! compile-time definition generation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// ToolType
// ============================================================================

/// Classification of tools by domain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    /// Literature/search tools
    Literature,
    /// Data processing tools
    Data,
    /// Computation tools (sympy, numpy, etc.)
    Computation,
    /// Visualization tools
    Visualization,
    /// Verification tools (Lean4, model checking)
    Verification,
    /// Simulation tools
    Simulation,
    /// Chemistry tools
    Chemistry,
    /// Biology tools
    Biology,
    /// File/IO tools
    FileSystem,
    /// Network tools
    Network,
    /// Generic/utility tools
    Generic,
}

// ============================================================================
// ToolParameter
// ============================================================================

/// Description of a tool parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema type
    pub param_type: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value (JSON)
    pub default: Option<serde_json::Value>,
    /// Enum of allowed values
    pub enum_values: Option<Vec<String>>,
}

// ============================================================================
// ToolDefinition
// ============================================================================

/// Complete definition of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Tool type category
    pub tool_type: ToolType,
    /// Version string
    pub version: String,
    /// Parameters
    pub parameters: Vec<ToolParameter>,
    /// JSON Schema for LLM function calling
    pub input_schema: String,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Whether tool requires human approval
    pub requires_approval: bool,
}

// ============================================================================
// RiskLevel
// ============================================================================

/// Risk classification for tools
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Read-only, no side effects (e.g., search, read file)
    Safe,
    /// May have side effects but limited blast radius (e.g., compute, plot)
    Low,
    /// Significant side effects (e.g., write file, install package)
    Moderate,
    /// Destructive potential (e.g., delete, execute arbitrary code)
    High,
    /// Requires explicit human approval regardless of policy
    Critical,
}

// ============================================================================
// ToolResult
// ============================================================================

/// Structured result from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool name
    pub tool: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Result data (structured JSON)
    pub data: serde_json::Value,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ToolResult {
    /// Create a successful result
    pub fn ok(tool: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            tool: tool.into(),
            success: true,
            data,
            error: None,
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create an error result
    pub fn err(tool: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            success: false,
            data: serde_json::Value::Null,
            error: Some(error.into()),
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// ToolError
// ============================================================================

/// Errors that can occur during tool execution
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool '{0}' not found")]
    NotFound(String),

    #[error("Invalid arguments for tool '{tool}': {message}")]
    InvalidArgs { tool: String, message: String },

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Internal error: {0}")]
    Internal(String),
}

// ============================================================================
// Tool Trait
// ============================================================================

/// Runtime trait for dynamically dispatched tools.
///
/// This is the runtime counterpart to tokitai's compile-time `#[tool]` macro.
/// Tools that need dynamic dispatch (loaded at runtime, swapped implementations)
/// implement this trait. Static tools can use the `#[tool]` macro directly.
///
/// # Example
///
/// ```rust,ignore
/// struct PaperSearchTool;
///
/// #[async_trait]
/// impl Tool for PaperSearchTool {
///     fn definition(&self) -> ToolDefinition { ... }
///     async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, ToolError> {
///         // Call arXiv API, Semantic Scholar, etc.
///         Ok(ToolResult::ok("search_paper", json!({"papers": [...]})))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get this tool's definition (name, parameters, schema, etc.)
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with given arguments.
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, ToolError>;

    /// Validate arguments against the tool's parameter definitions.
    fn validate_args(&self, args: &serde_json::Value) -> Result<(), ToolError> {
        let def = self.definition();
        for param in &def.parameters {
            if param.required && args.get(&param.name).is_none() {
                return Err(ToolError::InvalidArgs {
                    tool: def.name.clone(),
                    message: format!("Missing required parameter: {}", param.name),
                });
            }
        }
        Ok(())
    }

    /// Whether this tool requires human approval before execution.
    fn requires_approval(&self) -> bool {
        self.definition().requires_approval
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "test_tool".into(),
                description: "A test tool".into(),
                tool_type: ToolType::Generic,
                version: "0.1.0".into(),
                parameters: vec![ToolParameter {
                    name: "input".into(),
                    description: "Input value".into(),
                    param_type: "string".into(),
                    required: true,
                    default: None,
                    enum_values: None,
                }],
                input_schema: r#"{"type":"object","properties":{"input":{"type":"string"}},"required":["input"]}"#.into(),
                risk_level: RiskLevel::Safe,
                requires_approval: false,
            }
        }

        async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::ok("test_tool", args))
        }
    }

    #[tokio::test]
    async fn test_tool_definition() {
        let tool = TestTool;
        let def = tool.definition();
        assert_eq!(def.name, "test_tool");
        assert_eq!(def.parameters.len(), 1);
    }

    #[tokio::test]
    async fn test_tool_execute() {
        let tool = TestTool;
        let result = tool
            .execute(serde_json::json!({"input": "hello"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.tool, "test_tool");
    }

    #[tokio::test]
    async fn test_validate_args_missing_required() {
        let tool = TestTool;
        let err = tool.validate_args(&serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("input"));
    }
}
