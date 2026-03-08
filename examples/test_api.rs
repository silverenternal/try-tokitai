use reqwest::blocking::Client;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = std::env::var("AI_API_URL")
        .unwrap_or_else(|_| "https://ollama.com/v1/chat/completions".to_string());
    let api_key = std::env::var("AI_API_KEY")
        .expect("⚠️  未设置环境变量 AI_API_KEY，请创建 .env 文件或手动设置");
    
    let client = Client::new();
    
    println!("🧪 测试云端 Ollama API 连接...\n");
    println!("API URL: {}", api_url);
    println!();
    
    // 测试 1: 简单对话
    println!("📝 测试 1: 简单对话");
    let messages = json!({
        "model": "qwen3.5:397b",
        "messages": [
            {"role": "user", "content": "你好，请用一句话介绍你自己"}
        ]
    });
    
    let response = client.post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&messages)
        .send()?;
    
    let status = response.status();
    println!("响应状态：{}", status);
    
    if status.is_success() {
        let result: serde_json::Value = response.json()?;
        println!("✅ API 连接成功！\n");
        println!("响应内容：");
        if let Some(choices) = result.get("choices").and_then(|c| c.as_array()) {
            if let Some(first) = choices.first() {
                if let Some(content) = first.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                    println!("🤖 AI: {}\n", content);
                }
            }
        }
    } else {
        let error_text = response.text()?;
        println!("❌ API 请求失败：{}\n", error_text);
        return Ok(());
    }
    
    // 测试 2: 带工具定义的对话
    println!("📝 测试 2: 带工具定义的对话");
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
        }
    ]);
    
    let messages = json!({
        "model": "qwen3.5:397b",
        "messages": [
            {"role": "user", "content": "查看当前目录下有哪些文件"}
        ],
        "tools": tools,
        "tool_choice": "auto"
    });
    
    let response = client.post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&messages)
        .send()?;
    
    let status = response.status();
    println!("响应状态：{}", status);
    
    if status.is_success() {
        let result: serde_json::Value = response.json()?;
        println!("✅ 工具调用测试成功！\n");
        println!("完整响应：");
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let error_text = response.text()?;
        println!("❌ 工具调用测试失败：{}\n", error_text);
    }
    
    println!("\n✅ 所有测试完成！");
    
    Ok(())
}
