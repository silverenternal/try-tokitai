//! 助手通用模块
//!
//! 提供 CLI 助手和自主助手共享的配置和工具管理器

use anyhow::Result;
use reqwest::blocking::Client;
use std::sync::Arc;
use serde_json::Value;

use crate::tool_matrix::registry::ToolRegistry;
use crate::tool_matrix::dispatcher::ToolDispatcher;
use crate::tool_matrix::tool_selector::LightweightToolSelector;

// ============================================================================
// 共享配置
// ============================================================================

/// 助手配置
/// 
/// 包含 CLI 助手和自主助手共享的基础配置
#[derive(Clone)]
pub struct AssistantConfig {
    /// AI API 端点 URL
    pub api_url: String,
    /// AI API 密钥（可选）
    pub api_key: Option<String>,
    /// 使用的模型名称
    pub model: String,
    /// HTTP 客户端（持久连接池）
    pub reqwest_client: Client,
}

impl AssistantConfig {
    /// 创建新的助手配置
    /// 
    /// # 参数
    /// - `api_url`: AI API 端点 URL
    /// - `api_key`: AI API 密钥（可选）
    /// - `model`: 使用的模型名称
    pub fn new(api_url: String, api_key: Option<String>, model: String) -> Self {
        // 创建持久的 HTTP 客户端（带连接池和超时配置）
        let reqwest_client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))  // 120 秒超时
            .connect_timeout(std::time::Duration::from_secs(30))  // 30 秒连接超时
            .pool_max_idle_per_host(10)  // 每个主机最多 10 个空闲连接
            .build()
            .expect("创建 HTTP 客户端失败");

        Self {
            api_url,
            api_key,
            model,
            reqwest_client,
        }
    }
}

// ============================================================================
// 工具管理器
// ============================================================================

/// 工具管理器
/// 
/// 统一管理工具注册表、选择器和分发器
pub struct ToolManager {
    /// 工具注册表
    pub tool_registry: ToolRegistry,
    /// 轻量级工具选择器（AI 原生）
    pub lightweight_selector: Arc<LightweightToolSelector>,
    /// 工具调用分发器
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
    use crate::tools::{
        CodeTools, DownloadTools, FileOperations, GitOperations, SystemTools,
        SearchTools, HttpClientTools, FileSearchTools, ProcessTools,
        NetworkTools, WikipediaTools, ProjectTemplates, PdfTools,
    };
    use crate::tools::data::{JsonQueryTools, JsonMergeTools, DataConversionTools};
    use crate::tools::data::JsonFormatTools as JsonTools;
    use crate::tools::system::system_monitor::SystemMonitor;
    use crate::dialogue::DialogueTools;
    use crate::observability::ObservabilityTools;
    use crate::prompt_engineering::PromptTools;
    use crate::tool_matrix::matrix::ToolBox;
    use crate::tool_matrix::registry::ToolSource;

    // 创建工具箱
    let _ = tool_registry.create_toolbox(ToolBox::new("file_ops", "File Operations", "File operations tools"));
    let _ = tool_registry.create_toolbox(ToolBox::new("system", "System Tools", "System operations tools"));
    let _ = tool_registry.create_toolbox(ToolBox::new("code", "Code Tools", "Code analysis and processing tools"));
    let _ = tool_registry.create_toolbox(ToolBox::new("web", "Web Tools", "Web search and network tools"));
    let _ = tool_registry.create_toolbox(ToolBox::new("git", "Git Tools", "Git version control tools"));
    let _ = tool_registry.create_toolbox(ToolBox::new("data", "Data Tools", "Data processing tools"));

    // 从各个 ToolProvider 注册工具
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

    // 注册数据模块工具
    let _ = tool_registry.register_from_provider_sync::<JsonQueryTools>(Some("data"), ToolSource::Builtin);
    let _ = tool_registry.register_from_provider_sync::<JsonMergeTools>(Some("data"), ToolSource::Builtin);
    let _ = tool_registry.register_from_provider_sync::<DataConversionTools>(Some("data"), ToolSource::Builtin);

    // 注册系统监控工具
    let _ = tool_registry.register_from_provider_sync::<SystemMonitor>(Some("system"), ToolSource::Builtin);

    // 注册集成模块工具
    let _ = tool_registry.register_from_provider_sync::<DialogueTools>(Some("system"), ToolSource::Builtin);
    let _ = tool_registry.register_from_provider_sync::<ObservabilityTools>(Some("system"), ToolSource::Builtin);
    let _ = tool_registry.register_from_provider_sync::<PromptTools>(Some("system"), ToolSource::Builtin);
}
