//! 数据工具通用验证器
//!
//! 提取通用验证逻辑，支持复用和组合

use crate::tools::data::config::DataToolConfig;
use crate::tools::data::error::{DataToolError, DataToolResult};
use serde_json::Value;

/// 验证器 trait
pub trait Validator {
    /// 验证输入数据
    fn validate(&self, config: &DataToolConfig) -> DataToolResult<()>;
}

/// JSON 长度验证器
pub struct JsonLengthValidator<'a> {
    pub json_string: &'a str,
}

impl<'a> Validator for JsonLengthValidator<'a> {
    fn validate(&self, config: &DataToolConfig) -> DataToolResult<()> {
        if self.json_string.len() > config.max_length {
            return Err(DataToolError::resource_exceeded(
                "JSON 长度",
                self.json_string.len(),
                config.max_length,
            ));
        }
        Ok(())
    }
}

/// JSON 深度验证器
pub struct JsonDepthValidator<'a> {
    pub value: &'a Value,
}

impl<'a> Validator for JsonDepthValidator<'a> {
    fn validate(&self, config: &DataToolConfig) -> DataToolResult<()> {
        Self::check_depth(self.value, 0, config.max_depth)
    }
}

impl<'a> JsonDepthValidator<'a> {
    fn check_depth(value: &Value, current: usize, max: usize) -> DataToolResult<()> {
        if current > max {
            return Err(DataToolError::resource_exceeded(
                "JSON 深度",
                current,
                max,
            ));
        }

        match value {
            Value::Object(obj) => {
                for val in obj.values() {
                    Self::check_depth(val, current + 1, max)?;
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::check_depth(item, current + 1, max)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// 路径长度验证器
pub struct PathLengthValidator<'a> {
    pub path: &'a str,
}

impl<'a> Validator for PathLengthValidator<'a> {
    fn validate(&self, config: &DataToolConfig) -> DataToolResult<()> {
        if self.path.len() > config.max_path_length {
            return Err(DataToolError::resource_exceeded(
                "路径长度",
                self.path.len(),
                config.max_path_length,
            ));
        }
        Ok(())
    }
}

/// 合并数量验证器
pub struct MergeCountValidator {
    pub count: usize,
}

impl Validator for MergeCountValidator {
    fn validate(&self, config: &DataToolConfig) -> DataToolResult<()> {
        if self.count > config.max_merge_count {
            return Err(DataToolError::resource_exceeded(
                "合并数量",
                self.count,
                config.max_merge_count,
            ));
        }
        Ok(())
    }
}

/// 数组项数验证器
pub struct ItemCountValidator {
    pub count: usize,
}

impl Validator for ItemCountValidator {
    fn validate(&self, config: &DataToolConfig) -> DataToolResult<()> {
        if self.count > config.max_items {
            return Err(DataToolError::resource_exceeded(
                "数组项数",
                self.count,
                config.max_items,
            ));
        }
        Ok(())
    }
}

/// 组合验证器 - 按顺序执行多个验证器
#[allow(dead_code)]
pub struct CompositeValidator<'a> {
    validators: Vec<Box<dyn Validator + 'a>>,
}

#[allow(dead_code)]
impl<'a> std::fmt::Debug for CompositeValidator<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeValidator")
            .field("validators_count", &self.validators.len())
            .finish()
    }
}

#[allow(dead_code)]
impl<'a> CompositeValidator<'a> {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    pub fn add<V: Validator + 'a>(&mut self, validator: V) {
        self.validators.push(Box::new(validator));
    }

    pub fn validate(&self, config: &DataToolConfig) -> DataToolResult<()> {
        for validator in &self.validators {
            validator.validate(config)?;
        }
        Ok(())
    }
}

impl<'a> Default for CompositeValidator<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// 验证器辅助函数
#[allow(dead_code)]
pub fn validate_json(
    json_string: &str,
    config: &DataToolConfig,
) -> DataToolResult<Value> {
    // 验证长度
    JsonLengthValidator { json_string }.validate(config)?;

    // 解析 JSON
    let parsed: Value = serde_json::from_str(json_string)
        .map_err(|e| DataToolError::json_parse(e.to_string()))?;

    // 验证深度
    JsonDepthValidator { value: &parsed }.validate(config)?;

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_length_validator() {
        let config = DataToolConfig::builder()
            .max_length(100)
            .build();

        let validator = JsonLengthValidator {
            json_string: &"a".repeat(101),
        };
        assert!(validator.validate(&config).is_err());

        let validator = JsonLengthValidator {
            json_string: &"a".repeat(50),
        };
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_json_depth_validator() {
        let config = DataToolConfig::builder()
            .max_depth(5)
            .build();

        // 创建深度超限的 JSON
        let mut deep = Value::from(1);
        for _ in 0..6 {
            deep = Value::Array(vec![deep.clone()]);
        }

        let validator = JsonDepthValidator { value: &deep };
        assert!(validator.validate(&config).is_err());

        // 正常深度的 JSON
        let shallow = serde_json::json!({"a": 1});
        let validator = JsonDepthValidator { value: &shallow };
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_path_length_validator() {
        let config = DataToolConfig::builder()
            .max_path_length(10)
            .build();

        let validator = PathLengthValidator {
            path: "very.long.path",
        };
        assert!(validator.validate(&config).is_err());

        let validator = PathLengthValidator { path: "short" };
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_merge_count_validator() {
        let config = DataToolConfig::builder()
            .max_merge_count(5)
            .build();

        let validator = MergeCountValidator { count: 6 };
        assert!(validator.validate(&config).is_err());

        let validator = MergeCountValidator { count: 3 };
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_composite_validator() {
        let config = DataToolConfig::builder()
            .max_length(100)
            .max_path_length(10)
            .build();

        let text = "a".repeat(50);
        let mut composite = CompositeValidator::new();
        composite.add(JsonLengthValidator {
            json_string: &text,
        });
        composite.add(PathLengthValidator { path: "short" });

        assert!(composite.validate(&config).is_ok());

        let long_text = "a".repeat(101);
        let mut composite = CompositeValidator::new();
        composite.add(JsonLengthValidator {
            json_string: &long_text,
        });
        composite.add(PathLengthValidator { path: "short" });

        assert!(composite.validate(&config).is_err());
    }

    #[test]
    fn test_validate_json_helper() {
        let config = DataToolConfig::builder()
            .max_length(100)
            .max_depth(10)
            .build();

        let valid_json = r#"{"name": "Alice"}"#;
        let result = validate_json(valid_json, &config);
        assert!(result.is_ok());

        let invalid_json = "not valid json";
        let result = validate_json(invalid_json, &config);
        assert!(result.is_err());

        let too_long = format!(r#"{{"data": "{}"}}"#, "a".repeat(101));
        let result = validate_json(&too_long, &config);
        assert!(result.is_err());
    }
}
