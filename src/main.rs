#![recursion_limit = "256"]

mod config;
mod command_resolver;
mod path_resolver;
mod sandbox;
mod tools;
mod tui;
mod context;
mod autonomy;
mod observability;
mod dialogue;
mod prompt_engineering;
mod tool_matrix;
mod orchestrator;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use tokitai::ToolProvider;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use tools::{CodeTools, DownloadTools, FileOperations, GitOperations, SystemTools, WebSearchTools, HttpClientTools, JsonTools, FileSearchTools, ProcessTools, NetworkTools, WikipediaTools};
use autonomy::{AgentCoordinator, GitWorkflow};
use orchestrator::Orchestrator;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

/// AI 助手 - 整合所有工具
pub struct AiAssistant {
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: WebSearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    http_client: HttpClientTools,
    json_tools: JsonTools,
    file_search: FileSearchTools,
    process_tools: ProcessTools,
    network_tools: NetworkTools,
    wikipedia_tools: WikipediaTools,
    api_url: String,
    api_key: Option<String>,
    model: String,
    /// 自主进化协调器（可选）
    coordinator: Option<Arc<RwLock<AgentCoordinator>>>,
    /// Git 工作流（用于自主推送）
    git_workflow: Option<GitWorkflow>,
    /// 是否启用自主模式
    autonomous_mode: bool,
    /// 编排器（用于角色切换和上下文优化）
    orchestrator: Orchestrator,
}

impl AiAssistant {
    /// 创建新的 AI 助手（非自主模式）
    pub fn new(api_url: String, api_key: Option<String>, model: String) -> Self {
        Self {
            file_ops: FileOperations,
            system_tools: SystemTools,
            code_tools: CodeTools,
            web_search: WebSearchTools::new(),
            download_tools: DownloadTools,
            git_ops: GitOperations,
            http_client: HttpClientTools::new(),
            json_tools: JsonTools,
            file_search: FileSearchTools,
            process_tools: ProcessTools,
            network_tools: NetworkTools,
            wikipedia_tools: WikipediaTools::new(),
            api_url,
            api_key,
            model,
            coordinator: None,
            git_workflow: None,
            autonomous_mode: false,
            orchestrator: Orchestrator::new(),
        }
    }

    /// 创建自主模式的 AI 助手
    pub fn new_autonomous(
        api_url: String,
        api_key: Option<String>,
        model: String,
        project_root: PathBuf,
    ) -> Result<Self, String> {
        let autonomy_dir = project_root.join(".tokitai").join("autonomy");
        
        // 创建 Agent 协调器
        let coordinator = AgentCoordinator::new(autonomy_dir.clone())
            .map_err(|e| format!("创建 Agent 协调器失败：{}", e))?;
        
        // 创建 Git 工作流
        let git_workflow = GitWorkflow::new(project_root.clone(), autonomy_dir.join("git"))
            .map_err(|e| format!("创建 Git 工作流失败：{}", e))?;

        Ok(Self {
            file_ops: FileOperations,
            system_tools: SystemTools,
            code_tools: CodeTools,
            web_search: WebSearchTools::new(),
            download_tools: DownloadTools,
            git_ops: GitOperations,
            http_client: HttpClientTools::new(),
            json_tools: JsonTools,
            file_search: FileSearchTools,
            process_tools: ProcessTools,
            network_tools: NetworkTools,
            wikipedia_tools: WikipediaTools::new(),
            api_url,
            api_key,
            model,
            coordinator: Some(Arc::new(RwLock::new(coordinator))),
            git_workflow: Some(git_workflow),
            autonomous_mode: true,
            orchestrator: Orchestrator::new(),
        })
    }

    /// 获取所有工具定义（用于发送给 AI）
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        let mut tools = Vec::new();

        // 合并所有工具的 tool_definitions()
        tools.extend(FileOperations::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(SystemTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(CodeTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(WebSearchTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(DownloadTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(GitOperations::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(HttpClientTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(JsonTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(FileSearchTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(ProcessTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(NetworkTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(WikipediaTools::tool_definitions().iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools
    }

    /// 调用工具（带日志）
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        info!("🔧 执行工具：{} {:?}", name, args);

        // 尝试在各个工具集中查找并执行
        // 注意：call_tool 返回 Result<Value, ToolError>，我们需要检查是否找到了工具
        // 如果工具存在但执行失败，ToolError.kind 会是 InternalError 或 ValidationError
        // 如果工具不存在，ToolError.kind 会是 NotFound
        
        use tokitai_core::ToolErrorKind;
        
        macro_rules! try_tool {
            ($tools:expr, $tool_name:expr) => {
                match $tools.call_tool(name, args) {
                    Ok(result) => {
                        info!("✅ 工具执行成功：{}", name);
                        return Ok(result.to_string());
                    }
                    Err(e) => {
                        if e.kind == ToolErrorKind::NotFound {
                            // 工具不存在，继续尝试下一个
                        } else {
                            // 工具存在但执行失败
                            info!("❌ 工具执行失败：{} - {:?}", name, e);
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

        warn!("❌ 未知工具：{}", name);
        Err(anyhow::anyhow!("未知工具：{}", name))
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
        let client = reqwest::blocking::Client::new();

        let tools = self.get_tool_definitions();

        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto"
        });

        // 如果有 API key，添加认证头
        let mut req = client.post(&self.api_url);
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = req
            .json(&request_body)
            .send()
            .context("发送请求失败")?;

        let response_text = response.text().context("读取响应失败")?;
        
        let response_json: Value = serde_json::from_str(&response_text)
            .context("解析响应失败")?;

        // 处理响应
        let choices_opt = response_json.get("choices").and_then(|c: &Value| c.as_array());
        if let Some(choices) = choices_opt {
            let first_opt = choices.first();
            if let Some(first) = first_opt {
                let message_opt = first.get("message");
                if let Some(message) = message_opt {
                    // 检查是否有工具调用
                    let tool_calls_opt = message.get("tool_calls").and_then(|tc: &Value| tc.as_array());
                    if let Some(tool_calls) = tool_calls_opt {
                        return self.handle_tool_calls(tool_calls, messages);
                    }

                    // 普通回复
                    let content_opt = message.get("content").and_then(|c: &Value| c.as_str());
                    if let Some(content) = content_opt {
                        return Ok(content.to_string());
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
    /// 2. 自主规划改进任务
    /// 3. 执行任务（修改代码）
    /// 4. 本地审查（编译、测试、代码审查）
    /// 5. 审查通过后自动推送到 GitHub
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

    /// 执行单次进化迭代
    fn execute_evolution_iteration(
        &self,
        coordinator: &Arc<RwLock<AgentCoordinator>>,
        goal: &str,
    ) -> Result<bool> {
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

    /// 分析项目现状
    fn analyze_project_status(&self) -> Result<String> {
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

    /// 生成改进计划
    fn generate_improvement_plan(&self, goal: &str, analysis: &str) -> Result<String> {
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

    /// 执行改进任务
    fn execute_improvement_tasks(&self, plan: &str) -> Result<String> {
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

    /// 回滚变更
    fn rollback_changes(&self) -> Result<()> {
        self.system_tools.call_tool("run_command", &json!({
            "command": "git checkout -- ."
        }))?;
        Ok(())
    }

    /// 推送到 GitHub
    fn push_to_github(&self) -> Result<bool> {
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

    /// 生成提交消息
    fn generate_commit_message(&self, diff: &str) -> Result<String> {
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
}

fn main() -> Result<()> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let use_tui = args.iter().any(|arg| arg == "--tui" || arg == "-t");
    let use_autonomous = args.iter().any(|arg| arg == "--autonomous" || arg == "-a");
    
    // 解析 --project-path 参数
    let project_path = args.iter()
        .position(|arg| arg == "--project-path" || arg == "-p")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| PathBuf::from(s));

    // 如果指定了 --tui，启动 TUI 界面
    if use_tui {
        info!("🚀 启动 TUI 界面...");
        tui::run_tui().map_err(|e| anyhow::anyhow!("TUI 错误：{}", e))?;
        return Ok(());
    }

    // 初始化 tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("ai_assistant=info".parse().unwrap())
                .add_directive("tokitai=warn".parse().unwrap()),
        )
        .init();

    info!("🚀 AI Assistant 启动中...");

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

    // 检查配置
    if api_key.is_none() {
        println!("⚠️  警告：未设置 AI_API_KEY，某些 API 可能无法使用\n");
    }

    // 如果指定了 --autonomous，启动自主进化模式
    if use_autonomous {
        println!("🤖 AI Assistant powered by Tokitai");
        println!("=====================================");
        println!("🔄 自主进化模式");
        println!("模型：{} (Ollama Cloud)", model);
        println!();

        // 获取项目根目录：优先使用 --project-path 参数，否则使用当前目录
        let project_root = project_path
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map_err(|e| anyhow::anyhow!("获取当前目录失败：{}", e))
                    .unwrap()
            });

        // 切换工作目录到目标项目（确保沙箱隔离生效）
        std::env::set_current_dir(&project_root)
            .map_err(|e| anyhow::anyhow!("切换目录失败：{}", e))?;

        println!("📁 项目路径：{}", project_root.display());
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
    println!("🤖 AI Assistant powered by Tokitai");
    println!("=====================================");
    println!("模型：{} (Ollama Cloud)", config.ai.model);
    println!("按 Ctrl+C 退出\n");

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
        println!("👤 你：{}", input);
        
        match assistant.chat_and_handle_tools(&mut messages, &input) {
            Ok(response) => {
                println!("🤖 AI: {}", response);
            }
            Err(e) => {
                println!("❌ 错误：{}", e);
            }
        }
        return Ok(());
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("👤 你：");
        stdout.flush()?;

        let mut input = String::new();
        let bytes_read = stdin.lock().read_line(&mut input)?;
        
        // 检测 EOF（管道输入结束）
        if bytes_read == 0 {
            println!("\n👋 再见！");
            break;
        }
        
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            println!("\n👋 再见！");
            break;
        }

        // 处理编排器命令（以 / 开头）
        if input.starts_with('/') {
            let processed = assistant.orchestrator.process_input(input);
            if let Some(cmd) = processed.command {
                let result = assistant.orchestrator.execute_command(cmd);
                println!("\n{}\n", result.to_string());
                continue;
            }
        }

        if input == "help" {
            println!("\n📋 我可以帮你做这些事：");
            println!("  • 查看目录：'当前目录有哪些文件'");
            println!("  • 读取文件：'读取 README.md 的内容'");
            println!("  • 写入文件：'创建 test.txt，写入 Hello World'");
            println!("  • 执行命令：'运行 cargo --version'");
            println!("  • 分析代码：'分析 src/main.rs 的结构'");
            println!("  • 统计代码：'统计 main.rs 有多少行代码'");
            println!("  • 复制文件：'复制 README.md 到 backup.md'");
            println!("  • 删除文件：'删除 /tmp/test.txt'");
            println!("  • 环境变量：'查看 PATH 环境变量'");
            println!("  • 下载文件：'下载 https://example.com/file.pdf'");
            println!("  • 下载论文：'从 arXiv 下载论文 2301.00001'");
            println!("  • 搜索论文：'搜索关于 transformer 的 arXiv 论文'");
            println!("  • 查看下载目录：'我的下载目录在哪里'");
            println!("  • Git 状态：'查看 git 状态'");
            println!("  • Git 日志：'查看最近的提交记录'");
            println!("  • Git 分支：'查看当前分支'");
            println!();
            println!("  🔥 新增功能：");
            println!("  • HTTP 请求：'GET 请求 https://api.github.com'");
            println!("  • POST 请求：'POST 数据到 https://api.example.com'");
            println!("  • JSON 处理：'格式化这段 JSON'、'查询 JSON 中的 user.name'");
            println!("  • 文件搜索：'在 src 目录搜索 .rs 文件'、'查找大文件'");
            println!("  • 进程管理：'查看系统资源'、'列出占用 CPU 最高的进程'");
            println!("  • 网络工具：'ping github.com'、'扫描 localhost 的开放端口'");
            println!();
            println!("  💡 使用 @ 快速引用文件");
            println!("    示例：'@README.md 的内容是什么'");
            println!("           '分析 @src/main.rs 的结构'");
            println!("           '@file1.txt @file2.txt 比较这两个文件'");
            println!();
            println!("  🚀 启动方式：");
            println!("    • 交互模式：cargo run --release");
            println!("    • TUI 模式：cargo run --release -- --tui");
            println!("    • 自主进化：cargo run --release -- --autonomous");
            println!("    • 指定项目：cargo run --release -- --autonomous --project-path ./sandbox/test-project");
            println!();
            println!("  🎭 编排器命令（新增）：");
            println!("    • /role <name> - 切换角色（planner/executor/reviewer/researcher）");
            println!("    • /optimize - 优化上下文，减少 token 使用");
            println!("    • /context - 显示上下文状态");
            println!("    • /roles - 显示角色信息");
            println!("    • /workflow list - 列出可用工作流");
            println!("    • /workflow start <name> - 启动工作流");
            println!("    • /help - 显示所有命令");
            println!();
            continue;
        }

        // 处理 @path 语法
        let (processed_input, file_contents) = match path_resolver::resolve_paths(input) {
            Ok(result) => result,
            Err(e) => {
                println!("\n❌ 路径解析错误：{}\n", e);
                continue;
            }
        };

        // 如果解析到了文件内容，给出提示
        if !file_contents.is_empty() {
            println!("📎 已加载 {} 个文件内容", file_contents.len());
        }

        // 添加用户消息（使用处理后的输入）
        messages.push(json!({
            "role": "user",
            "content": processed_input
        }));

        // 使用编排器处理输入（角色切换 + 上下文管理）
        let processed = assistant.orchestrator.process_input(&processed_input);
        
        // 如果有角色切换，给出提示
        if processed.role_changed && assistant.orchestrator.config.verbose {
            println!("🎭 切换到角色：{}", processed.current_role.as_str());
        }

        println!("\n🤖 AI 思考中...");

        match assistant.chat(&mut messages) {
            Ok(response) => {
                println!("\n🤖 AI: {}\n", response);

                // 使用编排器处理响应（上下文管理）
                assistant.orchestrator.process_response(&response);

                // 添加 AI 回复到消息历史
                messages.push(json!({
                    "role": "assistant",
                    "content": response
                }));
            }
            Err(e) => {
                println!("\n❌ 错误：{}\n", e);
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
    use tools::{FileOperations, CodeTools, SystemTools, WebSearchTools};

    #[test]
    fn test_file_operations_read_write() {
        let file_ops = FileOperations;
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
        let file_ops = FileOperations;
        
        // 测试列出当前目录
        let result = file_ops.call_tool("list_dir", &json!({
            "path": "."
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_code_tools_detect_language() {
        let code_tools = CodeTools;
        
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
        let system_tools = SystemTools;
        
        let result = system_tools.call_tool("get_current_dir", &json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definitions_generation() {
        // 确保所有工具都有工具定义
        assert!(!FileOperations::tool_definitions().is_empty());
        assert!(!CodeTools::tool_definitions().is_empty());
        assert!(!SystemTools::tool_definitions().is_empty());
        assert!(!WebSearchTools::tool_definitions().is_empty());
        assert!(!DownloadTools::tool_definitions().is_empty());

        // 验证工具定义格式
        for def in FileOperations::tool_definitions().iter() {
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
            assert!(!def.input_schema.is_empty());
        }
    }
}
