//! TUI 版本的 AI 助手 - 整合 tokitai 工具调用功能

use crate::command_resolver::CommandResolver;
use crate::tools::{
    CodeTools, DownloadTools, FileOperations, GitOperations, SystemTools, WebSearchTools,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use tokitai::ToolProvider;
use tracing::{info, warn};

use super::api_client::{ApiConfig, StreamEvent};
use super::app::TuiError;

/// 全局线程池（工作线程数 4）
static ASSISTANT_THREAD_POOL: Lazy<ThreadPool> =
    Lazy::new(|| ThreadPool::with_name("assistant-worker".to_string(), 4));

use once_cell::sync::Lazy;

/// AI 助手 - 整合所有工具（TUI 版本）
pub struct Assistant {
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: WebSearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    api_config: ApiConfig,
    /// 命令解析器（用于安全检查）
    command_resolver: Arc<Mutex<CommandResolver>>,
}

impl Assistant {
    pub fn new(api_config: ApiConfig) -> Self {
        Self {
            file_ops: FileOperations,
            system_tools: SystemTools,
            code_tools: CodeTools,
            web_search: WebSearchTools::new(),
            download_tools: DownloadTools,
            git_ops: GitOperations,
            api_config,
            command_resolver: Arc::new(Mutex::new(CommandResolver::new())),
        }
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

        tools
    }

    /// 调用工具
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String, TuiError> {
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
        Err(TuiError::ApiRequest(format!("未知工具：{}", name)))
    }

    /// 同步对话（带工具调用支持）
    pub fn chat_sync(&self, messages: &[Value]) -> Result<String, TuiError> {
        let client = reqwest::blocking::Client::new();
        let tools = self.get_tool_definitions();

        let request_body = json!({
            "model": self.api_config.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto"
        });

        let mut req = client.post(&self.api_config.api_url);
        if let Some(key) = &self.api_config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req
            .json(&request_body)
            .send()
            .map_err(|e| TuiError::ApiRequest(format!("发送请求失败：{}", e)))?;

        let response_text = response
            .text()
            .map_err(|e| TuiError::ApiRequest(format!("读取响应失败：{}", e)))?;

        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| TuiError::ApiRequest(format!("解析响应失败：{}", e)))?;

        // 处理响应
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first) = choices.first() {
                if let Some(message) = first.get("message") {
                    // 检查是否有工具调用
                    if let Some(tool_calls) =
                        message.get("tool_calls").and_then(|tc| tc.as_array())
                    {
                        return self
                            .handle_tool_calls_sync(tool_calls, messages)
                            .map_err(|e| TuiError::ApiRequest(e.to_string()));
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

    /// 处理工具调用（同步版本）
    fn handle_tool_calls_sync(
        &self,
        tool_calls: &[Value],
        original_messages: &[Value],
    ) -> Result<String, TuiError> {
        let mut extended_messages: Vec<Value> = original_messages.to_vec();

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

            let args: Value =
                serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));

            println!("🔧 执行工具：{}", name);

            match self.call_tool(name, &args) {
                Ok(result) => {
                    println!("✅ 工具执行成功");
                    // 先添加 assistant 的 tool_calls 消息
                    extended_messages.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    // 再添加 tool 的响应消息
                    extended_messages.push(json!({
                        "role": "tool",
                        "content": result,
                        "tool_call_id": tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("")
                    }));
                }
                Err(e) => {
                    println!("❌ 工具执行失败：{}", e);
                    extended_messages.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    extended_messages.push(json!({
                        "role": "tool",
                        "content": format!("错误：{}", e),
                        "tool_call_id": tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("")
                    }));
                }
            }
        }

        // 再次调用 AI 获取最终回复
        self.chat_sync(&extended_messages)
    }

    /// 流式对话（带工具调用支持）
    pub fn chat_stream(
        &self,
        messages: &[Value],
        tx: std::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<(), TuiError> {
        // 在线程池中执行异步流式请求，避免阻塞主线程
        let assistant = Self::new(self.api_config.clone());
        let messages = messages.to_vec();
        let tx_clone = tx.clone();

        ASSISTANT_THREAD_POOL.execute(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(format!("创建 runtime 失败：{}", e)));
                    return;
                }
            };

            let result = rt.block_on(async {
                assistant.chat_stream_async(&messages, tx).await
            });

            if let Err(e) = result {
                let _ = tx_clone.send(StreamEvent::Error(e.to_string()));
            }
        });

        Ok(())
    }

    /// 内部异步流式对话
    async fn chat_stream_async(
        &self,
        messages: &[Value],
        tx: std::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<(), TuiError> {
        let client = reqwest::Client::new();
        let tools = self.get_tool_definitions();

        let request_body = json!({
            "model": self.api_config.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "stream": true
        });

        let mut req = client.post(&self.api_config.api_url);
        if let Some(key) = &self.api_config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req
            .json(&request_body)
            .send()
            .await
            .map_err(|e| TuiError::ApiRequest(format!("发送请求失败：{}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            return Err(TuiError::ApiRequest(format!("HTTP 错误：{}", error_text)));
        }

        // 读取 SSE 流
        use futures::StreamExt;
        let mut stream = response.bytes_stream();

        let mut tool_calls: Vec<Value> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                TuiError::ApiRequest(format!("读取流失败：{}", e))
            })?;
            let text = String::from_utf8_lossy(&chunk);

            // 解析 SSE 格式：data: {...}
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = line[6..].trim();
                    if data == "[DONE]" {
                        // 检查是否有工具调用需要处理
                        if !tool_calls.is_empty() {
                            // 递归调用处理工具（包含最终回复和 Done 事件）
                            let _ = self
                                .handle_stream_tool_calls_and_done(messages, &tool_calls, tx.clone())
                                .await;
                        } else {
                            // 没有工具调用，正常发送 Done
                            let _ = tx.send(StreamEvent::Done);
                        }
                        return Ok(());
                    }

                    // 尝试解析 JSON
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(first) = choices.first() {
                                if let Some(delta) = first.get("delta") {
                                    // 检查是否有工具调用
                                    if let Some(tc) = delta.get("tool_calls") {
                                        if let Some(tc_array) = tc.as_array() {
                                            tool_calls.extend(tc_array.clone());
                                        }
                                    }

                                    // 普通文本内容
                                    if let Some(content) =
                                        delta.get("content").and_then(|c| c.as_str())
                                    {
                                        if !content.is_empty() {
                                            if tx.send(StreamEvent::Text(content.to_string())).is_err()
                                            {
                                                return Ok(());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 处理流式中的工具调用（完整版：执行工具后再次调用 AI 获取人类可读回复，最后发送 Done）
    async fn handle_stream_tool_calls_and_done(
        &self,
        original_messages: &[Value],
        tool_calls: &[Value],
        tx: std::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<(), TuiError> {
        // 构建扩展消息：原始消息 + 工具调用和结果
        let mut extended_messages: Vec<Value> = original_messages.to_vec();

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

            let args: Value =
                serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));

            let _ = tx.send(StreamEvent::Text(format!("\n🔧 执行工具：{}...\n", name)));

            match self.call_tool(name, &args) {
                Ok(result) => {
                    let _ = tx.send(StreamEvent::Text(format!(
                        "✅ 工具执行成功\n"
                    )));
                    // 添加 assistant 的 tool_calls 消息
                    extended_messages.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    // 添加 tool 的响应消息
                    extended_messages.push(json!({
                        "role": "tool",
                        "content": result,
                        "tool_call_id": tool_call
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("")
                    }));
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Text(format!(
                        "❌ 工具执行失败：{}\n",
                        e
                    )));
                    extended_messages.push(json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tool_call]
                    }));
                    extended_messages.push(json!({
                        "role": "tool",
                        "content": format!("错误：{}", e),
                        "tool_call_id": tool_call
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("")
                    }));
                }
            }
        }

        // 再次调用 AI 获取最终的人类可读回复
        let _ = tx.send(StreamEvent::Text("\n🤔 思考中...\n".to_string()));

        // 使用同步调用获取最终回复（避免递归流式调用）
        match self.chat_sync_internal(&extended_messages).await {
            Ok(content) => {
                // 添加换行分隔
                let _ = tx.send(StreamEvent::Text(format!("\n{}", content)));
            }
            Err(e) => {
                let _ = tx.send(StreamEvent::Text(format!(
                    "\n获取最终回复失败：{}\n",
                    e
                )));
            }
        }

        // 发送 Done 事件，完成流式响应
        let _ = tx.send(StreamEvent::Done);

        Ok(())
    }

    /// 内部同步对话（不带工具调用，用于获取最终回复）
    async fn chat_sync_internal(&self, messages: &[Value]) -> Result<String, TuiError> {
        let client = reqwest::Client::new();

        let request_body = json!({
            "model": self.api_config.model,
            "messages": messages,
        });

        let mut req = client.post(&self.api_config.api_url);
        if let Some(key) = &self.api_config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req
            .json(&request_body)
            .send()
            .await
            .map_err(|e| TuiError::ApiRequest(format!("发送请求失败：{}", e)))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| TuiError::ApiRequest(format!("读取响应失败：{}", e)))?;

        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| TuiError::ApiRequest(format!("解析响应失败：{}", e)))?;

        // 处理响应
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first) = choices.first() {
                if let Some(message) = first.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        return Ok(content.to_string());
                    }
                }
            }
        }

        Ok(format!("AI 响应格式异常：{}", response_json))
    }
}
