//! CLI AI 助手 - 面向用户的交互式助手
//!
//! 提供交互式对话、工具调用、文件操作等功能
//!
//! # 使用场景
//! - 交互式对话
//! - 响应用户查询
//! - 执行用户指定的工具调用
//! - 文件操作、代码分析、网络请求等临时任务
//!
//! # 启动命令
//! ```bash
//! cargo run --release
//! ```
//!
//! # 服务边界
//! - ✅ 响应用户查询
//! - ✅ 执行用户指定的工具调用
//! - ❌ 不主动修改项目代码
//! - ❌ 不自主发起 Git 操作

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use serde_json::{json, Value};
use std::io::{self, Write};
use tracing::{info, warn};

use crate::assistant_common::{
    curated_ai_scientist_tool_names, register_all_builtin_tools, AssistantConfig, ToolManager,
};
use crate::config::Config;
use crate::domain_prompt::{science_expert_system_prompt, strip_emoji};
use crate::integration::IntegratedModules;
use crate::integration::IntegratedModulesConfig;
use crate::llm::{LLMManager, ModelCommandHandler, ProviderInitializer};
use crate::orchestrator::Orchestrator;
use crate::path_resolver;
use crate::scientist::tools::computation::ComputationTools;
use crate::scientist::tools::data::DataTools;
use crate::scientist::tools::github::GitHubTools;
use crate::scientist::tools::literature::LiteratureTools;
use crate::scientist::tools::sympy_tool::SymPyTool;
use crate::tools::data::{DataConversionTools, JsonMergeTools, JsonQueryTools};
use crate::tools::io::security::{SandboxConfig, SecurePathResolver};
use crate::tools::HttpClientTools;
use crate::tools::{
    CodeTools, DownloadTools, FileOperations, FileSearchTools, GitOperations, JsonFormatTools,
    NetworkTools, PdfTools, ProcessTools, ProjectTemplates, SearchTools, SystemTools,
    WikipediaTools,
};
use crate::tui::components::message_block::{MessageBlock, ToolCallStatus};
use crate::web::run_terminal_command_structured;
use std::sync::Arc;

fn is_path_recovery_tool(name: &str) -> bool {
    matches!(
        name,
        "inspect_path"
            | "list_dir"
            | "find_files"
            | "read_file"
            | "read_file_head"
            | "read_file_range"
    )
}

fn is_path_related_failure(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("file_not_found")
        || lowered.contains("path_not_found")
        || lowered.contains("not_a_directory")
        || lowered.contains("os error 3")
        || lowered.contains("system cannot find the path")
        || message.contains("文件不存在")
        || message.contains("路径不存在")
        || message.contains("不是目录")
        || message.contains("目标路径")
}

fn build_path_recovery_system_message(tool_name: &str, tool_error: &str) -> String {
    format!(
        "Path recovery rule:\n- The last {tool_name} call failed with a path-related error.\n- Do not continue using the failed path.\n- Do not infer sibling or child paths from design documents, TREE.md, README, or memory.\n- Your next step must be one of: inspect_path on the target or parent path, list_dir on a confirmed parent directory, or find_files in a confirmed ancestor directory.\n- Only after tool evidence confirms the real path may you call read_file or continue analysis.\n- Do not ask the user whether to continue until you have attempted path recovery.\n\nObserved error:\n{tool_error}"
    )
}

/// CLI AI 助手 - 面向用户的交互式助手
pub struct CliAssistant {
    /// 助手配置
    config: AssistantConfig,
    /// 工具管理器
    tool_manager: ToolManager,
    /// 编排器（角色切换和上下文优化）
    orchestrator: Orchestrator,
    /// 集成模块（dialogue、observability、prompt_engineering）
    #[allow(dead_code)]
    integrated_modules: IntegratedModules,
    /// 工具实例（用于 call_tool 调用）
    file_ops: FileOperations,
    file_search: FileSearchTools,
    pdf_tools: PdfTools,
    project_templates: ProjectTemplates,
    system_tools: SystemTools,
    process_tools: ProcessTools,
    code_tools: CodeTools,
    web_search: SearchTools,
    download_tools: DownloadTools,
    network_tools: NetworkTools,
    wikipedia_tools: WikipediaTools,
    git_ops: GitOperations,
    http_client: HttpClientTools,
    json_tools: JsonFormatTools,
    json_query: JsonQueryTools,
    json_merge: JsonMergeTools,
    data_conversion: DataConversionTools,
    /// 科学家工具实例
    literature_tools: LiteratureTools,
    computation_tools: ComputationTools,
    data_tools: DataTools,
    github_tools: GitHubTools,
    /// SymPy 数学验证工具
    sympy_tool: SymPyTool,
    /// LLM 管理器（多提供商支持）
    #[allow(dead_code)]
    llm_manager: Arc<LLMManager>,
    /// 模型命令处理器
    #[allow(dead_code)]
    model_handler: Option<ModelCommandHandler>,
    /// 配置文件
    #[allow(dead_code)]
    config_file: Config,
    /// 安全配置
    pub security_config: crate::security::SecurityConfig,
}

#[derive(Debug, Clone)]
pub struct ChatRunResult {
    pub content: String,
    pub trace: Vec<MessageBlock>,
}

impl CliAssistant {
    fn path_resolver(&self) -> SecurePathResolver {
        SecurePathResolver::with_config(SandboxConfig {
            allowed_roots: self.security_config.allowed_roots.clone(),
            allow_symlinks: self.security_config.allow_symlinks,
            max_depth: self.security_config.max_path_depth as usize,
        })
    }

    /// 创建新的 CLI AI 助手
    ///
    /// # 参数
    /// - `config`: 助手配置
    /// - `security_config`: 安全配置（从 config.toml + 环境变量加载）
    pub fn new(
        config: AssistantConfig,
        security_config: crate::security::SecurityConfig,
    ) -> Result<Self> {
        // 加载配置文件
        let config_file = Config::load(None).unwrap_or_else(|e| {
            warn!("加载配置文件失败：{}，使用默认配置", e);
            Config::default()
        });

        // 初始化 LLM 管理器（多提供商支持）
        let provider_initializer = ProviderInitializer::new(config_file.clone());
        let llm_manager = match provider_initializer.initialize_llm_manager() {
            Ok(manager) => {
                let providers = manager.list_providers();
                info!("✅ 初始化 LLM 管理器，加载 {} 个提供商", providers.len());
                Arc::new(manager)
            }
            Err(e) => {
                warn!("⚠️  初始化 LLM 管理器失败：{}，使用单提供商模式", e);
                // 尝试从环境变量创建单提供商
                let mut manager = LLMManager::new();
                if let Ok(api_url) = std::env::var("AI_API_URL") {
                    let api_key = std::env::var("AI_API_KEY").ok();
                    let model =
                        std::env::var("AI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

                    let provider = Arc::new(crate::llm::providers::OpenAIProvider::with_base_url(
                        api_key.unwrap_or_default(),
                        api_url,
                        Some(model),
                    ));
                    manager.register_provider(provider);
                }
                Arc::new(manager)
            }
        };

        // 创建模型命令处理器
        let model_handler = if llm_manager.list_providers().len() > 1 {
            info!("✅ 启用多提供商模式，创建模型命令处理器");
            match ModelCommandHandler::new(llm_manager.clone(), config_file.clone()).with_router() {
                Ok(handler) => Some(handler),
                Err(e) => {
                    warn!("⚠️  创建模型命令处理器失败：{}", e);
                    None
                }
            }
        } else {
            info!("ℹ️  单提供商模式，跳过模型命令处理器");
            None
        };

        // 创建工具注册表
        let tool_registry = crate::tool_matrix::registry::ToolRegistry::new();

        // 注册所有内置工具
        register_all_builtin_tools(&tool_registry);

        // 创建工具管理器
        let tool_manager = ToolManager::new(tool_registry);

        // 创建集成模块
        let integrated_modules = match IntegratedModules::new(IntegratedModulesConfig::default()) {
            Ok(m) => m,
            Err(e) => {
                warn!("⚠️  创建集成模块失败：{}", e);
                IntegratedModules::new(IntegratedModulesConfig::for_testing()).unwrap()
            }
        };

        let mut integrated_modules = integrated_modules;

        // 初始化集成模块
        match integrated_modules.initialize() {
            Ok(init_report) => {
                if !init_report.success {
                    warn!("⚠️  集成模块初始化警告：");
                    for error in &init_report.errors {
                        warn!("  - {}", error);
                    }
                }
            }
            Err(e) => {
                warn!("⚠️  集成模块初始化失败：{}", e);
            }
        }

        let path_resolver = SecurePathResolver::with_config(SandboxConfig {
            allowed_roots: security_config.allowed_roots.clone(),
            allow_symlinks: security_config.allow_symlinks,
            max_depth: security_config.max_path_depth as usize,
        });

        Ok(Self {
            config,
            tool_manager,
            orchestrator: Orchestrator::new(),
            integrated_modules,
            file_ops: FileOperations::with_resolver(path_resolver.clone()),
            file_search: FileSearchTools::with_resolver(path_resolver.clone()),
            pdf_tools: PdfTools::with_resolver(path_resolver.clone()),
            project_templates: ProjectTemplates::with_resolver(path_resolver),
            system_tools: SystemTools::default(),
            process_tools: ProcessTools::default(),
            code_tools: CodeTools::default(),
            web_search: SearchTools::new(),
            download_tools: DownloadTools::new(),
            network_tools: NetworkTools::new(),
            wikipedia_tools: WikipediaTools::new(),
            git_ops: GitOperations,
            http_client: HttpClientTools::new(),
            json_tools: JsonFormatTools::default(),
            json_query: JsonQueryTools::default(),
            json_merge: JsonMergeTools::default(),
            data_conversion: DataConversionTools::default(),
            literature_tools: LiteratureTools,
            computation_tools: ComputationTools,
            data_tools: DataTools,
            github_tools: GitHubTools,
            sympy_tool: SymPyTool::new(),
            llm_manager,
            model_handler,
            config_file,
            security_config,
        })
    }

    /// 获取所有工具定义
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        self.tool_manager
            .get_tools_by_name(curated_ai_scientist_tool_names())
    }

    pub fn get_tool_definitions_with_extra(&self, extra_tools: &[Value]) -> Vec<Value> {
        let mut tools = self.get_tool_definitions();
        tools.extend(extra_tools.iter().cloned());
        tools
    }

    /// 获取 LLM 管理器（用于 TUI 模式）
    pub fn get_llm_manager(&self) -> Arc<LLMManager> {
        self.llm_manager.clone()
    }

    /// 对 LLM 输出的工具参数做安全检查
    fn validate_tool_args(&self, name: &str, args: &Value) -> Result<()> {
        let resolver = self.path_resolver();

        // 对文件操作类工具验证 path 参数
        let file_tools = [
            "read_file",
            "read_file_head",
            "read_file_range",
            "inspect_path",
            "write_file",
            "edit_file",
            "search_and_replace_multi",
            "apply_patch",
            "copy_file",
            "move_file",
            "rename_path",
            "delete_file",
            "list_dir",
            "mkdir",
            "create_dir",
            "grep",
            "search_content",
            "find_files",
            "count_file_types",
            "find_large_files",
            "tree_dir",
            "get_file_info",
            "diagnostics",
            "go_to_definition",
            "symbol_search",
            "references_search",
            "find_implementations",
            "document_symbols",
            "workspace_symbols",
            "hover",
            "signature_help",
            "rename_symbol",
            "file_complexity",
            "import_map",
            "api_surface",
            "project_dependency_graph",
            "search_workspace_text",
            "change_hierarchy",
            "code_lens",
            "diagnostic_summary",
            "dependency_hotspots",
            "test_impact_analysis",
            "save_analysis_snapshot",
            "compare_snapshots",
            "workspace_risk_report",
            "recent_change_report",
            "git_add",
            "git_commit",
            "git_restore_file",
            "git_reset_file",
            "git_checkout",
            "git_stash_push",
            "git_stash_pop",
            "git_push",
            "git_pull",
            "git_branch_create",
            "git_branch_delete",
            "git_log_file",
            "format_file",
            "test_target",
            "read_pdf_text",
            "read_pdf",
        ];
        if file_tools.contains(&name) {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                let validation = resolver.resolve(path);
                if !validation.is_valid {
                    return Err(anyhow::anyhow!(
                        "路径安全验证失败：{}",
                        validation.error.unwrap_or_else(|| "未知错误".to_string())
                    ));
                }
            }
            if let Some(directory) = args.get("directory").and_then(|v| v.as_str()) {
                let validation = resolver.resolve(directory);
                if !validation.is_valid {
                    return Err(anyhow::anyhow!(
                        "路径安全验证失败：{}",
                        validation.error.unwrap_or_else(|| "未知错误".to_string())
                    ));
                }
            }
            // 也检查 source/dest 参数（用于 copy/move）
            for key in &["source", "dest", "destination"] {
                if let Some(p) = args.get(*key).and_then(|v| v.as_str()) {
                    let validation = resolver.resolve(p);
                    if !validation.is_valid {
                        return Err(anyhow::anyhow!(
                            "路径安全验证失败 ({}): {}",
                            key,
                            validation.error.unwrap_or_else(|| "未知错误".to_string())
                        ));
                    }
                }
            }
        }

        // 对命令执行工具检测注入
        if name == "run_safe_command" || name == "run_command" {
            if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                if cmd.len() > 4096 {
                    return Err(anyhow::anyhow!("命令过长 ({} > 4096)", cmd.len()));
                }
                // 检测 CR/LF 注入 — 防止通过换行注入额外命令
                if cmd.contains('\n') || cmd.contains('\r') {
                    return Err(anyhow::anyhow!("命令包含换行符，疑似注入攻击"));
                }
                // run_safe_command 额外检查危险元字符
                if name == "run_safe_command" {
                    for ch in &[';', '|', '&', '$', '`'] {
                        if cmd.contains(*ch) {
                            return Err(anyhow::anyhow!(
                                "安全命令包含危险元字符 '{}'，请使用 run_command 并确认",
                                ch
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取工具箱统计信息
    pub fn get_toolbox_stats(&self) -> Value {
        self.tool_manager.get_toolbox_stats()
    }

    /// 调用工具
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        info!("🔧 执行工具：{} {:?}", name, args);

        // 安全授权检查（作为第一道防线）
        let auth = crate::security::authorize_tool_call(
            name,
            &self.security_config,
            crate::security::ExecutionMode::Cli,
        );
        if let crate::security::AuthDecision::Deny(reason) = auth {
            warn!("🚫 工具被安全策略拦截：{} (tool={})", reason, name);
            return Err(anyhow::anyhow!("安全策略拦截：{}", reason));
        }

        // 对文件路径参数做路径验证
        if let Err(e) = self.validate_tool_args(name, args) {
            return Err(e);
        }

        if name == "terminal_run_structured" {
            let command = args
                .get("command")
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("terminal_run_structured requires command"))?;
            let timeout_ms = args
                .get("timeout_ms")
                .and_then(|value| value.as_u64())
                .unwrap_or(20_000);
            let workspace = std::env::current_dir()
                .map_err(|err| anyhow::anyhow!("failed to determine current workspace: {}", err))?;
            let result = run_terminal_command_structured(&workspace, command, timeout_ms)?;
            self.tool_manager.tool_registry.record_usage(name, true, 0);
            return Ok(result.to_string());
        }

        use tokitai_core::ToolErrorKind;

        // 尝试在各个工具集中查找并执行
        macro_rules! try_tool {
            ($tools:expr) => {
                match $tools.call_tool(name, args) {
                    Ok(result) => {
                        info!("✅ 工具执行成功：{}", name);
                        self.tool_manager.tool_registry.record_usage(name, true, 0);
                        return Ok(result.to_string());
                    }
                    Err(e) => {
                        if e.kind != ToolErrorKind::NotFound {
                            info!("❌ 工具执行失败：{} - {:?}", name, e);
                            self.tool_manager.tool_registry.record_usage(name, false, 0);
                            return Err(anyhow::anyhow!("工具 {} 执行失败：{}", name, e));
                        }
                    }
                }
            };
        }

        try_tool!(self.file_ops);
        try_tool!(self.file_search);
        try_tool!(self.pdf_tools);
        try_tool!(self.project_templates);
        try_tool!(self.system_tools);
        try_tool!(self.process_tools);
        try_tool!(self.code_tools);
        try_tool!(self.web_search);
        try_tool!(self.download_tools);
        try_tool!(self.network_tools);
        try_tool!(self.wikipedia_tools);
        try_tool!(self.git_ops);
        try_tool!(self.http_client);
        try_tool!(self.json_tools);
        try_tool!(self.json_query);
        try_tool!(self.json_merge);
        try_tool!(self.data_conversion);
        // Scientist tools
        try_tool!(self.literature_tools);
        try_tool!(self.computation_tools);
        try_tool!(self.data_tools);
        try_tool!(self.github_tools);
        try_tool!(self.sympy_tool);

        warn!("❌ 未知工具：{}", name);
        Err(anyhow::anyhow!("未知工具：{}", name))
    }

    /// 与 AI 对话并处理工具调用
    pub fn call_tool_without_auth(&self, name: &str, args: &Value) -> Result<String> {
        info!("Executing tool without auth gate: {} {:?}", name, args);

        if let Err(e) = self.validate_tool_args(name, args) {
            return Err(e);
        }

        if name == "terminal_run_structured" {
            let command = args
                .get("command")
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("terminal_run_structured requires command"))?;
            let timeout_ms = args
                .get("timeout_ms")
                .and_then(|value| value.as_u64())
                .unwrap_or(20_000);
            let workspace = std::env::current_dir()
                .map_err(|err| anyhow::anyhow!("failed to determine current workspace: {}", err))?;
            let result = run_terminal_command_structured(&workspace, command, timeout_ms)?;
            return Ok(result.to_string());
        }

        use tokitai_core::ToolErrorKind;

        macro_rules! try_tool {
            ($tools:expr) => {
                match $tools.call_tool(name, args) {
                    Ok(result) => {
                        info!("Tool execution succeeded: {}", name);
                        self.tool_manager.tool_registry.record_usage(name, true, 0);
                        return Ok(result.to_string());
                    }
                    Err(e) => {
                        if e.kind != ToolErrorKind::NotFound {
                            info!("Tool execution failed: {} - {:?}", name, e);
                            self.tool_manager.tool_registry.record_usage(name, false, 0);
                            return Err(anyhow::anyhow!("tool {} failed: {}", name, e));
                        }
                    }
                }
            };
        }

        try_tool!(self.file_ops);
        try_tool!(self.file_search);
        try_tool!(self.pdf_tools);
        try_tool!(self.project_templates);
        try_tool!(self.system_tools);
        try_tool!(self.process_tools);
        try_tool!(self.code_tools);
        try_tool!(self.web_search);
        try_tool!(self.download_tools);
        try_tool!(self.network_tools);
        try_tool!(self.wikipedia_tools);
        try_tool!(self.git_ops);
        try_tool!(self.http_client);
        try_tool!(self.json_tools);
        try_tool!(self.json_query);
        try_tool!(self.json_merge);
        try_tool!(self.data_conversion);
        try_tool!(self.literature_tools);
        try_tool!(self.computation_tools);
        try_tool!(self.data_tools);
        try_tool!(self.github_tools);
        try_tool!(self.sympy_tool);

        warn!("Unknown tool in no-auth path: {}", name);
        Err(anyhow::anyhow!("unknown tool: {}", name))
    }

    pub fn chat_and_handle_tools(&self, messages: &mut Vec<Value>, input: &str) -> Result<String> {
        messages.push(json!({
            "role": "user",
            "content": input
        }));

        self.chat(messages)
    }

    pub fn chat_with_trace(&self, messages: &mut Vec<Value>) -> Result<ChatRunResult> {
        let mut trace = Vec::new();
        let content = self.chat_internal(messages, Some(&mut trace), None)?;
        Ok(ChatRunResult { content, trace })
    }

    pub fn chat_with_trace_with_tools(
        &self,
        messages: &mut Vec<Value>,
        extra_tools: &[Value],
    ) -> Result<ChatRunResult> {
        let mut trace = Vec::new();
        let content = self.chat_internal(messages, Some(&mut trace), Some(extra_tools))?;
        Ok(ChatRunResult { content, trace })
    }

    /// 与 AI 对话
    pub fn chat(&self, messages: &mut Vec<Value>) -> Result<String> {
        self.chat_internal(messages, None, None)
    }

    fn chat_internal(
        &self,
        messages: &mut Vec<Value>,
        mut trace: Option<&mut Vec<MessageBlock>>,
        extra_tools: Option<&[Value]>,
    ) -> Result<String> {
        let tools = if let Some(extra) = extra_tools {
            self.get_tool_definitions_with_extra(extra)
        } else {
            self.get_tool_definitions()
        };

        // 从环境变量读取最新配置（支持运行时切换供应商）
        let api_url = std::env::var("AI_API_URL").unwrap_or_else(|_| self.config.api_url.clone());
        let api_key = std::env::var("AI_API_KEY").ok();
        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| self.config.model.clone());

        // 构建请求体（Ollama / OpenAI 兼容格式，支持工具调用）
        let request_body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "max_tokens": 4096
        });

        info!("📡 发送请求到：{}", api_url);
        info!("📡 使用模型：{}", model);
        if api_key.is_some() {
            info!("📡 使用 API Key 认证");
        }

        // 如果有 API key，添加认证头
        let mut req = self.config.reqwest_client.post(&api_url);
        if let Some(key) = &api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.json(&request_body).send().context("发送请求失败")?;

        let status = response.status();
        info!("📡 响应状态码：{}", status);

        let response_text = response.text().context("读取响应失败")?;
        info!("📡 AI 原始响应：{}", response_text);

        // 检查是否是错误响应
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "API 返回错误 ({}): {}",
                status,
                response_text
            ));
        }

        let response_json: Value = serde_json::from_str(&response_text).context("解析响应失败")?;

        // 处理响应
        let choices_opt = response_json
            .get("choices")
            .and_then(|c: &Value| c.as_array());
        if let Some(choices) = choices_opt {
            let first_opt = choices.first();
            if let Some(first) = first_opt {
                let message_opt = first.get("message");
                if let Some(message) = message_opt {
                    // 检查是否有工具调用（必须是非空数组）
                    let tool_calls_opt = message
                        .get("tool_calls")
                        .and_then(|tc: &Value| tc.as_array());
                    if let Some(tool_calls) = tool_calls_opt {
                        if !tool_calls.is_empty() {
                            return self.handle_tool_calls(
                                tool_calls,
                                messages,
                                trace.as_deref_mut(),
                            );
                        }
                    }

                    // 普通回复
                    let content_opt = message.get("content").and_then(|c: &Value| c.as_str());
                    if let Some(content) = content_opt {
                        if content.is_empty() {
                            warn!("⚠️  AI 返回空内容，完整响应：{:?}", message);
                            return Ok(
                                "⚠️  AI 返回空响应，可能是 API 服务异常或模型输出问题".to_string()
                            );
                        }
                        return Ok(strip_emoji(content).trim().to_string());
                    } else {
                        warn!("⚠️  AI 响应中 content 字段缺失，完整消息：{:?}", message);
                    }
                }
            }
        }

        Ok(format!("AI 响应格式异常：{}", response_json))
    }

    /// 处理工具调用
    fn handle_tool_calls(
        &self,
        tool_calls: &[Value],
        messages: &mut Vec<Value>,
        mut trace: Option<&mut Vec<MessageBlock>>,
    ) -> Result<String> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
            let name = tool_call
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");

            let arguments = tool_call
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
                .unwrap_or("{}");

            let args: Value = serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));

            println!("🔧 执行工具：{}", name);
            let tool_call_id = tool_call
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(ref mut blocks) = trace {
                blocks.push(MessageBlock::ToolCall {
                    name: name.to_string(),
                    args: args.clone(),
                    call_id: tool_call_id.clone(),
                    status: ToolCallStatus::Executing,
                });
            }

            // 安全授权检查
            let auth = crate::security::authorize_tool_call(
                name,
                &self.security_config,
                crate::security::ExecutionMode::Cli,
            );
            if let crate::security::AuthDecision::Deny(reason) = auth {
                println!("🚫 工具被安全策略拦截：{}", reason);
                if let Some(ref mut blocks) = trace {
                    blocks.push(MessageBlock::ToolResult {
                        call_id: tool_call_id.clone(),
                        result: format!("安全策略拦截：{}", reason),
                        success: false,
                    });
                }
                results.push(json!({
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [tool_call]
                }));
                results.push(json!({
                    "role": "tool",
                    "content": format!("安全策略拦截：{}", reason),
                    "tool_call_id": tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("")
                }));
                continue;
            }

            let tool_result = self.call_tool(name, &args);
            let path_recovery_message = tool_result.as_ref().err().and_then(|error| {
                let error_text = error.to_string();
                if is_path_recovery_tool(name) && is_path_related_failure(&error_text) {
                    Some(build_path_recovery_system_message(name, &error_text))
                } else {
                    None
                }
            });

            match tool_result {
                Ok(result) => {
                    println!("✅ 工具执行成功");
                    if let Some(ref mut blocks) = trace {
                        blocks.push(MessageBlock::ToolResult {
                            call_id: tool_call_id.clone(),
                            result: result.clone(),
                            success: true,
                        });
                    }
                    results.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    results.push(json!({
                        "role": "tool",
                        "content": result,
                        "tool_call_id": tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("")
                    }));
                }
                Err(e) => {
                    println!("❌ 工具执行失败：{}", e);
                    if let Some(ref mut blocks) = trace {
                        blocks.push(MessageBlock::ToolResult {
                            call_id: tool_call_id.clone(),
                            result: format!("错误：{}", e),
                            success: false,
                        });
                    }
                    results.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    results.push(json!({
                        "role": "tool",
                        "content": format!("错误：{}", e),
                        "tool_call_id": tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("")
                    }));
                }
            }

            if let Some(system_message) = path_recovery_message {
                results.push(json!({
                    "role": "system",
                    "content": system_message
                }));
            }
        }

        messages.extend(results);
        self.chat_internal(messages, trace, None)
    }

    /// 运行交互式 CLI
    pub fn run_cli(&mut self) -> Result<()> {
        println!();
        println!("  Atlas AI Assistant v2.1.0");
        println!();
        println!("  输入 help 查看功能，quit 退出");
        println!();

        let mut messages: Vec<Value> = vec![json!({
                "role": "system",
            "content": science_expert_system_prompt()
        })];

        let mut stdout = io::stdout();

        // 交互式输入循环
        loop {
            let input = read_line_interactive(&mut stdout, "> ")?;

            if input.is_empty() {
                continue;
            }

            if input == "quit" || input == "exit" {
                println!("\n再见！\n");
                break;
            }

            // 处理编排器命令（以 / 开头）
            if input.starts_with('/') {
                if input == "/toolbox" {
                    let stats = self.get_toolbox_stats();
                    println!(
                        "\n工具箱状态：\n{}",
                        serde_json::to_string_pretty(&stats).unwrap_or_default()
                    );
                    println!();
                    continue;
                }

                // 处理 /model 命令
                if input.starts_with("/model") {
                    let model_args = input.trim_start_matches("/model").trim();
                    if let Some(ref handler) = self.model_handler {
                        let result = handler.execute(model_args);
                        println!("\n{}\n", result);
                    } else {
                        println!("\n⚠️  模型命令处理器未初始化（可能在单提供商模式）\n");
                    }
                    continue;
                }

                let processed = self.orchestrator.process_input(&input);
                if let Some(cmd) = processed.command {
                    let result = self.orchestrator.execute_command(cmd);
                    println!("\n{}\n", result.to_string());
                    continue;
                }
            }

            if input == "help" {
                println!();
                println!("  功能分类");
                println!("  ──────────────────────────────────────");
                println!();
                println!("  文件操作     读取/写入/复制/删除文件");
                println!("  代码分析     分析代码结构、统计行数、搜索函数");
                println!("  网络工具     HTTP 请求、Ping 测试、端口扫描");
                println!("  数据处理     JSON 格式化、PDF 读取");
                println!("  搜索功能     文件搜索、代码搜索、网页搜索");
                println!("  系统管理     查看进程、环境变量、系统资源");
                println!("  Git 操作     查看状态、日志、分支、diff");
                println!();
                println!("  快捷命令");
                println!("  ──────────────────────────────────────");
                println!("  /switch       切换 AI 供应商");
                println!("  /role <name>  切换角色 (planner/executor/reviewer)");
                println!("  /context      查看上下文状态");
                println!("  /optimize     优化上下文");
                println!("  /toolbox      查看工具箱状态");
                println!("  /model        管理 AI 模型 (list/switch/benchmark/stats)");
                println!();
                println!("  使用技巧");
                println!("  ──────────────────────────────────────");
                println!("  • 使用 @文件 引用：@README.md 的内容是什么");
                println!("  • 复杂任务自动分解：分析项目结构并生成报告");
                println!();
                continue;
            }

            // 处理 @path 语法
            let (processed_input, file_contents) = match path_resolver::resolve_paths(&input) {
                Ok(result) => result,
                Err(e) => {
                    println!("\n路径解析错误：{}\n", e);
                    continue;
                }
            };

            if !file_contents.is_empty() {
                println!("已加载 {} 个文件", file_contents.len());
            }

            let processed = self.orchestrator.process_input(&processed_input);

            if processed.role_changed && self.orchestrator.config.verbose {
                println!("切换到角色：{}\n", processed.current_role.as_str());
            }

            print!("🤔 思考中");
            let _ = std::io::stdout().flush();
            let spin_start = std::time::Instant::now();

            match self.chat_and_handle_tools(&mut messages, &processed_input) {
                Ok(response) => {
                    print!("\r\x1b[K");

                    let elapsed = spin_start.elapsed();
                    if elapsed.as_millis() < 500 {
                        println!("✅ 完成 ({:.0}ms)", elapsed.as_millis() as f64);
                    } else {
                        println!("✅ 完成 ({:.1}s)", elapsed.as_secs_f64());
                    }

                    println!("\n{}\n", response);

                    messages.push(json!({
                        "role": "assistant",
                        "content": response
                    }));

                    self.orchestrator.process_response(&response);
                }
                Err(e) => {
                    print!("\r\x1b[K");

                    let elapsed = spin_start.elapsed();
                    println!("❌ 请求失败 ({:.1}s): {}", elapsed.as_secs_f64(), e);
                    println!("提示：可能是网络问题或 API 配置错误，检查 .env 文件后重试\n");
                    messages.pop();
                }
            }
        }

        Ok(())
    }
}

/// 交互式输入辅助函数（支持退格、光标移动、历史纪录）
fn read_line_interactive(stdout: &mut io::Stdout, prompt: &str) -> Result<String> {
    let mut buffer = String::new();
    let mut cursor_pos = 0;
    let mut history: Vec<String> = Vec::new();
    let mut history_pos = 0;

    let raw_mode_enabled = crossterm::terminal::enable_raw_mode().is_ok();

    print!("{}", prompt);
    stdout.flush()?;

    fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    fn redraw_line(
        stdout: &mut io::Stdout,
        prompt: &str,
        buffer: &str,
        cursor_char_pos: usize,
    ) -> std::io::Result<()> {
        print!("\r\x1b[2K");
        print!("{}", prompt);
        print!("{}", buffer);
        let visible_pos = buffer.chars().take(cursor_char_pos).count();
        print!("\x1b[{}G", prompt.chars().count() + visible_pos + 1);
        stdout.flush()
    }

    if !raw_mode_enabled {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();
        println!();
        return Ok(input);
    }

    loop {
        match event::read() {
            Ok(Event::Key(key)) => {
                if key.kind != KeyEventKind::Release {
                    match key.code {
                        KeyCode::Enter => {
                            let _ = crossterm::terminal::disable_raw_mode();
                            println!();
                            if !buffer.trim().is_empty() {
                                history.push(buffer.trim().to_string());
                            }
                            return Ok(buffer);
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let _ = crossterm::terminal::disable_raw_mode();
                            println!("\n👋 再见！");
                            std::process::exit(0);
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let byte_idx = char_to_byte_idx(&buffer, cursor_pos);
                                let next_byte_idx = char_to_byte_idx(&buffer, cursor_pos + 1);
                                buffer.drain(byte_idx..next_byte_idx);
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }
                        KeyCode::Delete => {
                            if cursor_pos < buffer.chars().count() {
                                let byte_idx = char_to_byte_idx(&buffer, cursor_pos);
                                let next_byte_idx = char_to_byte_idx(&buffer, cursor_pos + 1);
                                buffer.drain(byte_idx..next_byte_idx);
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }
                        KeyCode::Left => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }
                        KeyCode::Right => {
                            if cursor_pos < buffer.chars().count() {
                                cursor_pos += 1;
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }
                        KeyCode::Home => {
                            cursor_pos = 0;
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }
                        KeyCode::End => {
                            cursor_pos = buffer.chars().count();
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }
                        KeyCode::Up => {
                            if !history.is_empty() && history_pos < history.len() {
                                history_pos += 1;
                                let idx = history.len() - history_pos;
                                buffer = history[idx].clone();
                                cursor_pos = buffer.chars().count();
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }
                        KeyCode::Down => {
                            if history_pos > 0 {
                                history_pos -= 1;
                                if history_pos == 0 {
                                    buffer.clear();
                                    cursor_pos = 0;
                                } else {
                                    let idx = history.len() - history_pos;
                                    buffer = history[idx].clone();
                                    cursor_pos = buffer.chars().count();
                                }
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }
                        KeyCode::Esc => {
                            buffer.clear();
                            cursor_pos = 0;
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }
                        KeyCode::Char(c) => {
                            let byte_idx = char_to_byte_idx(&buffer, cursor_pos);
                            buffer.insert(byte_idx, c);
                            cursor_pos += 1;
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Resize(_, _)) => {
                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
            }
            Err(_) | Ok(_) => {
                if !raw_mode_enabled {
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let _ = crossterm::terminal::disable_raw_mode();
                    println!();
                    return Ok(input.trim().to_string());
                }
            }
        }
    }
}
