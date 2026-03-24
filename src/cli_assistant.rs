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
use serde_json::{json, Value};
use std::io::{self, Write};
use tracing::{info, warn};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
};

use crate::assistant_common::{AssistantConfig, ToolManager, register_all_builtin_tools};
use crate::orchestrator::Orchestrator;
use crate::integration::IntegratedModules;
use crate::integration::IntegratedModulesConfig;
use crate::path_resolver;
use crate::tools::{FileOperations, SystemTools, CodeTools, SearchTools, DownloadTools, GitOperations, JsonFormatTools};
use crate::tools::HttpClientTools;

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
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: SearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    http_client: HttpClientTools,
    json_tools: JsonFormatTools,
}

impl CliAssistant {
    /// 创建新的 CLI AI 助手
    ///
    /// # 参数
    /// - `config`: 助手配置
    pub fn new(config: AssistantConfig) -> Result<Self> {
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
                eprintln!("⚠️  创建集成模块失败：{}", e);
                IntegratedModules::new(IntegratedModulesConfig::for_testing()).unwrap()
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

        Ok(Self {
            config,
            tool_manager,
            orchestrator: Orchestrator::new(),
            integrated_modules,
            file_ops: FileOperations::default(),
            system_tools: SystemTools::default(),
            code_tools: CodeTools::default(),
            web_search: SearchTools::new(),
            download_tools: DownloadTools::new(),
            git_ops: GitOperations,
            http_client: HttpClientTools::new(),
            json_tools: JsonFormatTools::default(),
        })
    }

    /// 获取所有工具定义
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        self.tool_manager.get_all_tools()
    }

    /// 获取工具箱统计信息
    pub fn get_toolbox_stats(&self) -> Value {
        self.tool_manager.get_toolbox_stats()
    }

    /// 调用工具
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        info!("🔧 执行工具：{} {:?}", name, args);

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
        try_tool!(self.system_tools);
        try_tool!(self.code_tools);
        try_tool!(self.web_search);
        try_tool!(self.download_tools);
        try_tool!(self.git_ops);
        try_tool!(self.http_client);
        try_tool!(self.json_tools);

        warn!("❌ 未知工具：{}", name);
        Err(anyhow::anyhow!("未知工具：{}", name))
    }

    /// 与 AI 对话并处理工具调用
    pub fn chat_and_handle_tools(&self, messages: &mut Vec<Value>, input: &str) -> Result<String> {
        messages.push(json!({
            "role": "user",
            "content": input
        }));

        self.chat(messages)
    }

    /// 与 AI 对话
    pub fn chat(&self, messages: &mut Vec<Value>) -> Result<String> {
        let tools = self.get_tool_definitions();

        // 从环境变量读取最新配置（支持运行时切换供应商）
        let api_url = std::env::var("AI_API_URL")
            .unwrap_or_else(|_| self.config.api_url.clone());
        let api_key = std::env::var("AI_API_KEY").ok();
        let model = std::env::var("AI_MODEL")
            .unwrap_or_else(|_| self.config.model.clone());

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

        let response = req
            .json(&request_body)
            .send()
            .context("发送请求失败")?;

        let status = response.status();
        info!("📡 响应状态码：{}", status);

        let response_text = response.text().context("读取响应失败")?;
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

        messages.extend(results);
        self.chat(messages)
    }

    /// 运行交互式 CLI
    pub fn run_cli(&mut self) -> Result<()> {
        println!();
        println!("  Tokitai AI Assistant v2.1.0");
        println!();
        println!("  输入 help 查看功能，quit 退出");
        println!();

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
                    println!("\n工具箱状态：\n{}", serde_json::to_string_pretty(&stats).unwrap_or_default());
                    println!();
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

    fn redraw_line(stdout: &mut io::Stdout, prompt: &str, buffer: &str, cursor_char_pos: usize) -> std::io::Result<()> {
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
