//! JSON 合并工具
//!
//! 提供多个 JSON 对象合并功能

use tokitai::tool;
use serde_json::Value;
use std::sync::Arc;
use crate::tools::data::config::DataToolConfig;
use crate::tools::data::error::DataToolError;
use crate::tools::data::validator::{
    JsonLengthValidator, MergeCountValidator, Validator,
};
use crate::tools::data::metrics::{MetricsCollector, DataToolOperation};

/// JSON 合并工具集
#[derive(Debug)]
pub struct JsonMergeTools {
    pub config: DataToolConfig,
    pub metrics: Arc<MetricsCollector>,
}

impl JsonMergeTools {
    pub fn new() -> Self {
        Self::with_config(DataToolConfig::default())
    }

    pub fn with_config(config: DataToolConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
        }
    }

    fn validate_length(&self, json_string: &str) -> Result<(), Value> {
        JsonLengthValidator { json_string }.validate(&self.config)
            .map_err(|e| e.to_value())
    }

    fn parse_json(&self, json_string: &str) -> Result<Value, Value> {
        self.validate_length(json_string)?;
        serde_json::from_str(json_string)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())
    }

    fn _merge_json(&self, json_strings: &[String]) -> Result<Value, Value> {
        MergeCountValidator { count: json_strings.len() }.validate(&self.config)?;
        let mut merged = Value::Object(serde_json::Map::new());
        for json_string in json_strings {
            let parsed = self.parse_json(json_string)?;
            if let Value::Object(obj) = parsed {
                if let Value::Object(merged_obj) = &mut merged {
                    for (key, value) in obj {
                        merged_obj.insert(key, value);
                    }
                }
            }
        }
        Ok(merged)
    }

    fn _merge_json_with_defaults(&self, json_strings: &[String], defaults: &str) -> Result<Value, Value> {
        let default_value = self.parse_json(defaults)?;
        let mut result = self._merge_json(json_strings)?;
        if let (Value::Object(result_obj), Value::Object(default_obj)) = (&mut result, default_value) {
            for (key, value) in default_obj {
                if !result_obj.contains_key(&key) {
                    result_obj.insert(key, value);
                }
            }
        }
        Ok(result)
    }
}

#[tool]
impl JsonMergeTools {
    /// 合并多个 JSON 对象，后面的值会覆盖前面的同名键
    #[tool(
        description = "将多个 JSON 对象合并为一个，后面的值会覆盖前面的同名键",
        example = "合并：[{\"name\": \"Alice\"}, {\"age\": 30}]"
    )]
    pub fn merge_json(&self, json_strings: Vec<String>) -> Result<Value, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::MergeJson);
        match self._merge_json(&json_strings) {
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

    /// 合并 JSON 对象，使用默认值填充缺失的键
    #[tool(
        description = "合并 JSON 对象，使用默认值填充缺失的键",
        example = "合并：[{\"name\": \"Alice\"}] 默认值：{\"age\": 0}"
    )]
    pub fn merge_json_with_defaults(&self, json_strings: Vec<String>, defaults: String) -> Result<Value, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::MergeJson);
        match self._merge_json_with_defaults(&json_strings, &defaults) {
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

impl Default for JsonMergeTools {
    fn default() -> Self {
        Self::new()
    }
}
