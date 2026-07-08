//! 助手通用模块
//!
//! 提供 CLI 助手和自主助手共享的配置和工具管理器

use reqwest::blocking::Client;
use serde_json::Value;
use std::sync::Arc;

use crate::tool_matrix::dispatcher::ToolDispatcher;
use crate::tool_matrix::registry::ToolRegistry;
use crate::tool_matrix::tool_selector::LightweightToolSelector;

// ============================================================================
// 共享配置
// ============================================================================

/// 助手配置
///
/// 包含 CLI 助手和自主助手共享的基础配置
#[derive(Clone)]
pub struct AssistantConfig {
    pub api_url: String,
    #[allow(dead_code)]
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub reqwest_client: Client,
}

impl AssistantConfig {
    pub fn new(api_url: String, api_key: Option<String>, model: String) -> Self {
        Self::new_with_runtime(api_url, api_key, model, 0.7, 4096)
    }

    pub fn new_with_runtime(
        api_url: String,
        api_key: Option<String>,
        model: String,
        temperature: f32,
        max_tokens: usize,
    ) -> Self {
        let reqwest_client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("failed to create reqwest client");

        Self {
            api_url,
            api_key,
            model,
            temperature,
            max_tokens,
            reqwest_client,
        }
    }
}

pub struct ToolManager {
    /// 工具注册表
    pub tool_registry: ToolRegistry,
    /// 轻量级工具选择器（AI 原生）
    #[allow(dead_code)]
    pub lightweight_selector: Arc<LightweightToolSelector>,
    /// 工具调用分发器
    #[allow(dead_code)]
    pub tool_dispatcher: Arc<ToolDispatcher>,
}

impl ToolManager {
    /// 创建新的工具管理器
    ///
    /// # 参数
    /// - `tool_registry`: 工具注册表
    pub fn new(tool_registry: ToolRegistry) -> Self {
        // 获取所有工具定义用于创建轻量级选择器
        let all_tools = tool_registry.get_all_tools();

        // 创建轻量级工具选择器（不带 AI，使用默认配置）
        let lightweight_selector = Arc::new(LightweightToolSelector::new_without_ai(
            all_tools.clone(),
            None,
        ));

        // 创建工具分发器
        let tool_dispatcher = Arc::new(ToolDispatcher::new(lightweight_selector.clone()));

        Self {
            tool_registry,
            lightweight_selector,
            tool_dispatcher,
        }
    }

    /// 获取所有工具定义
    pub fn get_all_tools(&self) -> Vec<Value> {
        self.tool_registry
            .get_all_tools()
            .iter()
            .map(|t| {
                serde_json::json!({
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

    pub fn get_tools_by_name(&self, allowed_names: &[&str]) -> Vec<Value> {
        let allowed = allowed_names
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();

        self.tool_registry
            .get_all_tools()
            .into_iter()
            .filter(|tool| allowed.contains(tool.name.as_str()))
            .map(|t| {
                serde_json::json!({
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
        let mut stats = serde_json::json!({
            "total_tools": self.tool_registry.tool_count(),
            "total_toolboxes": self.tool_registry.toolbox_count(),
            "toolboxes": []
        });

        if let Some(boxes) = stats.get_mut("toolboxes").and_then(|v| v.as_array_mut()) {
            for box_ref in &toolboxes {
                boxes.push(serde_json::json!({
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
}

// ============================================================================
// 工具注册辅助函数
// ============================================================================

/// 注册所有内置工具到工具注册表
///
/// # 参数
/// - `tool_registry`: 工具注册表
pub fn register_all_builtin_tools(tool_registry: &ToolRegistry) {
    use crate::dialogue::DialogueTools;
    use crate::observability::ObservabilityTools;
    use crate::prompt_engineering::PromptTools;
    use crate::tool_matrix::matrix::ToolBox;
    use crate::tool_matrix::registry::ToolSource;
    use crate::tools::data::JsonFormatTools as JsonTools;
    use crate::tools::data::{DataConversionTools, JsonMergeTools, JsonQueryTools};
    use crate::tools::system::system_monitor::SystemMonitor;
    use crate::tools::{
        CodeTools, DownloadTools, FileOperations, FileSearchTools, GitOperations, HttpClientTools,
        IdeTools, NetworkTools, PdfTools, ProcessTools, ProjectTemplates, SearchTools, SystemTools,
        WikipediaTools,
    };

    // 创建工具箱
    let _ = tool_registry.create_toolbox(ToolBox::new(
        "file_ops",
        "File Operations",
        "File operations tools",
    ));
    let _ = tool_registry.create_toolbox(ToolBox::new(
        "system",
        "System Tools",
        "System operations tools",
    ));
    let _ = tool_registry.create_toolbox(ToolBox::new(
        "code",
        "Code Tools",
        "Code analysis and processing tools",
    ));
    let _ = tool_registry.create_toolbox(ToolBox::new(
        "web",
        "Web Tools",
        "Web search and network tools",
    ));
    let _ = tool_registry.create_toolbox(ToolBox::new(
        "git",
        "Git Tools",
        "Git version control tools",
    ));
    let _ =
        tool_registry.create_toolbox(ToolBox::new("data", "Data Tools", "Data processing tools"));
    let _ = tool_registry.create_toolbox(ToolBox::new(
        "scientist",
        "Scientist Tools",
        "AI Scientist research tools — literature, computation, data, symbolic verification",
    ));

    // 从各个 ToolProvider 注册工具
    let _ = tool_registry
        .register_from_provider_sync::<FileOperations>(Some("file_ops"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<SystemTools>(Some("system"), ToolSource::Builtin);
    let _ =
        tool_registry.register_from_provider_sync::<CodeTools>(Some("code"), ToolSource::Builtin);
    let _ =
        tool_registry.register_from_provider_sync::<SearchTools>(Some("web"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<DownloadTools>(Some("web"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<GitOperations>(Some("git"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<HttpClientTools>(Some("web"), ToolSource::Builtin);
    let _ =
        tool_registry.register_from_provider_sync::<JsonTools>(Some("data"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<FileSearchTools>(Some("file_ops"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<ProcessTools>(Some("system"), ToolSource::Builtin);
    let _ =
        tool_registry.register_from_provider_sync::<NetworkTools>(Some("web"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<WikipediaTools>(Some("web"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<ProjectTemplates>(Some("data"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<PdfTools>(Some("file_ops"), ToolSource::Builtin);

    // 注册数据模块工具
    let _ = tool_registry
        .register_from_provider_sync::<JsonQueryTools>(Some("data"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<JsonMergeTools>(Some("data"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<DataConversionTools>(Some("data"), ToolSource::Builtin);

    // 注册系统监控工具
    let _ = tool_registry
        .register_from_provider_sync::<SystemMonitor>(Some("system"), ToolSource::Builtin);
    let _ =
        tool_registry.register_from_provider_sync::<IdeTools>(Some("system"), ToolSource::Builtin);

    // 注册集成模块工具
    let _ = tool_registry
        .register_from_provider_sync::<DialogueTools>(Some("system"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<ObservabilityTools>(Some("system"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<PromptTools>(Some("system"), ToolSource::Builtin);

    // Register CS-focused scientist tools only. Domain-specific science toolboxes
    // remain available in source for isolated tests, but are intentionally not
    // mounted into the default scientist runtime.
    use crate::scientist::tools::computation::ComputationTools;
    use crate::scientist::tools::data::DataTools;
    use crate::scientist::tools::github::GitHubTools;
    use crate::scientist::tools::literature::LiteratureTools;
    use crate::scientist::tools::verification_center::VerificationCenterTools;
    let _ = tool_registry
        .register_from_provider_sync::<LiteratureTools>(Some("scientist"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<ComputationTools>(Some("scientist"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<DataTools>(Some("scientist"), ToolSource::Builtin);
    let _ = tool_registry
        .register_from_provider_sync::<GitHubTools>(Some("scientist"), ToolSource::Builtin);
    let _ = tool_registry.register_from_provider_sync::<VerificationCenterTools>(
        Some("scientist"),
        ToolSource::Builtin,
    );

    // Register SymPy tools
    use crate::scientist::tools::sympy_tool::SymPyTool;
    let _ = tool_registry
        .register_from_provider_sync::<SymPyTool>(Some("scientist"), ToolSource::Builtin);
}

pub fn curated_ai_scientist_tool_names() -> &'static [&'static str] {
    &[
        "read_file",
        "read_file_head",
        "read_file_range",
        "inspect_path",
        "write_file",
        "edit_file",
        "search_and_replace_multi",
        "apply_patch",
        "delete_file",
        "mkdir",
        "rename_path",
        "list_dir",
        "grep",
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
        "read_pdf",
        "search_web",
        "fetch_url",
        "search_arxiv",
        "search_paper",
        "fetch_paper",
        "fetch_papers",
        "run_python",
        "run_python_file",
        "inspect_dataset",
        "search_public_datasets",
        "fetch_public_dataset_manifest",
        "search_github_repositories",
        "search_github_code",
        "search_github_datasets",
        "sympy_simplify",
        "sympy_solve",
        "sympy_integrate",
        "sympy_diff",
        "sympy_matrix",
        "git_status",
        "git_diff",
        "git_diff_file",
        "git_log",
        "git_current_branch",
    ]
}
