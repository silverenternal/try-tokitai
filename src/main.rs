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
mod external_process;
mod assistant_common;
mod cli_assistant;
mod autonomous_assistant;

use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use assistant_common::AssistantConfig;
use cli_assistant::CliAssistant;
use autonomous_assistant::AutonomousAssistant;

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

    // 解析 --project-path 参数
    let project_path = args.iter()
        .position(|arg| arg == "--project-path" || arg == "-p")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| PathBuf::from(s));

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

    // 创建助手配置
    let config = AssistantConfig::new(api_url, api_key, model);

    // 如果指定了 --autonomous，启动自主进化模式
    if use_autonomous {
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

        // 切换工作目录到目标项目
        std::env::set_current_dir(&project_root)
            .map_err(|e| anyhow::anyhow!("切换目录失败：{}", e))?;

        println!("📂 工作目录：{}", std::env::current_dir().unwrap().display());
        println!();

        // 创建自主助手
        let assistant = AutonomousAssistant::new(
            config,
            std::env::current_dir().unwrap(),
        ).map_err(|e| anyhow::anyhow!("创建自主模式失败：{}", e))?;

        // 运行自主进化
        assistant.run_autonomous_evolution()?;

        return Ok(());
    }

    // 普通交互模式
    let mut assistant = CliAssistant::new(config)?;

    // 检查是否有命令行参数直接输入
    let non_arg_args: Vec<String> = args.iter()
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
    use crate::tools::{FileOperations, CodeTools, SystemTools, SearchTools, DownloadTools};
    use tokitai::ToolProvider;

    #[test]
    fn test_file_operations_read_write() {
        let file_ops = FileOperations::default();
        let test_path = "/tmp/test_tokitai.txt";
        let test_content = "Hello, Tokitai!";

        let write_result = file_ops.call_tool("write_file", &serde_json::json!({
            "path": test_path,
            "content": test_content
        }));
        assert!(write_result.is_ok());

        let read_result = file_ops.call_tool("read_file", &serde_json::json!({
            "path": test_path
        }));
        assert!(read_result.is_ok());
        assert!(read_result.unwrap().to_string().contains(test_content));

        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_file_operations_list_dir() {
        let file_ops = FileOperations::default();

        let result = file_ops.call_tool("list_dir", &serde_json::json!({
            "path": "."
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_code_tools_detect_language() {
        let code_tools = CodeTools::default();

        let result = code_tools.call_tool("detect_language", &serde_json::json!({
            "path": "src/main.rs"
        }));
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
