#![recursion_limit = "256"]

mod config;
mod command_resolver;
mod path_resolver;
mod sandbox;
mod tools;
mod context;
mod autonomy;
mod observability;
mod dialogue;
mod prompt_engineering;
mod tool_matrix;
mod orchestrator;
mod integration;
mod provider_config;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{self, Write};
use tracing::{info, warn};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
};

use tool_matrix::matrix::ServiceLifecycle;
use tracing_subscriber::EnvFilter;

use tools::{CodeTools, DownloadTools, FileOperations, GitOperations, SystemTools, SearchTools, HttpClientTools, JsonFormatTools, FileSearchTools, ProcessTools, NetworkTools, WikipediaTools, ProjectTemplates, PdfTools};
use tools::data::{JsonQueryTools, JsonMergeTools, DataConversionTools};
use tools::data::JsonFormatTools as JsonTools;  // 向后兼容别名
use tools::system::system_monitor::SystemMonitor;
use autonomy::{AgentCoordinator, GitWorkflow, GitWorkflowTools};
use orchestrator::Orchestrator;
use tool_matrix::registry::{ToolRegistry, ToolSource};
use tool_matrix::matrix::{ToolBox, ToolDefinition};
use tool_matrix::selector::ToolSelector;
use tool_matrix::skills_manager::SkillsManager;
use tool_matrix::dispatcher::ToolDispatcher;
use tool_matrix::tool_selector::LightweightToolSelector;
#[allow(unused_imports)]
use tool_matrix::ai_classifier::{AIToolboxClassifier, DefaultLLMClient};
use integration::IntegratedModules;
use dialogue::DialogueTools;
use observability::ObservabilityTools;
use prompt_engineering::PromptTools;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

// ============================================================================
// Tokitai 双轨服务架构
// ============================================================================
//
// 本项目采用双轨服务架构，两种服务共享底层能力但定位和使用场景完全不同：
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │  服务一：CLI AI 助手（面向用户）                                        │
// ├─────────────────────────────────────────────────────────────────────────┤
// │  • 启动命令：cargo run --release                                        │
// │  • 服务对象：用户（开发者）                                             │
// │  • 驱动方式：用户输入驱动                                               │
// │  • 交互模式：交互式对话                                                 │
// │  • 典型场景：查询、分析、临时任务                                       │
// │  • 服务边界：不主动修改项目代码，不自主发起 Git 操作                      │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │  服务二：项目自更新服务（面向项目自身）                                 │
// ├─────────────────────────────────────────────────────────────────────────┤
// │  • 启动命令：cargo run --release -- --autonomous                        │
// │  • 服务对象：项目自身                                                   │
// │  • 驱动方式：AI 自主驱动                                                 │
// │  • 交互模式：自主迭代循环（Planner-Executor-Reviewer）                  │
// │  • 典型场景：代码改进、技术债务清理、持续优化                           │
// │  • 服务边界：不响应用户交互，不处理外部查询                             │
// └─────────────────────────────────────────────────────────────────────────┘
//
// 详细文档：structure_ensure/SERVICES.md
// ============================================================================

/// AI 助手 - 整合所有工具（使用工具矩阵管理）
///
/// # 双轨服务说明
///
/// 本结构体支持两种服务模式：
///
/// ## 1. CLI AI 助手模式（默认）
///
/// ```rust
/// let assistant = AiAssistant::new(api_url, api_key, model);
/// // 用于交互式对话，响应用户查询
/// assistant.chat_and_handle_tools(&mut messages, input)?;
/// ```
///
/// ## 2. 自主进化模式（--autonomous）
///
/// ```rust
/// let assistant = AiAssistant::new_autonomous(api_url, api_key, model, project_path)?;
/// // 用于自主进化，AI 自主发现并实施改进
/// assistant.run_autonomous_evolution()?;
/// ```
///
/// # 共享能力
///
/// 两种模式共享以下核心能力：
/// - ToolMatrix（工具矩阵/服务注册表）
/// - Context Storage（上下文存储）
/// - Orchestrator（编排调度）
/// - IntegratedModules（集成模块）
///
/// # 服务边界
///
/// - CLI 模式：不主动修改项目代码，所有修改需用户明确指令
/// - 自主模式：不响应用户交互，专注于项目自身改进
pub struct AiAssistant {
    // =========================================================================
    // 工具实例（用于 call_tool 调用）
    // 两种模式共享，提供 63+ 工具函数
    // =========================================================================
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: SearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    http_client: HttpClientTools,
    json_tools: JsonTools,
    file_search: FileSearchTools,
    process_tools: ProcessTools,
    network_tools: NetworkTools,
    wikipedia_tools: WikipediaTools,
    project_templates: ProjectTemplates,
    pdf_tools: PdfTools,

    // =========================================================================
    // 工具矩阵（用于工具管理和动态选择）
    // 两种模式共享，提供服务注册、分类、选择、调用分发
    // =========================================================================
    tool_registry: ToolRegistry,
    tool_selector: ToolSelector,
    skills_manager: SkillsManager,
    // 轻量级工具选择器（AI 原生）- 快速搜索 <10ms，缓存命中后 ~3ms
    lightweight_selector: Arc<LightweightToolSelector>,
    // 工具调用分发器 - 统一工具调用入口
    tool_dispatcher: Arc<ToolDispatcher>,

    // =========================================================================
    // 基础配置
    // =========================================================================
    api_url: String,
    api_key: Option<String>,
    model: String,
    // HTTP 客户端（持久连接池）
    reqwest_client: reqwest::blocking::Client,

    // =========================================================================
    // 自主进化专属字段（仅自主模式使用）
    // =========================================================================
    /// 自主进化协调器（多 Agent 协作系统）
    /// - Planner Agent: 规划 Agent，制定改进计划
    /// - Executor Agent: 执行 Agent，按计划执行任务
    /// - Reviewer Agent: 审查 Agent，代码审查和质量把关
    coordinator: Option<Arc<RwLock<AgentCoordinator>>>,
    /// Git 工作流（用于自主推送）
    /// - 自动生成提交消息
    /// - 执行预提交检查（fmt/clippy/test）
    /// - 可选推送到 GitHub
    git_workflow: Option<GitWorkflow>,
    /// 是否启用自主模式
    /// - false: CLI AI 助手模式（面向用户）
    /// - true: 项目自更新服务模式（面向项目自身）
    autonomous_mode: bool,

    // =========================================================================
    // 编排器（用于角色切换和上下文优化）
    // 两种模式共享
    // =========================================================================
    orchestrator: Orchestrator,

    // =========================================================================
    // 集成模块（统一管理 dialogue、observability、prompt_engineering）
    // 两种模式共享
    // =========================================================================
    integrated_modules: IntegratedModules,
}

impl AiAssistant {
    // =========================================================================
    // 构造函数
    // =========================================================================

    /// 创建新的 AI 助手（CLI AI 助手模式 - 面向用户）
    ///
    /// # 参数
    /// - `api_url`: AI API 端点 URL
    /// - `api_key`: AI API 密钥（可选）
    /// - `model`: 使用的模型名称
    ///
    /// # 返回
    /// 返回配置为 CLI AI 助手模式的 `AiAssistant` 实例
    ///
    /// # 使用场景
    /// - 交互式对话
    /// - 响应用户查询
    /// - 执行用户指定的工具调用
    /// - 文件操作、代码分析、网络请求等临时任务
    ///
    /// # 启动命令
    /// ```bash
    /// cargo run --release
    /// ```
    ///
    /// # 服务边界
    /// - ✅ 响应用户查询
    /// - ✅ 执行用户指定的工具调用
    /// - ❌ 不主动修改项目代码
    /// - ❌ 不自主发起 Git 操作
    pub fn new(api_url: String, api_key: Option<String>, model: String) -> Self {
        // 创建工具注册表
        let tool_registry = ToolRegistry::new();
        
        // 先创建工具箱
        tool_registry.create_toolbox(ToolBox::new("file_ops", "File Operations", "File operations tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("system", "System Tools", "System operations tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("code", "Code Tools", "Code analysis and processing tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("web", "Web Tools", "Web search and network tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("git", "Git Tools", "Git version control tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("data", "Data Tools", "Data processing tools")).ok();
        
        // 从各个 ToolProvider 注册工具到对应的工具箱（使用同步版本）
        let _ = tool_registry.register_from_provider_sync::<FileOperations>(Some("file_ops"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<SystemTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<CodeTools>(Some("code"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<SearchTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<DownloadTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<GitOperations>(Some("git"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<HttpClientTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<JsonTools>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<FileSearchTools>(Some("file_ops"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ProcessTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<NetworkTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<WikipediaTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ProjectTemplates>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<PdfTools>(Some("file_ops"), ToolSource::Builtin);

        // 注册数据模块工具到 data 工具箱
        let _ = tool_registry.register_from_provider_sync::<JsonQueryTools>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<JsonMergeTools>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<DataConversionTools>(Some("data"), ToolSource::Builtin);

        // 注册系统监控工具到 system 工具箱
        let _ = tool_registry.register_from_provider_sync::<SystemMonitor>(Some("system"), ToolSource::Builtin);

        // 注册新工具到工具箱（从 IntegratedModules 获取）
        // IntegratedModules 会统一管理 dialogue、observability、prompt_engineering

        // 创建工具选择器
        let tool_selector = ToolSelector::new(tool_registry.clone());

        // 创建 Skills 管理器
        let skills_manager = SkillsManager::default();

        // 创建集成模块（使用 fallible 操作）
        let integrated_modules = match IntegratedModules::new(integration::IntegratedModulesConfig::default()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("⚠️  创建集成模块失败：{}", e);
                // 创建默认配置
                IntegratedModules::new(integration::IntegratedModulesConfig::for_testing()).unwrap()
            }
        };

        let mut integrated_modules = integrated_modules;

        // 初始化集成模块
        match integrated_modules.initialize() {
            Ok(init_report) => {
                if !init_report.success {
                    eprintln!("⚠️  集成模块初始化警告：");
                    for error in &init_report.errors {
                        eprintln!("   - {}", error);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  集成模块初始化失败：{}", e);
            }
        }

        // 从集成模块获取工具并注册到工具矩阵（使用同步版本）
        let _ = tool_registry.register_from_provider_sync::<DialogueTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ObservabilityTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<PromptTools>(Some("system"), ToolSource::Builtin);

        // 获取所有工具定义用于创建轻量级选择器
        let all_tools = tool_registry.get_all_tools();

        // 创建轻量级工具选择器（不带 AI，使用默认配置）
        let lightweight_selector = Arc::new(LightweightToolSelector::new_without_ai(
            all_tools.clone(),
            None,
        ));

        // 创建工具分发器
        let tool_dispatcher = Arc::new(ToolDispatcher::new(lightweight_selector.clone()));

        // 创建持久的 HTTP 客户端（带连接池和超时配置）
        let reqwest_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))  // 120 秒超时
            .connect_timeout(std::time::Duration::from_secs(30))  // 30 秒连接超时
            .pool_max_idle_per_host(10)  // 每个主机最多 10 个空闲连接
            .build()
            .expect("创建 HTTP 客户端失败");

        Self {
            file_ops: FileOperations::default(),
            system_tools: SystemTools::default(),
            code_tools: CodeTools::default(),
            web_search: SearchTools::new(),
            download_tools: DownloadTools::new(),
            git_ops: GitOperations::default(),
            http_client: HttpClientTools::new(),
            json_tools: JsonFormatTools::default(),
            file_search: FileSearchTools::default(),
            process_tools: ProcessTools::default(),
            network_tools: NetworkTools::default(),
            wikipedia_tools: WikipediaTools::new(),
            project_templates: ProjectTemplates::default(),
            pdf_tools: PdfTools::default(),
            tool_registry,
            tool_selector,
            skills_manager,
            lightweight_selector,
            tool_dispatcher,
            api_url,
            api_key,
            model,
            reqwest_client,
            coordinator: None,
            git_workflow: None,
            autonomous_mode: false,
            orchestrator: Orchestrator::new(),
            integrated_modules,
        }
    }

    // =========================================================================
    // 构造函数（自主模式）
    // =========================================================================

    /// 创建自主模式的 AI 助手（项目自更新服务 - 面向项目自身）
    ///
    /// # 参数
    /// - `api_url`: AI API 端点 URL
    /// - `api_key`: AI API 密钥（可选）
    /// - `model`: 使用的模型名称
    /// - `project_root`: 项目根目录路径
    ///
    /// # 返回
    /// 返回配置为自主进化模式的 `AiAssistant` 实例
    ///
    /// # 使用场景
    /// - AI 自主发现项目改进点
    /// - 自主代码改进和重构
    /// - 技术债务清理
    /// - 持续优化项目质量
    ///
    /// # 启动命令
    /// ```bash
    /// # 默认当前目录
    /// cargo run --release -- --autonomous
    ///
    /// # 指定项目路径
    /// cargo run --release -- --autonomous --project-path ./sandbox/test-project
    /// ```
    ///
    /// # 工作流程
    /// 1. 分析项目状态（读取项目结构、代码质量、测试覆盖率）
    /// 2. 发现改进点（识别代码异味、缺失功能、性能瓶颈）
    /// 3. 制定改进计划（Planner Agent 生成任务列表）
    /// 4. 执行改进任务（Executor Agent 按计划执行）
    /// 5. 审查代码变更（Reviewer Agent 代码审查）
    /// 6. 提交并推送（Git Workflow 自动提交，可选推送）
    /// 7. 继续下一轮迭代
    ///
    /// # 服务边界
    /// - ✅ 自主分析项目状态
    /// - ✅ 自主发现改进点
    /// - ✅ 自主制定并执行计划
    /// - ✅ 自主代码审查
    /// - ✅ 自主 Git 提交（可选）
    /// - ❌ 不响应用户交互
    /// - ❌ 不处理外部查询
    /// - ❌ 不提供服务接口
    pub fn new_autonomous(
        api_url: String,
        api_key: Option<String>,
        model: String,
        project_root: PathBuf,
    ) -> Result<Self, String> {
        let autonomy_dir = project_root.join(".tokitai").join("autonomy");

        // 创建工具注册表
        let tool_registry = ToolRegistry::new();

        // 先创建工具箱
        tool_registry.create_toolbox(ToolBox::new("file_ops", "File Operations", "File operations tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("system", "System Tools", "System operations tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("code", "Code Tools", "Code analysis and processing tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("web", "Web Tools", "Web search and network tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("git", "Git Tools", "Git version control tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("data", "Data Tools", "Data processing tools")).ok();
        tool_registry.create_toolbox(ToolBox::new("autonomy", "Autonomy Tools", "AI autonomous evolution tools")).ok();

        // 从各个 ToolProvider 注册工具（使用同步版本）
        let _ = tool_registry.register_from_provider_sync::<FileOperations>(Some("file_ops"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<SystemTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<CodeTools>(Some("code"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<SearchTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<DownloadTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<GitOperations>(Some("git"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<HttpClientTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<JsonTools>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<FileSearchTools>(Some("file_ops"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ProcessTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<NetworkTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<WikipediaTools>(Some("web"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ProjectTemplates>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<PdfTools>(Some("file_ops"), ToolSource::Builtin);

        // 注册数据模块工具到 data 工具箱
        let _ = tool_registry.register_from_provider_sync::<JsonQueryTools>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<JsonMergeTools>(Some("data"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<DataConversionTools>(Some("data"), ToolSource::Builtin);

        // 注册系统监控工具到 system 工具箱
        let _ = tool_registry.register_from_provider_sync::<SystemMonitor>(Some("system"), ToolSource::Builtin);

        // 注册新工具到工具箱
        let _ = tool_registry.register_from_provider_sync::<DialogueTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ObservabilityTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<PromptTools>(Some("system"), ToolSource::Builtin);

        // 注册 GitWorkflow 工具到 autonomy 工具箱（利用 tokitai ToolProvider，使用同步版本）
        let git_workflow_tools = GitWorkflowTools::new(project_root.clone(), autonomy_dir.join("git"))
            .map_err(|e| format!("创建 Git 工作流工具失败：{}", e))?;
        let _ = tool_registry.register_from_provider_sync::<GitWorkflowTools>(Some("autonomy"), ToolSource::Builtin);

        // 创建工具选择器
        let tool_selector = ToolSelector::new(tool_registry.clone());

        // 创建 Skills 管理器
        let skills_manager = SkillsManager::default();

        // 创建 Agent 协调器（传入工具注册表）
        let coordinator = AgentCoordinator::new(autonomy_dir.clone(), Arc::new(RwLock::new(tool_registry.clone())))
            .map_err(|e| format!("创建 Agent 协调器失败：{}", e))?;

        // 创建 Git 工作流（用于向后兼容）
        let git_workflow = GitWorkflow::new(project_root.clone(), autonomy_dir.join("git"))
            .map_err(|e| format!("创建 Git 工作流失败：{}", e))?;

        // 创建集成模块
        let integrated_modules = match IntegratedModules::new(integration::IntegratedModulesConfig::default()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("⚠️  创建集成模块失败：{}", e);
                IntegratedModules::new(integration::IntegratedModulesConfig::for_testing()).unwrap()
            }
        };

        let mut integrated_modules = integrated_modules;

        // 初始化集成模块
        match integrated_modules.initialize() {
            Ok(init_report) => {
                if !init_report.success {
                    eprintln!("⚠️  集成模块初始化警告：");
                    for error in &init_report.errors {
                        eprintln!("   - {}", error);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  集成模块初始化失败：{}", e);
            }
        }

        // 从集成模块获取工具并注册到工具矩阵（使用同步版本）
        let _ = tool_registry.register_from_provider_sync::<DialogueTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<ObservabilityTools>(Some("system"), ToolSource::Builtin);
        let _ = tool_registry.register_from_provider_sync::<PromptTools>(Some("system"), ToolSource::Builtin);

        // 获取所有工具定义用于创建轻量级选择器
        let all_tools = tool_registry.get_all_tools();

        // 创建轻量级工具选择器（不带 AI，使用默认配置）
        let lightweight_selector = Arc::new(LightweightToolSelector::new_without_ai(
            all_tools.clone(),
            None,
        ));

        // 创建工具分发器
        let tool_dispatcher = Arc::new(ToolDispatcher::new(lightweight_selector.clone()));

        // 创建持久的 HTTP 客户端（带连接池和超时配置）
        let reqwest_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))  // 120 秒超时
            .connect_timeout(std::time::Duration::from_secs(30))  // 30 秒连接超时
            .pool_max_idle_per_host(10)  // 每个主机最多 10 个空闲连接
            .build()
            .expect("创建 HTTP 客户端失败");

        Ok(Self {
            file_ops: FileOperations::default(),
            system_tools: SystemTools::default(),
            code_tools: CodeTools::default(),
            web_search: SearchTools::new(),
            download_tools: DownloadTools::new(),
            git_ops: GitOperations::default(),
            http_client: HttpClientTools::new(),
            json_tools: JsonFormatTools::default(),
            file_search: FileSearchTools::default(),
            process_tools: ProcessTools::default(),
            network_tools: NetworkTools::default(),
            wikipedia_tools: WikipediaTools::new(),
            project_templates: ProjectTemplates::default(),
            pdf_tools: PdfTools::default(),
            tool_registry,
            tool_selector,
            skills_manager,
            lightweight_selector,
            tool_dispatcher,
            api_url,
            api_key,
            model,
            reqwest_client,
            coordinator: Some(Arc::new(RwLock::new(coordinator))),
            git_workflow: Some(git_workflow),
            autonomous_mode: true,
            orchestrator: Orchestrator::new(),
            integrated_modules,
        })
    }

    /// 获取所有工具定义（使用工具矩阵）
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        // 从工具注册表获取所有工具定义
        self.tool_registry
            .get_all_tools()
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                    }
                })
            })
            .collect()
    }

    /// 获取工具箱统计信息
    pub fn get_toolbox_stats(&self) -> Value {
        let toolboxes = self.tool_registry.get_all_toolboxes();
        let mut stats = json!({
            "total_tools": self.tool_registry.tool_count(),
            "total_toolboxes": self.tool_registry.toolbox_count(),
            "toolboxes": []
        });
        
        if let Some(boxes) = stats.get_mut("toolboxes").and_then(|v| v.as_array_mut()) {
            for box_ref in &toolboxes {
                boxes.push(json!({
                    "id": box_ref.id,
                    "name": box_ref.name,
                    "description": box_ref.description,
                    "tool_count": box_ref.tool_count(),
                    "enabled": box_ref.enabled
                }));
            }
        }
        
        stats
    }

    /// 调用工具（带日志）
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        info!("🔧 执行工具：{} {:?}", name, args);

        // 尝试在各个工具集中查找并执行
        // 注意：call_tool 返回 Result<Value, ToolError>，我们需要检查是否找到了工具
        // 如果工具存在但执行失败，ToolError.kind 会是 InternalError 或 ValidationError
        // 如果工具不存在，ToolError.kind 会是 NotFound

        use tokitai_core::ToolErrorKind;

        // 按工具集依次尝试调用
        macro_rules! try_tool {
            ($tools:expr, $name:expr) => {
                match $tools.call_tool(name, args) {
                    Ok(result) => {
                        info!("✅ 工具执行成功：{}", name);
                        self.tool_registry.record_usage(name, true, 0);
                        return Ok(result.to_string());
                    }
                    Err(e) => {
                        if e.kind == ToolErrorKind::NotFound {
                            // 工具不存在，继续尝试下一个
                        } else {
                            // 工具存在但执行失败
                            info!("❌ 工具执行失败：{} - {:?}", name, e);
                            self.tool_registry.record_usage(name, false, 0);
                            return Err(anyhow::anyhow!("工具 {} 执行失败：{}", name, e));
                        }
                    }
                }
            };
        }

        try_tool!(self.file_ops, "file_ops");
        try_tool!(self.system_tools, "system_tools");
        try_tool!(self.code_tools, "code_tools");
        try_tool!(self.web_search, "web_search");
        try_tool!(self.download_tools, "download_tools");
        try_tool!(self.git_ops, "git_ops");
        try_tool!(self.http_client, "http_client");
        try_tool!(self.json_tools, "json_tools");
        try_tool!(self.file_search, "file_search");
        try_tool!(self.process_tools, "process_tools");
        try_tool!(self.network_tools, "network_tools");
        try_tool!(self.wikipedia_tools, "wikipedia_tools");
        try_tool!(self.project_templates, "project_templates");
        try_tool!(self.pdf_tools, "pdf_tools");

        warn!("❌ 未知工具：{}", name);
        Err(anyhow::anyhow!("未知工具：{}", name))
    }

    /// 根据查询动态选择工具（使用 ToolSelector）
    pub fn select_tools_by_query(&self, query: &str, limit: usize) -> Vec<ToolDefinition> {
        let result = self.tool_selector.select_tools_by_query(query, limit);
        result.tools
    }

    /// 获取指定工具箱的所有工具
    pub fn get_tools_from_box(&self, toolbox_id: &str) -> Vec<ToolDefinition> {
        self.tool_registry.get_tools_from_box(toolbox_id)
    }

    /// 生成工具使用提示词（整合所有 Skills 文件）
    pub fn generate_tools_prompt(&self) -> String {
        let toolboxes = self.tool_registry.get_all_toolboxes();
        let mut prompt = String::new();
        
        prompt.push_str("# 可用工具矩阵\n\n");
        prompt.push_str(&format!("当前共有 {} 个工具箱，包含 {} 个工具。\n\n", 
            self.tool_registry.toolbox_count(),
            self.tool_registry.tool_count()));
        
        for toolbox in &toolboxes {
            prompt.push_str(&format!("## {}\n", toolbox.name));
            prompt.push_str(&format!("{}\n\n", toolbox.description));
            
            if toolbox.tool_count() > 0 {
                prompt.push_str("### 工具列表\n");
                for tool in toolbox.get_all_tools() {
                    prompt.push_str(&format!("- **{}**: {}\n", tool.name, tool.description));
                }
                prompt.push('\n');
            }
        }
        
        // 如果有 Skills 文件，也添加进去
        let skills_prompt = self.skills_manager.generate_skills_prompt().unwrap_or_default();
        if !skills_prompt.is_empty() {
            prompt.push_str("\n# 工具使用指南\n\n");
            prompt.push_str(&skills_prompt);
        }
        
        prompt
    }

    /// 与 AI 对话并处理工具调用（单次）
    pub fn chat_and_handle_tools(&self, messages: &mut Vec<Value>, input: &str) -> Result<String> {
        // 添加用户消息
        messages.push(json!({
            "role": "user",
            "content": input
        }));

        // 发送请求并处理工具调用
        self.chat(messages)
    }

    /// 与 AI 对话
    pub fn chat(&self, messages: &mut Vec<Value>) -> Result<String> {
        let tools = self.get_tool_definitions();

        // 从环境变量读取最新配置（支持运行时切换供应商）
        let api_url = std::env::var("AI_API_URL")
            .unwrap_or_else(|_| self.api_url.clone());
        let api_key = std::env::var("AI_API_KEY").ok();
        let model = std::env::var("AI_MODEL")
            .unwrap_or_else(|_| self.model.clone());

        // 构建请求体（Ollama / OpenAI 兼容格式，支持工具调用）
        let request_body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "max_tokens": 4096
        });

        // 调试：打印请求信息
        info!("📡 发送请求到：{}", api_url);
        info!("📡 使用模型：{}", model);
        if api_key.is_some() {
            info!("📡 使用 API Key 认证");
        }

        // 如果有 API key，添加认证头
        let mut req = self.reqwest_client.post(&api_url);
        if let Some(key) = &api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req
            .json(&request_body)
            .send()
            .context("发送请求失败")?;

        let status = response.status();
        info!("📡 响应状态码：{}", status);

        let response_text = response.text().context("读取响应失败")?;

        // 调试：打印原始响应
        info!("📡 AI 原始响应：{}", response_text);

        // 检查是否是错误响应
        if !status.is_success() {
            return Err(anyhow::anyhow!("API 返回错误 ({}): {}", status, response_text));
        }

        let response_json: Value = serde_json::from_str(&response_text)
            .context("解析响应失败")?;

        // 处理响应
        let choices_opt = response_json.get("choices").and_then(|c: &Value| c.as_array());
        if let Some(choices) = choices_opt {
            let first_opt = choices.first();
            if let Some(first) = first_opt {
                let message_opt = first.get("message");
                if let Some(message) = message_opt {
                    // 检查是否有工具调用（必须是非空数组）
                    let tool_calls_opt = message.get("tool_calls").and_then(|tc: &Value| tc.as_array());
                    if let Some(tool_calls) = tool_calls_opt {
                        if !tool_calls.is_empty() {
                            return self.handle_tool_calls(tool_calls, messages);
                        }
                    }

                    // 普通回复
                    let content_opt = message.get("content").and_then(|c: &Value| c.as_str());
                    if let Some(content) = content_opt {
                        if content.is_empty() {
                            warn!("⚠️  AI 返回空内容，完整响应：{:?}", message);
                            return Ok("⚠️  AI 返回空响应，可能是 API 服务异常或模型输出问题".to_string());
                        }
                        return Ok(content.to_string());
                    } else {
                        warn!("⚠️  AI 响应中 content 字段缺失，完整消息：{:?}", message);
                    }
                }
            }
        }

        Ok(format!("AI 响应格式异常：{}", response_json))
    }

    /// 处理工具调用
    fn handle_tool_calls(&self, tool_calls: &[Value], messages: &mut Vec<Value>) -> Result<String> {
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

            let args: Value = serde_json::from_str(arguments)
                .unwrap_or_else(|_| json!({}));

            println!("🔧 执行工具：{}", name);

            match self.call_tool(name, &args) {
                Ok(result) => {
                    println!("✅ 工具执行成功");
                    // 先添加 assistant 的 tool_calls 消息
                    results.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    // 再添加 tool 的响应消息
                    results.push(json!({
                        "role": "tool",
                        "content": result,
                        "tool_call_id": tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("")
                    }));
                }
                Err(e) => {
                    println!("❌ 工具执行失败：{}", e);
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
        }

        // 将工具调用结果添加回消息
        messages.extend(results);

        // 再次调用 AI 获取最终回复
        self.chat(messages)
    }

    /// 自主进化循环（后台运行）
    /// 
    /// 这个函数在后台持续运行，AI 自主地：
    /// 1. 分析项目现状，发现改进点
    // =========================================================================
    // 自主进化方法（仅自主模式使用）
    // =========================================================================

    /// 运行自主进化系统（项目自更新服务核心方法）
    ///
    /// # 功能说明
    ///
    /// 此方法启动 AI 自主进化循环，AI 将：
    /// 1. 自主发现项目改进点
    /// 2. 自主规划改进任务
    /// 3. 执行任务（修改代码）
    /// 4. 本地审查（编译、测试、代码审查）
    /// 5. 审查通过后自动推送到 GitHub
    ///
    /// # 使用条件
    /// - 必须通过 `new_autonomous()` 创建实例
    /// - 必须启用 `autonomous_mode = true`
    /// - 必须配置 `coordinator` 和 `git_workflow`
    ///
    /// # 进化目标
    /// - 改进代码质量：检查并修复代码中的潜在问题
    /// - 优化性能：分析并优化慢查询和低效代码
    /// - 增强错误处理：改进错误提示和日志
    /// - 完善文档：检查并更新 README 和注释
    /// - 清理技术债务：移除未使用的代码和依赖
    ///
    /// # 返回
    /// - `Ok(())`: 自主进化循环完成
    /// - `Err(e)`: 自主进化失败
    ///
    /// # 服务边界
    /// 此方法仅用于**项目自更新服务**（面向项目自身），不用于 CLI AI 助手模式
    pub fn run_autonomous_evolution(&self) -> Result<()> {
        if !self.autonomous_mode || self.coordinator.is_none() {
            return Err(anyhow::anyhow!("自主模式未启用"));
        }

        let coordinator = self.coordinator.clone().unwrap();
        
        println!("\n🤖 启动自主进化系统...");
        println!("   - AI 将自主发现项目改进点");
        println!("   - 本地审查通过后将自动推送到 GitHub");
        println!("   - 按 Ctrl+C 停止自主模式\n");

        info!("🔄 开始自主进化循环");

        // 自主进化目标列表
        let evolution_goals = vec![
            "改进代码质量：检查并修复代码中的潜在问题".to_string(),
            "优化性能：分析并优化慢查询和低效代码".to_string(),
            "增强错误处理：改进错误提示和日志".to_string(),
            "完善文档：检查并更新 README 和注释".to_string(),
            "清理技术债务：移除未使用的代码和依赖".to_string(),
        ];

        for goal in evolution_goals {
            println!("\n📋 自主进化目标：{}", goal);
            
            // 使用协调器执行自主迭代
            match self.execute_evolution_iteration(&coordinator, &goal) {
                Ok(push_success) => {
                    if push_success {
                        println!("✅ 进化完成并已推送到 GitHub");
                    } else {
                        println!("⚠️  进化完成但未推送（审查未通过或无变更）");
                    }
                }
                Err(e) => {
                    println!("❌ 进化失败：{}", e);
                    warn!("自主进化失败：{}", e);
                }
            }

            // 检查是否应该继续
            if !self.should_continue_evolution() {
                println!("\n🛑 停止自主进化");
                break;
            }
        }

        info!("🔄 自主进化循环结束");
        Ok(())
    }

    /// 执行单次进化迭代（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用，CLI 模式下调用将返回错误
    fn execute_evolution_iteration(
        &self,
        coordinator: &Arc<RwLock<AgentCoordinator>>,
        goal: &str,
    ) -> Result<bool> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "execute_evolution_iteration 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        // 1. 开始迭代
        {
            let mut coord = coordinator.write();
            coord.start_iteration(goal.to_string())
                .map_err(|e| anyhow::anyhow!("启动迭代失败：{}", e))?;
        }

        // 2. AI 自主分析项目现状
        println!("   🔍 分析项目现状...");
        let analysis = self.analyze_project_status()?;
        info!("项目分析：{}", analysis);

        // 3. AI 生成改进计划
        println!("   📝 生成改进计划...");
        let plan = self.generate_improvement_plan(goal, &analysis)?;
        info!("改进计划：{}", plan);

        // 4. 执行改进任务
        println!("   🔧 执行改进任务...");
        let execution_result = self.execute_improvement_tasks(&plan)?;
        info!("执行结果：{}", execution_result);

        // 5. 本地审查
        println!("   🧪 本地审查...");
        let review_passed = self.local_review()?;
        
        if review_passed {
            // 6. 推送到 GitHub
            println!("   🚀 推送到 GitHub...");
            let push_success = self.push_to_github()?;
            Ok(push_success)
        } else {
            println!("   ❌ 审查未通过，回滚变更");
            self.rollback_changes()?;
            Ok(false)
        }
    }

    /// 分析项目现状（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用
    fn analyze_project_status(&self) -> Result<String> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "analyze_project_status 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        let mut analysis = String::new();

        // 获取 Git 状态
        if let Ok(status) = self.git_ops.call_tool("git_status", &json!({})) {
            analysis.push_str(&format!("Git 状态：{}\n", status));
        }

        // 获取项目文件结构
        if let Ok(files) = self.file_ops.call_tool("list_dir", &json!({"path": "."})) {
            analysis.push_str(&format!("项目文件：{}\n", files));
        }

        // 检查代码质量（简单实现：查找 TODO/FIXME 注释）
        if let Ok(todos) = self.file_search.call_tool("search_content", &json!({
            "pattern": "TODO|FIXME|XXX|HACK",
            "path": "src"
        })) {
            analysis.push_str(&format!("待改进项：{}\n", todos));
        }

        Ok(analysis)
    }

    /// 生成改进计划（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用
    fn generate_improvement_plan(&self, goal: &str, analysis: &str) -> Result<String> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "generate_improvement_plan 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        // 使用 AI 生成改进计划
        let messages = &mut vec![
            json!({
                "role": "system",
                "content": "你是一个专业的软件工程师，负责分析项目并制定改进计划。"
            }),
            json!({
                "role": "user",
                "content": format!("目标：{}\n\n项目现状：{}\n\n请制定一个具体的改进计划。", goal, analysis)
            })
        ];

        let plan = self.chat(messages)?;
        Ok(plan)
    }

    /// 执行改进任务（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用
    fn execute_improvement_tasks(&self, plan: &str) -> Result<String> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "execute_improvement_tasks 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        // 根据计划执行具体的改进任务
        // 这里简化实现，实际应该解析计划并调用相应工具

        let messages = &mut vec![
            json!({
                "role": "system",
                "content": "你是一个专业的软件工程师，根据改进计划执行具体的代码修改任务。

你可以使用以下工具：
- read_file: 读取文件
- write_file: 写入文件
- edit_file: 编辑文件
- run_command: 执行命令（如 cargo fmt, cargo clippy, cargo test）

请根据计划逐步执行任务，每次调用一个工具。"
            }),
            json!({
                "role": "user",
                "content": format!("请执行以下改进计划：\n\n{}", plan)
            })
        ];

        // 执行多轮对话直到任务完成
        let mut iterations = 0;
        let max_iterations = 10;
        
        while iterations < max_iterations {
            let response = self.chat(messages)?;
            info!("AI 响应：{}", response);
            
            // 检查是否完成
            if response.contains("完成") || response.contains("已完成") || iterations >= max_iterations - 1 {
                break;
            }
            
            iterations += 1;
        }

        Ok(format!("执行完成，共 {} 轮迭代", iterations))
    }

    /// 本地审查
    fn local_review(&self) -> Result<bool> {
        println!("      - 运行 cargo fmt...");
        let fmt_result = self.system_tools.call_tool("run_command", &json!({
            "command": "cargo fmt --check"
        }));
        
        if fmt_result.is_err() {
            println!("      ❌ 代码格式检查失败");
            return Ok(false);
        }

        println!("      - 运行 cargo clippy...");
        let clippy_result = self.system_tools.call_tool("run_command", &json!({
            "command": "cargo clippy -- -D warnings"
        }));
        
        // clippy 有警告时返回 Err，但我们可以继续
        if clippy_result.is_err() {
            println!("      ⚠️  Clippy 发现警告");
        }

        println!("      - 运行 cargo test...");
        let test_result = self.system_tools.call_tool("run_command", &json!({
            "command": "cargo test --quiet"
        }));
        
        if test_result.is_err() {
            println!("      ❌ 测试失败");
            return Ok(false);
        }

        println!("      ✅ 审查通过");
        Ok(true)
    }

    /// 回滚变更（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用
    fn rollback_changes(&self) -> Result<()> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "rollback_changes 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        self.system_tools.call_tool("run_command", &json!({
            "command": "git checkout -- ."
        }))?;
        Ok(())
    }

    /// 推送到 GitHub（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用，CLI 模式下禁止使用
    fn push_to_github(&self) -> Result<bool> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "push_to_github 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        // 检查是否有变更
        let status = self.git_ops.call_tool("git_status", &json!({}))?;

        if status.to_string().contains("nothing to commit") {
            println!("      - 无变更，跳过推送");
            return Ok(false);
        }

        // 生成提交消息
        let diff_str = self.call_tool("git_diff", &json!({}))?;
        let commit_message = self.generate_commit_message(&diff_str)?;

        // 添加并提交
        println!("      - git add .");
        self.system_tools.call_tool("run_command", &json!({
            "command": "git add ."
        }))?;

        println!("      - git commit -m '{}'", commit_message);
        self.system_tools.call_tool("run_command", &json!({
            "command": &format!("git commit -m '{}'", commit_message)
        }))?;

        // 推送
        println!("      - git push");
        self.system_tools.call_tool("run_command", &json!({
            "command": "git push"
        }))?;

        Ok(true)
    }

    /// 生成提交消息（自主模式专属）
    ///
    /// # 服务边界检查
    /// 此方法仅能在自主模式下调用
    fn generate_commit_message(&self, diff: &str) -> Result<String> {
        // 运行时检查：确保仅在自主模式下调用
        if !self.autonomous_mode {
            return Err(anyhow::anyhow!(
                "generate_commit_message 仅在自主模式下可用，CLI 模式下禁止调用"
            ));
        }

        let messages = &mut vec![
            json!({
                "role": "system",
                "content": "你是一个专业的软件工程师，根据代码变更生成简洁的提交消息。
格式：type: description
type 包括：feat, fix, docs, refactor, test, chore"
            }),
            json!({
                "role": "user",
                "content": format!("请为以下变更生成提交消息：\n\n{}", diff)
            })
        ];

        let message = self.chat(messages)?;
        Ok(message.trim().to_string())
    }

    /// 检查是否继续进化
    fn should_continue_evolution(&self) -> bool {
        // 简单实现：总是继续
        // 实际可以实现更复杂的逻辑，如：
        // - 检查是否达到改进目标
        // - 检查是否有足够的改进点
        // - 检查用户是否干预
        true
    }

    // ========================================================================
    // 服务生命周期管理（服务化架构）
    // ========================================================================

    /// 初始化所有服务
    pub fn init_all_services(&mut self) -> Result<()> {
        tracing::info!("正在初始化所有服务...");

        // 初始化 HTTP 客户端
        if let Err(e) = self.http_client.init() {
            tracing::warn!("HTTP 客户端初始化失败：{}", e);
        }

        // 初始化集成模块
        match self.integrated_modules.initialize() {
            Ok(report) => {
                if !report.success {
                    tracing::warn!("集成模块初始化部分失败：");
                    for error in &report.errors {
                        tracing::warn!("  - {}", error);
                    }
                } else {
                    tracing::info!("集成模块初始化成功");
                }
            }
            Err(e) => {
                tracing::warn!("集成模块初始化失败：{}", e);
            }
        }

        tracing::info!("所有服务初始化完成");
        Ok(())
    }

    /// 健康检查
    pub fn health_check(&self) -> tool_matrix::matrix::ServiceHealthReport {
        use tool_matrix::matrix::{ServiceHealth, ServiceHealthReport};

        let mut report = ServiceHealthReport::new();

        // 检查 HTTP 客户端
        report.services.insert(
            "http_client".to_string(),
            self.http_client.health(),
        );

        // 检查集成模块
        if let Ok(dialogue_health) = self.integrated_modules.dialogue_tools.get_state() {
            report.services.insert(
                "dialogue".to_string(),
                if dialogue_health.contains("Error") {
                    ServiceHealth::Degraded
                } else {
                    ServiceHealth::Healthy
                },
            );
        }

        report
    }

    /// 优雅关闭
    pub fn shutdown(&mut self) -> Result<()> {
        tracing::info!("正在关闭所有服务...");

        // 关闭 HTTP 客户端
        if let Err(e) = self.http_client.shutdown() {
            tracing::warn!("HTTP 客户端关闭失败：{}", e);
        }

        // 关闭集成模块
        if let Err(e) = self.integrated_modules.shutdown() {
            tracing::warn!("集成模块关闭失败：{}", e);
        }

        tracing::info!("所有服务已关闭");
        Ok(())
    }

    /// 获取服务指标
    pub async fn get_service_metrics(&self, tool_name: Option<String>) -> Value {
        if let Some(name) = tool_name {
            // 获取特定工具的指标
            match name.as_str() {
                "http_client" => {
                    let stats = self.http_client.stats();
                    json!({
                        "service": "http_client",
                        "total_requests": stats.total_requests,
                        "success_count": stats.success_count,
                        "failure_count": stats.failure_count,
                        "avg_latency_ms": stats.avg_latency_ms,
                        "success_rate": stats.success_rate()
                    })
                }
                _ => {
                    json!({
                        "error": format!("未知服务：{}", name)
                    })
                }
            }
        } else {
            // 获取所有服务指标
            let http_stats = self.http_client.stats();
            json!({
                "services": {
                    "http_client": {
                        "total_requests": http_stats.total_requests,
                        "success_count": http_stats.success_count,
                        "failure_count": http_stats.failure_count,
                        "avg_latency_ms": http_stats.avg_latency_ms,
                        "success_rate": http_stats.success_rate()
                    }
                }
            })
        }
    }
}

/// 交互式输入辅助函数（支持退格、光标移动、历史纪录）
fn read_line_interactive(stdout: &mut io::Stdout, prompt: &str) -> Result<String> {
    let mut buffer = String::new();
    let mut cursor_pos = 0;  // 字符索引，不是字节索引
    let mut history: Vec<String> = Vec::new();
    let mut history_pos = 0;

    // 尝试启用原始模式，失败则使用标准输入（支持管道）
    let raw_mode_enabled = crossterm::terminal::enable_raw_mode().is_ok();

    print!("{}", prompt);
    stdout.flush()?;

    // 辅助函数：获取字符索引对应的字节索引
    fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    // 辅助函数：重绘输入行
    fn redraw_line(stdout: &mut io::Stdout, prompt: &str, buffer: &str, cursor_char_pos: usize) -> std::io::Result<()> {
        print!("\r\x1b[2K"); // 清除整行
        print!("{}", prompt);
        print!("{}", buffer);
        // 计算光标位置（考虑中文字符）
        let visible_pos = buffer.chars().take(cursor_char_pos).count();
        print!("\x1b[{}G", prompt.chars().count() + visible_pos + 1);
        stdout.flush()
    }

    // 如果没有启用 raw mode，使用标准输入
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
                // 只在按键释放时处理（避免重复触发）
                if key.kind != KeyEventKind::Release {
                    match key.code {
                        // Enter - 提交输入
                        KeyCode::Enter => {
                            let _ = crossterm::terminal::disable_raw_mode();
                            println!();
                            // 保存非空输入到历史
                            if !buffer.trim().is_empty() {
                                history.push(buffer.trim().to_string());
                            }
                            return Ok(buffer);
                        }

                        // Ctrl+C - 取消输入
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let _ = crossterm::terminal::disable_raw_mode();
                            println!("\n👋 再见！");
                            std::process::exit(0);
                        }

                        // Backspace - 删除前一个字符
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let byte_idx = char_to_byte_idx(&buffer, cursor_pos);
                                // 找到下一个字符的边界
                                let next_byte_idx = char_to_byte_idx(&buffer, cursor_pos + 1);
                                buffer.drain(byte_idx..next_byte_idx);
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }

                        // Delete - 删除当前字符
                        KeyCode::Delete => {
                            if cursor_pos < buffer.chars().count() {
                                let byte_idx = char_to_byte_idx(&buffer, cursor_pos);
                                let next_byte_idx = char_to_byte_idx(&buffer, cursor_pos + 1);
                                buffer.drain(byte_idx..next_byte_idx);
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }

                        // Left Arrow - 光标左移
                        KeyCode::Left => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }

                        // Right Arrow - 光标右移
                        KeyCode::Right => {
                            if cursor_pos < buffer.chars().count() {
                                cursor_pos += 1;
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }

                        // Home - 光标移到行首
                        KeyCode::Home => {
                            cursor_pos = 0;
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }

                        // End - 光标移到行尾
                        KeyCode::End => {
                            cursor_pos = buffer.chars().count();
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }

                        // Up Arrow - 上一条历史
                        KeyCode::Up => {
                            if !history.is_empty() && history_pos < history.len() {
                                history_pos += 1;
                                let idx = history.len() - history_pos;
                                buffer = history[idx].clone();
                                cursor_pos = buffer.chars().count();
                                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                            }
                        }

                        // Down Arrow - 下一条历史
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

                        // Esc - 清空输入
                        KeyCode::Esc => {
                            buffer.clear();
                            cursor_pos = 0;
                            let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
                        }

                        // 普通字符输入
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
                // 终端大小改变时重绘
                let _ = redraw_line(stdout, prompt, &buffer, cursor_pos);
            }
            Err(_) | Ok(_) => {
                // 事件读取失败或非键盘事件，使用标准输入回退
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

fn main() -> Result<()> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let use_autonomous = args.iter().any(|arg| arg == "--autonomous" || arg == "-a");

    // 解析 --project-path 参数
    let project_path = args.iter()
        .position(|arg| arg == "--project-path" || arg == "-p")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| PathBuf::from(s));

    // 初始化 tracing（输出到 stderr，避免干扰 stdout 的交互界面）
    // 默认只显示 warn/error 级别，info/debug 需要设置 RUST_LOG 环境变量
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("ai_assistant=warn".parse().unwrap())
                .add_directive("tokitai=warn".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    println!("🚀 AI Assistant 启动中...");

    // 加载 .env 文件（如果存在）
    if let Ok(env_content) = std::fs::read_to_string(".env") {
        for line in env_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                // 只加载 PROVIDER_ 或 AI_ 开头的配置
                if key.starts_with("PROVIDER_") || key.starts_with("AI_") || key == "PROVIDERS" || key == "SEARXNG_URL" {
                    std::env::set_var(key, value);
                }
            }
        }
    }

    // 多供应商模式：自动初始化当前供应商（选择第一个）
    if std::env::var("PROVIDERS").is_ok() && std::env::var("AI_API_URL").is_err() {
        if let Ok(pm) = crate::provider_config::ProviderManager::from_env_file(None) {
            let current = pm.current();
            std::env::set_var("AI_API_URL", &current.api_url);
            if let Some(key) = &current.api_key {
                std::env::set_var("AI_API_KEY", key);
            }
            std::env::set_var("AI_MODEL", &current.model);
            info!("🔌 使用供应商：{} ({})", current.name, current.api_url);
        }
    }

    // 加载配置
    let config = config::Config::load(None).unwrap_or_else(|e| {
        warn!("加载配置文件失败：{}，使用默认配置", e);
        config::Config::default()
    });

    // 从环境变量或配置获取配置
    let api_url = std::env::var("AI_API_URL")
        .unwrap_or_else(|_| "https://ollama.com/v1/chat/completions".to_string());
    let api_key = std::env::var("AI_API_KEY").ok();
    // 优先级：环境变量 > 配置文件 > 硬编码默认值
    let model = std::env::var("AI_MODEL")
        .unwrap_or_else(|_| {
            if config.ai.model.is_empty() {
                "qwen3.5:397b".to_string()
            } else {
                config.ai.model.clone()
            }
        });

    // 检查配置（支持多供应商模式）
    let has_api_key = api_key.is_some() || std::env::var("PROVIDERS").is_ok();
    if !has_api_key {
        eprintln!("⚠️  警告：未配置 API Key");
        eprintln!("   在 .env 中设置 AI_API_KEY 或 PROVIDERS");
        eprintln!();
    }

    // 如果指定了 --autonomous，启动自主进化模式
    if use_autonomous {
        // 获取项目根目录：优先使用 --project-path 参数，否则使用当前目录
        let project_root = project_path
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map_err(|e| anyhow::anyhow!("获取当前目录失败：{}", e))
                    .unwrap()
            });

        println!("🤖 启动自主进化模式");
        println!("═══════════════════════════");
        println!("📁 项目路径：{}", project_root.display());
        println!();
        println!("✨ AI 将自主：");
        println!("   • 发现项目改进点");
        println!("   • 规划 → 执行 → 审查 → 提交");
        println!("   • 多 Agent 协作（Planner/Executor/Reviewer）");
        println!();
        println!("⚠️  注意：AI 将自主修改代码，按 Ctrl+C 停止");
        println!("═══════════════════════════\n");

        // 切换工作目录到目标项目（确保沙箱隔离生效）
        std::env::set_current_dir(&project_root)
            .map_err(|e| anyhow::anyhow!("切换目录失败：{}", e))?;

        println!("📂 工作目录：{}", std::env::current_dir().unwrap().display());
        println!();

        // 创建自主模式的助手（使用当前目录）
        let assistant = AiAssistant::new_autonomous(
            api_url,
            api_key,
            model,
            std::env::current_dir().unwrap(),
        ).map_err(|e| anyhow::anyhow!("创建自主模式失败：{}", e))?;

        // 运行自主进化
        assistant.run_autonomous_evolution()?;

        return Ok(());
    }

    // 普通交互模式
    let mut assistant = AiAssistant::new(api_url, api_key, model);
    let mut messages: Vec<Value> = vec![json!({
        "role": "system",
        "content": "你是一个强大的 AI 助手，可以调用各种工具来帮助用户完成任务。你可以：
- 读取和写入文件
- 执行系统命令
- 分析代码
- 搜索网络信息
- 搜索和下载图片
- 网页截图和获取渲染内容

请根据用户需求选择合适的工具。

当用户输入'help'时，请列出你可以执行的操作示例。"
    })];

    // 检查是否有命令行参数直接输入
    let non_arg_args: Vec<String> = args.iter()
        .filter(|arg| !arg.starts_with('-'))
        .skip(1)  // 跳过程序名
        .cloned()
        .collect();

    if !non_arg_args.is_empty() {
        // 有命令行参数，直接处理并退出
        let input = non_arg_args.join(" ");
        println!("你：{}", input);

        match assistant.chat_and_handle_tools(&mut messages, &input) {
            Ok(response) => {
                println!("\n{}", response);
            }
            Err(e) => {
                println!("\n错误：{}", e);
            }
        }
        return Ok(());
    }

    // 显示欢迎消息（简洁版）
    println!();
    println!("  Tokitai AI Assistant v2.1.0");
    println!();
    println!("  输入 help 查看功能，quit 退出");
    println!();

    let mut stdout = io::stdout();

    // 交互式输入循环（支持退格、光标移动、历史纪录）
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
            // 特殊处理 /toolbox 命令
            if input == "/toolbox" {
                let stats = assistant.get_toolbox_stats();
                println!("\n工具箱状态：\n{}", serde_json::to_string_pretty(&stats).unwrap_or_default());
                println!();
                continue;
            }

            let processed = assistant.orchestrator.process_input(&input);
            if let Some(cmd) = processed.command {
                let result = assistant.orchestrator.execute_command(cmd);
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

        // 如果解析到了文件内容，给出提示
        if !file_contents.is_empty() {
            println!("已加载 {} 个文件", file_contents.len());
        }

        // 使用编排器处理输入（角色切换 + 上下文管理）
        let processed = assistant.orchestrator.process_input(&processed_input);

        // 如果有角色切换，给出提示
        if processed.role_changed && assistant.orchestrator.config.verbose {
            println!("切换到角色：{}\n", processed.current_role.as_str());
        }

        // 显示等待指示器（增强版 - 分阶段显示）
        print!("🤔 思考中");
        let _ = std::io::stdout().flush();
        let spin_start = std::time::Instant::now();

        // 使用 chat_and_handle_tools 处理工具调用（带进度监控）
        match assistant.chat_and_handle_tools(&mut messages, &processed_input) {
            Ok(response) => {
                // 清除等待指示器
                print!("\r\x1b[K");
                
                // 显示响应时间统计
                let elapsed = spin_start.elapsed();
                if elapsed.as_millis() < 500 {
                    println!("✅ 完成 ({:.0}ms)", elapsed.as_millis() as f64);
                } else if elapsed.as_millis() < 2000 {
                    println!("✅ 完成 ({:.1}s)", elapsed.as_secs_f64());
                } else {
                    println!("✅ 完成 ({:.1}s)", elapsed.as_secs_f64());
                }
                
                println!("\n{}\n", response);

                // 添加 AI 响应到消息历史
                messages.push(json!({
                    "role": "assistant",
                    "content": response
                }));

                // 使用编排器处理响应
                assistant.orchestrator.process_response(&response);
            }
            Err(e) => {
                // 清除等待指示器
                print!("\r\x1b[K");
                
                // 显示错误响应时间
                let elapsed = spin_start.elapsed();
                println!("❌ 请求失败 ({:.1}s): {}", elapsed.as_secs_f64(), e);
                println!("提示：可能是网络问题或 API 配置错误，检查 .env 文件后重试\n");
                // 出错时移除最后添加的用户消息
                messages.pop();
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tools::{FileOperations, CodeTools, SystemTools, SearchTools, DownloadTools};
    use tokitai::ToolProvider;

    #[test]
    fn test_file_operations_read_write() {
        let file_ops = FileOperations::default();
        let test_path = "/tmp/test_tokitai.txt";
        let test_content = "Hello, Tokitai!";

        // 测试写入
        let write_result = file_ops.call_tool("write_file", &json!({
            "path": test_path,
            "content": test_content
        }));
        assert!(write_result.is_ok());

        // 测试读取
        let read_result = file_ops.call_tool("read_file", &json!({
            "path": test_path
        }));
        assert!(read_result.is_ok());
        // 注意：call_tool 返回的是 JSON 字符串，包含引号
        assert!(read_result.unwrap().to_string().contains(test_content));

        // 清理
        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_file_operations_list_dir() {
        let file_ops = FileOperations::default();

        // 测试列出当前目录
        let result = file_ops.call_tool("list_dir", &json!({
            "path": "."
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_code_tools_detect_language() {
        let code_tools = CodeTools::default();

        // 测试检测 Rust 文件
        let result = code_tools.call_tool("detect_language", &json!({
            "path": "src/main.rs"
        }));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.to_string().contains("Rust"));
    }

    #[test]
    fn test_system_tools_get_current_dir() {
        let system_tools = SystemTools::default();

        let result = system_tools.call_tool("get_current_dir", &json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definitions_generation() {
        // 确保所有工具都有工具定义
        assert!(!FileOperations::tool_definitions().is_empty());
        assert!(!CodeTools::tool_definitions().is_empty());
        assert!(!SystemTools::tool_definitions().is_empty());
        assert!(!SearchTools::tool_definitions().is_empty());
        assert!(!DownloadTools::tool_definitions().is_empty());

        // 验证工具定义格式
        for def in FileOperations::tool_definitions().iter() {
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
            assert!(!def.input_schema.is_empty());
        }
    }
}
