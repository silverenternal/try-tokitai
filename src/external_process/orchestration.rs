//! External Process Wrapper - Tool Orchestration
//!
//! Compose multiple external tools into workflows and pipelines.
//!
//! ## Overview
//! This module provides the ability to compose multiple external tools
//! into reusable workflows. Workflows can:
//! - Chain tools together (output of one becomes input of next)
//! - Execute tools in parallel
//! - Conditionally execute tools based on results
//! - Handle errors and retries
//!
//! ## Quick Start
//! ```rust,ignore
//! use crate::external_process::orchestration::{Workflow, WorkflowStep};
//!
//! let workflow = Workflow::new("git_commit_and_push")
//!     .description("Commit changes and push to remote")
//!     .step(WorkflowStep::new("git_commit", "git_commit"))
//!     .step(WorkflowStep::new("git_push", "git_push")
//!         .depends_on(&["git_commit"]))
//!     .build();
//!
//! let result = workflow.execute(json!({
//!     "message": "Initial commit",
//!     "remote": "origin"
//! })).await?;
//! ```

use crate::external_process::metadata::ToolExecutionResult;
use crate::external_process::wrapper::ExternalTool;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Unique step identifier
    pub id: String,
    /// Tool name to execute
    pub tool_name: String,
    /// Input template for the tool
    pub input_template: Option<Value>,
    /// Steps that must complete before this step
    pub depends_on: Vec<String>,
    /// Condition for executing this step (optional)
    pub condition: Option<String>,
    /// Retry count on failure
    pub retry_count: u32,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl WorkflowStep {
    /// Create a new workflow step
    pub fn new(id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tool_name: tool_name.into(),
            input_template: None,
            depends_on: Vec::new(),
            condition: None,
            retry_count: 0,
            timeout_ms: None,
        }
    }

    /// Set input template
    pub fn with_input_template(mut self, template: Value) -> Self {
        self.input_template = Some(template);
        self
    }

    /// Add dependencies
    pub fn depends_on(mut self, step_ids: &[&str]) -> Self {
        self.depends_on = step_ids.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set condition
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// Set retry count
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Set timeout
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Workflow execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Workflow name
    pub workflow_name: String,
    /// Execution success
    pub success: bool,
    /// Step results in execution order
    pub step_results: Vec<StepResult>,
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Final output
    pub output: Value,
}

/// Single step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step identifier
    pub step_id: String,
    /// Tool name
    pub tool_name: String,
    /// Execution success
    pub success: bool,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Tool execution result
    pub tool_result: Option<ToolExecutionResult>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: String,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Input schema for the workflow
    pub input_schema: Value,
    /// Output mapping (step_id.output_path -> workflow_output_path)
    pub output_mapping: HashMap<String, String>,
    /// Error handling strategy
    pub on_error: OnErrorStrategy,
    /// Domain
    pub domain: String,
    /// Tags
    pub tags: Vec<String>,
}

/// Error handling strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OnErrorStrategy {
    /// Stop workflow on first error
    Stop,
    /// Continue with next step despite error
    Continue,
    /// Retry failed steps
    Retry { max_retries: u32 },
}

impl Default for OnErrorStrategy {
    fn default() -> Self {
        OnErrorStrategy::Stop
    }
}

/// Workflow builder
pub struct WorkflowBuilder {
    name: String,
    description: String,
    steps: Vec<WorkflowStep>,
    input_schema: Option<Value>,
    output_mapping: HashMap<String, String>,
    on_error: OnErrorStrategy,
    domain: String,
    tags: Vec<String>,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            steps: Vec::new(),
            input_schema: None,
            output_mapping: HashMap::new(),
            on_error: OnErrorStrategy::default(),
            domain: String::from("workflow"),
            tags: Vec::new(),
        }
    }

    /// Set workflow description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a step
    pub fn step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set input schema
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Add output mapping
    pub fn output_mapping(mut self, step_id: impl Into<String>, output_path: impl Into<String>) -> Self {
        self.output_mapping.insert(step_id.into(), output_path.into());
        self
    }

    /// Set error handling strategy
    pub fn on_error(mut self, strategy: OnErrorStrategy) -> Self {
        self.on_error = strategy;
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

    /// Build the workflow
    pub fn build(self) -> Workflow {
        Workflow {
            name: self.name,
            description: self.description,
            steps: self.steps,
            input_schema: self.input_schema.unwrap_or_else(|| {
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            }),
            output_mapping: self.output_mapping,
            on_error: self.on_error,
            domain: self.domain,
            tags: self.tags,
        }
    }
}

/// Workflow executor
pub struct WorkflowExecutor {
    workflow: Workflow,
    tools: HashMap<String, Arc<dyn ExternalTool>>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(workflow: Workflow) -> Self {
        Self {
            workflow,
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register_tool(&mut self, name: impl Into<String>, tool: Arc<dyn ExternalTool>) {
        self.tools.insert(name.into(), tool);
    }

    /// Execute the workflow
    pub async fn execute(&self, input: Value) -> Result<WorkflowResult> {
        let start_time = std::time::Instant::now();
        let mut step_results = Vec::new();
        let mut step_outputs: HashMap<String, Value> = HashMap::new();

        info!("Executing workflow: {}", self.workflow.name);

        // Validate input
        if let Err(e) = self.validate_input(&input) {
            return Ok(WorkflowResult {
                workflow_name: self.workflow.name.clone(),
                success: false,
                step_results: Vec::new(),
                total_time_ms: 0,
                error: Some(format!("Input validation failed: {}", e)),
                output: Value::Null,
            });
        }

        // Execute steps in order (respecting dependencies)
        for step in &self.workflow.steps {
            // Check dependencies
            let deps_satisfied = step.depends_on.iter().all(|dep| {
                step_outputs.get(dep).map(|v| {
                    v.get("success").and_then(|s| s.as_bool()).unwrap_or(false)
                }).unwrap_or(false)
            });

            if !deps_satisfied {
                warn!("Step {} dependencies not satisfied, skipping", step.id);
                
                match self.workflow.on_error {
                    OnErrorStrategy::Continue => continue,
                    OnErrorStrategy::Stop => {
                        return Ok(WorkflowResult {
                            workflow_name: self.workflow.name.clone(),
                            success: false,
                            step_results,
                            total_time_ms: start_time.elapsed().as_millis() as u64,
                            error: Some(format!("Step {} dependencies not satisfied", step.id)),
                            output: Value::Null,
                        });
                    }
                    OnErrorStrategy::Retry { .. } => continue,
                }
            }

            // Check condition
            if let Some(condition) = &step.condition {
                if !self.evaluate_condition(condition, &input, &step_outputs)? {
                    debug!("Step {} condition not met, skipping", step.id);
                    continue;
                }
            }

            // Execute step
            let step_result = self.execute_step(step, &input, &step_outputs).await?;
            let success = step_result.success;

            if let Some(ref tool_result) = step_result.tool_result {
                step_outputs.insert(
                    step.id.clone(),
                    serde_json::json!({
                        "success": success,
                        "output": tool_result.output,
                        "stdout": tool_result.stdout,
                        "stderr": tool_result.stderr,
                    })
                );
            }

            step_results.push(step_result);

            // Handle error
            if !success {
                match &self.workflow.on_error {
                    OnErrorStrategy::Stop => {
                        return Ok(WorkflowResult {
                            workflow_name: self.workflow.name.clone(),
                            success: false,
                            step_results,
                            total_time_ms: start_time.elapsed().as_millis() as u64,
                            error: Some(format!("Step {} failed", step.id)),
                            output: Value::Null,
                        });
                    }
                    OnErrorStrategy::Continue => continue,
                    OnErrorStrategy::Retry { max_retries } => {
                        // Retry logic would go here
                        warn!("Step {} failed, max retries: {}", step.id, max_retries);
                    }
                }
            }
        }

        // Build final output
        let final_output = self.build_output(&step_outputs);

        Ok(WorkflowResult {
            workflow_name: self.workflow.name.clone(),
            success: true,
            step_results,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            error: None,
            output: final_output,
        })
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        workflow_input: &Value,
        step_outputs: &HashMap<String, Value>,
    ) -> Result<StepResult> {
        let tool = self.tools.get(&step.tool_name)
            .with_context(|| format!("Tool '{}' not found", step.tool_name))?;

        let start_time = std::time::Instant::now();

        // Build input for this step
        let step_input = self.build_step_input(step, workflow_input, step_outputs)?;

        // Execute with retries
        let mut last_result = None;
        let mut attempts = 0;

        while attempts <= step.retry_count {
            attempts += 1;

            match tool.execute(step_input.clone()).await {
                Ok(result) => {
                    let success = result.success;
                    last_result = Some(Ok(result));
                    if success {
                        break;
                    }
                }
                Err(e) => {
                    last_result = Some(Err(e));
                }
            }

            if attempts <= step.retry_count {
                debug!("Step {} failed, retrying ({}/{})", step.id, attempts, step.retry_count);
            }
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match last_result {
            Some(Ok(tool_result)) => {
                let success = tool_result.success;
                let error = tool_result.error.clone();
                Ok(StepResult {
                    step_id: step.id.clone(),
                    tool_name: step.tool_name.clone(),
                    success,
                    execution_time_ms,
                    tool_result: Some(tool_result),
                    error,
                })
            },
            Some(Err(e)) => Ok(StepResult {
                step_id: step.id.clone(),
                tool_name: step.tool_name.clone(),
                success: false,
                execution_time_ms,
                tool_result: None,
                error: Some(e.to_string()),
            }),
            None => Ok(StepResult {
                step_id: step.id.clone(),
                tool_name: step.tool_name.clone(),
                success: false,
                execution_time_ms,
                tool_result: None,
                error: Some("Unknown error".to_string()),
            }),
        }
    }

    /// Build step input from template
    fn build_step_input(
        &self,
        step: &WorkflowStep,
        workflow_input: &Value,
        step_outputs: &HashMap<String, Value>,
    ) -> Result<Value> {
        if let Some(template) = &step.input_template {
            // Substitute variables in template
            self.substitute_variables(template, workflow_input, step_outputs)
        } else {
            // Pass workflow input directly
            Ok(workflow_input.clone())
        }
    }

    /// Substitute variables in template
    fn substitute_variables(
        &self,
        template: &Value,
        workflow_input: &Value,
        step_outputs: &HashMap<String, Value>,
    ) -> Result<Value> {
        match template {
            Value::String(s) => {
                // Simple variable substitution: {{input.field}} or {{step_id.output}}
                let result = s.clone();
                let result = result.replace("{{input}}", &workflow_input.to_string());
                
                // Replace step outputs
                let mut final_result = result;
                for (step_id, output) in step_outputs {
                    final_result = final_result.replace(
                        &format!("{{{{{}}}}}", step_id),
                        &output.to_string()
                    );
                }
                
                Ok(Value::String(final_result))
            }
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), self.substitute_variables(v, workflow_input, step_outputs)?);
                }
                Ok(Value::Object(new_obj))
            }
            Value::Array(arr) => {
                let mut new_arr = Vec::new();
                for v in arr {
                    new_arr.push(self.substitute_variables(v, workflow_input, step_outputs)?);
                }
                Ok(Value::Array(new_arr))
            }
            _ => Ok(template.clone()),
        }
    }

    /// Evaluate condition
    fn evaluate_condition(
        &self,
        condition: &str,
        _workflow_input: &Value,
        _step_outputs: &HashMap<String, Value>,
    ) -> Result<bool> {
        // Simple condition evaluation
        // In a real implementation, this would use a proper expression language
        Ok(match condition {
            "always" => true,
            "never" => false,
            _ => true, // Default to true for now
        })
    }

    /// Build final output from step outputs
    fn build_output(&self, step_outputs: &HashMap<String, Value>) -> Value {
        if self.workflow.output_mapping.is_empty() {
            // Return all step outputs
            let mut output_map = serde_json::Map::new();
            for (step_id, output) in step_outputs {
                output_map.insert(step_id.clone(), output.clone());
            }
            Value::Object(output_map)
        } else {
            // Build output based on mapping
            let mut output_map = serde_json::Map::new();
            for (step_output_path, workflow_output_path) in &self.workflow.output_mapping {
                if let Some(output) = step_outputs.get(step_output_path) {
                    output_map.insert(workflow_output_path.clone(), output.clone());
                }
            }
            Value::Object(output_map)
        }
    }

    /// Validate workflow input
    fn validate_input(&self, input: &Value) -> Result<()> {
        // Basic validation - check if input is an object
        if !input.is_object() && !self.workflow.input_schema["properties"].is_null() {
            bail!("Input must be an object");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_process::process_wrapper::ProcessWrapperBuilder;
    use crate::external_process::metadata::schema_helpers;

    #[test]
    fn test_workflow_builder() {
        let workflow = WorkflowBuilder::new("test_workflow")
            .description("Test workflow")
            .step(WorkflowStep::new("step1", "tool1"))
            .step(WorkflowStep::new("step2", "tool2")
                .depends_on(&["step1"]))
            .domain("test")
            .tag("workflow")
            .build();

        assert_eq!(workflow.name, "test_workflow");
        assert_eq!(workflow.description, "Test workflow");
        assert_eq!(workflow.steps.len(), 2);
        assert_eq!(workflow.domain, "test");
        assert_eq!(workflow.tags.len(), 1);
    }

    #[test]
    fn test_workflow_step_builder() {
        let step = WorkflowStep::new("my_step", "my_tool")
            .with_input_template(serde_json::json!({"key": "value"}))
            .depends_on(&["prev_step"])
            .with_retry_count(3)
            .with_timeout_ms(5000);

        assert_eq!(step.id, "my_step");
        assert_eq!(step.tool_name, "my_tool");
        assert!(step.input_template.is_some());
        assert_eq!(step.depends_on.len(), 1);
        assert_eq!(step.retry_count, 3);
        assert_eq!(step.timeout_ms, Some(5000));
    }

    #[tokio::test]
    async fn test_workflow_executor_basic() {
        // Create a simple echo tool
        let echo_tool = ProcessWrapperBuilder::new("echo_test", "echo")
            .description("Echo test")
            .args(vec!["{{message}}".to_string()])
            .input_schema(schema_helpers::create_string_params_schema(vec![
                ("message", "Message", true),
            ]))
            .domain("test")
            .build();

        // Create workflow
        let workflow = WorkflowBuilder::new("echo_workflow")
            .description("Echo workflow")
            .step(WorkflowStep::new("echo", "echo_test")
                .with_input_template(serde_json::json!({
                    "message": "Hello Workflow"
                })))
            .domain("test")
            .build();

        // Create executor and register tool
        let mut executor = WorkflowExecutor::new(workflow);
        executor.register_tool("echo_test", Arc::new(echo_tool));

        // Execute
        let result = executor.execute(serde_json::json!({})).await.unwrap();

        assert!(result.success);
        assert_eq!(result.workflow_name, "echo_workflow");
        assert!(!result.step_results.is_empty());
    }
}
