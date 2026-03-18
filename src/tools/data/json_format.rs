//! JSON 格式化工具
//!
//! 提供 JSON 格式化、压缩、验证等基础格式处理功能

use tokitai::tool;
use serde_json::{json, Value};
use std::sync::Arc;
use crate::tools::data::config::DataToolConfig;
use crate::tools::data::error::DataToolError;
use crate::tools::data::validator::{JsonLengthValidator, JsonDepthValidator, Validator};
use crate::tools::data::metrics::{MetricsCollector, DataToolOperation};

/// JSON 格式化工具集
#[derive(Debug)]
pub struct JsonFormatTools {
    pub config: DataToolConfig,
    pub metrics: Arc<MetricsCollector>,
}

impl JsonFormatTools {
    pub fn new() -> Self {
        Self::with_config(DataToolConfig::default())
    }

    pub fn with_config(config: DataToolConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
        }
    }
}

impl JsonFormatTools {
    /// 解析并验证 JSON
    fn parse_and_validate(&self, json_string: &str) -> Result<Value, Value> {
        JsonLengthValidator { json_string }.validate(&self.config)
            .map_err(|e| e.to_value())?;

        let parsed: Value = serde_json::from_str(json_string)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())?;

        JsonDepthValidator { value: &parsed }.validate(&self.config)
            .map_err(|e| e.to_value())?;

        Ok(parsed)
    }

    fn _format_json(&self, json_string: &str) -> Result<String, Value> {
        let parsed = self.parse_and_validate(json_string)?;
        let formatted = serde_json::to_string_pretty(&parsed)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())?;
        Ok(formatted)
    }

    fn _minify_json(&self, json_string: &str) -> Result<String, Value> {
        JsonLengthValidator { json_string }.validate(&self.config)
            .map_err(|e| e.to_value())?;

        let parsed: Value = serde_json::from_str(json_string)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())?;

        let minified = serde_json::to_string(&parsed)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())?;
        Ok(minified)
    }

    fn _validate_json(&self, json_string: &str) -> Result<Value, Value> {
        JsonLengthValidator { json_string }.validate(&self.config)
            .map_err(|e| e.to_value())?;

        match serde_json::from_str::<Value>(json_string) {
            Ok(_) => Ok(json!({
                "valid": true,
                "message": "JSON 格式有效"
            })),
            Err(e) => Err(DataToolError::json_parse(e.to_string()).to_value()),
        }
    }
}

#[tool]
impl JsonFormatTools {
    /// 格式化 JSON 字符串，添加缩进和换行使其易于阅读
    #[tool(
        description = "格式化 JSON 字符串，添加缩进和换行使其易于阅读",
        example = "格式化这个 JSON: {\"name\":\"Alice\",\"age\":30}"
    )]
    pub fn format_json(&self, json_string: String) -> Result<String, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::FormatJson);

        match self._format_json(&json_string) {
            Ok(result) => {
                _timer.success();
                Ok(result)
            }
            Err(e) => {
                _timer.failure(&e.to_string());
                Err(e)
            }
        }
    }

    /// 压缩 JSON 字符串，移除所有空白字符减小体积
    #[tool(
        description = "压缩 JSON 字符串，移除所有空白字符减小体积",
        example = "压缩这个 JSON: {\\n  \"name\": \"Alice\"\\n}"
    )]
    pub fn minify_json(&self, json_string: String) -> Result<String, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::MinifyJson);

        match self._minify_json(&json_string) {
            Ok(result) => {
                _timer.success();
                Ok(result)
            }
            Err(e) => {
                _timer.failure(&e.to_string());
                Err(e)
            }
        }
    }

    /// 验证 JSON 格式是否有效，返回验证结果
    #[tool(
        description = "验证 JSON 格式是否有效，返回验证结果",
        example = "验证这个 JSON 是否有效：{\"name\": \"Alice\"}"
    )]
    pub fn validate_json(&self, json_string: String) -> Result<Value, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::ValidateJson);

        match self._validate_json(&json_string) {
            Ok(result) => {
                _timer.success();
                Ok(result)
            }
            Err(e) => {
                _timer.failure(&e.to_string());
                Ok(json!({
                    "valid": false,
                    "error": e
                }))
            }
        }
    }
}

impl Default for JsonFormatTools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_json() {
        let tools = JsonFormatTools::new();
        let input = r#"{"name":"Alice","age":30}"#;
        let result = tools.format_json(input.to_string()).unwrap();
        assert!(result.contains('\n'));
        assert!(result.contains("\"name\": \"Alice\""));
    }

    #[test]
    fn test_minify_json() {
        let tools = JsonFormatTools::new();
        let input = r#"{
            "name": "Alice",
            "age": 30
        }"#;
        let result = tools.minify_json(input.to_string()).unwrap();
        assert!(!result.contains('\n'));
        // 验证 JSON 语义等价，而不是字符串完全相同（键顺序可能不同）
        let result_json: serde_json::Value = serde_json::from_str(&result).unwrap();
        let expected_json: serde_json::Value = serde_json::from_str(r#"{"name":"Alice","age":30}"#).unwrap();
        assert_eq!(result_json, expected_json);
    }

    #[test]
    fn test_validate_json_valid() {
        let tools = JsonFormatTools::new();
        let result = tools.validate_json(r#"{"valid": true}"#.to_string()).unwrap();
        assert_eq!(result.get("valid").unwrap(), true);
    }

    #[test]
    fn test_validate_json_invalid() {
        let tools = JsonFormatTools::new();
        let result = tools.validate_json("invalid json".to_string()).unwrap();
        assert_eq!(result.get("valid").unwrap(), false);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_format_json_depth_limit() {
        let tools = JsonFormatTools::with_config(
            DataToolConfig::builder().max_depth(10).build()
        );
        let mut deep = String::from("1");
        for _ in 0..15 {
            deep = format!("[{}]", deep);
        }
        let result = tools.format_json(deep);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_json_length_limit() {
        let tools = JsonFormatTools::with_config(
            DataToolConfig::builder().max_length(100).build()
        );
        let long_json = format!("{{\"data\": \"{}\"}}", "a".repeat(101));
        let result = tools.format_json(long_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_metrics_recording() {
        let tools = JsonFormatTools::new();

        let _ = tools.format_json(r#"{"a":1}"#.to_string());
        let _ = tools.format_json(r#"{"b":2}"#.to_string());
        let _ = tools.minify_json(r#"{"c":3}"#.to_string());

        let metrics = tools.metrics.get_metrics(DataToolOperation::FormatJson).unwrap();
        assert_eq!(metrics.total_calls, 2);
        assert_eq!(metrics.successful_calls, 2);
    }
}
