use reqwest::blocking::Client;
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = std::env::var("AI_API_URL")
        .unwrap_or_else(|_| "https://ollama.com/v1/chat/completions".to_string());
    let api_key = std::env::var("AI_API_KEY")
        .expect("⚠️  未设置环境变量 AI_API_KEY，请创建 .env 文件或手动设置");
    
    let client = Client::new();
    
    println!("🧪 测试完整对话流程...\n");
    
    // 定义工具
    let tools = json!([
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "列出目录内容",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "目录路径"}
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "读取文件内容",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径"}
                    },
                    "required": ["path"]
                }
            }
        }
    ]);
    
    // 第一轮：用户请求
    let mut messages = vec![json!({
        "role": "user",
        "content": "查看当前目录下有哪些文件"
    })];
    
    println!("👤 用户：查看当前目录下有哪些文件\n");
    
    // 第一次调用
    println!("🤖 调用 AI...");
    let request_body = json!({
        "model": "qwen3.5:397b",
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto"
    });
    
    let response = client.post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()?;
    
    let result: Value = response.json()?;
    println!("响应：{}\n", serde_json::to_string_pretty(&result)?);
    
    if let Some(choices) = result.get("choices").and_then(|c| c.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(message) = first.get("message") {
                // 检查是否有工具调用
                if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
                    println!("🔧 检测到工具调用，数量：{}\n", tool_calls.len());
                    
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
                        
                        let tool_call_id = tool_call.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        
                        println!("工具名：{}", name);
                        println!("参数：{}", arguments);
                        println!("tool_call_id: {}", tool_call_id);
                        
                        // 模拟工具执行
                        let tool_result = if name == "list_dir" {
                            "Cargo.toml\nCargo.lock\nsrc/\nREADME.md\ntarget/".to_string()
                        } else {
                            "未知工具".to_string()
                        };
                        
                        println!("工具执行结果：{}\n", tool_result);
                        
                        // 添加工具结果到消息
                        messages.push(json!({
                            "role": "assistant",
                            "content": "",
                            "tool_calls": [tool_call]
                        }));
                        
                        messages.push(json!({
                            "role": "tool",
                            "content": tool_result,
                            "tool_call_id": tool_call_id
                        }));
                    }
                    
                    // 第二次调用 AI 获取最终回复
                    println!("🤖 再次调用 AI 获取最终回复...");
                    let request_body = json!({
                        "model": "qwen3.5:397b",
                        "messages": messages,
                        "tools": tools,
                        "tool_choice": "auto"
                    });
                    
                    let response = client.post(api_url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("Content-Type", "application/json")
                        .json(&request_body)
                        .send()?;
                    
                    let result: Value = response.json()?;
                    println!("最终响应：{}\n", serde_json::to_string_pretty(&result)?);
                    
                    if let Some(choices) = result.get("choices").and_then(|c| c.as_array()) {
                        if let Some(first) = choices.first() {
                            if let Some(content) = first.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                println!("\n🤖 AI 最终回复：{}\n", content);
                            }
                        }
                    }
                } else if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                    println!("\n🤖 AI 回复：{}\n", content);
                }
            }
        }
    }
    
    println!("✅ 测试完成！");
    
    Ok(())
}
