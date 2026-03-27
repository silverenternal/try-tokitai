//! TOML 工具定义
//!
//! 支持从 TOML 文件声明式定义工具，实现热加载能力
//!
//! ## 设计目标
//! - 声明式配置：用 TOML 定义工具元数据和参数
//! - 热加载支持：无需重新编译即可加载新工具
//! - 版本管理：支持工具版本控制和依赖管理
//! - 权限控制：声明工具所需权限
//!
//! ## TOML 示例
//! ```toml
//! [tool]
//! name = "web_search"
//! version = "1.0.0"
//! description = "Execute web searches"
//! author = "community"
//! category = "network"
//! entry_point = "tools::web::search"
//!
//! [[parameters]]
//! name = "query"
//! type = "string"
//! required = true
//! description = "Search query"
//!
//! [[parameters]]
//! name = "limit"
//! type = "integer"
//! required = false
//! default = 10
//! description = "Number of results"
//!
//! [permissions]
//! network_access = true
//! file_read = false
//! file_write = false
//!
//! [rate_limit]
//! requests_per_minute = 60
//! ```

// 允许未使用的代码，这些是热加载功能的基础设施
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, debug, warn};
use anyhow::{Result, Context, bail};

/// TOML 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlToolDefinition {
    /// 工具元数据
    pub tool: ToolMetadata,
    /// 参数列表
    #[serde(default)]
    pub parameters: Vec<ParameterSpec>,
    /// 权限配置
    #[serde(default)]
    pub permissions: Permissions,
    /// 速率限制
    #[serde(default)]
    pub rate_limit: Option<RateLimit>,
    /// 依赖项
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// 工具名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 工具描述
    pub description: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 工具箱类别
    #[serde(default)]
    pub category: String,
    /// 入口点（Rust 路径）
    #[serde(default)]
    pub entry_point: String,
    /// 工具标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 文档 URL
    #[serde(default)]
    pub documentation_url: Option<String>,
    /// 仓库 URL
    #[serde(default)]
    pub repository_url: Option<String>,
    /// 许可证
    #[serde(default)]
    pub license: String,
}

/// 参数规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSpec {
    /// 参数名称
    pub name: String,
    /// 参数类型
    #[serde(rename = "type")]
    pub param_type: String,
    /// 参数描述
    pub description: String,
    /// 是否必填
    #[serde(default)]
    pub required: bool,
    /// 默认值
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// 示例值
    #[serde(default)]
    pub example: Option<serde_json::Value>,
    /// 验证规则
    #[serde(default)]
    pub validation: Option<ValidationRule>,
}

impl ParameterSpec {
    /// Rust 类型转 JSON Schema 类型
    pub fn rust_type_to_json(&self) -> &str {
        match self.param_type.as_str() {
            "string" | "String" => "string",
            "integer" | "i64" | "i32" | "u64" | "u32" => "integer",
            "number" | "f64" | "f32" => "number",
            "boolean" | "bool" => "boolean",
            "array" | "Vec<String>" | "Vec<i64>" => "array",
            "object" | "serde_json::Value" | "Value" => "object",
            _ => "string",
        }
    }
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// 最小值（数字类型）
    pub min: Option<f64>,
    /// 最大值（数字类型）
    pub max: Option<f64>,
    /// 最小长度（字符串/数组）
    pub min_length: Option<usize>,
    /// 最大长度（字符串/数组）
    pub max_length: Option<usize>,
    /// 正则表达式模式
    pub pattern: Option<String>,
    /// 枚举值
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// 权限配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Permissions {
    /// 网络访问
    #[serde(default)]
    pub network_access: bool,
    /// 文件读取
    #[serde(default)]
    pub file_read: bool,
    /// 文件写入
    #[serde(default)]
    pub file_write: bool,
    /// 执行命令
    #[serde(default)]
    pub execute_command: bool,
    /// 环境变量访问
    #[serde(default)]
    pub env_access: bool,
    /// 剪贴板访问
    #[serde(default)]
    pub clipboard_access: bool,
}

/// 速率限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// 每分钟请求数
    pub requests_per_minute: u32,
    /// 每秒请求数
    #[serde(default)]
    pub requests_per_second: Option<u32>,
    /// 每日请求数
    #[serde(default)]
    pub requests_per_day: Option<u32>,
}

impl TomlToolDefinition {
    /// 从 TOML 文件加载工具定义
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取工具定义文件失败：{:?}", path))?;

        let definition: TomlToolDefinition = toml::from_str(&content)
            .with_context(|| format!("解析 TOML 工具定义失败：{:?}", path))?;

        debug!("加载工具定义：{} v{}", definition.tool.name, definition.tool.version);

        Ok(definition)
    }

    /// 从 TOML 字符串解析工具定义
    pub fn from_str(toml_str: &str) -> Result<Self> {
        let definition: TomlToolDefinition = toml::from_str(toml_str)
            .with_context(|| "解析 TOML 工具定义失败")?;

        Ok(definition)
    }

    /// 转换为 JSON Schema
    pub fn to_json_schema(&self) -> String {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), serde_json::json!(param.rust_type_to_json()));
            prop.insert("description".to_string(), serde_json::json!(&param.description));

            if let Some(default) = &param.default {
                prop.insert("default".to_string(), default.clone());
            }

            if let Some(example) = &param.example {
                prop.insert("examples".to_string(), serde_json::json!([example]));
            }

            if let Some(validation) = &param.validation {
                if let Some(min) = validation.min {
                    prop.insert("minimum".to_string(), serde_json::json!(min));
                }
                if let Some(max) = validation.max {
                    prop.insert("maximum".to_string(), serde_json::json!(max));
                }
                if let Some(min_len) = validation.min_length {
                    prop.insert("minLength".to_string(), serde_json::json!(min_len));
                }
                if let Some(max_len) = validation.max_length {
                    prop.insert("maxLength".to_string(), serde_json::json!(max_len));
                }
                if let Some(pattern) = &validation.pattern {
                    prop.insert("pattern".to_string(), serde_json::json!(pattern));
                }
                if let Some(enum_vals) = &validation.enum_values {
                    prop.insert("enum".to_string(), serde_json::json!(enum_vals));
                }
            }

            properties.insert(param.name.clone(), serde_json::Value::Object(prop));

            if param.required {
                required.push(param.name.clone());
            }
        }

        let schema = serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        });

        serde_json::to_string_pretty(&schema).unwrap_or_default()
    }

    /// 转换为 ToolDefinition (兼容现有注册表)
    pub fn to_tool_definition(&self) -> crate::tool_matrix::matrix::ToolDefinition {
        let schema = self.to_json_schema();
        let mut tool = crate::tool_matrix::matrix::ToolDefinition::new(
            &self.tool.name,
            &self.tool.description,
            &schema,
        );

        tool.source = "toml".to_string();
        tool.metadata.version = self.tool.version.clone();
        tool.metadata.dependencies = self.dependencies.clone();
        tool.metadata.category = self.parse_category(&self.tool.category);
        tool.metadata.tags = self.tool.tags.clone();

        tool
    }

    /// 解析类别字符串到 ServiceCategory
    fn parse_category(&self, category: &str) -> crate::tool_matrix::matrix::ServiceCategory {
        use crate::tool_matrix::matrix::ServiceCategory;
        match category.to_lowercase().as_str() {
            "file" | "file_ops" => ServiceCategory::File,
            "network" => ServiceCategory::Network,
            "system" => ServiceCategory::System,
            "data" => ServiceCategory::Data,
            "ai" => ServiceCategory::Ai,
            "vcs" | "version_control" | "git" => ServiceCategory::VersionControl,
            "development" | "dev" => ServiceCategory::Development,
            "utility" | "utils" => ServiceCategory::Utility,
            _ => ServiceCategory::Default,
        }
    }

    /// 验证工具定义的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证名称
        if self.tool.name.is_empty() {
            bail!("工具名称不能为空");
        }

        // 验证版本号格式
        if !self.is_valid_version(&self.tool.version) {
            bail!("版本号格式无效：{} (应为 semver 格式)", self.tool.version);
        }

        // 验证参数类型
        for param in &self.parameters {
            if !self.is_valid_param_type(&param.param_type) {
                bail!("参数 '{}' 类型无效：{}", param.name, param.param_type);
            }
        }

        // 验证速率限制
        if let Some(rate_limit) = &self.rate_limit {
            if rate_limit.requests_per_minute == 0 {
                bail!("速率限制必须大于 0");
            }
        }

        Ok(())
    }

    /// 检查版本号是否有效 (semver 格式)
    fn is_valid_version(&self, version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u32>().is_ok())
    }

    /// 检查参数类型是否有效
    fn is_valid_param_type(&self, param_type: &str) -> bool {
        matches!(
            param_type,
            "string" | "String" |
            "integer" | "i64" | "i32" | "u64" | "u32" |
            "number" | "f64" | "f32" |
            "boolean" | "bool" |
            "array" | "Vec<String>" | "Vec<i64>" |
            "object" | "serde_json::Value" | "Value"
        )
    }
}

/// TOML 工具加载器
pub struct TomlToolLoader {
    /// 工具目录
    tools_dir: PathBuf,
    /// 已加载的工具
    loaded_tools: HashMap<String, TomlToolDefinition>,
    /// 文件修改时间追踪
    file_timestamps: HashMap<PathBuf, u64>,
}

impl TomlToolLoader {
    /// 创建新的加载器
    pub fn new<P: AsRef<Path>>(tools_dir: P) -> Result<Self> {
        let tools_dir = tools_dir.as_ref().to_path_buf();

        // 确保目录存在
        fs::create_dir_all(&tools_dir)
            .with_context(|| format!("创建工具目录失败：{:?}", tools_dir))?;

        let mut loader = Self {
            tools_dir,
            loaded_tools: HashMap::new(),
            file_timestamps: HashMap::new(),
        };

        // 加载现有工具
        loader.reload()?;

        Ok(loader)
    }

    /// 从默认目录创建加载器
    pub fn from_default_dir() -> Result<Self> {
        let workspace_root = std::env::current_dir()?;
        let tools_dir = workspace_root.join("tools");
        Self::new(tools_dir)
    }

    /// 重新加载所有工具
    pub fn reload(&mut self) -> Result<usize> {
        let mut loaded_count = 0;

        if !self.tools_dir.exists() {
            debug!("工具目录不存在：{:?}", self.tools_dir);
            return Ok(0);
        }

        // 递归查找所有 .toml 文件
        for entry in walkdir::WalkDir::new(&self.tools_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map(|ext| ext == "toml").unwrap_or(false))
        {
            let path = entry.path().to_path_buf();

            // 检查文件是否已修改
            let modified = self.get_file_modified(&path);
            let needs_reload = match self.file_timestamps.get(&path) {
                Some(ts) => modified != *ts,
                None => true,
            };

            if needs_reload {
                match TomlToolDefinition::from_file(&path) {
                    Ok(definition) => {
                        if let Err(e) = definition.validate() {
                            warn!("工具验证失败 {:?}: {}", path, e);
                            continue;
                        }

                        info!("加载工具：{} v{}", definition.tool.name, definition.tool.version);
                        self.loaded_tools.insert(definition.tool.name.clone(), definition);
                        self.file_timestamps.insert(path, modified);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("加载工具失败 {:?}: {}", path, e);
                    }
                }
            }
        }

        info!("共加载 {} 个工具 (热加载)", loaded_count);

        Ok(loaded_count)
    }

    /// 获取文件修改时间
    fn get_file_modified(&self, path: &Path) -> u64 {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0)
    }

    /// 获取已加载的工具
    pub fn get_tool(&self, name: &str) -> Option<&TomlToolDefinition> {
        self.loaded_tools.get(name)
    }

    /// 获取所有已加载的工具
    pub fn get_all_tools(&self) -> Vec<&TomlToolDefinition> {
        self.loaded_tools.values().collect()
    }

    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.loaded_tools.len()
    }

    /// 按类别筛选工具
    pub fn filter_by_category(&self, category: &str) -> Vec<&TomlToolDefinition> {
        self.loaded_tools
            .values()
            .filter(|t| t.tool.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// 搜索工具
    pub fn search(&self, query: &str) -> Vec<&TomlToolDefinition> {
        let query_lower = query.to_lowercase();
        self.loaded_tools
            .values()
            .filter(|t| {
                t.tool.name.to_lowercase().contains(&query_lower)
                    || t.tool.description.to_lowercase().contains(&query_lower)
                    || t.tool.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// 导出所有工具为 TOML 示例
    pub fn export_examples(&self, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        for (name, definition) in &self.loaded_tools {
            let toml_str = toml::to_string_pretty(definition)
                .with_context(|| format!("序列化工具失败：{}", name))?;

            let output_path = output_dir.join(format!("{}.toml", name));
            fs::write(&output_path, toml_str)
                .with_context(|| format!("写入文件失败：{:?}", output_path))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_toml_tool_definition() -> Result<()> {
        let toml_str = r#"
[tool]
name = "web_search"
version = "1.0.0"
description = "Execute web searches"
author = "community"
category = "network"
tags = ["search", "web"]
license = "MIT"

[[parameters]]
name = "query"
type = "string"
required = true
description = "Search query"

[[parameters]]
name = "limit"
type = "integer"
required = false
default = 10
description = "Number of results"

[permissions]
network_access = true
file_read = false

[rate_limit]
requests_per_minute = 60
"#;

        let definition = TomlToolDefinition::from_str(toml_str)?;

        assert_eq!(definition.tool.name, "web_search");
        assert_eq!(definition.tool.version, "1.0.0");
        assert_eq!(definition.parameters.len(), 2);
        assert!(definition.permissions.network_access);
        assert!(!definition.permissions.file_read);
        assert_eq!(definition.rate_limit.unwrap().requests_per_minute, 60);

        Ok(())
    }

    #[test]
    fn test_tool_validation() -> Result<()> {
        let valid_toml = r#"
[tool]
name = "test_tool"
version = "1.0.0"
description = "A test tool"

[[parameters]]
name = "input"
type = "string"
required = true
description = "Input"
"#;

        let definition = TomlToolDefinition::from_str(valid_toml)?;
        assert!(definition.validate().is_ok());

        // 测试无效版本号
        let invalid_version = r#"
[tool]
name = "bad_tool"
version = "invalid"
description = "Bad version"
"#;

        let definition = TomlToolDefinition::from_str(invalid_version)?;
        assert!(definition.validate().is_err());

        Ok(())
    }

    #[test]
    fn test_json_schema_generation() -> Result<()> {
        let toml_str = r#"
[tool]
name = "test_tool"
version = "1.0.0"
description = "A test tool"

[[parameters]]
name = "name"
type = "string"
required = true
description = "Name"

[[parameters]]
name = "count"
type = "integer"
required = false
default = 5
description = "Count"
"#;

        let definition = TomlToolDefinition::from_str(toml_str)?;
        let schema = definition.to_json_schema();

        assert!(schema.contains("\"type\": \"object\""));
        assert!(schema.contains("\"name\""));
        assert!(schema.contains("\"count\""));
        assert!(schema.contains("\"required\": [\"name\"]"));

        Ok(())
    }

    #[test]
    fn test_tool_loader() -> Result<()> {
        let dir = tempdir()?;

        // 创建测试工具文件
        let tool1_path = dir.path().join("search.toml");
        let mut tool1_file = fs::File::create(&tool1_path)?;
        writeln!(tool1_file, r#"
[tool]
name = "web_search"
version = "1.0.0"
description = "Search the web"
category = "network"

[[parameters]]
name = "query"
type = "string"
required = true
description = "Search query"
"#)?;

        let tool2_path = dir.path().join("file.toml");
        let mut tool2_file = fs::File::create(&tool2_path)?;
        writeln!(tool2_file, r#"
[tool]
name = "read_file"
version = "2.0.0"
description = "Read a file"
category = "file"

[[parameters]]
name = "path"
type = "string"
required = true
description = "File path"
"#)?;

        let mut loader = TomlToolLoader::new(dir.path())?;

        assert_eq!(loader.tool_count(), 2);
        assert!(loader.get_tool("web_search").is_some());
        assert!(loader.get_tool("read_file").is_some());

        // 测试搜索
        let results = loader.search("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool.name, "web_search");

        // 测试按类别筛选
        let network_tools = loader.filter_by_category("network");
        assert_eq!(network_tools.len(), 1);

        // 测试热加载：修改文件
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut tool1_file = fs::File::create(&tool1_path)?;
        writeln!(tool1_file, r#"
[tool]
name = "web_search"
version = "1.1.0"
description = "Search the web - updated"
category = "network"

[[parameters]]
name = "query"
type = "string"
required = true
description = "Search query"
"#)?;

        let reloaded = loader.reload()?;
        assert_eq!(reloaded, 1);
        assert_eq!(loader.get_tool("web_search").unwrap().tool.version, "1.1.0");

        Ok(())
    }
}
