//! Mock HTTP 服务器用于测试
//!
//! 提供本地测试服务器，模拟各种 HTTP 响应场景

use std::net::TcpListener;
use std::thread;
use std::time::Duration;

/// Mock 服务器配置
#[derive(Debug, Clone)]
pub struct MockServerConfig {
    pub port: u16,
    pub response_delay_ms: u64,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            port: 0, // 0 表示使用随机可用端口
            response_delay_ms: 0,
        }
    }
}

/// Mock HTTP 服务器
pub struct MockServer {
    pub base_url: String,
    pub port: u16,
    shutdown: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl MockServer {
    /// 创建并启动 mock 服务器
    pub fn new() -> Self {
        Self::with_config(MockServerConfig::default())
    }

    /// 创建带配置的 mock 服务器
    pub fn with_config(config: MockServerConfig) -> Self {
        let port = if config.port == 0 {
            get_available_port()
        } else {
            config.port
        };

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .expect("无法绑定端口");

        let actual_port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", actual_port);

        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // 启动服务器线程
        thread::spawn(move || {
            listener.set_nonblocking(true).ok();

            for stream in listener.incoming() {
                if shutdown_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                if let Ok(mut stream) = stream {
                    // 读取请求
                    let mut buffer = [0; 1024];
                    let _ = stream.read(&mut buffer);

                    // 发送简单响应
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
                    let _ = stream.write_all(response.as_bytes());

                    // 模拟延迟
                    if config.response_delay_ms > 0 {
                        thread::sleep(Duration::from_millis(config.response_delay_ms));
                    }
                }
            }
        });

        // 等待服务器启动
        thread::sleep(Duration::from_millis(100));

        Self {
            base_url,
            port: actual_port,
            shutdown: Some(shutdown),
        }
    }

    /// 创建带自定义处理函数的服务器
    pub fn with_handler<F>(port: u16, handler: F) -> Self
    where
        F: Fn(&str) -> String + Send + 'static,
    {
        let actual_port = if port == 0 {
            get_available_port()
        } else {
            port
        };

        let listener = TcpListener::bind(format!("127.0.0.1:{}", actual_port))
            .expect("无法绑定端口");

        let base_url = format!("http://127.0.0.1:{}", actual_port);
        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        thread::spawn(move || {
            listener.set_nonblocking(true).ok();

            for stream in listener.incoming() {
                if shutdown_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                if let Ok(mut stream) = stream {
                    let mut buffer = [0; 4096];
                    let n = stream.read(&mut buffer).unwrap_or(0);

                    let request = String::from_utf8_lossy(&buffer[..n]);
                    let response_body = handler(&request);

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            }
        });

        thread::sleep(Duration::from_millis(100));

        Self {
            base_url,
            port: actual_port,
            shutdown: Some(shutdown),
        }
    }

    /// 获取测试 URL
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// 关闭服务器
    pub fn shutdown(&mut self) {
        if let Some(shutdown) = &self.shutdown {
            shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

impl Default for MockServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 获取一个可用的端口
fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("无法绑定端口")
        .local_addr()
        .unwrap()
        .port()
}

/// 创建 mock 响应
pub fn mock_response(status: u16, body: &str) -> String {
    let status_text = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    )
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn test_mock_server_creation() {
        let server = MockServer::new();
        assert!(server.port > 0);
        assert!(server.base_url.starts_with("http://127.0.0.1:"));
    }

    #[test]
    fn test_mock_server_with_custom_port() {
        let port = get_available_port();
        let server = MockServer::with_config(MockServerConfig {
            port,
            response_delay_ms: 0,
        });
        assert_eq!(server.port, port);
    }

    #[test]
    fn test_mock_server_with_handler() {
        let server = MockServer::with_handler(0, |_request| {
            r#"{"status": "ok", "message": "test"}"#.to_string()
        });

        // 验证服务器响应
        let response = reqwest::blocking::get(&server.base_url).unwrap();
        assert!(response.status().is_success());

        let json: serde_json::Value = response.json().unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["message"], "test");
    }

    #[test]
    fn test_mock_server_url() {
        let server = MockServer::new();
        let test_url = server.url("/api/test");
        assert!(test_url.contains("/api/test"));
    }

    #[test]
    fn test_mock_server_shutdown() {
        let mut server = MockServer::new();
        server.shutdown();
        // 服务器应该停止响应
        let result = reqwest::blocking::get(&server.base_url);
        // 连接应该失败
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_response_helper() {
        let response = mock_response(200, r#"{"key": "value"}"#);
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains(r#"{"key": "value"}"#));
    }
}
