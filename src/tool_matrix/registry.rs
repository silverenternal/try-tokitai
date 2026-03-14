//! 工具注册表
//!
//! 实现工具的注册、发现和运行时添加功能
//! 支持与 tokitai::tool 宏生成的工具集成

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::tool_matrix::matrix::{ToolDefinition, ToolBox, ToolUsageStats};

/// 工具来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSource {
    /// 内置工具（编译时注册）
    Builtin,
    /// 动态注册工具（运行时添加）
    Dynamic,
    /// 从文件加载的工具箱
    FileLoaded,
}

/// 注册的工具信息
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    /// 工具定义
    pub definition: ToolDefinition,
    /// 工具来源
    pub source: ToolSource,
    /// 所属工具箱 ID
    pub toolbox_id: Option<String>,
}

/// 工具注册表
pub struct ToolRegistry {
    /// 所有注册的工具
    tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,
    /// 工具箱集合
    toolboxes: Arc<RwLock<HashMap<String, ToolBox>>>,
    /// 工具使用统计
    usage_stats: Arc<RwLock<HashMap<String, ToolUsageStats>>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            toolboxes: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册单个工具
    pub fn register_tool(&self, tool: ToolDefinition, source: ToolSource) -> Result<()> {
        let tool_name = tool.name.clone();
        
        // 检查是否已存在
        if self.tools.read().contains_key(&tool_name) {
            anyhow::bail!("工具 {} 已存在", tool_name);
        }

        let registered_tool = RegisteredTool {
            definition: tool.clone(),
            source,
            toolbox_id: None,
        };

        self.tools.write().insert(tool_name, registered_tool);

        // 初始化使用统计
        let tool_name_for_stats = tool.name.clone();
        self.usage_stats
            .write()
            .insert(tool_name_for_stats, ToolUsageStats::new(tool.name));

        Ok(())
    }

    /// 注册工具到指定工具箱
    pub fn register_tool_to_box(
        &self,
        tool: ToolDefinition,
        toolbox_id: &str,
        source: ToolSource,
    ) -> Result<()> {
        let tool_name = tool.name.clone();

        // 检查工具箱是否存在
        if !self.toolboxes.read().contains_key(toolbox_id) {
            anyhow::bail!("工具箱 {} 不存在", toolbox_id);
        }

        // 注册工具
        self.register_tool(tool.clone(), source)?;

        // 添加到工具箱
        let mut toolboxes = self.toolboxes.write();
        if let Some(box_ref) = toolboxes.get_mut(toolbox_id) {
            box_ref.add_tool(tool);
        }

        // 更新注册信息
        if let Some(tool_ref) = self.tools.write().get_mut(&tool_name) {
            tool_ref.toolbox_id = Some(toolbox_id.to_string());
        }

        Ok(())
    }

    /// 从 tokitai ToolProvider 批量注册工具
    pub fn register_from_provider<T: tokitai::ToolProvider>(
        &self,
        toolbox_id: Option<&str>,
        source: ToolSource,
    ) -> Result<Vec<String>> {
        let definitions = T::tool_definitions();
        let mut registered = Vec::new();

        for def in definitions {
            let tool_def = ToolDefinition {
                name: def.name.clone(),
                description: def.description.clone(),
                input_schema: def.input_schema.clone(),
                tags: Vec::new(),
                risk_level: "safe".to_string(),
                source: match source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Dynamic => "dynamic".to_string(),
                    ToolSource::FileLoaded => "file_loaded".to_string(),
                },
            };

            let tool_name = tool_def.name.clone();

            // 注册工具
            self.register_tool(tool_def.clone(), source.clone())?;

            // 如果指定了工具箱，添加到工具箱
            if let Some(box_id) = toolbox_id {
                if let Some(box_ref) = self.toolboxes.write().get_mut(box_id) {
                    box_ref.add_tool(tool_def);
                }
                if let Some(tool_ref) = self.tools.write().get_mut(&tool_name) {
                    tool_ref.toolbox_id = Some(box_id.to_string());
                }
            }

            registered.push(tool_name);
        }

        Ok(registered)
    }

    /// 获取工具定义
    pub fn get_tool(&self, name: &str) -> Option<ToolDefinition> {
        self.tools
            .read()
            .get(name)
            .map(|rt| rt.definition.clone())
    }

    /// 获取所有工具
    pub fn get_all_tools(&self) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .values()
            .map(|rt| rt.definition.clone())
            .collect()
    }

    /// 获取工具箱中的所有工具
    pub fn get_tools_from_box(&self, toolbox_id: &str) -> Vec<ToolDefinition> {
        self.toolboxes
            .read()
            .get(toolbox_id)
            .map(|box_ref| box_ref.get_all_tools().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 按标签过滤工具
    pub fn filter_by_tag(&self, tag: &str) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .values()
            .filter(|rt| rt.definition.tags.contains(&tag.to_string()))
            .map(|rt| rt.definition.clone())
            .collect()
    }

    /// 按风险等级过滤工具
    pub fn filter_by_risk(&self, max_risk: &str) -> Vec<ToolDefinition> {
        let risk_order = ["safe", "medium", "dangerous"];
        let max_idx = risk_order.iter().position(|&r| r == max_risk).unwrap_or(2);

        self.tools
            .read()
            .values()
            .filter(|rt| {
                let idx = risk_order
                    .iter()
                    .position(|&r| r == rt.definition.risk_level.as_str())
                    .unwrap_or(0);
                idx <= max_idx
            })
            .map(|rt| rt.definition.clone())
            .collect()
    }

    /// 创建工具箱
    pub fn create_toolbox(&self, toolbox: ToolBox) -> Result<()> {
        let box_id = toolbox.id.clone();

        if self.toolboxes.read().contains_key(&box_id) {
            anyhow::bail!("工具箱 {} 已存在", box_id);
        }

        self.toolboxes.write().insert(box_id, toolbox);
        Ok(())
    }

    /// 获取工具箱
    pub fn get_toolbox(&self, id: &str) -> Option<ToolBox> {
        self.toolboxes.read().get(id).cloned()
    }

    /// 获取所有工具箱
    pub fn get_all_toolboxes(&self) -> Vec<ToolBox> {
        self.toolboxes.read().values().cloned().collect()
    }

    /// 删除工具箱
    pub fn remove_toolbox(&self, id: &str) -> Result<()> {
        if !self.toolboxes.read().contains_key(id) {
            anyhow::bail!("工具箱 {} 不存在", id);
        }

        // 移除工具箱中的工具引用
        if let Some(box_ref) = self.toolboxes.write().remove(id) {
            for tool_name in box_ref.tools.keys() {
                if let Some(tool_ref) = self.tools.write().get_mut(tool_name) {
                    tool_ref.toolbox_id = None;
                }
            }
        }

        Ok(())
    }

    /// 记录工具使用
    pub fn record_usage(&self, tool_name: &str, success: bool, execution_time_ms: u64) {
        if let Some(stats) = self.usage_stats.write().get_mut(tool_name) {
            stats.record_usage(success, execution_time_ms);
        }
    }

    /// 获取工具使用统计
    pub fn get_usage_stats(&self, tool_name: &str) -> Option<ToolUsageStats> {
        self.usage_stats.read().get(tool_name).cloned()
    }

    /// 获取按使用次数排序的工具
    pub fn get_popular_tools(&self, limit: usize) -> Vec<ToolDefinition> {
        let stats: Vec<ToolUsageStats> = {
            let binding = self.usage_stats.read();
            binding.values().cloned().collect()
        };
        let mut stats = stats;
        stats.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        stats
            .into_iter()
            .take(limit)
            .filter_map(|s| self.get_tool(&s.tool_name))
            .collect()
    }

    /// 检查工具是否存在
    pub fn tool_exists(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }

    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.tools.read().len()
    }

    /// 获取工具箱数量
    pub fn toolbox_count(&self) -> usize {
        self.toolboxes.read().len()
    }

    /// 清空所有工具
    pub fn clear(&self) {
        self.tools.write().clear();
        self.toolboxes.write().clear();
        self.usage_stats.write().clear();
    }
}

/// 便捷宏：从 ToolProvider 注册工具到注册表
#[macro_export]
macro_rules! register_tools {
    ($registry:expr, $($provider:ty),+ $(,)?) => {{
        let mut registered = Vec::new();
        $(
            match $registry.register_from_provider::<$provider>(None, $crate::tool_matrix::ToolSource::Builtin) {
                Ok(names) => registered.extend(names),
                Err(e) => tracing::warn!("注册工具 {:?} 失败：{}", stringify!($provider), e),
            }
        )*
        registered
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_tool() {
        let registry = ToolRegistry::new();
        let tool = ToolDefinition::new("test_tool", "A test tool", r#"{}"#);

        assert!(registry.register_tool(tool.clone(), ToolSource::Builtin).is_ok());
        assert!(registry.tool_exists("test_tool"));
        assert_eq!(registry.tool_count(), 1);
    }

    #[test]
    fn test_create_toolbox() {
        let registry = ToolRegistry::new();
        let toolbox = ToolBox::new("test_box", "Test Box", "A test toolbox");

        assert!(registry.create_toolbox(toolbox).is_ok());
        assert!(registry.get_toolbox("test_box").is_some());
        assert_eq!(registry.toolbox_count(), 1);
    }

    #[test]
    fn test_register_tool_to_box() {
        let registry = ToolRegistry::new();

        // 创建工具箱
        let toolbox = ToolBox::new("test_box", "Test Box", "A test toolbox");
        registry.create_toolbox(toolbox).unwrap();

        // 注册工具到工具箱
        let tool = ToolDefinition::new("test_tool", "A test tool", r#"{}"#);
        registry
            .register_tool_to_box(tool, "test_box", ToolSource::Builtin)
            .unwrap();

        // 验证
        assert!(registry.tool_exists("test_tool"));
        let tools = registry.get_tools_from_box("test_box");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[test]
    fn test_filter_by_tag() {
        let registry = ToolRegistry::new();

        let tool1 = ToolDefinition::new("tool1", "Tool 1", r#"{}"#).with_tag("utility");
        let tool2 = ToolDefinition::new("tool2", "Tool 2", r#"{}"#).with_tag("network");

        registry.register_tool(tool1, ToolSource::Builtin).unwrap();
        registry.register_tool(tool2, ToolSource::Builtin).unwrap();

        let utility_tools = registry.filter_by_tag("utility");
        assert_eq!(utility_tools.len(), 1);
        assert_eq!(utility_tools[0].name, "tool1");
    }
}
