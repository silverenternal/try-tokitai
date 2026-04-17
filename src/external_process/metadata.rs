//! External Process Wrapper - Metadata Definitions
//!
//! Core data structures for external tool metadata and configuration.
//!
//! ## Overview
//! This module defines the metadata structures for external tools including:
//! - `ExternalToolType`: Enum for different tool types (Process, HTTP, Script)
//! - `ExternalToolMetadata`: Complete tool metadata for registration
//! - `ToolExecutionResult`: Standardized execution result structure
//! - `RiskLevel`: Security risk classification

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Risk level for external tool execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RiskLevel {
    /// Low risk: Safe to execute without confirmation
    Low,
    /// Medium risk: Requires logging and monitoring
    #[default]
    Medium,
    /// High risk: Requires user confirmation before execution
    High,
    /// Critical risk: Requires explicit approval and sandboxing
    Critical,
}

/// Configuration for process-based tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    /// Executable path or name
    pub executable: String,
    /// Argument template (supports variable substitution like {{message}})
    pub args_template: Vec<String>,
    /// Working directory for the process
    pub working_dir: Option<PathBuf>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Environment variables
    pub env: HashMap<String, String>,
}

impl ProcessConfig {
    /// Create a new process configuration
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            args_template: Vec::new(),
            working_dir: None,
            timeout_ms: 30000, // Default 30 seconds
            env: HashMap::new(),
        }
    }

    /// Set argument template
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args_template = args;
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Set timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.insert(key, value);
        self
    }
}

/// Authentication configuration for HTTP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// Bearer token authentication
    BearerToken { token_env: String },
    /// API key authentication
    ApiKey {
        header_name: String,
        key_env: String,
    },
    /// Basic authentication
    Basic {
        username_env: String,
        password_env: String,
    },
    /// OAuth 2.0 authentication
    OAuth2 {
        client_id_env: String,
        client_secret_env: String,
        token_url: String,
        scopes: Vec<String>,
    },
}

/// Configuration for HTTP-based tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Base URL for the HTTP service
    pub base_url: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Path template (supports variable substitution)
    pub path_template: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl HttpConfig {
    /// Create a new HTTP configuration
    pub fn new(base_url: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            method: method.into(),
            path_template: String::new(),
            headers: HashMap::new(),
            auth: None,
            timeout_ms: 30000, // Default 30 seconds
        }
    }

    /// Set path template
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path_template = path.into();
        self
    }

    /// Add header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set authentication
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Configuration for script-based tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Path to the script file
    pub script_path: PathBuf,
    /// Interpreter (bash, python3, node, etc.)
    pub interpreter: Option<String>,
    /// Argument template
    pub args_template: Vec<String>,
    /// Working directory for script execution
    pub working_dir: Option<PathBuf>,
}

impl ScriptConfig {
    /// Create a new script configuration
    pub fn new(script_path: PathBuf) -> Self {
        Self {
            script_path,
            interpreter: None,
            args_template: Vec::new(),
            working_dir: None,
        }
    }

    /// Set interpreter
    pub fn with_interpreter(mut self, interpreter: impl Into<String>) -> Self {
        self.interpreter = Some(interpreter.into());
        self
    }

    /// Set argument template
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args_template = args;
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }
}

/// External tool type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalToolType {
    /// Local executable/CLI tool
    Process { config: ProcessConfig },
    /// Remote HTTP service/REST API
    Http { config: HttpConfig },
    /// Script file (.sh, .py, .js, etc.)
    Script { config: ScriptConfig },
}

impl ExternalToolType {
    /// Create a process tool type
    pub fn process(config: ProcessConfig) -> Self {
        ExternalToolType::Process { config }
    }

    /// Create an HTTP tool type
    pub fn http(config: HttpConfig) -> Self {
        ExternalToolType::Http { config }
    }

    /// Create a script tool type
    pub fn script(config: ScriptConfig) -> Self {
        ExternalToolType::Script { config }
    }
}

/// External tool metadata for registration and discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolMetadata {
    /// Tool name (unique identifier)
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool type (Process, HTTP, Script)
    pub tool_type: ExternalToolType,
    /// Input parameter schema (JSON Schema)
    pub input_schema: serde_json::Value,
    /// Output schema (optional)
    pub output_schema: Option<serde_json::Value>,
    /// Domain/Category of the tool
    pub domain: String,
    /// Tags for search and classification
    pub tags: Vec<String>,
    /// Risk level for execution
    pub risk_level: RiskLevel,
    /// Creator (AI Agent or user)
    pub created_by: String,
    /// Creation timestamp
    pub created_at: String,
    /// Whether the tool is enabled
    pub enabled: bool,
}

impl ExternalToolMetadata {
    /// Create new external tool metadata
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        tool_type: ExternalToolType,
        input_schema: serde_json::Value,
        domain: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            tool_type,
            input_schema,
            output_schema: None,
            domain: domain.into(),
            tags: Vec::new(),
            risk_level: RiskLevel::default(),
            created_by: created_by.into(),
            created_at: chrono::Local::now().to_rfc3339(),
            enabled: true,
        }
    }

    /// Set output schema
    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set risk level
    pub fn with_risk_level(mut self, risk: RiskLevel) -> Self {
        self.risk_level = risk;
        self
    }
}

/// Result of external tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Output result (JSON value)
    pub output: serde_json::Value,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Standard output (for process tools)
    pub stdout: Option<String>,
    /// Standard error (for process tools)
    pub stderr: Option<String>,
}

impl ToolExecutionResult {
    /// Create a successful result
    pub fn success(output: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            output,
            error: None,
            execution_time_ms,
            stdout: None,
            stderr: None,
        }
    }

    /// Create a failed result
    pub fn failure(error: impl Into<String>, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(error.into()),
            execution_time_ms,
            stdout: None,
            stderr: None,
        }
    }

    /// Create a failed result with stdout/stderr
    pub fn failure_with_output(
        error: impl Into<String>,
        execution_time_ms: u64,
        stdout: Option<String>,
        stderr: Option<String>,
    ) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(error.into()),
            execution_time_ms,
            stdout,
            stderr,
        }
    }

    /// Set stdout
    pub fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }

    /// Set stderr
    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }
}

/// Helper functions for JSON Schema generation
pub mod schema_helpers {
    use serde_json::json;

    /// Create a simple string parameter schema
    pub fn string_param(description: &str) -> serde_json::Value {
        json!({
            "type": "string",
            "description": description
        })
    }

    /// Create a simple integer parameter schema
    pub fn integer_param(description: &str) -> serde_json::Value {
        json!({
            "type": "integer",
            "description": description
        })
    }

    /// Create a simple boolean parameter schema
    pub fn boolean_param(description: &str) -> serde_json::Value {
        json!({
            "type": "boolean",
            "description": description
        })
    }

    /// Create an object schema with properties
    pub fn object_schema(
        properties: serde_json::Map<String, serde_json::Value>,
        required: Vec<String>,
    ) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }

    /// Create a simple input schema for a tool with string parameters
    pub fn create_string_params_schema(params: Vec<(&str, &str, bool)>) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for (name, description, is_required) in params {
            properties.insert(
                name.to_string(),
                json!({
                    "type": "string",
                    "description": description
                }),
            );
            if is_required {
                required.push(name.to_string());
            }
        }

        object_schema(properties, required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_process_config_builder() {
        let config = ProcessConfig::new("git")
            .with_args(vec![
                "commit".to_string(),
                "-m".to_string(),
                "{{message}}".to_string(),
            ])
            .with_working_dir(PathBuf::from("/workspace"))
            .with_timeout(60000)
            .with_env("GIT_AUTHOR_NAME".to_string(), "AI Agent".to_string());

        assert_eq!(config.executable, "git");
        assert_eq!(config.args_template.len(), 3);
        assert_eq!(config.timeout_ms, 60000);
        assert!(config.working_dir.is_some());
        assert_eq!(config.env.len(), 1);
    }

    #[test]
    fn test_http_config_builder() {
        let config = HttpConfig::new("https://api.github.com", "POST")
            .with_path("/repos/{{owner}}/{{repo}}/issues")
            .with_header("Accept".to_string(), "application/json".to_string())
            .with_timeout(15000);

        assert_eq!(config.base_url, "https://api.github.com");
        assert_eq!(config.method, "POST");
        assert_eq!(config.timeout_ms, 15000);
        assert!(config.headers.contains_key("Accept"));
    }

    #[test]
    fn test_script_config_builder() {
        let config = ScriptConfig::new(PathBuf::from("scripts/analyze.py"))
            .with_interpreter("python3")
            .with_args(vec!["--input".to_string(), "{{input_file}}".to_string()])
            .with_working_dir(PathBuf::from("/workspace"));

        assert_eq!(config.script_path, PathBuf::from("scripts/analyze.py"));
        assert_eq!(config.interpreter, Some("python3".to_string()));
        assert!(config.working_dir.is_some());
    }

    #[test]
    fn test_tool_metadata_creation() {
        let process_config = ProcessConfig::new("git").with_args(vec![
            "commit".to_string(),
            "-m".to_string(),
            "{{message}}".to_string(),
        ]);

        let input_schema =
            schema_helpers::create_string_params_schema(vec![("message", "Commit message", true)]);

        let metadata = ExternalToolMetadata::new(
            "git_commit",
            "Commit changes to Git repository",
            ExternalToolType::process(process_config),
            input_schema.clone(),
            "version_control",
            "ai_agent",
        )
        .with_tags(vec![
            "git".to_string(),
            "commit".to_string(),
            "vcs".to_string(),
        ])
        .with_risk_level(RiskLevel::Medium);

        assert_eq!(metadata.name, "git_commit");
        assert_eq!(metadata.domain, "version_control");
        assert_eq!(metadata.tags.len(), 3);
        assert_eq!(metadata.risk_level, RiskLevel::Medium);
        assert!(metadata.enabled);
    }

    #[test]
    fn test_execution_result_creation() {
        let success_result = ToolExecutionResult::success(json!({"result": "ok"}), 150);

        assert!(success_result.success);
        assert!(success_result.error.is_none());
        assert_eq!(success_result.execution_time_ms, 150);

        let failure_result = ToolExecutionResult::failure("Timeout exceeded", 5000);

        assert!(!failure_result.success);
        assert_eq!(failure_result.error, Some("Timeout exceeded".to_string()));
    }
}
