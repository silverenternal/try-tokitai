//! External Process Wrapper - Core Trait Definition
//!
//! Defines the `ExternalTool` trait that all external tool wrappers must implement.
//!
//! ## Overview
//! This trait provides a unified interface for external tools, allowing them to be:
//! - Discovered and registered in the tool matrix
//! - Executed with standardized input/output
//! - Validated before execution
//!

#![allow(dead_code)]
//! ## Implementations
//! - `ProcessWrapper`: Local executable/CLI tools
//! - `HTTPWrapper`: Remote HTTP services/REST APIs
//! - `ScriptWrapper`: Script files (.sh, .py, .js)

use crate::external_process::metadata::{ExternalToolMetadata, ToolExecutionResult};
use anyhow::Result;
use serde_json::Value;

/// Core trait for external tools
///
/// All external tool wrappers must implement this trait to be compatible with the tokitai tool matrix.
///
/// ## Example
/// ```rust,ignore
/// struct MyTool {
///     metadata: ExternalToolMetadata,
/// }
///
/// impl ExternalTool for MyTool {
///     fn metadata(&self) -> &ExternalToolMetadata {
///         &self.metadata
///     }
///
///     async fn execute(&self, input: Value) -> Result<ToolExecutionResult> {
///         // Implementation
///     }
///
///     fn validate_input(&self, input: &Value) -> Result<()> {
///         // Validation logic
///     }
///
///     fn to_tool_definition(&self) -> ToolDefinition {
///         // Convert to ToolDefinition
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait ExternalTool: Send + Sync {
    /// Get tool metadata
    ///
    /// Returns a reference to the tool's metadata, which includes:
    /// - Name and description
    /// - Tool type (Process, HTTP, Script)
    /// - Input/output schemas
    /// - Risk level and tags
    fn metadata(&self) -> &ExternalToolMetadata;

    /// Execute the tool with the given input
    ///
    /// # Arguments
    /// * `input` - JSON value containing the input parameters
    ///
    /// # Returns
    /// * `Ok(ToolExecutionResult)` - Execution result with output and timing
    /// * `Err(anyhow::Error)` - Error if execution failed
    ///
    /// # Example
    /// ```rust,ignore
    /// let input = json!({"message": "Initial commit"});
    /// let result = tool.execute(input).await?;
    /// assert!(result.success);
    /// ```
    async fn execute(&self, input: Value) -> Result<ToolExecutionResult>;

    /// Validate input parameters against the tool's input schema
    ///
    /// # Arguments
    /// * `input` - JSON value to validate
    ///
    /// # Returns
    /// * `Ok(())` - Input is valid
    /// * `Err(anyhow::Error)` - Input validation failed with error details
    ///
    /// # Example
    /// ```rust,ignore
    /// let input = json!({"message": "Commit message"});
    /// tool.validate_input(&input)?;
    /// ```
    fn validate_input(&self, input: &Value) -> Result<()>;

    /// Convert to ToolDefinition for registration in the tool matrix
    ///
    /// This method converts the external tool metadata into a tokitai
    /// ToolDefinition that can be registered in the tool matrix.
    ///
    /// # Returns
    /// * `ToolDefinition` - Tool definition compatible with the tool matrix
    ///
    /// # Example
    /// ```rust,ignore
    /// let tool_def = tool.to_tool_definition();
    /// registry.register_tool(tool_def)?;
    /// ```
    fn to_tool_definition(&self) -> crate::tool_matrix::matrix::ToolDefinition;

    /// Get the tool's domain/category
    ///
    /// # Returns
    /// * `&str` - Domain name (e.g., "version_control", "http_client")
    fn domain(&self) -> &str {
        &self.metadata().domain
    }

    /// Get the tool's name
    ///
    /// # Returns
    /// * `&str` - Tool name
    fn name(&self) -> &str {
        &self.metadata().name
    }

    /// Get the tool's description
    ///
    /// # Returns
    /// * `&str` - Tool description
    fn description(&self) -> &str {
        &self.metadata().description
    }

    /// Check if the tool is enabled
    ///
    /// # Returns
    /// * `bool` - True if enabled, false otherwise
    fn is_enabled(&self) -> bool {
        self.metadata().enabled
    }

    /// Get the tool's risk level
    ///
    /// # Returns
    /// * `RiskLevel` - Risk level for execution
    fn risk_level(&self) -> crate::external_process::metadata::RiskLevel {
        self.metadata().risk_level.clone()
    }

    /// Get the tool's tags
    ///
    /// # Returns
    /// * `&[String]` - Slice of tag strings
    fn tags(&self) -> &[String] {
        &self.metadata().tags
    }
}

/// Helper functions for JSON Schema validation
pub mod validation {
    use anyhow::{bail, Context, Result};
    use serde_json::Value;

    /// Validate input against a JSON schema
    ///
    /// This is a simple validator that checks:
    /// 1. Required fields are present
    /// 2. Field types match the schema
    ///
    /// # Arguments
    /// * `input` - JSON value to validate
    /// * `schema` - JSON schema to validate against
    ///
    /// # Returns
    /// * `Ok(())` - Input is valid
    /// * `Err(anyhow::Error)` - Validation failed
    pub fn validate_json_schema(input: &Value, schema: &Value) -> Result<()> {
        // Check if schema requires an object type
        if let Some(schema_obj) = schema.as_object() {
            if let Some(expected_type) = schema_obj.get("type") {
                if expected_type.as_str() == Some("object") {
                    // Validate object structure
                    return validate_object_schema(input, schema_obj);
                }
            }
        }
        Ok(())
    }

    /// Validate object schema
    fn validate_object_schema(
        input: &Value,
        schema: &serde_json::Map<String, Value>,
    ) -> Result<()> {
        // Check if input is an object
        let input_obj = input.as_object().context("Input must be a JSON object")?;

        // Check required fields
        if let Some(required) = schema.get("required") {
            if let Some(required_arr) = required.as_array() {
                for req_field in required_arr {
                    if let Some(field_name) = req_field.as_str() {
                        if !input_obj.contains_key(field_name) {
                            bail!("Missing required field: {}", field_name);
                        }
                    }
                }
            }
        }

        // Check property types
        if let Some(properties) = schema.get("properties") {
            if let Some(props_obj) = properties.as_object() {
                for (field_name, field_schema) in props_obj {
                    if let Some(input_value) = input_obj.get(field_name) {
                        validate_field_type(input_value, field_schema, field_name)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate field type
    fn validate_field_type(value: &Value, schema: &Value, field_name: &str) -> Result<()> {
        if let Some(schema_obj) = schema.as_object() {
            if let Some(expected_type) = schema_obj.get("type") {
                if let Some(type_str) = expected_type.as_str() {
                    let actual_type = match value {
                        Value::Null => "null",
                        Value::Bool(_) => "boolean",
                        Value::Number(_) => "number",
                        Value::String(_) => "string",
                        Value::Array(_) => "array",
                        Value::Object(_) => "object",
                    };

                    if type_str != actual_type {
                        bail!(
                            "Field '{}' has wrong type: expected {}, got {}",
                            field_name,
                            type_str,
                            actual_type
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract string from JSON value
    pub fn extract_string(value: &Value, field: &str) -> Result<String> {
        value
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .with_context(|| format!("Field '{}' must be a string", field))
    }

    /// Extract optional string from JSON value
    pub fn extract_optional_string(value: &Value, field: &str) -> Option<String> {
        value
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extract integer from JSON value
    pub fn extract_integer(value: &Value, field: &str) -> Result<i64> {
        value
            .get(field)
            .and_then(|v| v.as_i64())
            .with_context(|| format!("Field '{}' must be an integer", field))
    }

    /// Extract boolean from JSON value
    pub fn extract_boolean(value: &Value, field: &str) -> Result<bool> {
        value
            .get(field)
            .and_then(|v| v.as_bool())
            .with_context(|| format!("Field '{}' must be a boolean", field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_json_schema_valid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name"]
        });

        let input = json!({
            "name": "John",
            "age": 30
        });

        let result = validation::validate_json_schema(&input, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_schema_missing_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let input = json!({});

        let result = validation::validate_json_schema(&input, &schema);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required field"));
    }

    #[test]
    fn test_validate_json_schema_wrong_type() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let input = json!({
            "name": 123
        });

        let result = validation::validate_json_schema(&input, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrong type"));
    }

    #[test]
    fn test_extract_string() {
        let value = json!({"name": "John"});
        let result = validation::extract_string(&value, "name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "John");
    }

    #[test]
    fn test_extract_string_missing_field() {
        let value = json!({});
        let result = validation::extract_string(&value, "name");
        assert!(result.is_err());
    }
}
