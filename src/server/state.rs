//! AppState：跨 handler 共享的应用状态
//!
//! 字段按可共享性分两类：
//! - 大部分组件用 `Arc<…>` 共享，便于跨 handler 并发访问。
//! - 部分类型因自身 API 需要 `&mut self` 用 `parking_lot::Mutex` 包装。

use std::sync::Arc;

use crate::assistant_common::{AssistantConfig, ToolManager};
use crate::dialogue::DialogueStateMachine;
use crate::llm::LLMManager;
use crate::mcp::McpClientManager;
use crate::orchestrator::Orchestrator;
use crate::tool_market::ToolMarket;
use parking_lot::Mutex as PlMutex;

use super::routes::autonomy::AutonomyStore;
use super::routes::context::ContextState;
use super::stores::SharedStores;
use super::tool_set::ServerToolSet;

/// 服务器配置（host 永远是 127.0.0.1）
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 监听端口
    pub port: u16,
    /// 可选 Bearer token
    pub api_key: Option<String>,
    /// 是否启用 CORS（开发用）
    pub cors_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            api_key: None,
            cors_enabled: true,
        }
    }
}

/// 跨 handler 共享的应用状态
///
/// 注意：本版本**不**持有 CliAssistant。Server 模式下独立构造子组件，
/// 工具调用走 `ServerToolSet`，对话/编排/LLM 直接访问各自子组件。
#[derive(Clone)]
pub struct AppState {
    /// 助手基础配置
    pub config: Arc<AssistantConfig>,
    /// 工具管理器（注册表 + 选择器 + dispatcher）
    pub tool_manager: Arc<ToolManager>,
    /// server 模式专用的工具实例集（8 个 #[tool] provider）
    pub tool_set: Arc<ServerToolSet>,
    /// 编排器（角色切换、上下文优化、命令分发）
    pub orchestrator: Arc<PlMutex<Orchestrator>>,
    /// LLM 供应商管理（可切换 current provider）
    pub llm: Arc<PlMutex<LLMManager>>,
    /// 对话状态机
    pub dialogue: Arc<PlMutex<DialogueStateMachine>>,
    /// 会话 / 工作流仓库
    pub stores: SharedStores,
    /// 工具市场（可选；初始化失败时为 None）
    pub tool_market: Arc<tokio::sync::Mutex<Option<ToolMarket>>>,
    /// MCP 客户端管理
    pub mcp: Arc<PlMutex<McpClientManager>>,
    /// tokitai-context facade（仅缓存路径；Context 在 handler 内 spawn_blocking 打开）
    pub context: Arc<PlMutex<ContextState>>,
    /// 自主进化后台任务句柄
    pub autonomy: AutonomyStore,
    /// 服务器配置
    pub server_cfg: ServerConfig,
}

impl AppState {
    /// 构造 AppState（不含 tool_market / mcp / context）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: AssistantConfig,
        tool_manager: ToolManager,
        tool_set: ServerToolSet,
        orchestrator: Orchestrator,
        llm: LLMManager,
        dialogue: DialogueStateMachine,
    ) -> Self {
        Self {
            config: Arc::new(config),
            tool_manager: Arc::new(tool_manager),
            tool_set: Arc::new(tool_set),
            orchestrator: Arc::new(PlMutex::new(orchestrator)),
            llm: Arc::new(PlMutex::new(llm)),
            dialogue: Arc::new(PlMutex::new(dialogue)),
            stores: SharedStores::new(),
            tool_market: Arc::new(tokio::sync::Mutex::new(None)),
            mcp: Arc::new(PlMutex::new(McpClientManager::new())),
            context: super::routes::context::build_default_context(),
            autonomy: AutonomyStore::default(),
            server_cfg: ServerConfig::default(),
        }
    }

    /// 构造 AppState，附带 tool_market 与 mcp（main.rs 用）
    #[allow(clippy::too_many_arguments)]
    pub fn with_extras(
        config: AssistantConfig,
        tool_manager: ToolManager,
        tool_set: ServerToolSet,
        orchestrator: Orchestrator,
        llm: LLMManager,
        dialogue: DialogueStateMachine,
        tool_market: Arc<tokio::sync::Mutex<Option<ToolMarket>>>,
        mcp: Arc<PlMutex<McpClientManager>>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            tool_manager: Arc::new(tool_manager),
            tool_set: Arc::new(tool_set),
            orchestrator: Arc::new(PlMutex::new(orchestrator)),
            llm: Arc::new(PlMutex::new(llm)),
            dialogue: Arc::new(PlMutex::new(dialogue)),
            stores: SharedStores::new(),
            tool_market,
            mcp,
            context: super::routes::context::build_default_context(),
            autonomy: AutonomyStore::default(),
            server_cfg: ServerConfig::default(),
        }
    }
}
