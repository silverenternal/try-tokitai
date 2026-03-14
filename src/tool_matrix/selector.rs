//! 动态工具选择器
//!
//! 根据查询语义、角色和上下文自动选择最相关的工具子集

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::tool_matrix::matrix::ToolDefinition;
use crate::tool_matrix::registry::ToolRegistry;

/// 工具选择结果
#[derive(Debug, Clone)]
pub struct ToolSelectionResult {
    /// 选中的工具
    pub tools: Vec<ToolDefinition>,
    /// 选择的理由
    pub reason: String,
    /// 使用的工具箱 ID 列表
    pub toolboxes_used: Vec<String>,
}

/// 工具选择器
pub struct ToolSelector {
    /// 工具注册表
    registry: ToolRegistry,
    /// 角色到工具箱的映射
    role_toolbox_mapping: HashMap<String, Vec<String>>,
    /// 关键词到工具标签的映射
    keyword_tag_mapping: HashMap<String, Vec<String>>,
}

impl Default for ToolSelector {
    fn default() -> Self {
        Self::new(ToolRegistry::new())
    }
}

impl ToolSelector {
    /// 创建新的工具选择器
    pub fn new(registry: ToolRegistry) -> Self {
        let mut selector = Self {
            registry,
            role_toolbox_mapping: HashMap::new(),
            keyword_tag_mapping: HashMap::new(),
        };

        // 初始化关键词到标签的映射
        selector.init_keyword_mapping();

        selector
    }

    /// 初始化关键词映射
    fn init_keyword_mapping(&mut self) {
        // 文件操作相关
        self.keyword_tag_mapping.insert(
            "file".to_string(),
            vec!["io".to_string(), "file".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "read".to_string(),
            vec!["read_only".to_string(), "io".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "write".to_string(),
            vec!["write".to_string(), "io".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "delete".to_string(),
            vec!["write".to_string(), "io".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "copy".to_string(),
            vec!["write".to_string(), "io".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "directory".to_string(),
            vec!["io".to_string(), "file".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "list".to_string(),
            vec!["read_only".to_string(), "io".to_string()],
        );

        // 网络相关
        self.keyword_tag_mapping.insert(
            "http".to_string(),
            vec!["http".to_string(), "network".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "request".to_string(),
            vec!["http".to_string(), "network".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "search".to_string(),
            vec!["search".to_string(), "network".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "download".to_string(),
            vec!["download".to_string(), "network".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "url".to_string(),
            vec!["http".to_string(), "network".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "web".to_string(),
            vec!["network".to_string(), "search".to_string()],
        );

        // 代码相关
        self.keyword_tag_mapping.insert(
            "code".to_string(),
            vec!["analysis".to_string(), "code".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "analyze".to_string(),
            vec!["analysis".to_string(), "code".to_string()],
        );
        self.keyword_tag_mapping.insert(
            "git".to_string(),
            vec!["vcs".to_string(), "git".to_string()],
        );
    }

    /// 根据角色选择工具
    pub fn select_tools_for_role(&self, role: &str, max_tools: usize) -> ToolSelectionResult {
        let mut selected_tools = Vec::new();
        let mut toolboxes_used = Vec::new();

        // 获取角色对应的工具箱
        if let Some(toolbox_ids) = self.role_toolbox_mapping.get(role) {
            for toolbox_id in toolbox_ids {
                let tools = self.registry.get_tools_from_box(toolbox_id);
                toolboxes_used.push(toolbox_id.clone());

                for tool in tools {
                    if selected_tools.len() >= max_tools {
                        break;
                    }
                    if !selected_tools.iter().any(|t: &ToolDefinition| t.name == tool.name) {
                        selected_tools.push(tool);
                    }
                }
            }
        } else {
            // 如果没有配置，返回所有工具
            selected_tools = self.registry.get_all_tools();
            selected_tools.truncate(max_tools);
        }

        let tool_count = selected_tools.len();
        ToolSelectionResult {
            tools: selected_tools,
            reason: format!("为角色 {} 选择了 {} 个工具", role, tool_count),
            toolboxes_used,
        }
    }

    /// 根据查询关键词选择工具
    pub fn select_tools_by_query(&self, query: &str, max_tools: usize) -> ToolSelectionResult {
        let query_lower = query.to_lowercase();
        let mut tool_scores: HashMap<String, f32> = HashMap::new();
        let mut matched_tags = Vec::new();

        // 分析查询，匹配关键词
        for (keyword, tags) in &self.keyword_tag_mapping {
            if query_lower.contains(keyword) {
                matched_tags.extend(tags.clone());
            }
        }

        // 去重
        matched_tags.sort();
        matched_tags.dedup();

        // 为每个工具评分
        for tool in self.registry.get_all_tools() {
            let mut score = 0.0;

            // 标签匹配得分
            for tag in &matched_tags {
                if tool.tags.contains(tag) {
                    score += 1.0;
                }
            }

            // 描述匹配得分
            if tool.description.to_lowercase().contains(&query_lower) {
                score += 2.0;
            }

            // 名称匹配得分
            if tool.name.to_lowercase().contains(&query_lower) {
                score += 3.0;
            }

            if score > 0.0 {
                tool_scores.insert(tool.name.clone(), score);
            }
        }

        // 按得分排序
        let mut scored_tools: Vec<_> = tool_scores.into_iter().collect();
        scored_tools.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 选择 top N 工具
        let selected_names: Vec<_> = scored_tools
            .into_iter()
            .take(max_tools)
            .map(|(name, _)| name)
            .collect();

        let selected_tools: Vec<_> = selected_names
            .iter()
            .filter_map(|name| self.registry.get_tool(name))
            .collect();

        ToolSelectionResult {
            tools: selected_tools.clone(),
            reason: format!(
                "根据查询 '{}' 匹配到 {} 个相关工具（标签：{:?}）",
                query,
                selected_tools.len(),
                matched_tags
            ),
            toolboxes_used: vec![],
        }
    }

    /// 组合选择：角色 + 查询
    pub fn select_tools_combined(
        &self,
        role: &str,
        query: &str,
        max_tools: usize,
    ) -> ToolSelectionResult {
        // 获取角色工具
        let role_result = self.select_tools_for_role(role, max_tools);

        // 获取查询匹配工具
        let query_result = self.select_tools_by_query(query, max_tools);

        // 合并结果（去重）
        let mut all_tools = role_result.tools;
        for tool in query_result.tools {
            if all_tools.len() >= max_tools {
                break;
            }
            if !all_tools.iter().any(|t| t.name == tool.name) {
                all_tools.push(tool);
            }
        }

        ToolSelectionResult {
            tools: all_tools.clone(),
            reason: format!(
                "组合选择：角色 {} + 查询 '{}' = {} 个工具",
                role,
                query,
                all_tools.len()
            ),
            toolboxes_used: role_result.toolboxes_used,
        }
    }

    /// 添加工具箱到角色的映射
    pub fn add_role_toolbox_mapping(&mut self, role: &str, toolbox_ids: Vec<String>) {
        self.role_toolbox_mapping
            .insert(role.to_string(), toolbox_ids);
    }

    /// 添加工具箱到角色的映射（批量）
    pub fn load_role_mappings(&mut self, mappings: &Value) -> Result<()> {
        if let Some(roles) = mappings.get("roles").and_then(|r| r.as_object()) {
            for (role, config) in roles {
                if let Some(toolboxes) = config.get("toolboxes").and_then(|t| t.as_array()) {
                    let toolbox_ids: Vec<String> = toolboxes
                        .iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect();
                    self.add_role_toolbox_mapping(role, toolbox_ids);
                }
            }
        }
        Ok(())
    }

    /// 获取所有可用工具
    pub fn get_all_tools(&self) -> Vec<ToolDefinition> {
        self.registry.get_all_tools()
    }

    /// 获取工具箱中的所有工具
    pub fn get_tools_from_box(&self, toolbox_id: &str) -> Vec<ToolDefinition> {
        self.registry.get_tools_from_box(toolbox_id)
    }

    /// 获取注册表引用
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

/// 过滤器配置
#[derive(Debug, Clone)]
pub struct FilterCriteria {
    /// 按角色过滤
    pub role: Option<String>,
    /// 按标签过滤
    pub tags: Vec<String>,
    /// 最大风险等级
    pub max_risk: String,
    /// 最大工具数量
    pub max_tools: usize,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            role: None,
            tags: Vec::new(),
            max_risk: "dangerous".to_string(),
            max_tools: 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_matrix::matrix::ToolBox;

    #[test]
    fn test_select_tools_by_query() {
        let registry = ToolRegistry::new();

        // 创建工具箱
        let mut file_box = ToolBox::new("file_ops", "File Operations", "File tools");
        file_box.add_tool(ToolDefinition::new("read_file", "Read a file", "{}").with_tag("io"));
        file_box.add_tool(ToolDefinition::new("write_file", "Write a file", "{}").with_tag("write"));

        registry.create_toolbox(file_box).unwrap();

        let selector = ToolSelector::new(registry);

        // 测试文件相关查询
        let result = selector.select_tools_by_query("read file", 10);
        assert!(!result.tools.is_empty());
        assert!(result.reason.contains("read"));
    }

    #[test]
    fn test_keyword_mapping() {
        let selector = ToolSelector::default();

        // 验证关键词映射已初始化
        assert!(selector.keyword_tag_mapping.contains_key("file"));
        assert!(selector.keyword_tag_mapping.contains_key("http"));
    }
}
