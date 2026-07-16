#![recursion_limit = "256"]

mod agent_skills;
mod app_paths;
mod command_resolver;
mod config;
mod desktop_host;
mod domain_prompt;
mod host;
mod image_generation;
mod path_resolver;
mod process_window;
mod project_index;
mod research_domains;
mod research_os;
mod sandbox;
pub mod security;
mod task_queue;
mod text_encoding;
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
pub mod scientist;
pub mod tool_market;
mod tool_matrix;
mod toolchain;
pub mod tui;
mod visualization;
mod web;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use app_paths::AppPaths;
use assistant_common::AssistantConfig;
use autonomous_assistant::AutonomousAssistant;
use cli_assistant::CliAssistant;
use domain_prompt::science_expert_system_prompt;

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

fn main() -> Result<()> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let use_autonomous = args.iter().any(|arg| arg == "--autonomous" || arg == "-a");
    let use_mcp = args.iter().any(|arg| arg == "--mcp" || arg == "-m");
    let use_tui = args.iter().any(|arg| arg == "--tui" || arg == "-t");
    let use_web = args.iter().any(|arg| arg == "--web" || arg == "-w");

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

    let detected_project_root = project_path
        .clone()
        .unwrap_or_else(AppPaths::discover_project_root);

    let _ = std::env::set_current_dir(&detected_project_root);

    // 初始化 tracing（TUI 模式输出到文件，其他模式输出到 stderr）
    if use_tui {
        let app_paths = AppPaths::for_local_dev(detected_project_root.clone());
        let _ = std::fs::create_dir_all(app_paths.state_dir());
        let log_file = std::fs::File::create(app_paths.tui_log_path()).unwrap_or_else(|_| {
            let fallback =
                std::env::temp_dir().join(format!("tokitai_tui_{}.log", std::process::id()));
            std::fs::File::create(fallback).unwrap()
        });
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("ai_assistant=error".parse().unwrap())
                    .add_directive("tokitai=error".parse().unwrap()),
            )
            .with_writer(std::sync::Mutex::new(log_file))
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("ai_assistant=warn".parse().unwrap())
                    .add_directive("tokitai=warn".parse().unwrap()),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    println!("🚀 AI Assistant 启动中...");

    // 加载 .env 文件（如果存在）
    if let Ok(env_content) = std::fs::read_to_string(detected_project_root.join(".env")) {
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
                    || key.starts_with("BRAVE_SEARCH_")
                    || key == "GITHUB_TOKEN"
                    || key == "GH_TOKEN"
                    || key == "GITHUB_API_TOKEN"
                    || key == "GITHUB_PAT"
                    || key == "GITHUB_ACCESS_TOKEN"
                    || key == "GITHUB_API_BASE"
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

    // Quick streaming test (after .env loaded)
    let test_stream = args.iter().any(|arg| arg == "--test-stream");
    if test_stream {
        let rt = tokio::runtime::Runtime::new()?;
        return rt.block_on(test_llm_stream());
    }

    if !has_api_key {
        eprintln!("⚠️  警告：未配置 API Key");
        eprintln!("   在 .env 中设置 AI_API_KEY 或 PROVIDERS");
        eprintln!();
    }

    // 构建安全配置（在 config 被 AssistantConfig 遮蔽之前）
    let security_config = config.security.clone().into_security_config();

    // 创建助手配置
    let config = AssistantConfig::new(api_url, api_key, model);

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

        // 启动 MCP Server（传入安全配置）
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(mcp::start_mcp_mode(&security_config))?;

        return Ok(());
    }

    if use_web {
        println!("?? ??? Web Workspace ???\n");
        let config_file = crate::config::Config::load(None).unwrap_or_default();
        let host = web::WebHostConfig::from_env_or_local_dev(detected_project_root.clone());
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(web::start_web_mode(
                host,
                config.clone(),
                config_file,
                security_config.clone(),
            ))?;
        return Ok(());
    }

    // 如果指定了 --tui，启动 TUI 模式
    if use_tui {
        println!("🚀 启动 Claude Code-style TUI 模式\n");

        // 初始化 LLM 和工具（复用 CliAssistant 的初始化逻辑）
        let assistant = CliAssistant::new(config.clone(), security_config.clone())?;
        let llm_manager = assistant.get_llm_manager();
        let tool_defs = Some(assistant.get_tool_definitions());

        // Build tool executor closure from CliAssistant's call_tool
        // We need to put assistant in a shared container
        let assistant = std::sync::Arc::new(std::sync::Mutex::new(assistant));
        let tool_executor: Arc<
            dyn Fn(&str, &serde_json::Value) -> Result<String, String> + Send + Sync,
        > = Arc::new({
            let a = assistant.clone();
            move |name: &str, args: &serde_json::Value| {
                a.lock()
                    .unwrap()
                    .call_tool(name, args)
                    .map_err(|e| e.to_string())
            }
        });

        if let Some(provider) = llm_manager.current_provider() {
            let provider: Arc<dyn crate::llm::LLMProvider> = Arc::clone(provider);
            tui::run_tui(
                provider,
                tool_defs,
                Some(tool_executor),
                security_config.clone(),
            )?;
        } else {
            anyhow::bail!("No LLM provider configured. Please set up .env with API keys.");
        }

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
        let assistant =
            AutonomousAssistant::new(config, std::env::current_dir().unwrap(), security_config)
                .map_err(|e| anyhow::anyhow!("创建自主模式失败：{}", e))?;

        // 运行自主进化
        assistant.run_autonomous_evolution()?;

        return Ok(());
    }

    // 普通交互模式
    let mut assistant = CliAssistant::new(config, security_config)?;

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
            "content": science_expert_system_prompt()
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

/// Quick test: verify LLM streaming works
async fn test_llm_stream() -> Result<()> {
    use crate::llm::providers::OpenAIProvider;
    use crate::llm::{ChatRequest, LLMProvider, Message};
    use futures::StreamExt;
    use std::sync::Arc;

    println!("=== LLM Streaming Test ===\n");

    let api_url = std::env::var("AI_API_URL")
        .unwrap_or_else(|_| "https://api.deepseek.com/v1/chat/completions".to_string());
    let api_key = std::env::var("AI_API_KEY").unwrap_or_default();
    let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());

    println!("URL: {}", api_url);
    println!("Model: {}", model);
    println!("Key: {}...\n", &api_key[..api_key.len().min(15)]);

    let provider = OpenAIProvider::with_base_url(api_key, api_url, Some(model.clone()));
    let provider: Arc<dyn LLMProvider> = Arc::new(provider);

    let request = ChatRequest {
        model,
        messages: vec![
            Message::system("You are a helpful assistant. Keep responses very short."),
            Message::user("Say hello in exactly one word."),
        ],
        multimodal_content: None,
        temperature: 0.7,
        max_tokens: Some(50),
        top_p: None,
        stop: None,
        stream: true,
        tools: None,
        thinking_mode: None,
        reasoning_effort: None,
    };

    print!("Response: ");
    match provider.chat_stream(request).await {
        Ok(mut stream) => {
            let mut total = String::new();
            let mut chunks = 0;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(c) => {
                        chunks += 1;
                        print!("{}", c.content);
                        total.push_str(&c.content);
                    }
                    Err(e) => {
                        eprintln!("\nSTREAM ERROR at chunk {}: {}", chunks, e);
                        anyhow::bail!("Stream error: {}", e);
                    }
                }
            }
            println!("\n\nTotal: '{}' ({} chunks)", total.trim(), chunks);
            if total.trim().is_empty() {
                anyhow::bail!("Empty response - streaming may not be working");
            }
            println!("OK: Streaming works!");
        }
        Err(e) => {
            eprintln!("FAILED to start stream: {:#}", e);
            anyhow::bail!("Stream start failed: {}", e);
        }
    }
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
