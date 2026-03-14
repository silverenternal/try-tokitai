//! 工具矩阵结构定义
//!
//! 支持动态工具箱和 Skills 文件的设计理念：
//! - 工具箱（ToolBox）：按领域分类的工具集合（如文件操作箱、网络工具箱）
//! - Skills 文件：每个工具箱的"说明书"，告诉 AI 如何正确使用工具
//! - 动态注册：支持运行时添加新工具到工具箱

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// 工具定义（与 tokitai 兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称（唯一标识）
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入参数 schema（JSON Schema）
    pub input_schema: String,
    /// 工具标签（用于分类和过滤）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 风险等级（safe/medium/dangerous）
    #[serde(default = "default_risk_level")]
    pub risk_level: String,
    /// 工具来源（内置/动态注册）
    #[serde(default = "default_tool_source")]
    pub source: String,
}

fn default_risk_level() -> String {
    "safe".to_string()
}

fn default_tool_source() -> String {
    "builtin".to_string()
}

impl ToolDefinition {
    /// 创建新的工具定义
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: input_schema.into(),
            tags: Vec::new(),
            risk_level: "safe".to_string(),
            source: "builtin".to_string(),
        }
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 设置风险等级
    pub fn with_risk_level(mut self, level: impl Into<String>) -> Self {
        self.risk_level = level.into();
        self
    }

    /// 设置工具来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// 转换为 AI API 格式
    pub fn to_api_format(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": serde_json::from_str::<Value>(&self.input_schema).unwrap_or_default()
            }
        })
    }
}

/// 工具箱 - 按领域分类的工具集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBox {
    /// 工具箱唯一标识
    pub id: String,
    /// 工具箱名称
    pub name: String,
    /// 工具箱描述
    pub description: String,
    /// 工具箱包含的工具
    pub tools: HashMap<String, ToolDefinition>,
    /// 关联的角色（Planner/Executor 等）
    #[serde(default)]
    pub roles: Vec<String>,
    /// 工具箱标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 创建时间
    #[serde(default)]
    pub created_at: Option<String>,
    /// 更新时间
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_true() -> bool {
    true
}

impl ToolBox {
    /// 创建新的工具箱
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tools: HashMap::new(),
            roles: Vec::new(),
            tags: Vec::new(),
            enabled: true,
            created_at: None,
            updated_at: None,
        }
    }

    /// 添加工具
    pub fn add_tool(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
        self.updated_at = Some(chrono::Local::now().to_rfc3339());
    }

    /// 移除工具
    pub fn remove_tool(&mut self, tool_name: &str) -> Option<ToolDefinition> {
        let tool = self.tools.remove(tool_name);
        if tool.is_some() {
            self.updated_at = Some(chrono::Local::now().to_rfc3339());
        }
        tool
    }

    /// 获取工具
    pub fn get_tool(&self, tool_name: &str) -> Option<&ToolDefinition> {
        self.tools.get(tool_name)
    }

    /// 获取所有工具
    pub fn get_all_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// 按标签过滤工具
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .collect()
    }

    /// 按风险等级过滤工具
    pub fn filter_by_risk(&self, max_risk: &str) -> Vec<&ToolDefinition> {
        let risk_order = ["safe", "medium", "dangerous"];
        let max_idx = risk_order.iter().position(|&r| r == max_risk).unwrap_or(2);

        self.tools
            .values()
            .filter(|t| {
                let idx = risk_order.iter().position(|&r| r == t.risk_level.as_str()).unwrap_or(0);
                idx <= max_idx
            })
            .collect()
    }
}

/// Skills 文件 - 工具箱的使用说明书
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsFile {
    /// 关联的工具箱 ID
    pub toolbox_id: String,
    /// Skills 文件名称
    pub name: String,
    /// 工具箱简介
    pub introduction: String,
    /// 使用场景说明
    pub use_cases: Vec<UseCase>,
    /// 工具使用指南
    pub tool_guides: Vec<ToolGuide>,
    /// 最佳实践
    pub best_practices: Vec<String>,
    /// 注意事项/警告
    pub warnings: Vec<String>,
    /// 示例
    pub examples: Vec<SkillExample>,
    /// 版本号
    pub version: String,
    /// 更新时间
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// 使用场景
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    /// 场景名称
    pub name: String,
    /// 场景描述
    pub description: String,
    /// 推荐使用的工具
    pub recommended_tools: Vec<String>,
}

/// 工具使用指南
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGuide {
    /// 工具名称
    pub tool_name: String,
    /// 功能说明
    pub description: String,
    /// 使用示例
    pub examples: Vec<String>,
    /// 参数说明
    pub parameters: Vec<ParameterDoc>,
    /// 返回值说明
    pub returns: Option<String>,
    /// 注意事项
    pub notes: Vec<String>,
}

/// 参数文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDoc {
    /// 参数名称
    pub name: String,
    /// 参数类型
    pub param_type: String,
    /// 是否必填
    pub required: bool,
    /// 参数说明
    pub description: String,
}

/// 技能示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    /// 示例标题
    pub title: String,
    /// 用户输入
    pub user_input: String,
    /// 推荐工具调用序列
    pub tool_sequence: Vec<ToolCallExample>,
    /// 预期输出
    pub expected_output: String,
}

/// 工具调用示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallExample {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub arguments: Value,
    /// 说明
    pub note: Option<String>,
}

impl SkillsFile {
    /// 创建新的 Skills 文件
    pub fn new(
        toolbox_id: impl Into<String>,
        name: impl Into<String>,
        introduction: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            toolbox_id: toolbox_id.into(),
            name: name.into(),
            introduction: introduction.into(),
            use_cases: Vec::new(),
            tool_guides: Vec::new(),
            best_practices: Vec::new(),
            warnings: Vec::new(),
            examples: Vec::new(),
            version: version.into(),
            updated_at: None,
        }
    }

    /// 添加工具指南
    pub fn add_tool_guide(&mut self, guide: ToolGuide) {
        self.tool_guides.push(guide);
        self.updated_at = Some(chrono::Local::now().to_rfc3339());
    }

    /// 添加使用场景
    pub fn add_use_case(&mut self, use_case: UseCase) {
        self.use_cases.push(use_case);
        self.updated_at = Some(chrono::Local::now().to_rfc3339());
    }

    /// 添加示例
    pub fn add_example(&mut self, example: SkillExample) {
        self.examples.push(example);
        self.updated_at = Some(chrono::Local::now().to_rfc3339());
    }

    /// 生成 AI 可读的 Skills 提示词
    pub fn to_prompt(&self) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!("# {}\n\n", self.name));
        prompt.push_str(&format!("{}\n\n", self.introduction));

        if !self.use_cases.is_empty() {
            prompt.push_str("## 使用场景\n\n");
            for uc in &self.use_cases {
                prompt.push_str(&format!(
                    "### {}\n{}\n推荐工具：{}\n\n",
                    uc.name,
                    uc.description,
                    uc.recommended_tools.join(", ")
                ));
            }
        }

        if !self.tool_guides.is_empty() {
            prompt.push_str("## 工具使用指南\n\n");
            for guide in &self.tool_guides {
                prompt.push_str(&format!("### {}\n", guide.tool_name));
                prompt.push_str(&format!("{}\n\n", guide.description));

                if !guide.examples.is_empty() {
                    prompt.push_str("示例：\n");
                    for (i, ex) in guide.examples.iter().enumerate() {
                        prompt.push_str(&format!("{}. {}\n", i + 1, ex));
                    }
                    prompt.push('\n');
                }

                if !guide.notes.is_empty() {
                    prompt.push_str("注意：\n");
                    for note in &guide.notes {
                        prompt.push_str(&format!("- {}\n", note));
                    }
                    prompt.push('\n');
                }
            }
        }

        if !self.best_practices.is_empty() {
            prompt.push_str("## 最佳实践\n\n");
            for (i, practice) in self.best_practices.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, practice));
            }
            prompt.push('\n');
        }

        if !self.warnings.is_empty() {
            prompt.push_str("## ⚠️ 警告\n\n");
            for warning in &self.warnings {
                prompt.push_str(&format!("- {}\n", warning));
            }
            prompt.push('\n');
        }

        if !self.examples.is_empty() {
            prompt.push_str("## 完整示例\n\n");
            for ex in &self.examples {
                prompt.push_str(&format!("### {}\n", ex.title));
                prompt.push_str(&format!("用户：{}\n\n", ex.user_input));
                prompt.push_str("工具调用序列：\n");
                for (i, call) in ex.tool_sequence.iter().enumerate() {
                    prompt.push_str(&format!(
                        "{}. `{}` - {}\n",
                        i + 1,
                        call.tool_name,
                        call.note.as_deref().unwrap_or("")
                    ));
                }
                prompt.push('\n');
            }
        }

        prompt
    }
}

/// 工具使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    /// 工具名称
    pub tool_name: String,
    /// 使用次数
    pub usage_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub failure_count: u64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f64,
    /// 最后使用时间
    pub last_used_at: Option<String>,
}

impl ToolUsageStats {
    /// 创建新的统计
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            usage_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_execution_time_ms: 0.0,
            last_used_at: None,
        }
    }

    /// 记录使用
    pub fn record_usage(&mut self, success: bool, execution_time_ms: u64) {
        self.usage_count += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        // 更新平均执行时间
        let total_time = self.avg_execution_time_ms * (self.usage_count - 1) as f64
            + execution_time_ms as f64;
        self.avg_execution_time_ms = total_time / self.usage_count as f64;

        self.last_used_at = Some(chrono::Local::now().to_rfc3339());
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.usage_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.usage_count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_definition_creation() {
        let tool = ToolDefinition::new("test_tool", "A test tool", r#"{"type": "object"}"#)
            .with_tag("utility")
            .with_risk_level("safe");

        assert_eq!(tool.name, "test_tool");
        assert!(tool.tags.contains(&"utility".to_string()));
        assert_eq!(tool.risk_level, "safe");
    }

    #[test]
    fn test_toolbox_operations() {
        let mut toolbox = ToolBox::new("test_box", "Test Box", "A test toolbox");

        let tool = ToolDefinition::new("tool1", "Tool 1", r#"{}"#);
        toolbox.add_tool(tool);

        assert_eq!(toolbox.tool_count(), 1);
        assert!(toolbox.get_tool("tool1").is_some());
        assert!(toolbox.get_tool("nonexistent").is_none());
    }

    #[test]
    fn test_skills_file_to_prompt() {
        let mut skills = SkillsFile::new(
            "test_box",
            "Test Skills",
            "Introduction to test skills",
            "1.0.0",
        );

        skills.add_tool_guide(ToolGuide {
            tool_name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            examples: vec!["Example 1".to_string()],
            parameters: vec![],
            returns: None,
            notes: vec!["Be careful".to_string()],
        });

        let prompt = skills.to_prompt();
        assert!(prompt.contains("Test Skills"));
        assert!(prompt.contains("test_tool"));
        assert!(prompt.contains("Be careful"));
    }
}
