//! Mock 工具模块
//!
//! 提供 LLM、文件系统、网络等外部依赖的 Mock 实现

use mockall::automock;

/// LLM Client 的 Mock Trait
///
/// 注意：此 Trait 仅用于测试，不关心 auto traits 如 `Send`
#[allow(async_fn_in_trait)]
#[automock]
pub trait LLMClient {
    /// 发送聊天请求
    fn chat(&self, prompt: &str) -> anyhow::Result<String>;

    /// 发送流式聊天请求
    async fn chat_stream(&self, prompt: &str) -> anyhow::Result<String>;

    /// 获取模型信息
    fn get_model_info(&self) -> &str;
}

/// Mock LLM 响应构建器
pub struct MockLLMResponseBuilder {
    status: u16,
    content: String,
    latency_ms: u64,
}

impl MockLLMResponseBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            status: 200,
            content: String::new(),
            latency_ms: 0,
        }
    }

    /// 设置响应状态码
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// 设置响应内容
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    /// 设置模拟延迟（毫秒）
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// 构建成功响应
    pub fn build_success(content: &str) -> String {
        content.to_string()
    }

    /// 构建错误响应
    pub fn build_error(error_code: u16, message: &str) -> anyhow::Error {
        anyhow::anyhow!("API Error {}: {}", error_code, message)
    }
}

impl Default for MockLLMResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建常见的 Mock LLM 响应
pub mod responses {
    /// 创建代码分析响应
    pub fn code_analysis() -> &'static str {
        r#"{
            "summary": "代码结构清晰，模块化良好",
            "issues": [
                {"type": "warning", "message": "函数过长，建议拆分"},
                {"type": "info", "message": "可以考虑添加更多单元测试"}
            ],
            "suggestions": [
                "使用策略模式重构",
                "添加缓存层提升性能"
            ]
        }"#
    }

    /// 创建代码生成响应
    pub fn code_generation() -> &'static str {
        r#"```rust
pub fn example_function(x: i32) -> i32 {
    x * 2
}
```"#
    }

    /// 创建错误修复响应
    pub fn error_fix() -> &'static str {
        r#"```rust
// Fixed version
pub fn safe_divide(a: f64, b: f64) -> Option<f64> {
    if b == 0.0 {
        None
    } else {
        Some(a / b)
    }
}
```"#
    }

    /// 创建测试生成响应
    pub fn test_generation() -> &'static str {
        r#"```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_example() {
        assert_eq!(example_function(2), 4);
    }
}
```"#
    }
}

/// 创建 Mock 工具调用响应
pub mod tool_responses {
    use serde_json::json;

    /// 文件读取成功响应
    pub fn file_read_success(path: &str, content: &str) -> serde_json::Value {
        json!({
            "success": true,
            "path": path,
            "content": content,
            "size": content.len()
        })
    }

    /// 文件写入成功响应
    pub fn file_write_success(path: &str) -> serde_json::Value {
        json!({
            "success": true,
            "path": path,
            "bytes_written": true
        })
    }

    /// Git 操作成功响应
    pub fn git_success(operation: &str, output: &str) -> serde_json::Value {
        json!({
            "success": true,
            "operation": operation,
            "output": output
        })
    }

    /// 网络请求成功响应
    pub fn http_success(status: u16, body: &str) -> serde_json::Value {
        json!({
            "success": true,
            "status": status,
            "body": body
        })
    }

    /// 错误响应
    pub fn error(message: &str, code: &str) -> serde_json::Value {
        json!({
            "success": false,
            "error": {
                "message": message,
                "code": code
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_llm_response_builder() {
        let response = MockLLMResponseBuilder::build_success("test content");
        assert_eq!(response, "test content");
    }

    #[test]
    fn test_mock_llm_response_error() {
        let error = MockLLMResponseBuilder::build_error(400, "Bad Request");
        assert!(error.to_string().contains("400"));
        assert!(error.to_string().contains("Bad Request"));
    }

    #[test]
    fn test_responses_code_analysis() {
        let response = responses::code_analysis();
        assert!(response.contains("summary"));
        assert!(response.contains("issues"));
    }

    #[test]
    fn test_responses_code_generation() {
        let response = responses::code_generation();
        assert!(response.contains("```rust"));
    }

    #[test]
    fn test_tool_responses_file_read() {
        let response = tool_responses::file_read_success("/test.txt", "content");
        assert_eq!(response["success"], true);
        assert_eq!(response["path"], "/test.txt");
    }

    #[test]
    fn test_tool_responses_error() {
        let response = tool_responses::error("Something went wrong", "ERR_001");
        assert_eq!(response["success"], false);
        assert_eq!(response["error"]["code"], "ERR_001");
    }
}
