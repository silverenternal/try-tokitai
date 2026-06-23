//! 工具生成器
//!
//! 基于模板的工具代码生成系统，支持：
//! - 模板渲染
//! - 类型检查
//! - 测试生成
//! - 自动注册
//!
//! ## IMP-002: tokitai-macros 集成
//! - 使用 `tokitai::tool` 宏生成工具代码骨架

#![allow(dead_code)]

use anyhow::{Context as AnyhowContext, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use tracing::{info, warn};

use crate::security::SecurityConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTemplate {
    pub template: TemplateMetadata,
    pub parameters: HashMap<String, ParameterDefinition>,
    pub code: CodeTemplate,
    pub tests: TestTemplate,
    #[serde(default)]
    pub examples: ExampleTemplate,
    #[serde(default)]
    pub safety: SafetyNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTemplate {
    pub language: String,
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTemplate {
    pub language: String,
    pub template: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExampleTemplate {
    #[serde(default)]
    pub usage: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SafetyNotes {
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct ToolGenerationRequest {
    pub tool_name: String,
    pub tool_description: String,
    pub template_id: String,
    pub parameters: HashMap<String, String>,
    pub target_path: PathBuf,
    pub generate_tests: bool,
}

#[derive(Debug, Clone)]
pub struct ToolGenerationResult {
    pub code: String,
    pub tests: Option<String>,
    pub file_path: PathBuf,
    pub test_file_path: Option<PathBuf>,
}

pub struct ToolGenerator {
    tera: Tera,
    templates: HashMap<String, ToolTemplate>,
    template_dir: PathBuf,
    security_config: SecurityConfig,
}

impl ToolGenerator {
    pub fn new<P: AsRef<Path>>(template_dir: P) -> Result<Self> {
        Self::with_security_config(template_dir, SecurityConfig::default())
    }

    pub fn with_security_config<P: AsRef<Path>>(
        template_dir: P,
        security_config: SecurityConfig,
    ) -> Result<Self> {
        let template_dir = template_dir.as_ref().to_path_buf();
        let tera = match Tera::new(&format!("{}/**/*.toml", template_dir.display())) {
            Ok(t) => t,
            Err(e) => {
                warn!("解析模板目录失败: {}, 使用空模板引擎", e);
                Tera::default()
            }
        };

        let mut generator = Self {
            tera,
            templates: HashMap::new(),
            template_dir,
            security_config,
        };
        generator.load_templates()?;
        Ok(generator)
    }

    pub fn from_default_dir() -> Result<Self> {
        Self::from_default_dir_with_security_config(SecurityConfig::default())
    }

    pub fn from_default_dir_with_security_config(
        security_config: SecurityConfig,
    ) -> Result<Self> {
        let default_dir = PathBuf::from("templates/tools");
        if default_dir.exists() {
            Self::with_security_config(default_dir, security_config)
        } else {
            let crate_root = std::env::current_dir()?;
            Self::with_security_config(crate_root.join("templates/tools"), security_config)
        }
    }

    pub fn load_templates(&mut self) -> Result<()> {
        if !self.template_dir.exists() {
            warn!("模板目录不存在: {:?}", self.template_dir);
            return Ok(());
        }

        for entry in fs::read_dir(&self.template_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }

            match self.load_template(&path) {
                Ok(template) => {
                    info!("加载模板: {} ({})", template.template.name, template.template.id);
                    self.templates
                        .insert(template.template.id.clone(), template);
                }
                Err(e) => warn!("加载模板失败 {:?}: {}", path, e),
            }
        }

        info!("共加载 {} 个工具模板", self.templates.len());
        Ok(())
    }

    fn load_template(&self, path: &Path) -> Result<ToolTemplate> {
        let content =
            fs::read_to_string(path).with_context(|| format!("读取模板文件失败: {:?}", path))?;
        let template: ToolTemplate =
            toml::from_str(&content).with_context(|| format!("解析模板 TOML 失败: {:?}", path))?;
        Ok(template)
    }

    pub fn generate_tool(&self, request: ToolGenerationRequest) -> Result<ToolGenerationResult> {
        let template = self
            .templates
            .get(&request.template_id)
            .ok_or_else(|| anyhow::anyhow!("模板不存在: {}", request.template_id))?;

        let mut context = Context::new();
        context.insert("tool_name", &request.tool_name);
        context.insert("tool_description", &request.tool_description);
        context.insert("template_id", &request.template_id);

        let param_names: Vec<&str> = request.parameters.keys().map(|s| s.as_str()).collect();
        let param_types: Vec<String> = param_names.iter().map(|_| "String".to_string()).collect();
        context.insert("param_names", &param_names.join(", "));
        context.insert("param_types", &param_types.join(", "));

        for (key, value) in &request.parameters {
            context.insert(key, value);
            context.insert(format!("{}_param", key), &"{}");
        }

        context.insert(
            "tool_body",
            &self.generate_tool_body(template, &request.parameters),
        );

        let code = self.render_template(&template.code.template, &context)?;
        let tests = if request.generate_tests {
            let test_context = self.build_test_context(&request, template)?;
            Some(self.render_template(&template.tests.template, &test_context)?)
        } else {
            None
        };

        let allowed_paths = &self.security_config.allowed_tool_gen_paths;
        if !allowed_paths.is_empty() {
            let canonical_target = request
                .target_path
                .canonicalize()
                .unwrap_or_else(|_| request.target_path.clone());
            let is_allowed = allowed_paths.iter().any(|root| {
                canonical_target.starts_with(root)
                    || canonical_target
                        .starts_with(root.canonicalize().unwrap_or_else(|_| root.clone()))
            });
            if !is_allowed {
                return Err(anyhow::anyhow!(
                    "Tool generation path {:?} is not in allowed directories: {:?}",
                    canonical_target,
                    allowed_paths
                ));
            }
        }

        if let Some(parent) = request.target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&request.target_path, &code)?;
        info!("生成工具代码: {:?}", request.target_path);

        let test_file_path = if request.generate_tests {
            if let Some(tests) = &tests {
                let test_path = request
                    .target_path
                    .with_file_name(format!("test_{}.rs", request.tool_name));
                if let Some(parent) = test_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&test_path, tests)?;
                info!("生成测试代码: {:?}", test_path);
                Some(test_path)
            } else {
                None
            }
        } else {
            None
        };

        Ok(ToolGenerationResult {
            code,
            tests,
            file_path: request.target_path,
            test_file_path,
        })
    }

    fn generate_tool_body(
        &self,
        template: &ToolTemplate,
        parameters: &HashMap<String, String>,
    ) -> String {
        match template.template.category.as_str() {
            "file_ops" => self.generate_file_ops_body(parameters),
            "network_ops" => self.generate_network_ops_body(parameters),
            "code_ops" => self.generate_analysis_ops_body(parameters),
            "system_ops" => self.generate_cli_ops_body(parameters),
            "data_ops" => self.generate_data_ops_body(parameters),
            other => format!(
                "// 默认实现\nunimplemented!(\"Tool category '{}' not implemented\")",
                other
            ),
        }
    }

    fn generate_file_ops_body(&self, parameters: &HashMap<String, String>) -> String {
        let operation = parameters
            .get("operation")
            .map(|s| s.as_str())
            .unwrap_or("read");

        match operation {
            "read" => "let content = fs::read_to_string(&path)?;\nOk(content)".to_string(),
            "write" => "fs::write(&path, &content)?;\nOk(format!(\"Successfully wrote to {}\", path))".to_string(),
            "copy" => "fs::copy(&path, &dest_path)?;\nOk(format!(\"Copied {} to {}\", path, dest_path))".to_string(),
            "delete" => "fs::remove_file(&path)?;\nOk(format!(\"Deleted {}\", path))".to_string(),
            "list" => r#"let mut files = Vec::new();
for entry in fs::read_dir(&path)? {
    let entry = entry?;
    files.push(entry.file_name().to_string_lossy().to_string());
}
Ok(files.join("\n"))"#
                .to_string(),
            _ => format!(
                "unimplemented!(\"Operation '{}' not supported for file_ops\")",
                operation
            ),
        }
    }

    fn generate_network_ops_body(&self, parameters: &HashMap<String, String>) -> String {
        let method = parameters
            .get("method")
            .map(|s| s.as_str())
            .unwrap_or("get");

        match method {
            "get" => "let response = client.get(&url).send().await?;\nOk(response.text().await?)"
                .to_string(),
            "post" => r#"let response = client.post(&url)
    .json(&body)
    .send()
    .await?;
Ok(response.text().await?)"#
                .to_string(),
            _ => format!("unimplemented!(\"HTTP method '{}' not supported\")", method),
        }
    }

    fn generate_analysis_ops_body(&self, _parameters: &HashMap<String, String>) -> String {
        "let lines = content.lines().count();\nOk(lines.to_string())".to_string()
    }

    fn generate_cli_ops_body(&self, _parameters: &HashMap<String, String>) -> String {
        r#"let output = std::process::Command::new(&command).output().map_err(|e| e.to_string())?;
Ok(String::from_utf8_lossy(&output.stdout).to_string())"#
            .to_string()
    }

    fn generate_data_ops_body(&self, _parameters: &HashMap<String, String>) -> String {
        r#"let value: serde_json::Value = serde_json::from_str(&input).map_err(|e| e.to_string())?;
Ok(serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?)"#
            .to_string()
    }

    fn build_test_context(
        &self,
        request: &ToolGenerationRequest,
        template: &ToolTemplate,
    ) -> Result<Context> {
        let mut context = Context::new();
        context.insert("tool_name", &request.tool_name);
        let test_input = self.generate_test_input(template, &request.parameters);
        context.insert("test_input", &test_input);
        Ok(context)
    }

    fn generate_test_input(
        &self,
        template: &ToolTemplate,
        _parameters: &HashMap<String, String>,
    ) -> String {
        match template.template.category.as_str() {
            "file_ops" => "\"test_file.txt\"".to_string(),
            "network_ops" => "\"https://httpbin.org/get\"".to_string(),
            "code_ops" => "\"src/main.rs\"".to_string(),
            "system_ops" => "\"echo\", vec![\"hello\"]".to_string(),
            "data_ops" => r#"{"key": "value"}"#.to_string(),
            _ => "\"test_input\"".to_string(),
        }
    }

    fn render_template(&self, template: &str, context: &Context) -> Result<String> {
        let mut result = template.to_string();
        let keys = [
            "tool_name",
            "tool_description",
            "template_id",
            "param_names",
            "param_types",
            "tool_body",
            "path_param",
            "url_param",
            "timeout_secs",
            "test_input",
        ];

        for key in keys {
            let placeholder = format!("{{{{{}}}}}", key);
            if let Some(val) = context.get(key) {
                if let Some(val_str) = val.as_str() {
                    result = result.replace(&placeholder, val_str);
                } else if let Some(val_bool) = val.as_bool() {
                    result = result.replace(&placeholder, &val_bool.to_string());
                } else if let Some(val_i64) = val.as_i64() {
                    result = result.replace(&placeholder, &val_i64.to_string());
                } else if let Some(val_u64) = val.as_u64() {
                    result = result.replace(&placeholder, &val_u64.to_string());
                }
            }
        }

        Ok(self.process_conditionals(&result, context))
    }

    fn process_conditionals(&self, template: &str, context: &Context) -> String {
        let mut result = template.to_string();
        let if_pattern = regex::Regex::new(r"\{\{#if\s+(\w+)\}\}(.*?)\{\{/if\}\}").unwrap();

        for cap in if_pattern.captures_iter(template) {
            let var_name = &cap[1];
            let block_content = &cap[2];
            let has_var =
                context.get(var_name).is_some() || context.get(&format!("{}_param", var_name)).is_some();
            let replacement = if has_var { block_content } else { "" };
            result = result.replace(&cap[0], replacement);
        }

        result
    }

    pub fn list_templates(&self) -> Vec<&ToolTemplate> {
        self.templates.values().collect()
    }

    pub fn get_template(&self, template_id: &str) -> Option<&ToolTemplate> {
        self.templates.get(template_id)
    }

    pub fn generate_with_tokitai_macro(
        tool_name: &str,
        tool_description: &str,
        parameters: Vec<(String, String)>,
        struct_name: Option<&str>,
    ) -> Result<String> {
        info!("使用 tokitai::tool 宏生成工具代码: {}", tool_name);

        let struct_name = struct_name
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| to_camel_case(tool_name));

        let fn_params = if parameters.is_empty() {
            "&self".to_string()
        } else {
            format!(
                "&self, {}",
                parameters
                    .iter()
                    .map(|(name, typ)| format!("{}: {}", name, typ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let param_docs = if parameters.is_empty() {
            "/// - 无参数".to_string()
        } else {
            parameters
                .iter()
                .map(|(name, typ)| format!("/// - `{}`: {}", name, typ))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let code = format!(
            r#"//! {tool_description}
//!
//! ## IMP-002: tokitai-macros 生成
//! - 使用 tokitai::tool 宏自动生成元数据
//! - 零手动定义 ToolDefinition
//! - 编译时类型检查

use tokitai::tool;

/// {struct_name} - {tool_description}
#[derive(Debug, Clone, Default)]
pub struct {struct_name};

#[tool]
impl {struct_name} {{
    /// {tool_description}
    ///
{param_docs}
    pub fn {tool_name}({fn_params}) -> Result<String, String> {{
        unimplemented!("Implement {tool_name} tool logic")
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_{tool_name}_creation() {{
        let _tool = {struct_name}::default();
    }}
}}
"#
        );

        info!("生成工具代码完成: {} 字节", code.len());
        Ok(code)
    }

    pub fn generate_tool_file(
        tool_name: &str,
        tool_description: &str,
        parameters: Vec<(String, String)>,
        output_path: &Path,
    ) -> Result<PathBuf> {
        let code =
            Self::generate_with_tokitai_macro(tool_name, tool_description, parameters, None)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, &code)?;
        info!("工具代码已保存到: {:?}", output_path);
        Ok(output_path.to_path_buf())
    }

    pub fn generate_from_tool_definition(
        tool_name: &str,
        tool_description: &str,
        input_schema_json: &str,
        output_path: &Path,
    ) -> Result<PathBuf> {
        let parameters = Self::parse_json_schema(input_schema_json)?;
        Self::generate_tool_file(tool_name, tool_description, parameters, output_path)
    }

    fn parse_json_schema(schema_json: &str) -> Result<Vec<(String, String)>> {
        use serde_json::Value;

        let schema: Value = serde_json::from_str(schema_json)
            .map_err(|e| anyhow::anyhow!("解析 JSON Schema 失败: {}", e))?;

        let mut parameters = Vec::new();
        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            for (name, prop) in props {
                let param_type = prop
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("string");

                let rust_type = match param_type {
                    "string" => "String",
                    "integer" => "i64",
                    "number" => "f64",
                    "boolean" => "bool",
                    "array" => "Vec<String>",
                    "object" => "serde_json::Value",
                    _ => "String",
                };

                parameters.push((name.clone(), rust_type.to_string()));
            }
        }

        Ok(parameters)
    }
}

fn to_camel_case(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_tool_generator_creation() {
        let dir = tempdir().unwrap();
        let template_path = dir.path().join("file_tool.toml");

        let mut template_file = fs::File::create(&template_path).unwrap();
        writeln!(
            template_file,
            r#"
[template]
id = "test_file_tool"
name = "Test File Tool"
description = "A test template"
category = "file_ops"
version = "1.0.0"

[parameters]
path = {{ type = "string", description = "File path", required = true }}

[code]
language = "rust"
template = "pub fn {{tool_name}}() {{}}"

[tests]
language = "rust"
template = '''#[tokio::test] async fn test_{{tool_name}}() {{}}'''
"#
        )
        .unwrap();

        let generator = ToolGenerator::new(dir.path()).unwrap();
        assert_eq!(generator.templates.len(), 1);
    }

    #[test]
    fn test_tool_generator_render() {
        let dir = tempdir().unwrap();
        let template_path = dir.path().join("simple_tool.toml");

        let template_content = r#"
[template]
id = "simple"
name = "Simple"
description = "Simple template"
category = "file_ops"
version = "1.0.0"

[parameters]
name = { type = "string", description = "Name", required = true }

[code]
language = "rust"
template = "pub fn {{tool_name}}() {}"

[tests]
language = "rust"
template = ""

[examples]
usage = ""

[safety]
notes = ""
"#;

        fs::write(&template_path, template_content).unwrap();
        let generator = ToolGenerator::new(dir.path()).unwrap();

        let mut parameters = HashMap::new();
        parameters.insert("name".to_string(), "test_param".to_string());

        let request = ToolGenerationRequest {
            tool_name: "my_tool".to_string(),
            tool_description: "My tool description".to_string(),
            template_id: "simple".to_string(),
            parameters,
            target_path: dir.path().join("my_tool.rs"),
            generate_tests: false,
        };

        let result = generator.generate_tool(request).unwrap();
        assert!(result.code.contains("pub fn my_tool()"));
    }

    #[test]
    fn test_generate_with_tokitai_macro() {
        let parameters = vec![
            ("path".to_string(), "String".to_string()),
            ("content".to_string(), "String".to_string()),
        ];

        let code = ToolGenerator::generate_with_tokitai_macro(
            "write_file",
            "Write content to file",
            parameters,
            Some("WriteFileTool"),
        )
        .unwrap();

        assert!(code.contains("use tokitai::tool"));
        assert!(code.contains("pub struct WriteFileTool"));
        assert!(code.contains("#[tool]"));
        assert!(code.contains("pub fn write_file"));
        assert!(code.contains("path: String"));
        assert!(code.contains("content: String"));
        assert!(code.contains("unimplemented!"));
        assert!(code.contains("mod tests"));
    }

    #[test]
    fn test_parse_json_schema() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "count": {"type": "integer"},
                "ratio": {"type": "number"},
                "enabled": {"type": "boolean"},
                "tags": {"type": "array"},
                "metadata": {"type": "object"}
            }
        }"#;

        let params = ToolGenerator::parse_json_schema(schema).unwrap();
        assert_eq!(params.len(), 6);

        let param_names: Vec<&String> = params.iter().map(|(name, _)| name).collect();
        assert!(param_names.contains(&&"path".to_string()));
        assert!(param_names.contains(&&"count".to_string()));
        assert!(param_names.contains(&&"ratio".to_string()));
        assert!(param_names.contains(&&"enabled".to_string()));
        assert!(param_names.contains(&&"tags".to_string()));
        assert!(param_names.contains(&&"metadata".to_string()));

        let params_map: std::collections::HashMap<_, _> = params.into_iter().collect();
        assert_eq!(params_map.get("path"), Some(&"String".to_string()));
        assert_eq!(params_map.get("count"), Some(&"i64".to_string()));
        assert_eq!(params_map.get("ratio"), Some(&"f64".to_string()));
        assert_eq!(params_map.get("enabled"), Some(&"bool".to_string()));
        assert_eq!(params_map.get("tags"), Some(&"Vec<String>".to_string()));
        assert_eq!(
            params_map.get("metadata"),
            Some(&"serde_json::Value".to_string())
        );
    }

    #[test]
    fn test_generate_tool_file() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test_tool.rs");
        let parameters = vec![("url".to_string(), "String".to_string())];

        let result_path = ToolGenerator::generate_tool_file(
            "fetch_url",
            "Fetch content from URL",
            parameters,
            &output_path,
        )
        .unwrap();

        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("use tokitai::tool"));
        assert!(content.contains("pub fn fetch_url"));
    }
}
