use reqwest::blocking::Client;
use serde_json::json;

fn test_endpoint(url: &str, api_key: &str) -> bool {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    
    let messages = json!({
        "model": "qwen2.5:latest",
        "messages": [
            {"role": "user", "content": "hi"}
        ]
    });
    
    let response = client.post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&messages)
        .send();
    
    match response {
        Ok(r) => {
            let status = r.status();
            if status.is_success() {
                println!("✅ 成功：{}", url);
                if let Ok(result) = r.json::<serde_json::Value>() {
                    if let Some(choices) = result.get("choices").and_then(|c| c.as_array()) {
                        if let Some(first) = choices.first() {
                            if let Some(content) = first.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                println!("   AI: {}\n", content);
                            }
                        }
                    }
                }
                true
            } else {
                println!("❌ 失败 ({}): {}", url, status);
                false
            }
        }
        Err(e) => {
            println!("❌ 失败 ({}): {}", url, e);
            false
        }
    }
}

fn main() {
    let api_key = "645c36802a434774b0ff2101596e1c2d.Re7mAsiOwiRTGx6UNNk1sv_M";
    
    println!("🧪 测试可能的 Ollama 云端 API 端点...\n");
    
    // 常见的 Ollama 兼容 API 端点
    let endpoints = [
        "https://api.olui.ai/v1/chat/completions",
        "https://ollama.ai/v1/chat/completions",
        "https://api.openai.com/v1/chat/completions",
        "https://api.deepseek.com/v1/chat/completions",
        "https://api.siliconflow.cn/v1/chat/completions",
        "https://api.together.xyz/v1/chat/completions",
    ];
    
    for endpoint in &endpoints {
        test_endpoint(endpoint, api_key);
    }
    
    println!("\n💡 提示：请确认你使用的 Ollama 云端服务提供方和正确的 API 端点");
    println!("   常见的 Ollama 兼容服务包括:");
    println!("   - Ollama 官方云 (如果有)");
    println!("   - SiliconFlow (硅基流动)");
    println!("   - 其他第三方服务");
}
