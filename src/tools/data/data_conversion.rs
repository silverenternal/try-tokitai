//! 数据格式转换工具
//!
//! 提供 JSON 到 CSV 等数据格式转换功能

use tokitai::tool;
use serde_json::Value;
use std::sync::Arc;
use std::collections::HashSet;
use crate::tools::data::config::DataToolConfig;
use crate::tools::data::error::DataToolError;
use crate::tools::data::validator::{
    JsonLengthValidator, ItemCountValidator, Validator,
};
use crate::tools::data::metrics::{MetricsCollector, DataToolOperation};

/// 数据格式转换工具集
#[derive(Debug)]
pub struct DataConversionTools {
    pub config: DataToolConfig,
    pub metrics: Arc<MetricsCollector>,
}

impl DataConversionTools {
    pub fn new() -> Self {
        Self::with_config(DataToolConfig::default())
    }

    pub fn with_config(config: DataToolConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
        }
    }

    fn value_to_csv_field(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            Value::Object(_) | Value::Array(_) => value.to_string(),
        }
    }

    fn parse_json_array(&self, json_string: &str) -> Result<Vec<Value>, Value> {
        JsonLengthValidator { json_string }.validate(&self.config)
            .map_err(|e| e.to_value())?;
        let parsed: Value = serde_json::from_str(json_string)
            .map_err(|e| DataToolError::json_parse(e.to_string()).to_value())?;
        match parsed {
            Value::Array(arr) => Ok(arr),
            _ => Err(DataToolError::invalid_type("array", parsed.to_string()).to_value()),
        }
    }

    fn _json_to_csv(&self, json_string: &str) -> Result<String, Value> {
        let array = self.parse_json_array(json_string)?;
        ItemCountValidator { count: array.len() }.validate(&self.config)?;
        if array.is_empty() {
            return Ok(String::new());
        }
        let mut all_keys = Vec::new();
        let mut seen_keys = HashSet::new();
        for item in &array {
            if let Value::Object(obj) = item {
                for key in obj.keys() {
                    if seen_keys.insert(key.clone()) {
                        all_keys.push(key.clone());
                    }
                }
            }
        }
        let mut csv = String::new();
        csv.push_str(&all_keys.join(","));
        csv.push('\n');
        for item in &array {
            if let Value::Object(obj) = item {
                let row: Vec<String> = all_keys.iter()
                    .map(|key| {
                        obj.get(key)
                            .map(Self::value_to_csv_field)
                            .unwrap_or_default()
                    })
                    .collect();
                csv.push_str(&row.join(","));
                csv.push('\n');
            }
        }
        Ok(csv)
    }

    fn _batch_json_to_csv(&self, json_strings: &[String]) -> Result<Vec<String>, Value> {
        let mut results = Vec::new();
        for json_string in json_strings {
            let csv = self._json_to_csv(json_string)?;
            results.push(csv);
        }
        Ok(results)
    }
}

#[tool]
impl DataConversionTools {
    /// 将 JSON 对象数组转换为 CSV 表格格式
    #[tool(
        description = "将 JSON 对象数组转换为 CSV 表格格式",
        example = "把这个 JSON 数组转成 CSV: [{\"name\": \"Alice\", \"age\": 30}, {\"name\": \"Bob\", \"age\": 25}]"
    )]
    pub fn json_to_csv(&self, json_string: String) -> Result<String, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::JsonToCsv);
        match self._json_to_csv(&json_string) {
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

    /// 批量转换多个 JSON 数组到 CSV，返回转换结果列表
    #[tool(
        description = "批量转换多个 JSON 数组到 CSV，返回转换结果列表",
        example = "批量转换：[\"[{\\\"name\\\": \\\"Alice\\\"}]\", \"[{\\\"age\\\": 30}]\"]"
    )]
    pub fn batch_json_to_csv(&self, json_strings: Vec<String>) -> Result<Vec<String>, Value> {
        let _timer = self.metrics.start_call(DataToolOperation::JsonToCsv);
        match self._batch_json_to_csv(&json_strings) {
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

impl Default for DataConversionTools {
    fn default() -> Self {
        Self::new()
    }
}
