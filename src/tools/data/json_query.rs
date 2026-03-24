//! JSON 查询工具
//!
//! 提供 JSON 路径查询、键提取等功能

use tokitai::tool;
use serde_json::{json, Value};
use std::sync::Arc;
use crate::tools::data::config::DataToolConfig;
use crate::tools::data::error::DataToolError;
use crate::tools::data::validator::{
    JsonLengthValidator, PathLengthValidator, Validator,
};
use crate::tools::data::metrics::{MetricsCollector, DataToolOperation};

/// JSON 查询工具集
#[derive(Debug)]
pub struct JsonQueryTools {
    pub config: DataToolConfig,
    pub metrics: Arc<MetricsCollector>,
}

impl JsonQueryTools {
    pub fn new() -> Self {
        Self::with_config(DataToolConfig::default())
    }

    pub fn with_config(config: DataToolConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
        }
    }

    fn parse_and_validate(&self, json_string: &str) -> Result<Value, Value> {
        JsonLengthValidator { json_string }.validate(&self.config)
            .map_err(|e| e.to_value())?;
        serde_json::from_str(json_string)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())
    }

    fn validate_path(&self, path: &str) -> Result<(), Value> {
        PathLengthValidator { path }.validate(&self.config)
            .map_err(|e| e.to_value())
    }

    fn navigate<'a>(&self, value: &'a Value, path: &str) -> Result<&'a Value, Value> {
        let mut current = value;
        for part in path.split('.') {
            current = if let Ok(index) = part.parse::<usize>() {
                current.as_array()
                    .and_then(|arr| arr.get(index))
                    .ok_or_else(|| DataToolError::path_not_found(path.to_string()).to_value())?
            } else {
                current.as_object()
                    .and_then(|obj| obj.get(part))
                    .ok_or_else(|| DataToolError::path_not_found(path.to_string()).to_value())?
            };
        }
        Ok(current)
    }

    fn collect_keys(&self, value: &Value, keys: &mut Vec<String>, depth: usize) {
        if depth > self.config.max_depth {
            return;
        }
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    keys.push(key.clone());
                    self.collect_keys(val, keys, depth + 1);
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.collect_keys(item, keys, depth + 1);
                }
            }
            _ => {}
        }
    }

    fn _query_json(&self, json_string: &str, path: &str) -> Result<Value, Value> {
        self.validate_path(path)?;
        let parsed = self.parse_and_validate(json_string)?;
        let result = self.navigate(&parsed, path)?;
        Ok(result.clone())
    }

    fn _extract_keys(&self, json_string: &str) -> Result<Value, Value> {
        let parsed = self.parse_and_validate(json_string)?;
        let mut keys = Vec::new();
        self.collect_keys(&parsed, &mut keys, 0);
        Ok(json!({
            "keys": keys,
            "count": keys.len()
        }))
    }
}

#[tool]
impl JsonQueryTools {
    /// 使用点号分隔的路径查询 JSON 数据，支持数组索引
    #[tool(
        description = "使用点号分隔的路径查询 JSON 数据，支持数组索引（如 user.name 或 data.0.id）",
        example = "查询 user.name 字段：{\"user\": {\"name\": \"Alice\"}}"
    )]
    pub fn query_json(&self, json_string: String, path: String) -> Result<Value, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::QueryJson);
        match self._query_json(&json_string, &path) {
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

    /// 递归提取 JSON 中的所有键名，返回键列表和总数
    #[tool(
        description = "递归提取 JSON 中的所有键名，返回键列表和总数",
        example = "提取所有键：{\"user\": {\"name\": \"Alice\", \"address\": {\"city\": \"Beijing\"}}}"
    )]
    pub fn extract_keys(&self, json_string: String) -> Result<Value, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::ExtractKeys);
        match self._extract_keys(&json_string) {
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
}

impl Default for JsonQueryTools {
    fn default() -> Self {
        Self::new()
    }
}
