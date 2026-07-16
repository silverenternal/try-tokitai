//! External Process Wrapper - Process Implementation
//!
//! Implementation of the ExternalTool trait for local executable/CLI tools.
//!
//! ## Overview
//! This module provides the `ProcessWrapper` struct that wraps local executables
//! and CLI tools, making them callable as tokitai tools.
//!
//! ## Features
//! - Argument template substitution

#![allow(dead_code)]
//! - Working directory support
//! - Environment variable injection
//! - Timeout handling
//! - Stdout/stderr capture

use crate::external_process::metadata::{
    ExternalToolMetadata, ExternalToolType, ProcessConfig, RiskLevel, ToolExecutionResult,
};
use crate::external_process::wrapper::{validation, ExternalTool};
use crate::process_window::CommandWindowExt;
use crate::text_encoding::decode_bytes;
use crate::tool_matrix::matrix::ToolDefinition;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

/// Process wrapper for local executable/CLI tools
///
/// Wraps a local executable and makes it callable as a tokitai tool.
///
/// ## Example
/// ```rust,ignore
/// use crate::external_process::metadata::{ProcessConfig, ExternalToolMetadata, ExternalToolType};
/// use crate::external_process::process_wrapper::ProcessWrapper;
///
/// let config = ProcessConfig::new("git")
///     .with_args(vec!["commit".to_string(), "-m".to_string(), "{{message}}".to_string()]);
///
/// let metadata = ExternalToolMetadata::new(
///     "git_commit",
///     "Commit changes to Git repository",
///     ExternalToolType::process(config),
///     serde_json::json!({
///         "type": "object",
///         "properties": {
///             "message": {"type": "string", "description": "Commit message"}
///         },
///         "required": ["message"]
///     }),
///     "version_control",
///     "ai_agent",
/// );
///
/// let wrapper = ProcessWrapper::new(metadata);
/// let result = wrapper.execute(serde_json::json!({"message": "Initial commit"})).await?;
/// ```
pub struct ProcessWrapper {
    metadata: ExternalToolMetadata,
}

impl ProcessWrapper {
    /// Create a new process wrapper
    ///
    /// # Arguments
    /// * `metadata` - Tool metadata containing process configuration
    ///
    /// # Returns
    /// * `Self` - New process wrapper instance
    ///
    /// # Panics
    /// Panics if the metadata's tool_type is not ExternalToolType::Process
    pub fn new(metadata: ExternalToolMetadata) -> Self {
        // Verify that the tool type is Process
        match &metadata.tool_type {
            ExternalToolType::Process { .. } => {}
            _ => panic!("ProcessWrapper requires ExternalToolType::Process"),
        }
        Self { metadata }
    }

    /// Get the process configuration
    ///
    /// # Returns
    /// * `&ProcessConfig` - Process configuration reference
    pub fn config(&self) -> &ProcessConfig {
        match &self.metadata.tool_type {
            ExternalToolType::Process { config } => config,
            _ => unreachable!(),
        }
    }

    /// Substitute variables in argument template
    ///
    /// Replaces placeholders like `{{variable_name}}` with actual values from input.
    ///
    /// # Arguments
    /// * `template` - Argument template containing placeholders
    /// * `input` - Input JSON object with variable values
    ///
    /// # Returns
    /// * `String` - Argument with substituted values
    fn substitute_arg(&self, template: &str, input: &serde_json::Map<String, Value>) -> String {
        let mut result = template.to_string();

        // Find all {{variable}} patterns and replace them
        while let Some(start) = result.find("{{") {
            if let Some(end) = result[start..].find("}}") {
                let var_name = &result[start + 2..start + end];
                if let Some(value) = input.get(var_name) {
                    if let Some(str_value) = value.as_str() {
                        let placeholder = format!("{{{{{}}}}}", var_name);
                        result = result.replace(&placeholder, str_value);
                    } else {
                        // Convert non-string values to string
                        let placeholder = format!("{{{{{}}}}}", var_name);
                        result = result.replace(&placeholder, &value.to_string());
                    }
                } else {
                    warn!("Variable '{}' not found in input", var_name);
                    // Leave the placeholder as-is or replace with empty string
                    let placeholder = format!("{{{{{}}}}}", var_name);
                    result = result.replace(&placeholder, "");
                }
            } else {
                break;
            }
        }

        result
    }

    /// Execute the process with timeout
    ///
    /// # Arguments
    /// * `cmd` - Configured command to execute
    /// * `timeout_ms` - Timeout in milliseconds
    ///
    /// # Returns
    /// * `Result<(String, String)>` - Tuple of (stdout, stderr)
    async fn execute_with_timeout(
        &self,
        mut cmd: Command,
        timeout_ms: u64,
    ) -> Result<(String, String)> {
        let child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn process")?;

        let result = timeout(Duration::from_millis(timeout_ms), async {
            child.wait_with_output().await
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = decode_bytes(&output.stdout);
                let stderr = decode_bytes(&output.stderr);
                Ok((stdout, stderr))
            }
            Ok(Err(e)) => bail!("Process execution failed: {}", e),
            Err(_) => bail!("Process execution timed out after {}ms", timeout_ms),
        }
    }

    /// Build the command from config and input
    ///
    /// # Arguments
    /// * `input` - Input JSON object
    ///
    /// # Returns
    /// * `Result<Command>` - Configured tokio::process::Command
    fn build_command(&self, input: &serde_json::Map<String, Value>) -> Result<Command> {
        let config = self.config();
        let mut cmd = Command::new(&config.executable);
        cmd.hide_window();

        // Substitute and add arguments
        for arg_template in &config.args_template {
            let arg = self.substitute_arg(arg_template, input);
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        Ok(cmd)
    }
}

#[async_trait::async_trait]
impl ExternalTool for ProcessWrapper {
    fn metadata(&self) -> &ExternalToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
        let start_time = Instant::now();

        // Validate input first
        if let Err(e) = self.validate_input(&input) {
            return Ok(ToolExecutionResult::failure(
                format!("Input validation failed: {}", e),
                start_time.elapsed().as_millis() as u64,
            ));
        }

        // Get input as object
        let input_obj = input.as_object().context("Input must be a JSON object")?;

        // Build command
        let cmd = self.build_command(input_obj)?;

        // Execute with timeout
        let config = self.config();
        let execution_result = self.execute_with_timeout(cmd, config.timeout_ms).await;

        let elapsed = start_time.elapsed().as_millis() as u64;

        match execution_result {
            Ok((stdout, stderr)) => {
                debug!("Process executed successfully in {}ms", elapsed);

                // Try to parse stdout as JSON, otherwise return as text
                let output = serde_json::from_str::<Value>(&stdout)
                    .unwrap_or_else(|_| Value::String(stdout.clone()));

                Ok(ToolExecutionResult::success(output, elapsed)
                    .with_stdout(stdout)
                    .with_stderr(stderr))
            }
            Err(e) => {
                warn!("Process execution failed: {}", e);
                Ok(ToolExecutionResult::failure(e.to_string(), elapsed))
            }
        }
    }

    fn validate_input(&self, input: &Value) -> Result<()> {
        validation::validate_json_schema(input, &self.metadata.input_schema)
    }

    fn to_tool_definition(&self) -> ToolDefinition {
        // Convert ExternalToolMetadata to ToolDefinition
        use crate::tool_matrix::matrix::{ServiceCategory, ServiceMetadata};

        // Map RiskLevel to string
        let risk_level_str = match self.metadata.risk_level {
            RiskLevel::Low => "safe",
            RiskLevel::Medium => "moderate",
            RiskLevel::High => "dangerous",
            RiskLevel::Critical => "dangerous",
        };

        // Map domain to service category
        let category = match self.metadata.domain.as_str() {
            "version_control" | "vcs" => ServiceCategory::VersionControl,
            "file_ops" | "files" => ServiceCategory::File,
            "network" | "http" => ServiceCategory::Network,
            "system" => ServiceCategory::System,
            "data" => ServiceCategory::Data,
            "ai" | "ml" => ServiceCategory::Ai,
            "development" | "dev" => ServiceCategory::Development,
            _ => ServiceCategory::Utility,
        };

        ToolDefinition {
            name: self.metadata.name.clone(),
            description: self.metadata.description.clone(),
            input_schema: self.metadata.input_schema.to_string(),
            metadata: ServiceMetadata {
                category,
                tags: self.metadata.tags.clone(),
                ..Default::default()
            },
            tags: self.metadata.tags.clone(),
            risk_level: risk_level_str.to_string(),
            source: "external".to_string(),
        }
    }
}

/// Builder for creating ProcessWrapper with fluent API
pub struct ProcessWrapperBuilder {
    name: String,
    description: String,
    executable: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    timeout_ms: u64,
    env: HashMap<String, String>,
    input_schema: Value,
    domain: String,
    tags: Vec<String>,
    risk_level: RiskLevel,
    created_by: String,
}

impl ProcessWrapperBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>, executable: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            executable: executable.into(),
            args: Vec::new(),
            working_dir: None,
            timeout_ms: 30000,
            env: HashMap::new(),
            input_schema: Value::Object(serde_json::Map::new()),
            domain: String::new(),
            tags: Vec::new(),
            risk_level: RiskLevel::Low,
            created_by: "user".to_string(),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set arguments
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set input schema
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// Set domain
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set risk level
    pub fn risk_level(mut self, risk: RiskLevel) -> Self {
        self.risk_level = risk;
        self
    }

    /// Set created by
    pub fn created_by(mut self, creator: impl Into<String>) -> Self {
        self.created_by = creator.into();
        self
    }

    /// Build the ProcessWrapper
    pub fn build(self) -> ProcessWrapper {
        let config = ProcessConfig {
            executable: self.executable,
            args_template: self.args,
            working_dir: self.working_dir,
            timeout_ms: self.timeout_ms,
            env: self.env,
        };

        let metadata = ExternalToolMetadata::new(
            self.name,
            self.description,
            ExternalToolType::process(config),
            self.input_schema,
            self.domain,
            self.created_by,
        )
        .with_tags(self.tags)
        .with_risk_level(self.risk_level);

        ProcessWrapper::new(metadata)
    }
}

/// Helper function to create a simple echo tool for testing
#[cfg(test)]
pub fn create_test_echo_wrapper() -> ProcessWrapper {
    use crate::external_process::metadata::schema_helpers;

    ProcessWrapperBuilder::new("echo_test", "echo")
        .description("Echo test tool")
        .args(vec!["{{message}}".to_string()])
        .input_schema(schema_helpers::create_string_params_schema(vec![(
            "message",
            "Message to echo",
            true,
        )]))
        .domain("test")
        .tag("test")
        .tag("echo")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_process::metadata::schema_helpers;
    use serde_json::json;

    #[tokio::test]
    async fn test_process_wrapper_echo() {
        // Use 'echo' command which is available on most systems
        let wrapper = create_test_echo_wrapper();

        let input = json!({"message": "Hello, World!"});
        let result = wrapper.execute(input).await.unwrap();

        assert!(result.success);
        assert!(result.stdout.unwrap().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_process_wrapper_timeout() {
        // Use 'sleep' command to test timeout
        let wrapper = ProcessWrapperBuilder::new("sleep_test", "sleep")
            .description("Sleep test tool")
            .args(vec!["{{duration}}".to_string()])
            .timeout(100) // 100ms timeout
            .input_schema(schema_helpers::create_string_params_schema(vec![(
                "duration",
                "Sleep duration in seconds",
                true,
            )]))
            .domain("test")
            .build();

        // Sleep for 1 second (should timeout)
        let input = json!({"duration": "1"});
        let result = wrapper.execute(input).await.unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn test_process_wrapper_invalid_input() {
        let wrapper = create_test_echo_wrapper();

        // Missing required field
        let input = json!({});
        let result = wrapper.execute(input).await.unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap().contains("Missing required field"));
    }

    #[test]
    fn test_arg_substitution() {
        let wrapper = create_test_echo_wrapper();

        let input = serde_json::Map::from_iter(vec![(
            "message".to_string(),
            Value::String("test".to_string()),
        )]);

        let result = wrapper.substitute_arg("{{message}}", &input);
        assert_eq!(result, "test");
    }

    #[test]
    fn test_arg_substitution_multiple_vars() {
        let wrapper = create_test_echo_wrapper();

        let input = serde_json::Map::from_iter(vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Number(30.into())),
        ]);

        let template = "Hello, {{name}}! You are {{age}} years old.";
        let result = wrapper.substitute_arg(template, &input);
        assert_eq!(result, "Hello, Alice! You are 30 years old.");
    }

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
            .with_env("GIT_AUTHOR_NAME".to_string(), "AI".to_string());

        assert_eq!(config.executable, "git");
        assert_eq!(config.args_template.len(), 3);
        assert_eq!(config.timeout_ms, 60000);
        assert!(config.working_dir.is_some());
        assert_eq!(config.env.len(), 1);
    }

    #[test]
    fn test_wrapper_builder() {
        let wrapper = ProcessWrapperBuilder::new("test_tool", "test")
            .description("Test tool")
            .args(vec!["--flag".to_string()])
            .working_dir(PathBuf::from("/tmp"))
            .timeout(5000)
            .env("KEY".to_string(), "VALUE".to_string())
            .domain("testing")
            .tag("test")
            .risk_level(RiskLevel::Low)
            .build();

        assert_eq!(wrapper.name(), "test_tool");
        assert_eq!(wrapper.domain(), "testing");
        assert_eq!(wrapper.tags().len(), 1);
        assert_eq!(wrapper.risk_level(), RiskLevel::Low);
    }
}
