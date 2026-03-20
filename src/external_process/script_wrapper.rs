//! External Process Wrapper - Script Implementation
//!
//! Implementation of the ExternalTool trait for script files (.sh, .py, .js, etc.)
//!
//! ## Overview
//! This module provides the `ScriptWrapper` struct that wraps script files
//! and makes them callable as tokitai tools.
//!
//! ## Features
//! - Multiple interpreter support (bash, python3, node, etc.)
//! - Auto-detect interpreter from file extension
//! - Working directory support
//! - Environment variable injection
//! - Timeout handling
//! - Stdout/stderr capture

use crate::external_process::metadata::{
    ExternalToolMetadata,
    ExternalToolType,
    ScriptConfig,
    ToolExecutionResult,
    RiskLevel,
};
use crate::external_process::wrapper::{ExternalTool, validation};
use crate::tool_matrix::matrix::ToolDefinition;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn, info};

/// Script wrapper for script files
///
/// Wraps a script file and makes it callable as a tokitai tool.
///
/// ## Example
/// ```rust,ignore
/// use crate::external_process::metadata::{ScriptConfig, ExternalToolMetadata, ExternalToolType};
/// use crate::external_process::script_wrapper::ScriptWrapper;
///
/// let config = ScriptConfig::new(PathBuf::from("scripts/analyze.py"))
///     .with_interpreter("python3")
///     .with_args(vec!["--input".to_string(), "{{input_file}}".to_string()]);
///
/// let metadata = ExternalToolMetadata::new(
///     "analyze_data",
///     "Analyze data using Python script",
///     ExternalToolType::script(config),
///     serde_json::json!({
///         "type": "object",
///         "properties": {
///             "input_file": {"type": "string", "description": "Input file path"}
///         },
///         "required": ["input_file"]
///     }),
///     "data_analysis",
///     "ai_agent",
/// );
///
/// let wrapper = ScriptWrapper::new(metadata);
/// let result = wrapper.execute(serde_json::json!({"input_file": "data.csv"})).await?;
/// ```
pub struct ScriptWrapper {
    metadata: ExternalToolMetadata,
}

impl ScriptWrapper {
    /// Create a new script wrapper
    ///
    /// # Arguments
    /// * `metadata` - Tool metadata containing script configuration
    ///
    /// # Returns
    /// * `Self` - New script wrapper instance
    ///
    /// # Panics
    /// Panics if the metadata's tool_type is not ExternalToolType::Script
    pub fn new(metadata: ExternalToolMetadata) -> Self {
        // Verify that the tool type is Script
        match &metadata.tool_type {
            ExternalToolType::Script { .. } => {}
            _ => panic!("ScriptWrapper requires ExternalToolType::Script"),
        }
        Self { metadata }
    }

    /// Get the script configuration
    ///
    /// # Returns
    /// * `&ScriptConfig` - Script configuration reference
    pub fn config(&self) -> &ScriptConfig {
        match &self.metadata.tool_type {
            ExternalToolType::Script { config } => config,
            _ => unreachable!(),
        }
    }

    /// Auto-detect interpreter from file extension
    ///
    /// # Arguments
    /// * `script_path` - Path to the script file
    ///
    /// # Returns
    /// * `Option<String>` - Interpreter name
    pub fn detect_interpreter<P: AsRef<Path>>(script_path: P) -> Option<String> {
        let path = script_path.as_ref();
        let extension = path.extension().and_then(|e| e.to_str())?;

        let interpreter = match extension.to_lowercase().as_str() {
            "sh" => "bash",
            "bash" => "bash",
            "zsh" => "zsh",
            "fish" => "fish",
            "py" => "python3",
            "pyw" => "pythonw",
            "py3" => "python3",
            "js" => "node",
            "mjs" => "node",
            "cjs" => "node",
            "ts" => "ts-node",
            "tsx" => "ts-node",
            "rb" => "ruby",
            "pl" => "perl",
            "pm" => "perl",
            "php" => "php",
            "r" => "Rscript",
            "R" => "Rscript",
            "jl" => "julia",
            "lua" => "lua",
            "ps1" => "powershell",
            "bat" => "cmd",
            "cmd" => "cmd",
            "vbs" => "cscript",
            "awk" => "awk",
            "sed" => "sed",
            _ => return None,
        };

        Some(interpreter.to_string())
    }

    /// Check if interpreter exists
    ///
    /// # Arguments
    /// * `interpreter` - Interpreter name
    ///
    /// # Returns
    /// * `bool` - True if interpreter is available
    pub fn interpreter_exists(interpreter: &str) -> bool {
        // Try to run interpreter with --version or help flag
        let result = std::process::Command::new(interpreter)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();

        if result.is_ok() {
            return true;
        }

        // Try with -version (some interpreters use this)
        let result = std::process::Command::new(interpreter)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();

        result.is_ok()
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
                        let placeholder = format!("{{{{{}}}}}", var_name);
                        result = result.replace(&placeholder, &value.to_string());
                    }
                } else {
                    warn!("Variable '{}' not found in input", var_name);
                    let placeholder = format!("{{{{{}}}}}", var_name);
                    result = result.replace(&placeholder, "");
                }
            } else {
                break;
            }
        }

        result
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

        // Determine interpreter
        let interpreter = config.interpreter.clone()
            .or_else(|| Self::detect_interpreter(&config.script_path))
            .with_context(|| format!(
                "Could not determine interpreter for script: {:?}",
                config.script_path
            ))?;

        // Verify script exists
        if !config.script_path.exists() {
            bail!("Script file not found: {:?}", config.script_path);
        }

        // Build command
        let mut cmd = Command::new(&interpreter);

        // Add script path as first argument
        cmd.arg(&config.script_path);

        // Substitute and add additional arguments
        for arg_template in &config.args_template {
            let arg = self.substitute_arg(arg_template, input);
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        debug!("Script command: {:?}", cmd);

        Ok(cmd)
    }

    /// Execute the script with timeout
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
            .context("Failed to spawn script process")?;

        let result = timeout(Duration::from_millis(timeout_ms), async {
            child.wait_with_output().await
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Ok((stdout, stderr))
            }
            Ok(Err(e)) => bail!("Script execution failed: {}", e),
            Err(_) => bail!("Script execution timed out after {}ms", timeout_ms),
        }
    }
}

#[async_trait::async_trait]
impl ExternalTool for ScriptWrapper {
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
        let input_obj = input.as_object()
            .context("Input must be a JSON object")?;

        // Build command
        let cmd = self.build_command(input_obj)?;

        // Execute with timeout
        let execution_result = self.execute_with_timeout(cmd, 30000).await;

        let elapsed = start_time.elapsed().as_millis() as u64;

        match execution_result {
            Ok((stdout, stderr)) => {
                debug!("Script executed successfully in {}ms", elapsed);

                // Try to parse stdout as JSON, otherwise return as text
                let output = serde_json::from_str::<Value>(&stdout)
                    .unwrap_or_else(|_| Value::String(stdout.clone()));

                Ok(ToolExecutionResult::success(output, elapsed)
                    .with_stdout(stdout)
                    .with_stderr(stderr))
            }
            Err(e) => {
                warn!("Script execution failed: {}", e);
                Ok(ToolExecutionResult::failure(e.to_string(), elapsed))
            }
        }
    }

    fn validate_input(&self, input: &Value) -> Result<()> {
        validation::validate_json_schema(input, &self.metadata.input_schema)
    }

    fn to_tool_definition(&self) -> ToolDefinition {
        use crate::tool_matrix::matrix::{ServiceMetadata, ServiceCategory};

        let risk_level_str = match self.metadata.risk_level {
            RiskLevel::Low => "safe",
            RiskLevel::Medium => "moderate",
            RiskLevel::High => "dangerous",
            RiskLevel::Critical => "dangerous",
        };

        let category = match self.metadata.domain.as_str() {
            "data_analysis" | "analytics" => ServiceCategory::Data,
            "automation" => ServiceCategory::System,
            "build" => ServiceCategory::Development,
            "deployment" => ServiceCategory::Development,
            "testing" => ServiceCategory::Development,
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

/// Builder for creating ScriptWrapper with fluent API
pub struct ScriptWrapperBuilder {
    name: String,
    description: String,
    script_path: PathBuf,
    interpreter: Option<String>,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    input_schema: Value,
    domain: String,
    tags: Vec<String>,
    risk_level: RiskLevel,
    created_by: String,
}

impl ScriptWrapperBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>, script_path: PathBuf) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            script_path,
            interpreter: None,
            args: Vec::new(),
            working_dir: None,
            input_schema: Value::Object(serde_json::Map::new()),
            domain: String::new(),
            tags: Vec::new(),
            risk_level: RiskLevel::Medium,
            created_by: "user".to_string(),
        }
    }

    /// Create a new builder with auto-detected interpreter
    pub fn with_auto_interpreter(name: impl Into<String>, script_path: PathBuf) -> Option<Self> {
        let interpreter = ScriptWrapper::detect_interpreter(&script_path)?;
        let mut builder = Self::new(name, script_path);
        builder.interpreter = Some(interpreter);
        Some(builder)
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set interpreter
    pub fn interpreter(mut self, interpreter: impl Into<String>) -> Self {
        self.interpreter = Some(interpreter.into());
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

    /// Build the ScriptWrapper
    pub fn build(self) -> ScriptWrapper {
        let config = ScriptConfig {
            script_path: self.script_path,
            interpreter: self.interpreter,
            args_template: self.args,
            working_dir: self.working_dir,
        };

        let metadata = ExternalToolMetadata::new(
            self.name,
            self.description,
            ExternalToolType::script(config),
            self.input_schema,
            self.domain,
            self.created_by,
        )
        .with_tags(self.tags)
        .with_risk_level(self.risk_level);

        ScriptWrapper::new(metadata)
    }
}

/// Script scanner for auto-discovery
pub mod script_scanner {
    use super::*;
    use std::fs;
    use walkdir::WalkDir;

    /// Scan a directory for script files
    ///
    /// # Arguments
    /// * `dir` - Directory to scan
    /// * `recursive` - Whether to scan recursively
    ///
    /// # Returns
    /// * `Vec<PathBuf>` - Found script files
    pub fn scan_directory<P: AsRef<Path>>(dir: P, recursive: bool) -> Vec<PathBuf> {
        let mut scripts = Vec::new();
        let dir = dir.as_ref();

        if !dir.exists() || !dir.is_dir() {
            warn!("Directory does not exist: {:?}", dir);
            return scripts;
        }

        if recursive {
            // Use WalkDir for recursive scanning
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    if ScriptWrapper::detect_interpreter(path).is_some() {
                        scripts.push(path.to_path_buf());
                    }
                }
            }
        } else {
            // Non-recursive: only read directory
            match fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() && ScriptWrapper::detect_interpreter(&path).is_some() {
                            scripts.push(path.to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read directory {:?}: {}", dir, e);
                }
            }
        }

        info!("Found {} scripts in {:?}", scripts.len(), dir);
        scripts
    }

    /// Scan and create wrappers for scripts in a directory
    ///
    /// # Arguments
    /// * `dir` - Directory to scan
    /// * `created_by` - Creator identifier
    /// * `recursive` - Whether to scan recursively
    ///
    /// # Returns
    /// * `Result<Vec<ScriptWrapper>>` - Created script wrappers
    pub fn scan_and_create_wrappers<P: AsRef<Path>>(
        dir: P,
        created_by: &str,
        recursive: bool,
    ) -> Result<Vec<ScriptWrapper>> {
        let scripts = scan_directory(dir, recursive);
        let mut wrappers = Vec::new();

        for script_path in scripts {
            // Generate tool name from filename
            let tool_name = script_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown_script")
                .to_string();

            // Generate basic description
            let description = format!("Execute script: {}", script_path.display());

            // Create wrapper with auto-detected interpreter
            if let Some(builder) = ScriptWrapperBuilder::with_auto_interpreter(&tool_name, script_path.clone()) {
                let wrapper = builder
                    .description(description)
                    .domain("script")
                    .tag("script")
                    .tag("auto_discovered")
                    .created_by(created_by)
                    .build();

                wrappers.push(wrapper);
            } else {
                warn!("Could not determine interpreter for: {:?}", script_path);
            }
        }

        Ok(wrappers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_process::metadata::schema_helpers;
    use serde_json::json;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_interpreter_detection() {
        assert_eq!(ScriptWrapper::detect_interpreter("test.sh"), Some("bash".to_string()));
        assert_eq!(ScriptWrapper::detect_interpreter("test.py"), Some("python3".to_string()));
        assert_eq!(ScriptWrapper::detect_interpreter("test.js"), Some("node".to_string()));
        assert_eq!(ScriptWrapper::detect_interpreter("test.rb"), Some("ruby".to_string()));
        assert_eq!(ScriptWrapper::detect_interpreter("test.unknown"), None);
    }

    #[test]
    fn test_script_wrapper_builder() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'Hello'").unwrap();

        let wrapper = ScriptWrapperBuilder::new("test_script", script_path)
            .description("Test script")
            .interpreter("bash")
            .args(vec!["--flag".to_string()])
            .domain("testing")
            .tag("test")
            .build();

        assert_eq!(wrapper.name(), "test_script");
        assert_eq!(wrapper.domain(), "testing");
        assert_eq!(wrapper.tags().len(), 1);
    }

    #[tokio::test]
    async fn test_script_wrapper_echo() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("echo.sh");
        
        // Create a simple echo script
        let script_content = r#"#!/bin/bash
echo "Message: $1"
"#;
        fs::write(&script_path, script_content).unwrap();
        
        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).unwrap();
        }

        let wrapper = ScriptWrapperBuilder::new("echo_script", script_path)
            .description("Echo script")
            .interpreter("bash")
            .args(vec!["{{message}}".to_string()])
            .input_schema(schema_helpers::create_string_params_schema(vec![
                ("message", "Message to echo", true),
            ]))
            .domain("test")
            .tag("test")
            .build();

        let input = json!({"message": "Hello from script!"});
        let result = wrapper.execute(input).await.unwrap();

        assert!(result.success);
        assert!(result.stdout.unwrap().contains("Hello from script!"));
    }

    #[tokio::test]
    async fn test_script_wrapper_python() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("hello.py");
        
        // Create a simple Python script
        let script_content = r#"import sys
print(f"Hello, {sys.argv[1]}!")
"#;
        fs::write(&script_path, script_content).unwrap();

        let wrapper = ScriptWrapperBuilder::new("python_hello", script_path)
            .description("Python hello script")
            .interpreter("python3")
            .args(vec!["{{name}}".to_string()])
            .input_schema(schema_helpers::create_string_params_schema(vec![
                ("name", "Name to greet", true),
            ]))
            .domain("test")
            .tag("python")
            .build();

        let input = json!({"name": "World"});
        let result = wrapper.execute(input).await.unwrap();

        // Python might not be installed, so handle gracefully
        if result.success {
            assert!(result.stdout.unwrap().contains("Hello, World!"));
        } else {
            // Python not available, just verify graceful failure
            assert!(result.error.is_some());
        }
    }

    #[test]
    fn test_script_scanner() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create some test scripts
        fs::write(temp_dir.path().join("test1.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("test2.py"), "# Python").unwrap();
        fs::write(temp_dir.path().join("test3.js"), "// JavaScript").unwrap();
        fs::write(temp_dir.path().join("test4.txt"), "Not a script").unwrap();

        let scripts = script_scanner::scan_directory(temp_dir.path(), false);
        
        // Should find 3 scripts (sh, py, js)
        assert_eq!(scripts.len(), 3);
    }

    #[test]
    fn test_arg_substitution() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash").unwrap();

        let wrapper = ScriptWrapperBuilder::new("test", script_path)
            .build();

        let input = serde_json::Map::from_iter(vec![
            ("name".to_string(), Value::String("Alice".to_string())),
            ("count".to_string(), Value::Number(5.into())),
        ]);

        let template = "--name {{name}} --count {{count}}";
        let result = wrapper.substitute_arg(template, &input);
        assert_eq!(result, "--name Alice --count 5");
    }
}
