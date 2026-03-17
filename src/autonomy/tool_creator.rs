//! 工具创建器
//!
//! 根据 ToolGap 创造新工具并使用 tokitai 宏注册
//!
//! ## 核心功能
//! - 根据缺口描述生成工具定义
//! - 生成工具代码模板
//! - 自动注册到工具矩阵
//! - 生成测试代码

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

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
}

impl Default for CreatorConfig {
    fn default() -> Self {
        Self {
            generate_tests: true,
            generate_docs: true,
            auto_register: true,
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
        
        if !request.tool_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
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
        
        // 生成代码
        let code = self.generate_rust_code(request);
        
        std::fs::write(&tool_file, code)
            .with_context(|| format!("Failed to write tool file: {:?}", tool_file))?;
        
        Ok(tool_file)
    }

    /// 生成 Rust 代码
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
                code.push_str(&format!("    pub {}: {},\n", param.name, self.map_type_to_rust(&param.param_type)));
            }
            code.push_str("}\n\n");
        }
        
        // 工具函数
        code.push_str(&format!("/// {}\n", request.description));
        code.push_str("#[tokitai::tool(\n");
        code.push_str(&format!("    name = \"{}\",\n", request.tool_name));
        code.push_str(&format!("    description = \"{}\",\n", request.description));
        if !request.tags.is_empty() {
            code.push_str(&format!("    tags = [{}],\n", 
                request.tags.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ")));
        }
        code.push_str(")]\n");
        
        // 函数签名
        if !request.parameters.is_empty() {
            code.push_str(&format!("pub async fn {}(params: Params) -> Result<{}> {{\n", 
                request.tool_name, self.map_type_to_rust(&request.return_type)));
        } else {
            code.push_str(&format!("pub async fn {}() -> Result<{}> {{\n", 
                request.tool_name, self.map_type_to_rust(&request.return_type)));
        }
        
        // 函数体（占位符）
        code.push_str("    // TODO: 实现工具逻辑\n");
        code.push_str(&format!("    unimplemented!(\"Tool {} not yet implemented\")\n", request.tool_name));
        code.push_str("}\n\n");
        
        // 测试模块
        if self.config.generate_tests {
            code.push_str("#[cfg(test)]\n");
            code.push_str("mod tests {\n");
            code.push_str("    use super::*;\n\n");
            code.push_str("    #[tokio::test]\n");
            code.push_str(&format!("    async fn test_{}() {{\n", request.tool_name));
            code.push_str("        // TODO: 实现测试\n");
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

use ai_assistant::tools::{}::{};

#[tokio::test]
async fn test_{}_basic() {{
    // TODO: 实现基本功能测试
}}

#[tokio::test]
async fn test_{}_edge_cases() {{
    // TODO: 实现边界条件测试
}}
"#,
            request.description,
            self.normalize_domain(&request.domain),
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
        
        let doc_content = format!(
            r#"# {tool_name}

{description}

## 所属领域
{domain}

## 标签
{tags}

## 参数

{parameters}

## 返回值

{return_type}

## 使用示例

```rust
// TODO: 添加使用示例
```

## 注意事项

- TODO: 添加注意事项

## 相关文件

- 实现：`src/tools/{domain}/{tool_name}.rs`
- 测试：`src/tools/{domain}/tests/test_{tool_name}.rs`
"#,
            tool_name = request.tool_name,
            description = request.description,
            domain = request.domain,
            tags = request.tags.join(", "),
            parameters = if request.parameters.is_empty() {
                "无参数".to_string()
            } else {
                request.parameters.iter()
                    .map(|p| format!("- `{}` ({}): {}", p.name, p.param_type, p.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            return_type = request.return_type,
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
            rules_obj.insert(domain_key.clone(), serde_json::json!({
                "keywords": [],
                "patterns": []
            }));
        }
        
        // 添加工具关键词
        let domain_rules = rules_obj.get_mut(&domain_key).unwrap();
        
        if let Some(keywords) = domain_rules.get_mut("keywords").and_then(|v| v.as_array_mut()) {
            // 添加工具名称作为关键词
            let tool_name_lower = request.tool_name.to_lowercase();
            if !keywords.iter().any(|k| k.as_str() == Some(&tool_name_lower)) {
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
    fn normalize_domain(&self, domain: &str) -> String {
        domain.to_lowercase()
            .replace(' ', "_")
            .replace("操作", "_ops")
            .replace("文件", "file")
            .replace("网络", "network")
            .replace("系统", "system")
            .replace("数据", "data")
            .replace("代码", "code")
            .replace("搜索", "search")
            .replace("版本控制", "vcs")
            .replace("知识管理", "knowledge")
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
    pub fn create_from_gap(&self, gap_name: &str, gap_description: &str, suggested_capabilities: &[String]) -> Result<ToolCreationResult> {
        // 从缺口信息生成工具定义
        let tool_name = self.generate_tool_name_from_gap(gap_name);
        
        let request = ToolCreationRequest {
            tool_name,
            description: gap_description.to_string(),
            domain: "通用".to_string(),
            tags: vec!["auto_generated".to_string()],
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
        gap_name
            .to_lowercase()
            .replace(' ', "_")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
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
    fn test_tool_creation() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();
        
        let request = ToolCreationRequest {
            tool_name: "test_tool".to_string(),
            description: "Test tool for demonstration".to_string(),
            domain: "file_ops".to_string(),
            tags: vec!["test".to_string()],
            parameters: vec![
                ParameterDef {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "File path".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            return_type: "String".to_string(),
            creation_reason: "Testing".to_string(),
            priority: 5,
        };
        
        let result = creator.create_tool(request).unwrap();
        
        assert!(result.success);
        assert!(!result.generated_files.is_empty());
    }

    #[test]
    fn test_normalize_domain() {
        let temp_dir = TempDir::new().unwrap();
        let creator = ToolCreator::new(temp_dir.path()).unwrap();
        
        assert_eq!(creator.normalize_domain("文件操作"), "file_ops");
        assert_eq!(creator.normalize_domain("网络"), "network");
        assert_eq!(creator.normalize_domain("Data"), "data");
    }
}
