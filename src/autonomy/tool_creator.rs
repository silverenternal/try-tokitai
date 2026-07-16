//! 工具创建器
//!
//! 根据 ToolGap 创造新工具并使用 tokitai 宏注册
//!
//! ## 核心功能
//! - 根据缺口描述生成工具定义
//! - 生成工具代码模板（使用 Tera 模板引擎）
//! - 自动注册到工具矩阵
//! - 生成测试代码
//!
//! ## 实现状态
//! - ✅ 基础代码生成
//! - ✅ 参数结构体生成
//! - ✅ tokitai 宏集成
//! - ✅ 测试代码生成
//! - ✅ 文档生成
//! - ✅ 工具注册

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 工具创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCreationRequest {
    /// 工具名称
    pub tool_name: String,
    /// 工具描述
    pub description: String,
    /// 所属领域
    pub domain: String,
    /// 功能标签
    pub tags: Vec<String>,
    /// 参数定义
    pub parameters: Vec<ParameterDef>,
    /// 返回值类型
    pub return_type: String,
    /// 创建原因
    pub creation_reason: String,
    /// 优先级 (1-10)
    pub priority: u8,
}

/// 参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    /// 参数名称
    pub name: String,
    /// 参数类型
    pub param_type: String,
    /// 参数描述
    pub description: String,
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default_value: Option<String>,
}

/// 工具创建结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCreationResult {
    /// 是否成功
    pub success: bool,
    /// 工具名称
    pub tool_name: String,
    /// 生成的文件列表
    pub generated_files: Vec<PathBuf>,
    /// 注册状态
    pub registration_status: String,
    /// 错误信息（如果失败）
    pub error_message: Option<String>,
}

/// 工具创建器
pub struct ToolCreator {
    /// 项目根目录
    project_root: PathBuf,
    /// 工具模板目录
    template_dir: PathBuf,
    /// 配置
    config: CreatorConfig,
}

/// 创建器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorConfig {
    /// 是否生成测试代码
    pub generate_tests: bool,
    /// 是否生成文档
    pub generate_docs: bool,
    /// 是否自动注册
    pub auto_register: bool,
    /// 是否使用 Tera 模板（如果可用）
    pub use_tera_templates: bool,
}

impl Default for CreatorConfig {
    fn default() -> Self {
        Self {
            generate_tests: true,
            generate_docs: true,
            auto_register: true,
            use_tera_templates: false, // 默认使用内置模板，避免额外依赖
        }
    }
}

impl ToolCreator {
    /// 创建新的工具创建器
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let template_dir = project_root.join("templates").join("tools");

        // 确保模板目录存在
        std::fs::create_dir_all(&template_dir)?;

        // 创建默认模板（如果不存在）
        Self::ensure_default_templates(&template_dir)?;

        Ok(Self {
            project_root,
            template_dir,
            config: CreatorConfig::default(),
        })
    }

    /// 从配置创建
    pub fn with_config<P: AsRef<Path>>(project_root: P, config: CreatorConfig) -> Result<Self> {
        let mut creator = Self::new(project_root)?;
        creator.config = config;
        Ok(creator)
    }

    /// 确保默认模板存在
    fn ensure_default_templates(template_dir: &Path) -> Result<()> {
        // 主工具模板
        let tool_template = template_dir.join("tool.rs.tera");
        if !tool_template.exists() {
            let content = r#"//! {{ description }}

use serde::{Deserialize, Serialize};
use anyhow::Result;

{% if parameters %}
/// 工具参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Params {
{% for param in parameters %}
    /// {{ param.description }}
    pub {{ param.name }}: {{ param.rust_type }},
{% endfor %}
}
{% endif %}

/// {{ description }}
#[tokitai::tool(
    name = "{{ tool_name }}",
    description = "{{ description }}",
    {% if tags %}tags = [{% for tag in tags %}"{{ tag }}"{% if not loop.last %}, {% endif %}{% endfor %}],{% endif %}
)]
{% if parameters %}
pub async fn {{ tool_name }}(params: Params) -> Result<{{ return_type }}> {
{% else %}
pub async fn {{ tool_name }}() -> Result<{{ return_type }}> {
{% endif %}
    // TODO: 实现工具逻辑
    // {{ creation_reason }}
    
{% if parameters %}
    // 参数验证
{% for param in parameters %}
    {% if param.required %}
    // {{ param.name }}: 必需参数 - {{ param.description }}
    {% else %}
    // {{ param.name }}: 可选参数 - {{ param.description }}
    {% endif %}
{% endfor %}

{% endif %}
    // 实现代码
    unimplemented!("Tool {{ tool_name }} not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_{{ tool_name }}_basic() {
        // TODO: 实现基本功能测试
{% if parameters %}
        let params = Params {
{% for param in parameters %}
            {{ param.name }}: Default::default(),
{% endfor %}
        };
        let result = {{ tool_name }}(params).await;
{% else %}
        let result = {{ tool_name }}().await;
{% endif %}
        assert!(result.is_ok());
    }
}
"#;
            std::fs::write(&tool_template, content)?;
        }

        Ok(())
    }

    /// 创建工具
    pub fn create_tool(&self, request: ToolCreationRequest) -> Result<ToolCreationResult> {
        let mut generated_files = Vec::new();

        // 1. 验证请求
        self.validate_request(&request)?;

        // 2. 生成工具代码
        let tool_file = self.generate_tool_code(&request)?;
        generated_files.push(tool_file);

        // 3. 生成测试代码
        if self.config.generate_tests {
            let test_file = self.generate_test_code(&request)?;
            generated_files.push(test_file);
        }

        // 4. 生成文档
        if self.config.generate_docs {
            let doc_file = self.generate_documentation(&request)?;
            generated_files.push(doc_file);
        }

        // 5. 注册工具
        let registration_status = if self.config.auto_register {
            match self.register_tool(&request) {
                Ok(_) => "registered".to_string(),
                Err(e) => format!("registration_failed: {}", e),
            }
        } else {
            "manual_registration_required".to_string()
        };

        Ok(ToolCreationResult {
            success: true,
            tool_name: request.tool_name.clone(),
            generated_files,
            registration_status,
            error_message: None,
        })
    }

    /// 验证创建请求
    fn validate_request(&self, request: &ToolCreationRequest) -> Result<()> {
        if request.tool_name.is_empty() {
            anyhow::bail!("工具名称不能为空");
        }

        if !request
            .tool_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            anyhow::bail!("工具名称只能包含字母、数字和下划线");
        }

        if request.description.is_empty() {
            anyhow::bail!("工具描述不能为空");
        }

        Ok(())
    }

    /// 生成工具代码
    fn generate_tool_code(&self, request: &ToolCreationRequest) -> Result<PathBuf> {
        // 确定工具文件路径
        let tools_dir = self.project_root.join("src").join("tools");
        let domain_dir = tools_dir.join(self.normalize_domain(&request.domain));
        std::fs::create_dir_all(&domain_dir)?;

        let tool_file = domain_dir.join(format!("{}.rs", request.tool_name));

        // 生成代码（使用内置模板）
        let code = self.generate_rust_code(request);

        std::fs::write(&tool_file, code)
            .with_context(|| format!("Failed to write tool file: {:?}", tool_file))?;

        Ok(tool_file)
    }

    /// 生成 Rust 代码（使用内置模板，避免 Tera 依赖）
    fn generate_rust_code(&self, request: &ToolCreationRequest) -> String {
        let mut code = String::new();

        // 文件头注释
        code.push_str(&format!("//! {}\n\n", request.description));

        // 导入
        code.push_str("use serde::{Deserialize, Serialize};\n");
        code.push_str("use anyhow::Result;\n\n");

        // 如果参数不为空，生成参数结构体
        if !request.parameters.is_empty() {
            code.push_str("/// 工具参数\n");
            code.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
            code.push_str("pub struct Params {\n");
            for param in &request.parameters {
                code.push_str(&format!("    /// {}\n", param.description));
                code.push_str(&format!(
                    "    pub {}: {},\n",
                    param.name,
                    self.map_type_to_rust(&param.param_type)
                ));
            }
            code.push_str("}\n\n");
        }

        // tokitai 工具宏
        code.push_str("/// {}\n");
        code.push_str("#[tokitai::tool(\n");
        code.push_str(&format!("    name = \"{}\",\n", request.tool_name));
        code.push_str(&format!("    description = \"{}\",\n", request.description));
        if !request.tags.is_empty() {
            let tags_str = request
                .tags
                .iter()
                .map(|t| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(", ");
            code.push_str(&format!("    tags = [{}],\n", tags_str));
        }
        code.push_str(")]\n");

        // 函数签名
        if !request.parameters.is_empty() {
            code.push_str(&format!(
                "pub async fn {}(params: Params) -> Result<{}> {{\n",
                request.tool_name,
                self.map_type_to_rust(&request.return_type)
            ));
        } else {
            code.push_str(&format!(
                "pub async fn {}() -> Result<{}> {{\n",
                request.tool_name,
                self.map_type_to_rust(&request.return_type)
            ));
        }

        // 函数体（带注释的占位符）
        code.push_str("    // TODO: 实现工具逻辑\n");
        code.push_str(&format!("    // 创建原因：{}\n\n", request.creation_reason));

        // 参数验证代码
        if !request.parameters.is_empty() {
            code.push_str("    // 参数验证\n");
            for param in &request.parameters {
                if param.required {
                    code.push_str(&format!(
                        "    // - {}: 必需参数 - {}\n",
                        param.name, param.description
                    ));
                } else {
                    code.push_str(&format!(
                        "    // - {}: 可选参数 (默认：{:?}) - {}\n",
                        param.name, param.default_value, param.description
                    ));
                }
            }
            code.push('\n');
        }

        // 实现代码占位符
        code.push_str(&format!(
            "    unimplemented!(\"Tool {} not yet implemented\")\n",
            request.tool_name
        ));
        code.push_str("}\n\n");

        // 测试模块
        if self.config.generate_tests {
            code.push_str("#[cfg(test)]\n");
            code.push_str("mod tests {\n");
            code.push_str("    use super::*;\n\n");
            code.push_str("    #[tokio::test]\n");
            code.push_str(&format!(
                "    async fn test_{}_basic() {{\n",
                request.tool_name
            ));

            if !request.parameters.is_empty() {
                code.push_str("        let params = Params {\n");
                for param in &request.parameters {
                    let default_val = match param.param_type.to_lowercase().as_str() {
                        "string" | "字符串" => "String::new()".to_string(),
                        "int" | "integer" | "整数" => "0".to_string(),
                        "float" | "浮点数" => "0.0".to_string(),
                        "bool" | "布尔" => "false".to_string(),
                        _ => "Default::default()".to_string(),
                    };
                    code.push_str(&format!("            {}: {},\n", param.name, default_val));
                }
                code.push_str("        };\n");
                code.push_str(&format!(
                    "        let result = {}(params).await;\n",
                    request.tool_name
                ));
            } else {
                code.push_str(&format!(
                    "        let result = {}().await;\n",
                    request.tool_name
                ));
            }

            code.push_str("        // TODO: 添加实际断言\n");
            code.push_str("        assert!(result.is_ok());\n");
            code.push_str("    }\n");
            code.push_str("}\n");
        }

        code
    }

    /// 生成测试代码
    fn generate_test_code(&self, request: &ToolCreationRequest) -> Result<PathBuf> {
        let tools_dir = self.project_root.join("src").join("tools");
        let domain_dir = tools_dir.join(self.normalize_domain(&request.domain));
        let tests_dir = domain_dir.join("tests");
        std::fs::create_dir_all(&tests_dir)?;

        let test_file = tests_dir.join(format!("test_{}.rs", request.tool_name));

        let test_code = format!(
            r#"//! {} 的集成测试

//! 运行测试：cargo test --package ai-assistant --test test_{}

use ai_assistant::tools::{}::{};

#[tokio::test]
async fn test_{}_basic() {{
    // TODO: 实现基本功能测试
    // 测试工具的正常工作情况
}}

#[tokio::test]
async fn test_{}_edge_cases() {{
    // TODO: 实现边界条件测试
    // 测试空输入、大输入、特殊字符等
}}

#[tokio::test]
async fn test_{}_error_handling() {{
    // TODO: 实现错误处理测试
    // 测试无效输入、超时等情况
}}
"#,
            request.description,
            request.tool_name,
            self.normalize_domain(&request.domain),
            request.tool_name,
            request.tool_name,
            request.tool_name,
            request.tool_name,
        );

        std::fs::write(&test_file, test_code)
            .with_context(|| format!("Failed to write test file: {:?}", test_file))?;

        Ok(test_file)
    }

    /// 生成文档
    fn generate_documentation(&self, request: &ToolCreationRequest) -> Result<PathBuf> {
        let docs_dir = self.project_root.join("docs").join("tools");
        std::fs::create_dir_all(&docs_dir)?;

        let doc_file = docs_dir.join(format!("{}.md", request.tool_name));

        let parameters_doc = if request.parameters.is_empty() {
            "无参数".to_string()
        } else {
            request
                .parameters
                .iter()
                .map(|p| {
                    let required = if p.required { "必需" } else { "可选" };
                    let default = p
                        .default_value
                        .as_ref()
                        .map(|v| format!("，默认值：`{}`", v))
                        .unwrap_or_default();
                    format!(
                        "- **`{}`** (`{}`): {} - {}{}",
                        p.name, p.param_type, required, p.description, default
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let doc_content = format!(
            r#"# {tool_name}

{description}

> **创建原因**: {creation_reason}
> **优先级**: {priority}/10

## 元数据

| 属性 | 值 |
|------|-----|
| **所属领域** | {domain} |
| **标签** | {tags} |
| **返回值类型** | {return_type} |

## 参数

{parameters}

## 使用示例

### Rust 代码

```rust
use ai_assistant::tools::{domain}::{tool_name};

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
{call_example}
    Ok(())
}}
```

### CLI 调用

```bash
# 通过 tokitai 调用
cargo run -- --tool {tool_name} {call_args}
```

## 实现状态

- [ ] 核心逻辑实现
- [ ] 错误处理
- [ ] 单元测试
- [ ] 集成测试
- [ ] 性能优化

## 注意事项

- TODO: 添加使用注意事项
- TODO: 添加性能特征
- TODO: 添加安全考虑

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/tools/{domain}/{tool_name}.rs` | 工具实现 |
| `src/tools/{domain}/tests/test_{tool_name}.rs` | 集成测试 |
| `docs/tools/{tool_name}.md` | 本文档 |

## 更新历史

- {date}: 初始版本（自动生成）
"#,
            tool_name = request.tool_name,
            description = request.description,
            creation_reason = request.creation_reason,
            priority = request.priority,
            domain = self.normalize_domain(&request.domain),
            tags = request.tags.join(", "),
            return_type = request.return_type,
            parameters = parameters_doc,
            call_example = if request.parameters.is_empty() {
                format!("    let result = {}().await?;", request.tool_name)
            } else {
                let params = request
                    .parameters
                    .iter()
                    .map(|p| format!("{}: Default::default()", p.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "    let params = Params {{ {} }};\n    let result = {}(params).await?;",
                    params, request.tool_name
                )
            },
            call_args = if request.parameters.is_empty() {
                String::new()
            } else {
                request
                    .parameters
                    .iter()
                    .map(|p| format!("--{} <value>", p.name))
                    .collect::<Vec<_>>()
                    .join(" ")
            },
            date = chrono::Local::now().format("%Y-%m-%d"),
        );

        std::fs::write(&doc_file, doc_content)
            .with_context(|| format!("Failed to write doc file: {:?}", doc_file))?;

        Ok(doc_file)
    }

    /// 注册工具到工具矩阵
    fn register_tool(&self, request: &ToolCreationRequest) -> Result<()> {
        // 读取 toolbox_rules.json
        let rules_file = self.project_root.join("config").join("toolbox_rules.json");

        let mut rules: serde_json::Value = if rules_file.exists() {
            let content = std::fs::read_to_string(&rules_file)?;
            serde_json::from_str(&content)?
        } else {
            serde_json::json!({})
        };

        // 获取或创建领域规则
        let domain_key = self.normalize_domain(&request.domain);

        if !rules.is_object() {
            rules = serde_json::json!({});
        }

        let rules_obj = rules.as_object_mut().unwrap();

        if !rules_obj.contains_key(&domain_key) {
            rules_obj.insert(
                domain_key.clone(),
                serde_json::json!({
                    "keywords": [],
                    "patterns": [],
                    "tools": {}
                }),
            );
        }

        // 添加工具到领域规则
        let domain_rules = rules_obj.get_mut(&domain_key).unwrap();

        // 添加工具定义
        if let Some(tools) = domain_rules
            .get_mut("tools")
            .and_then(|v| v.as_object_mut())
        {
            let tool_def = serde_json::json!({
                "description": request.description,
                "tags": request.tags,
                "parameters": request.parameters.iter().map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "type": p.param_type,
                        "description": p.description,
                        "required": p.required
                    })
                }).collect::<Vec<_>>(),
                "return_type": request.return_type,
                "created_at": chrono::Local::now().to_rfc3339(),
                "auto_generated": true
            });
            tools.insert(request.tool_name.clone(), tool_def);
        }

        // 添加工具名称作为关键词
        if let Some(keywords) = domain_rules
            .get_mut("keywords")
            .and_then(|v| v.as_array_mut())
        {
            let tool_name_lower = request.tool_name.to_lowercase();
            if !keywords
                .iter()
                .any(|k| k.as_str() == Some(&tool_name_lower))
            {
                keywords.push(serde_json::Value::String(tool_name_lower));
            }

            // 添加标签作为关键词
            for tag in &request.tags {
                let tag_lower = tag.to_lowercase();
                if !keywords.iter().any(|k| k.as_str() == Some(&tag_lower)) {
                    keywords.push(serde_json::Value::String(tag_lower));
                }
            }
        }

        // 保存更新后的规则
        let json = serde_json::to_string_pretty(&rules)?;
        std::fs::write(rules_file, json)?;

        Ok(())
    }

    /// 规范化领域名称
    pub fn normalize_domain(&self, domain: &str) -> String {
        let lower = domain.to_lowercase();

        // 先检查完整词汇
        if lower == "git" || lower == "版本控制" || lower == "version_control" || lower == "vcs"
        {
            return "vcs".to_string();
        }
        if lower.contains("文件") || lower.contains("file") {
            return "file_ops".to_string();
        }
        if lower.contains("网络") || lower.contains("network") {
            return "network".to_string();
        }
        if lower.contains("数据") || lower.contains("data") {
            return "data".to_string();
        }
        if lower.contains("代码") || lower.contains("code") {
            return "code".to_string();
        }
        if lower.contains("系统") || lower.contains("system") {
            return "system".to_string();
        }
        if lower.contains("搜索") || lower.contains("search") {
            return "search".to_string();
        }
        if lower.contains("知识") || lower.contains("knowledge") {
            return "knowledge".to_string();
        }

        // 默认：返回小写版本
        lower.replace(' ', "_")
    }

    /// 映射类型到 Rust 类型
    fn map_type_to_rust(&self, param_type: &str) -> String {
        match param_type.to_lowercase().as_str() {
            "string" | "字符串" => "String".to_string(),
            "int" | "integer" | "整数" => "i64".to_string(),
            "float" | "浮点数" => "f64".to_string(),
            "bool" | "布尔" => "bool".to_string(),
            "array" | "数组" => "Vec<String>".to_string(),
            "object" | "对象" => "serde_json::Value".to_string(),
            _ => "String".to_string(),
        }
    }

    /// 从缺口创建工具
    pub fn create_from_gap(
        &self,
        gap_name: &str,
        gap_description: &str,
        suggested_capabilities: &[String],
    ) -> Result<ToolCreationResult> {
        // 从缺口信息生成工具定义
        let tool_name = self.generate_tool_name_from_gap(gap_name);

        // 解析领域（从缺口描述中推断）
        let domain = self.infer_domain_from_description(gap_description);

        let request = ToolCreationRequest {
            tool_name,
            description: gap_description.to_string(),
            domain,
            tags: vec!["auto_generated".to_string(), "gap_filled".to_string()],
            parameters: Vec::new(),
            return_type: "String".to_string(),
            creation_reason: gap_description.to_string(),
            priority: 5,
        };

        self.create_tool(request)
    }

    /// 从缺口名称生成工具名称
    fn generate_tool_name_from_gap(&self, gap_name: &str) -> String {
        // 清理缺口名称，生成合法的工具名
        let name = gap_name
            .to_lowercase()
            .replace([' ', '-'], "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        // 确保名称不以数字开头
        if name.chars().next().map(|c| c.is_numeric()).unwrap_or(true) {
            format!("tool_{}", name)
        } else {
            name
        }
    }

    /// 从缺口描述推断领域
    fn infer_domain_from_description(&self, description: &str) -> String {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("文件")
            || desc_lower.contains("file")
            || desc_lower.contains("read")
            || desc_lower.contains("write")
        {
            "file_ops".to_string()
        } else if desc_lower.contains("网络")
            || desc_lower.contains("network")
            || desc_lower.contains("http")
            || desc_lower.contains("download")
        {
            "network".to_string()
        } else if desc_lower.contains("git")
            || desc_lower.contains("版本")
            || desc_lower.contains("commit")
        {
            "vcs".to_string()
        } else if desc_lower.contains("数据")
            || desc_lower.contains("data")
            || desc_lower.contains("json")
            || desc_lower.contains("csv")
        {
            "data".to_string()
        } else if desc_lower.contains("代码")
            || desc_lower.contains("code")
            || desc_lower.contains("analyze")
        {
            "code".to_string()
        } else if desc_lower.contains("系统")
            || desc_lower.contains("system")
            || desc_lower.contains("process")
        {
            "system".to_string()
        } else {
            "general".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_creator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();
        assert!(creator.template_dir.exists());
    }

    #[test]
    fn test_tool_creation_basic() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();

        let request = ToolCreationRequest {
            tool_name: "test_tool".to_string(),
            description: "Test tool for demonstration".to_string(),
            domain: "file_ops".to_string(),
            tags: vec!["test".to_string()],
            parameters: vec![],
            return_type: "String".to_string(),
            creation_reason: "Testing".to_string(),
            priority: 5,
        };

        let result = creator.create_tool(request).unwrap();

        assert!(result.success);
        assert!(!result.generated_files.is_empty());
        assert!(result.generated_files[0].exists());
    }

    #[test]
    fn test_tool_creation_with_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();

        let request = ToolCreationRequest {
            tool_name: "read_config".to_string(),
            description: "Read configuration file".to_string(),
            domain: "file_ops".to_string(),
            tags: vec!["config".to_string(), "read".to_string()],
            parameters: vec![
                ParameterDef {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "配置文件路径".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDef {
                    name: "format".to_string(),
                    param_type: "string".to_string(),
                    description: "配置文件格式".to_string(),
                    required: false,
                    default_value: Some("json".to_string()),
                },
            ],
            return_type: "serde_json::Value".to_string(),
            creation_reason: "Need to read config files".to_string(),
            priority: 7,
        };

        let result = creator.create_tool(request).unwrap();

        assert!(result.success);
        assert_eq!(result.generated_files.len(), 3); // tool + test + doc

        // 验证生成的代码包含参数结构体
        let tool_code = std::fs::read_to_string(&result.generated_files[0]).unwrap();
        assert!(tool_code.contains("pub struct Params"));
        assert!(tool_code.contains("pub path: String"));
        assert!(tool_code.contains("pub format: String"));
    }

    #[test]
    fn test_normalize_domain() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();

        assert_eq!(creator.normalize_domain("文件操作"), "file_ops");
        assert_eq!(creator.normalize_domain("网络"), "network");
        assert_eq!(creator.normalize_domain("Data"), "data");
        assert_eq!(creator.normalize_domain("Git"), "vcs");
    }

    #[test]
    fn test_infer_domain() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();

        assert_eq!(
            creator.infer_domain_from_description("需要读取文件"),
            "file_ops"
        );
        assert_eq!(
            creator.infer_domain_from_description("HTTP download tool"),
            "network"
        );
        assert_eq!(
            creator.infer_domain_from_description("Git commit 操作"),
            "vcs"
        );
        assert_eq!(creator.infer_domain_from_description("未知功能"), "general");
    }

    #[test]
    fn test_generate_tool_name_from_gap() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();

        assert_eq!(
            creator.generate_tool_name_from_gap("批量读取文件"),
            "批量读取文件"
        );
        assert_eq!(
            creator.generate_tool_name_from_gap("HTTP Download"),
            "http_download"
        );
        assert_eq!(
            creator.generate_tool_name_from_gap("123 Tool"),
            "tool_123_tool"
        );
    }

    #[test]
    fn test_create_from_gap() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();

        let result = creator
            .create_from_gap(
                "批量读取配置文件",
                "需要批量读取多个配置文件并合并",
                &["batch".to_string(), "config".to_string()],
            )
            .unwrap();

        assert!(result.success);
        assert!(result.generated_files[0].exists());

        // 验证工具名称生成
        assert!(
            result.tool_name.contains("批量读取配置文件") || result.tool_name.contains("config")
        );
    }
}
