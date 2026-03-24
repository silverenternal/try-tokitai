//! tokitai 工具元数据增强
//!
//! 提供工具元数据的自动提取和增强功能：
//! - 从 `#[tool]` 宏生成的代码提取元数据
//! - 自动推断工具分类和标签
//! - 生成工具依赖关系建议
//!
//! # 使用示例
//!
//! ```rust,ignore

#![allow(dead_code)]
//! use crate::tool_matrix::metadata_enhancer::MetadataEnhancer;
//! use crate::tools::FileOperations;
//! use tokitai::ToolProvider;
//!
//! // 获取工具定义
//! let tools = FileOperations::tool_definitions();
//!
//! // 增强元数据
//! let enhancer = MetadataEnhancer::new();
//! let enhanced_tools = enhancer.enhance_all(tools);
//!
//! // 现在工具包含更丰富的元数据，可用于 AI 分类和搜索
//! ```

use crate::tool_matrix::matrix::{ToolDefinition, ServiceCategory};
use std::collections::HashMap;

/// 元数据增强器
pub struct MetadataEnhancer {
    /// 关键词到分类的映射
    keyword_category_map: HashMap<&'static str, ServiceCategory>,
    /// 关键词到标签的映射
    keyword_tag_map: HashMap<&'static str, Vec<&'static str>>,
}

impl Default for MetadataEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataEnhancer {
    /// 创建新的增强器
    pub fn new() -> Self {
        let mut keyword_category_map = HashMap::new();
        let mut keyword_tag_map = HashMap::new();

        // 文件操作相关
        keyword_category_map.insert("file", ServiceCategory::File);
        keyword_category_map.insert("read", ServiceCategory::File);
        keyword_category_map.insert("write", ServiceCategory::File);
        keyword_category_map.insert("directory", ServiceCategory::File);
        keyword_category_map.insert("path", ServiceCategory::File);

        // 网络相关
        keyword_category_map.insert("http", ServiceCategory::Network);
        keyword_category_map.insert("url", ServiceCategory::Network);
        keyword_category_map.insert("download", ServiceCategory::Network);
        keyword_category_map.insert("upload", ServiceCategory::Network);
        keyword_category_map.insert("request", ServiceCategory::Network);

        // 系统相关
        keyword_category_map.insert("process", ServiceCategory::System);
        keyword_category_map.insert("command", ServiceCategory::System);
        keyword_category_map.insert("shell", ServiceCategory::System);
        keyword_category_map.insert("env", ServiceCategory::System);

        // 数据相关
        keyword_category_map.insert("json", ServiceCategory::Data);
        keyword_category_map.insert("parse", ServiceCategory::Data);
        keyword_category_map.insert("serialize", ServiceCategory::Data);

        // 代码相关
        keyword_category_map.insert("code", ServiceCategory::Development);
        keyword_category_map.insert("analyze", ServiceCategory::Development);
        keyword_category_map.insert("build", ServiceCategory::Development);
        keyword_category_map.insert("compile", ServiceCategory::Development);

        // Git 相关
        keyword_category_map.insert("git", ServiceCategory::VersionControl);
        keyword_category_map.insert("commit", ServiceCategory::VersionControl);
        keyword_category_map.insert("push", ServiceCategory::VersionControl);
        keyword_category_map.insert("pull", ServiceCategory::VersionControl);

        // 关键词到标签的映射
        keyword_tag_map.insert("file", vec!["file", "io"]);
        keyword_tag_map.insert("read", vec!["read_only", "io"]);
        keyword_tag_map.insert("write", vec!["write", "io"]);
        keyword_tag_map.insert("http", vec!["http", "network"]);
        keyword_tag_map.insert("url", vec!["url", "network"]);
        keyword_tag_map.insert("json", vec!["json", "data"]);
        keyword_tag_map.insert("git", vec!["git", "vcs"]);

        Self {
            keyword_category_map,
            keyword_tag_map,
        }
    }

    /// 增强单个工具的元数据
    pub fn enhance(&self, mut tool: ToolDefinition) -> ToolDefinition {
        // 1. 从名称和描述推断分类（总是更新分类）
        tool.metadata.category = self.infer_category(&tool.name, &tool.description);

        // 2. 从名称和描述提取标签
        let mut tags = self.extract_tags(&tool.name, &tool.description);
        tags.extend(tool.tags.clone());
        tags.sort();
        tags.dedup();
        tool.tags = tags;

        // 3. 推断风险等级
        tool.risk_level = self.infer_risk_level(&tool.name, &tool.description);

        tool
    }

    /// 增强所有工具
    pub fn enhance_all(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition> {
        tools.into_iter().map(|t| self.enhance(t)).collect()
    }

    /// 从名称和描述推断分类
    fn infer_category(&self, name: &str, description: &str) -> ServiceCategory {
        let text = format!("{} {}", name.to_lowercase(), description.to_lowercase());

        // 按优先级检查关键词
        for (keyword, category) in &self.keyword_category_map {
            if text.contains(keyword) {
                return category.clone();
            }
        }

        ServiceCategory::Utility
    }

    /// 从名称和描述提取标签
    fn extract_tags(&self, name: &str, description: &str) -> Vec<String> {
        let text = format!("{} {}", name.to_lowercase(), description.to_lowercase());
        let mut tags = Vec::new();

        for (keyword, tag_list) in &self.keyword_tag_map {
            if text.contains(keyword) {
                tags.extend(tag_list.iter().map(|s| s.to_string()));
            }
        }

        tags
    }

    /// 推断风险等级
    fn infer_risk_level(&self, name: &str, description: &str) -> String {
        let text = format!("{} {}", name.to_lowercase(), description.to_lowercase());

        // 高风险操作
        if text.contains("delete") || text.contains("remove") || text.contains("destroy") {
            return "dangerous".to_string();
        }

        // 中等风险操作
        if text.contains("write") || text.contains("modify") || text.contains("update") 
            || text.contains("execute") || text.contains("run") {
            return "moderate".to_string();
        }

        // 低风险操作
        if text.contains("read") || text.contains("list") || text.contains("get") 
            || text.contains("search") || text.contains("analyze") {
            return "safe".to_string();
        }

        // 默认中等风险
        "moderate".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_matrix::matrix::ToolDefinition;

    #[test]
    fn test_enhance_read_file() {
        let enhancer = MetadataEnhancer::new();
        
        let tool = ToolDefinition::new("read_file", "Read file content from disk", r#"{}"#);
        let enhanced = enhancer.enhance(tool);

        assert_eq!(enhanced.metadata.category, ServiceCategory::File);
        assert!(enhanced.tags.contains(&"file".to_string()));
        assert!(enhanced.tags.contains(&"io".to_string()));
        assert!(enhanced.tags.contains(&"read_only".to_string()));
    }

    #[test]
    fn test_enhance_http_request() {
        let enhancer = MetadataEnhancer::new();
        
        let tool = ToolDefinition::new("http_request", "Send HTTP request to URL", r#"{}"#);
        let enhanced = enhancer.enhance(tool);

        assert_eq!(enhanced.metadata.category, ServiceCategory::Network);
        assert!(enhanced.tags.contains(&"http".to_string()));
        assert!(enhanced.tags.contains(&"network".to_string()));
    }

    #[test]
    fn test_infer_risk_level() {
        let enhancer = MetadataEnhancer::new();

        // 高风险
        let delete_tool = ToolDefinition::new("delete_file", "Delete a file", r#"{}"#);
        let enhanced = enhancer.enhance(delete_tool);
        assert_eq!(enhanced.risk_level, "dangerous");

        // 中等风险
        let write_tool = ToolDefinition::new("write_file", "Write content to file", r#"{}"#);
        let enhanced = enhancer.enhance(write_tool);
        assert_eq!(enhanced.risk_level, "moderate");

        // 低风险
        let read_tool = ToolDefinition::new("read_file", "Read file content", r#"{}"#);
        let enhanced = enhancer.enhance(read_tool);
        assert_eq!(enhanced.risk_level, "safe");
    }
}
