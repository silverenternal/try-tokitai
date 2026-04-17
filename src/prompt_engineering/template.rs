//! 提示词模板结构定义
//!
//! 定义 PromptTemplate 及其相关结构，支持变量替换和条件渲染

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 提示词模板变量定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// 变量名称
    pub name: String,
    /// 变量类型 (string, array, object)
    #[serde(default = "default_var_type")]
    pub var_type: String,
    /// 变量描述
    #[serde(default)]
    pub description: String,
    /// 默认值
    #[serde(default)]
    pub default: Option<Value>,
}

fn default_var_type() -> String {
    "string".to_string()
}

/// 示例对话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// 用户输入
    pub user: String,
    /// AI 响应
    pub assistant: String,
}

/// 提示词模板结构
///
/// 支持 Mustache 风格变量替换和条件渲染
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板唯一标识
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 适用角色 (Planner/Executor/Reviewer/Researcher)
    pub role: String,
    /// 系统提示词
    pub system_prompt: String,
    /// 变量定义
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// 示例对话
    #[serde(default)]
    pub examples: Vec<Example>,
    /// 约束条件
    #[serde(default)]
    pub constraints: Vec<String>,
    /// 版本号
    pub version: String,
    /// 创建时间
    #[serde(default)]
    pub created_at: Option<String>,
    /// 更新时间
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl PromptTemplate {
    /// 创建新的提示词模板
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        role: impl Into<String>,
        system_prompt: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role: role.into(),
            system_prompt: system_prompt.into(),
            variables: Vec::new(),
            examples: Vec::new(),
            constraints: Vec::new(),
            version: version.into(),
            created_at: None,
            updated_at: None,
        }
    }

    /// 添加变量
    pub fn with_variable(mut self, var: Variable) -> Self {
        self.variables.push(var);
        self
    }

    /// 添加示例
    pub fn with_example(mut self, example: Example) -> Self {
        self.examples.push(example);
        self
    }

    /// 添加约束条件
    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    /// 获取模板 ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 获取角色名称
    pub fn role(&self) -> &str {
        &self.role
    }

    /// 获取系统提示词
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// 检查模板是否包含某个变量
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.iter().any(|v| v.name == name)
    }

    /// 获取变量的默认值
    pub fn get_variable_default(&self, name: &str) -> Option<&Value> {
        self.variables
            .iter()
            .find(|v| v.name == name)
            .and_then(|v| v.default.as_ref())
    }
}

/// 工具定义结构（用于提示词中插入工具信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入参数 schema（JSON Schema）
    pub input_schema: String,
}

impl ToolDefinition {
    /// 创建工具定义
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: input_schema.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template_creation() {
        let template = PromptTemplate::new(
            "test_id",
            "Test Template",
            "Planner",
            "You are a planner",
            "1.0.0",
        );

        assert_eq!(template.id(), "test_id");
        assert_eq!(template.role(), "Planner");
        assert_eq!(template.name, "Test Template");
    }

    #[test]
    fn test_prompt_template_with_variable() {
        let var = Variable {
            name: "tools".to_string(),
            var_type: "string".to_string(),
            description: "Available tools".to_string(),
            default: None,
        };

        let template =
            PromptTemplate::new("test_id", "Test", "Planner", "Prompt", "1.0.0").with_variable(var);

        assert!(template.has_variable("tools"));
        assert!(!template.has_variable("nonexistent"));
    }

    #[test]
    fn test_prompt_template_serialization() {
        let template = PromptTemplate::new(
            "test_id",
            "Test",
            "Executor",
            "You are an executor",
            "1.0.0",
        );

        let json = serde_json::to_string(&template).unwrap();
        let parsed: PromptTemplate = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, template.id);
        assert_eq!(parsed.role, template.role);
    }
}
