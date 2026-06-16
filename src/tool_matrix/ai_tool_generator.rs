//! AI 驱动的工具生成器
//!
//! 基于 AI 的工具代码生成系统，支持：
//! - 根据用户描述自动生成工具代码
//! - 4 种模板（基础/网络/文件/AI）
//! - 自动生成测试用例
//! - 编译验证
//!
//! ## 工作流程
//! 1. 用户描述工具功能
//! 2. AI 生成工具代码框架
//! 3. 自动添加 #[tool] 宏
//! 4. 编译验证
//! 5. 生成单元测试模板

use crate::llm::{ChatRequest, LLMManager, Message};
use crate::tool_matrix::tool_generator::ToolGenerator;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// AI 工具生成请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIToolGenerationRequest {
    /// 工具名称
    pub tool_name: String,
    /// 工具功能描述
    pub description: String,
    /// 目标类别
    pub category: ToolCategory,
    /// 输入参数描述
    pub parameters_description: Option<String>,
    /// 是否需要测试
    pub generate_tests: bool,
    /// 输出目录
    pub output_dir: PathBuf,
}

/// 工具类别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// 基础工具（简单本地操作）
    Basic,
    /// 网络工具（HTTP 请求、API 调用）
    Network,
    /// 文件工具（文件读写、目录操作）
    File,
    /// AI 工具（LLM 调用、embeddings）
    Ai,
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCategory::Basic => write!(f, "basic"),
            ToolCategory::Network => write!(f, "network"),
            ToolCategory::File => write!(f, "file"),
            ToolCategory::Ai => write!(f, "ai"),
        }
    }
}

/// AI 工具生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIToolGenerationResult {
    /// 生成的代码
    pub code: String,
    /// 生成的测试代码
    pub tests: Option<String>,
    /// 工具文件路径
    pub tool_file_path: PathBuf,
    /// 测试文件路径
    pub test_file_path: Option<PathBuf>,
    /// 使用的模板
    pub template_used: String,
    /// AI 生成日志
    pub generation_log: Vec<String>,
}

/// AI 工具生成器
pub struct AIToolGenerator {
    /// LLM 管理器
    llm_manager: LLMManager,
    /// 基础工具生成器
    tool_generator: ToolGenerator,
}

impl AIToolGenerator {
    /// 创建新的 AI 工具生成器
    pub async fn new() -> Result<Self> {
        let llm_manager = LLMManager::new();

        // TODO: 从配置加载提供商
        // 目前 AI 工具生成器需要用户先配置 LLM 提供商

        // 尝试从默认目录加载模板
        let tool_generator = ToolGenerator::from_default_dir().unwrap_or_else(|e| {
            warn!("加载模板目录失败：{}, 使用空模板", e);
            let empty_dir = tempfile::tempdir().unwrap_or_else(|_| tempfile::tempdir().unwrap());
            ToolGenerator::new(empty_dir.path()).unwrap()
        });

        Ok(Self {
            llm_manager,
            tool_generator,
        })
    }

    /// 设置 LLM 管理器
    pub fn with_llm_manager(mut self, llm_manager: LLMManager) -> Self {
        self.llm_manager = llm_manager;
        self
    }

    /// 生成工具代码
    pub async fn generate_tool(
        &self,
        request: AIToolGenerationRequest,
    ) -> Result<AIToolGenerationResult> {
        let mut generation_log = Vec::new();

        info!(
            "开始 AI 生成工具：{} (类别：{})",
            request.tool_name, request.category
        );
        generation_log.push(format!("开始生成工具：{}", request.tool_name));

        // 检查是否有可用的提供商
        let provider = self.llm_manager.get_default_provider();

        if provider.is_none() {
            warn!("没有可用的 LLM 提供商，使用模板生成");
            generation_log.push("无可用 LLM 提供商，使用模板生成".to_string());
            return self.generate_with_template(request, &mut generation_log);
        }

        let provider = provider.unwrap();

        // 1. 使用 AI 生成工具代码框架
        let prompt = self.build_generation_prompt(&request);
        generation_log.push(format!("生成 AI 提示：{} 字符", prompt.len()));

        let chat_request = ChatRequest {
            model: provider.default_model().to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: 0.7,
            max_tokens: Some(4096),
            top_p: None,
            stop: None,
            stream: false,
            tools: None,
        };

        let ai_response = provider.chat(chat_request).await;

        let ai_code = match ai_response {
            Ok(response) => {
                generation_log.push("AI 响应接收成功".to_string());
                response.content
            }
            Err(e) => {
                warn!("AI 生成失败：{}, 使用模板生成", e);
                generation_log.push(format!("AI 生成失败：{}", e));
                // 降级到模板生成
                return self.generate_with_template(request, &mut generation_log);
            }
        };

        // 2. 解析和格式化 AI 生成的代码
        let formatted_code = self.parse_and_format_code(&ai_code, &request);
        generation_log.push("代码格式化完成".to_string());

        // 3. 生成测试代码（如果需要）
        let test_code = if request.generate_tests {
            let test_prompt = self.build_test_generation_prompt(&request, &formatted_code);

            let test_chat_request = ChatRequest {
                model: provider.default_model().to_string(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: test_prompt,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }],
                temperature: 0.5,
                max_tokens: Some(2048),
                top_p: None,
                stop: None,
                stream: false,
                tools: None,
            };

            let test_response: Option<crate::llm::ChatResponse> =
                provider.chat(test_chat_request).await.ok();

            test_response.map(|r| {
                generation_log.push("测试代码生成完成".to_string());
                r.content
            })
        } else {
            None
        };

        // 4. 保存文件
        let tool_file_path =
            self.save_tool_code(&request.tool_name, &formatted_code, &request.output_dir)?;
        generation_log.push(format!("工具代码已保存：{:?}", tool_file_path));

        let test_file_path = if let Some(tests) = &test_code {
            let path = self.save_test_code(&request.tool_name, tests, &request.output_dir)?;
            generation_log.push(format!("测试代码已保存：{:?}", path));
            Some(path)
        } else {
            None
        };

        Ok(AIToolGenerationResult {
            code: formatted_code,
            tests: test_code,
            tool_file_path,
            test_file_path,
            template_used: "ai_generated".to_string(),
            generation_log,
        })
    }

    /// 使用模板生成（降级方案）
    fn generate_with_template(
        &self,
        request: AIToolGenerationRequest,
        log: &mut Vec<String>,
    ) -> Result<AIToolGenerationResult> {
        log.push("使用模板生成（降级方案）".to_string());

        // 根据类别选择模板
        let template_id = match request.category {
            ToolCategory::Basic => "tool_template_basic",
            ToolCategory::Network => "tool_template_network",
            ToolCategory::File => "tool_template_file",
            ToolCategory::Ai => "tool_template_ai",
        };

        log.push(format!("使用模板：{}", template_id));

        // 使用基础生成器
        let code = ToolGenerator::generate_with_tokitai_macro(
            &request.tool_name,
            &request.description,
            vec![], // 空参数，AI 生成失败时使用默认
            None,
        )?;

        let tool_file_path = self.save_tool_code(&request.tool_name, &code, &request.output_dir)?;

        Ok(AIToolGenerationResult {
            code,
            tests: None,
            tool_file_path,
            test_file_path: None,
            template_used: template_id.to_string(),
            generation_log: log.clone(),
        })
    }

    /// 构建工具生成提示
    fn build_generation_prompt(&self, request: &AIToolGenerationRequest) -> String {
        let category_desc = match request.category {
            ToolCategory::Basic => "基础工具，适用于简单的本地操作（字符串处理、计算等）",
            ToolCategory::Network => "网络工具，需要发起 HTTP 请求、调用 API 或抓取网页",
            ToolCategory::File => "文件工具，需要读写文件、遍历目录或操作文件系统",
            ToolCategory::Ai => "AI 工具，需要调用 LLM、生成 embeddings 或其他 AI 服务",
        };

        let params_hint = if let Some(params_desc) = &request.parameters_description {
            format!("输入参数要求：{}\n", params_desc)
        } else {
            "请根据工具功能自行推断合适的输入参数。\n".to_string()
        };

        format!(
            r#"# 工具生成任务

你是一个专业的 Rust 开发者，需要为一个基于 tokitai 的 AI 助手生成工具代码。

## 工具信息
- 工具名称：{}
- 工具描述：{}
- 工具类别：{} ({})

{}
## 要求

1. 使用 `#[tool]` 宏标记工具结构体
2. 工具结构体命名为 `{struct_name}`（工具名的驼峰式）
3. 实现工具方法，方法名与工具名相同
4. 方法签名：`pub fn {tool_name}(&self, ...) -> Result<String, String>`
5. 包含适当的错误处理
6. 添加 Rust 文档注释

## 输出格式

只输出 Rust 代码，不要包含 markdown 代码块标记。代码应该可以直接编译。

## 示例结构

```rust
use tokitai::tool;

/// {struct_name} - {description}
#[derive(Debug, Clone, Default)]
pub struct {struct_name};

#[tool]
impl {struct_name} {{
    /// {description}
    pub fn {tool_name}(&self, /* 参数 */) -> Result<String, String> {{
        // 实现逻辑
        Ok("结果".to_string())
    }}
}}
```"#,
            request.tool_name,
            request.description,
            request.category,
            category_desc,
            params_hint,
            struct_name = self.to_camel_case(&request.tool_name),
            tool_name = request.tool_name,
            description = request.description,
        )
    }

    /// 构建测试生成提示
    fn build_test_generation_prompt(
        &self,
        request: &AIToolGenerationRequest,
        code: &str,
    ) -> String {
        format!(
            r#"# 测试生成任务

为以下 Rust 工具生成单元测试代码。

## 工具代码
{}

## 要求

1. 使用 `#[tokio::test]` 标记异步测试
2. 测试应该覆盖正常情况和错误情况
3. 使用 `assert!` 系列宏进行断言
4. 测试函数命名：`test_{{tool_name}}_{{scenario}}`

## 输出格式

只输出 Rust 测试代码，不要包含 markdown 代码块标记。"#,
            code
        )
    }

    /// 解析和格式化 AI 生成的代码
    fn parse_and_format_code(&self, ai_code: &str, request: &AIToolGenerationRequest) -> String {
        // 移除可能的 markdown 代码块标记
        let code = ai_code
            .trim()
            .trim_start_matches("```rust")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // 确保包含必要的导入
        let mut formatted = String::new();

        if !code.contains("use tokitai::tool") {
            formatted.push_str("use tokitai::tool;\n\n");
        }

        formatted.push_str(code);

        // 确保结构体名称正确
        let expected_struct = self.to_camel_case(&request.tool_name);
        if !formatted.contains(&format!("struct {}", expected_struct)) {
            // 尝试修复结构体名称
            formatted = formatted.replace(
                &format!("struct {}", request.tool_name),
                &format!("struct {}", expected_struct),
            );
        }

        formatted
    }

    /// 保存工具代码
    fn save_tool_code(&self, tool_name: &str, code: &str, output_dir: &Path) -> Result<PathBuf> {
        fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败：{:?}", output_dir))?;

        let file_path = output_dir.join(format!("{}.rs", tool_name));
        fs::write(&file_path, code)
            .with_context(|| format!("写入工具代码失败：{:?}", file_path))?;

        info!("工具代码已保存：{:?}", file_path);

        Ok(file_path)
    }

    /// 保存测试代码
    fn save_test_code(&self, tool_name: &str, tests: &str, output_dir: &Path) -> Result<PathBuf> {
        let file_path = output_dir.join(format!("test_{}.rs", tool_name));
        fs::write(&file_path, tests)
            .with_context(|| format!("写入测试代码失败：{:?}", file_path))?;

        info!("测试代码已保存：{:?}", file_path);

        Ok(file_path)
    }

    /// 转换为驼峰式命名
    fn to_camel_case(&self, name: &str) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case_conversion() {
        let generator = AIToolGenerator {
            llm_manager: LLMManager::new(),
            tool_generator: ToolGenerator::from_default_dir().unwrap_or_else(|_| {
                let dir = tempfile::tempdir().unwrap();
                ToolGenerator::new(dir.path()).unwrap()
            }),
        };

        assert_eq!(generator.to_camel_case("web_search"), "WebSearch");
        assert_eq!(generator.to_camel_case("read_file"), "ReadFile");
        assert_eq!(generator.to_camel_case("http_client"), "HttpClient");
    }

    #[test]
    fn test_tool_category_display() {
        assert_eq!(ToolCategory::Basic.to_string(), "basic");
        assert_eq!(ToolCategory::Network.to_string(), "network");
        assert_eq!(ToolCategory::File.to_string(), "file");
        assert_eq!(ToolCategory::Ai.to_string(), "ai");
    }
}
