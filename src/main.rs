mod config;
mod sandbox;
mod tools;
mod tui;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use tools::{CodeTools, DownloadTools, FileOperations, GitOperations, SystemTools, WebSearchTools};

/// AI 助手 - 整合所有工具
pub struct AiAssistant {
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: WebSearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    api_url: String,
    api_key: Option<String>,
    model: String,
}

impl AiAssistant {
    pub fn new(api_url: String, api_key: Option<String>, model: String) -> Self {
        Self {
            file_ops: FileOperations,
            system_tools: SystemTools,
            code_tools: CodeTools,
            web_search: WebSearchTools::new(),
            download_tools: DownloadTools,
            git_ops: GitOperations,
            api_url,
            api_key,
            model,
        }
    }

    /// 获取所有工具定义（用于发送给 AI）
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        let mut tools = Vec::new();

        // 合并所有工具的 TOOl_DEFINITIONS
        tools.extend(FileOperations::TOOL_DEFINITIONS.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(SystemTools::TOOL_DEFINITIONS.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(CodeTools::TOOL_DEFINITIONS.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(WebSearchTools::TOOL_DEFINITIONS.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(DownloadTools::TOOL_DEFINITIONS.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema).unwrap_or_default()
                }
            })
        }));

        tools.extend(GitOperations::TOOL_DEFINITIONS.iter().map(|t| {
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
        if let Ok(result) = self.file_ops.call_tool(name, args) {
            info!("✅ 工具执行成功：{}", name);
            return Ok(result.to_string());
        }
        if let Ok(result) = self.system_tools.call_tool(name, args) {
            info!("✅ 工具执行成功：{}", name);
            return Ok(result.to_string());
        }
        if let Ok(result) = self.code_tools.call_tool(name, args) {
            info!("✅ 工具执行成功：{}", name);
            return Ok(result.to_string());
        }
        if let Ok(result) = self.web_search.call_tool(name, args) {
            info!("✅ 工具执行成功：{}", name);
            return Ok(result.to_string());
        }
        if let Ok(result) = self.download_tools.call_tool(name, args) {
            info!("✅ 工具执行成功：{}", name);
            return Ok(result.to_string());
        }
        if let Ok(result) = self.git_ops.call_tool(name, args) {
            info!("✅ 工具执行成功：{}", name);
            return Ok(result.to_string());
        }

        warn!("❌ 未知工具：{}", name);
        Err(anyhow::anyhow!("未知工具：{}", name))
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
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first) = choices.first() {
                if let Some(message) = first.get("message") {
                    // 检查是否有工具调用
                    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
                        return self.handle_tool_calls(tool_calls, messages);
                    }

                    // 普通回复
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
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
}

fn main() -> Result<()> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let use_tui = args.iter().any(|arg| arg == "--tui" || arg == "-t");

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

    println!("🤖 AI Assistant powered by Tokitai");
    println!("=====================================");
    println!("模型：{} (Ollama Cloud)", config.ai.model);
    println!("按 Ctrl+C 退出\n");

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

    let assistant = AiAssistant::new(api_url, api_key, model);
    let mut messages: Vec<Value> = vec![json!({
        "role": "system",
        "content": "你是一个强大的 AI 助手，可以调用各种工具来帮助用户完成任务。你可以：
- 读取和写入文件
- 执行系统命令
- 分析代码
- 搜索网络信息

请根据用户需求选择合适的工具。

当用户输入'help'时，请列出你可以执行的操作示例。"
    })];

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
            continue;
        }

        // 添加用户消息
        messages.push(json!({
            "role": "user",
            "content": input
        }));

        println!("\n🤖 AI 思考中...");

        match assistant.chat(&mut messages) {
            Ok(response) => {
                println!("\n🤖 AI: {}\n", response);

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
        assert!(!FileOperations::TOOL_DEFINITIONS.is_empty());
        assert!(!CodeTools::TOOL_DEFINITIONS.is_empty());
        assert!(!SystemTools::TOOL_DEFINITIONS.is_empty());
        assert!(!WebSearchTools::TOOL_DEFINITIONS.is_empty());
        assert!(!DownloadTools::TOOL_DEFINITIONS.is_empty());

        // 验证工具定义格式
        for def in FileOperations::TOOL_DEFINITIONS.iter() {
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
            assert!(!def.input_schema.is_empty());
        }
    }
}
