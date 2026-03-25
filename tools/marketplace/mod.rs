//! 工具市场加载器
//!
//! 支持从 TOML 文件加载社区工具，实现热插拔
//!
//! ## 功能
//! - 从 tools/*.toml 加载工具定义
//! - 工具热加载无需重新编译
//! - 支持工具版本管理
//! - 支持工具依赖解析

use crate::tool_matrix::tool_definition::{TomlToolDefinition, TomlToolLoader};
use crate::tool_matrix::matrix::ToolDefinition;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, debug, warn};
use anyhow::{Result, Context};

/// 市场工具记录
#[derive(Debug, Clone)]
pub struct MarketplaceTool {
    /// 工具定义
    pub definition: TomlToolDefinition,
    /// 工具文件路径
    pub file_path: PathBuf,
    /// 是否已启用
    pub enabled: bool,
    /// 是否已安装
    pub installed: bool,
}

/// 工具市场加载器
pub struct MarketplaceLoader {
    /// 市场目录
    marketplace_dir: PathBuf,
    /// TOML 加载器
    toml_loader: TomlToolLoader,
    /// 已加载的工具
    tools: HashMap<String, MarketplaceTool>,
}

impl MarketplaceLoader {
    /// 创建新的市场加载器
    pub fn new<P: AsRef<Path>>(marketplace_dir: P) -> Result<Self> {
        let marketplace_dir = marketplace_dir.as_ref().to_path_buf();

        // 确保目录存在
        std::fs::create_dir_all(&marketplace_dir)
            .with_context(|| format!("创建市场目录失败：{:?}", marketplace_dir))?;

        let toml_loader = TomlToolLoader::new(&marketplace_dir)?;

        let mut loader = Self {
            marketplace_dir,
            toml_loader,
            tools: HashMap::new(),
        };

        // 加载现有工具
        loader.reload()?;

        Ok(loader)
    }

    /// 从默认目录创建加载器
    pub fn from_default_dir() -> Result<Self> {
        let workspace_root = std::env::current_dir()?;
        let marketplace_dir = workspace_root.join("tools/marketplace");
        Self::new(marketplace_dir)
    }

    /// 重新加载所有工具
    pub fn reload(&mut self) -> Result<usize> {
        self.toml_loader.reload()?;

        let mut loaded_count = 0;
        self.tools.clear();

        for definition in self.toml_loader.get_all_tools() {
            let tool = MarketplaceTool {
                definition: definition.clone(),
                file_path: self.marketplace_dir.join(format!("{}.toml", definition.tool.name)),
                enabled: true,
                installed: false, // 仅定义，未安装实现
            };

            self.tools.insert(definition.tool.name.clone(), tool);
            loaded_count += 1;
        }

        info!("市场加载器：共加载 {} 个工具", loaded_count);

        Ok(loaded_count)
    }

    /// 获取工具
    pub fn get_tool(&self, name: &str) -> Option<&MarketplaceTool> {
        self.tools.get(name)
    }

    /// 获取所有工具
    pub fn get_all_tools(&self) -> Vec<&MarketplaceTool> {
        self.tools.values().collect()
    }

    /// 搜索工具
    pub fn search(&self, query: &str) -> Vec<&MarketplaceTool> {
        let query_lower = query.to_lowercase();
        self.tools
            .values()
            .filter(|t| {
                t.definition.tool.name.to_lowercase().contains(&query_lower)
                    || t.definition.tool.description.to_lowercase().contains(&query_lower)
                    || t.definition.tool.tags.iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// 按类别筛选工具
    pub fn filter_by_category(&self, category: &str) -> Vec<&MarketplaceTool> {
        self.tools
            .values()
            .filter(|t| t.definition.tool.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// 启用工具
    pub fn enable_tool(&mut self, name: &str) -> Result<()> {
        let tool = self.tools.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("工具不存在：{}", name))?;
        tool.enabled = true;
        info!("工具已启用：{}", name);
        Ok(())
    }

    /// 禁用工具
    pub fn disable_tool(&mut self, name: &str) -> Result<()> {
        let tool = self.tools.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("工具不存在：{}", name))?;
        tool.enabled = false;
        info!("工具已禁用：{}", name);
        Ok(())
    }

    /// 获取已启用的工具
    pub fn get_enabled_tools(&self) -> Vec<&MarketplaceTool> {
        self.tools.values().filter(|t| t.enabled).collect()
    }

    /// 获取工具统计
    pub fn stats(&self) -> MarketplaceStats {
        MarketplaceStats {
            total_tools: self.tools.len(),
            enabled_tools: self.tools.values().filter(|t| t.enabled).count(),
            disabled_tools: self.tools.values().filter(|t| !t.enabled).count(),
            categories: self.get_category_stats(),
        }
    }

    /// 获取类别统计
    fn get_category_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        for tool in self.tools.values() {
            *stats.entry(tool.definition.tool.category.clone()).or_insert(0) += 1;
        }
        stats
    }
}

/// 市场统计信息
#[derive(Debug, Clone)]
pub struct MarketplaceStats {
    /// 工具总数
    pub total_tools: usize,
    /// 已启用工具数
    pub enabled_tools: usize,
    /// 已禁用工具数
    pub disabled_tools: usize,
    /// 类别统计
    pub categories: HashMap<String, usize>,
}

/// 将市场工具转换为 ToolDefinition
pub fn marketplace_to_tool_definition(marketplace_tool: &MarketplaceTool) -> ToolDefinition {
    marketplace_tool.definition.to_tool_definition()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_marketplace_loader() -> Result<()> {
        let dir = tempdir()?;

        // 创建测试工具
        let tool_path = dir.path().join("test_tool.toml");
        let mut file = std::fs::File::create(&tool_path)?;
        writeln!(file, r#"
[tool]
name = "test_tool"
version = "1.0.0"
description = "A test tool"
category = "utility"
tags = ["test", "demo"]

[[parameters]]
name = "input"
type = "string"
required = true
description = "Input parameter"
"#)?;

        let mut loader = MarketplaceLoader::new(dir.path())?;

        assert_eq!(loader.stats().total_tools, 1);
        assert!(loader.get_tool("test_tool").is_some());

        // 测试搜索
        let results = loader.search("test");
        assert_eq!(results.len(), 1);

        // 测试启用/禁用
        loader.disable_tool("test_tool")?;
        assert!(!loader.get_tool("test_tool").unwrap().enabled);
        assert_eq!(loader.get_enabled_tools().len(), 0);

        loader.enable_tool("test_tool")?;
        assert!(loader.get_tool("test_tool").unwrap().enabled);

        Ok(())
    }

    #[test]
    fn test_marketplace_stats() -> Result<()> {
        let dir = tempdir()?;

        // 创建多个工具
        for (i, category) in ["network", "file", "network"].iter().enumerate() {
            let tool_path = dir.path().join(format!("tool_{}.toml", i));
            let mut file = std::fs::File::create(&tool_path)?;
            writeln!(file, r#"
[tool]
name = "tool_{}"
version = "1.0.0"
description = "Tool {}"
category = "{}"

[[parameters]]
name = "input"
type = "string"
required = true
description = "Input"
"#, i, i, category)?;
        }

        let loader = MarketplaceLoader::new(dir.path())?;
        let stats = loader.stats();

        assert_eq!(stats.total_tools, 3);
        assert_eq!(stats.enabled_tools, 3);
        assert_eq!(stats.categories.get("network"), Some(&2));
        assert_eq!(stats.categories.get("file"), Some(&1));

        Ok(())
    }
}
