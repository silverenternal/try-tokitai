#![recursion_limit = "256"]

mod command_resolver;
mod config;
mod path_resolver;
mod sandbox;
mod tools;
// Context is now a separate crate: tokitai-context
mod assistant_common;
mod autonomous_assistant;
mod autonomy;
mod cli_assistant;
mod context_cli;
mod dialogue;
mod experiments;
mod external_process;
mod integration;
pub mod llm;
pub mod mcp;
mod observability;
mod orchestrator;
mod prompt_engineering;
mod provider_config;
pub mod tool_market;
mod tool_matrix;
pub mod tui;

// HTTP REST API Server（feature-gated）
#[cfg(feature = "server")]
mod server;

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use assistant_common::AssistantConfig;
use autonomous_assistant::AutonomousAssistant;
use cli_assistant::CliAssistant;

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
// ┌─────────────────────────────────────────────────────────────────────────┐
// │  服务三：HTTP REST API Server（新增，面向程序/浏览器）                  │
// ├─────────────────────────────────────────────────────────────────────────┤
// │  • 启动命令：cargo run --features server -- --server --port 8080        │
// │  • 服务对象：其他程序、脚本、浏览器插件                                  │
// │  • 驱动方式：HTTP 请求驱动                                              │
// │  • 交互模式：JSON over HTTP / SSE                                       │
// │  • 典型场景：嵌入其他工作流、Web UI、自动化                              │
// │  • 服务边界：仅监听 127.0.0.1；可选用 Bearer token                      │
// └─────────────────────────────────────────────────────────────────────────┘
//
// 详细文档：docs/SERVER.md
// ============================================================================

fn main() -> Result<()> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let use_autonomous = args.iter().any(|arg| arg == "--autonomous" || arg == "-a");
    let use_mcp = args.iter().any(|arg| arg == "--mcp" || arg == "-m");
    let use_tui = args.iter().any(|arg| arg == "--tui" || arg == "-t");
    #[cfg(feature = "server")]
    let use_server = args.iter().any(|arg| arg == "--server" || arg == "-s");
    #[cfg(not(feature = "server"))]
    let use_server = false;

    // 检查工具市场命令
    if args.len() >= 2 && args[1] == "tokitai" {
        return handle_tool_market_command(&args[2..]);
    }

    // 检查实验命令
    if args.len() >= 2 && args[1] == "experiment" {
        // 异步运行实验命令
        let rt = tokio::runtime::Runtime::new().unwrap();
        return rt.block_on(crate::experiments::cli::run_experiment_command(&args[2..]));
    }

    // 检查平行上下文命令
    if args.len() >= 2 && args[1] == "context" {
        return crate::context_cli::handle_context_command(&args[2..]);
    }

    // 解析 --project-path 参数
    let project_path = args
        .iter()
        .position(|arg| arg == "--project-path" || arg == "-p")
        .and_then(|pos| args.get(pos + 1))
        .map(PathBuf::from);

    // 解析 --port 参数（仅在 --server 模式下使用）
    #[cfg(feature = "server")]
    let server_port: u16 = args
        .iter()
        .position(|arg| arg == "--port" || arg == "-P")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    // 解析 --api-key 参数（仅在 --server 模式下使用）
    #[cfg(feature = "server")]
    let server_api_key: Option<String> = args
        .iter()
        .position(|arg| arg == "--api-key")
        .and_then(|pos| args.get(pos + 1))
        .cloned();

    // 初始化 tracing（输出到 stderr，避免干扰 stdout 的交互界面）
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
                if key.starts_with("PROVIDER_")
                    || key.starts_with("AI_")
                    || key == "PROVIDERS"
                    || key == "SEARXNG_URL"
                {
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
    let model = std::env::var("AI_MODEL").unwrap_or_else(|_| {
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

    // 创建助手配置
    let config = AssistantConfig::new(api_url, api_key, model);

    // 如果指定了 --server，启动 HTTP REST API Server 模式
    #[cfg(feature = "server")]
    if use_server {
        use crate::assistant_common::{register_all_builtin_tools, ToolManager};
        use crate::dialogue::DialogueStateMachine;
        use crate::llm::{LLMManager, ProviderInitializer};
        use crate::orchestrator::Orchestrator;
        use crate::server::state::AppState;

        println!("🌐 启动 HTTP REST API Server 模式");
        println!("═══════════════════════════");
        println!("📡 传输模式：HTTP/1.1 (axum)");
        println!("🔒 绑定地址：127.0.0.1:{}", server_port);
        if server_api_key.is_some() {
            println!("🔑 鉴权：Bearer token 已启用");
        } else {
            println!("🔓 鉴权：关闭（仅 loopback 可访问）");
        }
        println!();
        println!("✨ HTTP Server 功能：");
        println!("   • 暴露全部 CLI/TUI 能力为 REST API");
        println!("   • /v1/chat、/v1/tools/call、/v1/orchestrator/* 等");
        println!("   • /v1/chat/stream 支持 SSE 流式响应");
        println!();
        println!("📚 文档：docs/SERVER.md");
        println!("⚠️  注意：按 Ctrl+C 停止");
        println!("═══════════════════════════\n");

        // 独立构造 server 模式下的子组件
        // （不复用 CliAssistant，因 CliAssistant 持有大量 owned 字段不易共享）
        let app_config = crate::config::Config::load(None).unwrap_or_default();
        let llm_manager = ProviderInitializer::new(app_config)
            .initialize_llm_manager()
            .unwrap_or_else(|_| LLMManager::new());

        let tool_registry = crate::tool_matrix::registry::ToolRegistry::new();
        register_all_builtin_tools(&tool_registry);
        let tool_manager = ToolManager::new(tool_registry);

        let orchestrator = Orchestrator::new();
        let dialogue = DialogueStateMachine::new_without_persistence();

        let app_state = AppState::new(config, tool_manager, orchestrator, llm_manager, dialogue);

        let rt = tokio::runtime::Runtime::new().unwrap();
        return rt.block_on(crate::server::run_server(
            server_port,
            server_api_key,
            app_state,
        ));
    }

    // 如果指定了 --mcp，启动 MCP Server 模式
    if use_mcp {
        println!("🔌 启动 MCP Server 模式");
        println!("═══════════════════════════");
        println!("📡 传输模式：stdio");
        println!();
        println!("✨ MCP Server 将：");
        println!("   • 暴露所有 #[tool] 函数");
        println!("   • 通过 stdio 与 AI 客户端通信");
        println!("   • 符合 Model Context Protocol 规范");
        println!();
        println!("⚠️  注意：按 Ctrl+C 停止");
        println!("═══════════════════════════\n");

        // 启动 MCP Server
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(mcp::start_mcp_mode())?;

        return Ok(());
    }

    // 如果指定了 --tui，启动 TUI 模式
    if use_tui {
        println!("🎨 启动 TUI 模式");
        println!("═══════════════════════════");
        println!("📱 界面：ratatui 终端图形界面");
        println!();
        println!("✨ TUI 功能：");
        println!("   • 三面板布局（工具/对话/上下文）");
        println!("   • 实时状态显示");
        println!("   • 快捷键系统");
        println!();
        println!("⚠️  注意：按 Ctrl+Q 退出");
        println!("═══════════════════════════\n");

        // 启动 TUI
        tui::run_tui()?;

        return Ok(());
    }

    // 如果指定了 --autonomous，启动自主进化模式
    if use_autonomous {
        let project_root = project_path.unwrap_or_else(|| {
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

        // 切换工作目录到目标项目
        std::env::set_current_dir(&project_root)
            .map_err(|e| anyhow::anyhow!("切换目录失败：{}", e))?;

        println!(
            "📂 工作目录：{}",
            std::env::current_dir().unwrap().display()
        );
        println!();

        // 创建自主助手
        let assistant = AutonomousAssistant::new(config, std::env::current_dir().unwrap())
            .map_err(|e| anyhow::anyhow!("创建自主模式失败：{}", e))?;

        // 运行自主进化
        assistant.run_autonomous_evolution()?;

        return Ok(());
    }

    // 普通交互模式
    let mut assistant = CliAssistant::new(config)?;

    // 检查是否有命令行参数直接输入
    let non_arg_args: Vec<String> = args
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .skip(1)
        .cloned()
        .collect();

    if !non_arg_args.is_empty() {
        let input = non_arg_args.join(" ");
        println!("你：{}", input);

        // 创建临时消息向量
        let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "system",
            "content": "你是一个强大的 AI 助手，可以调用各种工具来帮助用户完成任务。"
        })];

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

    // 运行交互式 CLI
    assistant.run_cli()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::tools::{CodeTools, DownloadTools, FileOperations, SearchTools, SystemTools};
    use tokitai::ToolProvider;

    #[test]
    fn test_file_operations_read_write() {
        let file_ops = FileOperations::default();
        let test_path = "/tmp/test_tokitai.txt";
        let test_content = "Hello, Tokitai!";

        let write_result = file_ops.call_tool(
            "write_file",
            &serde_json::json!({
                "path": test_path,
                "content": test_content
            }),
        );
        assert!(write_result.is_ok());

        let read_result = file_ops.call_tool(
            "read_file",
            &serde_json::json!({
                "path": test_path
            }),
        );
        assert!(read_result.is_ok());
        assert!(read_result.unwrap().to_string().contains(test_content));

        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_file_operations_list_dir() {
        let file_ops = FileOperations::default();

        let result = file_ops.call_tool(
            "list_dir",
            &serde_json::json!({
                "path": "."
            }),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_code_tools_detect_language() {
        let code_tools = CodeTools::default();

        let result = code_tools.call_tool(
            "detect_language",
            &serde_json::json!({
                "path": "src/main.rs"
            }),
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.to_string().contains("Rust"));
    }

    #[test]
    fn test_system_tools_get_current_dir() {
        let system_tools = SystemTools::default();

        let result = system_tools.call_tool("get_current_dir", &serde_json::json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definitions_generation() {
        assert!(!FileOperations::tool_definitions().is_empty());
        assert!(!CodeTools::tool_definitions().is_empty());
        assert!(!SystemTools::tool_definitions().is_empty());
        assert!(!SearchTools::tool_definitions().is_empty());
        assert!(!DownloadTools::tool_definitions().is_empty());

        for def in FileOperations::tool_definitions().iter() {
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
            assert!(!def.input_schema.is_empty());
        }
    }
}

/// 处理工具市场命令
fn handle_tool_market_command(args: &[String]) -> Result<()> {
    use tool_market::ToolMarket;

    if args.is_empty() {
        println!("🛠️  Tokitai 工具市场");
        println!();
        println!("用法：cargo run -- tokitai <command> [arguments]");
        println!();
        println!("命令:");
        println!("  publish <tool-name>    发布工具到注册表");
        println!("  search <query>         搜索社区工具");
        println!("  install <tool-name>    安装工具");
        println!("  list                   列出现有工具");
        println!();
        println!("示例:");
        println!("  cargo run -- tokitai publish my-tool");
        println!("  cargo run -- tokitai search code-analysis");
        println!("  cargo run -- tokitai install smart-search");
        return Ok(());
    }

    let command = &args[0];

    // 创建工具市场实例
    let market = ToolMarket::new(None)?;

    // 创建 tokio 运行时执行异步操作
    let rt = tokio::runtime::Runtime::new()?;

    match command.as_str() {
        "publish" => {
            if args.len() < 2 {
                eprintln!("❌ 错误：缺少工具名称");
                eprintln!("用法：tokitai publish <tool-name>");
                return Ok(());
            }
            rt.block_on(market.publish(&args[1]))?;
        }
        "search" => {
            if args.len() < 2 {
                eprintln!("❌ 错误：缺少搜索关键词");
                eprintln!("用法：tokitai search <query>");
                return Ok(());
            }
            rt.block_on(market.search(&args[1]))?;
        }
        "install" => {
            if args.len() < 2 {
                eprintln!("❌ 错误：缺少工具名称");
                eprintln!("用法：tokitai install <tool-name>");
                return Ok(());
            }
            rt.block_on(market.install(&args[1]))?;
        }
        "list" => {
            let tools = market.list()?;
            if tools.is_empty() {
                println!("📭 未安装任何工具");
            } else {
                println!("📦 已安装的工具:");
                for tool in tools {
                    println!("   • {}", tool);
                }
            }
        }
        _ => {
            eprintln!("❌ 未知命令：{}", command);
            eprintln!("运行 'cargo run -- tokitai' 查看帮助");
        }
    }

    Ok(())
}
