//! 测试工具函数
//!
//! 提供通用的测试辅助函数

use std::time::Duration;

/// 测试超时配置
pub const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// 创建测试用的 HTTP 客户端（带短超时）
pub fn create_test_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("创建测试客户端失败")
}

/// 等待条件成立（带超时）
pub fn wait_until<F>(condition: F, timeout: Duration) -> bool
where
    F: Fn() -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// 重试操作（带最大重试次数）
pub fn retry<F, T>(mut operation: F, max_retries: u32) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_error = None;

    for i in 0..max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if i < max_retries - 1 {
                    std::thread::sleep(Duration::from_millis(100 * (i + 1)));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "未知错误".to_string()))
}

/// 跳过某些环境的测试（如 CI）
pub fn skip_in_ci() -> bool {
    std::env::var("CI").unwrap_or_default() == "true"
}

/// 获取测试数据目录
pub fn get_test_data_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(manifest_dir)
        .join("tests")
        .join("data")
}

/// 创建临时测试文件
pub fn create_temp_file(content: &str) -> std::path::PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!(
        "test_{}_{}.tmp",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos()
    ));

    std::fs::write(&file_path, content.as_bytes())
        .expect("创建临时文件失败");

    file_path
}

/// 清理临时测试文件
pub fn cleanup_temp_file(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

/// RAII 风格的临时文件
pub struct TempFile {
    pub path: std::path::PathBuf,
}

impl TempFile {
    pub fn new(content: &str) -> Self {
        let path = create_temp_file(content);
        Self { path }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn content(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(&self.path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        cleanup_temp_file(&self.path);
    }
}

/// 断言结果类型
#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {
        assert!($result.is_ok(), "期望 Ok，得到 {:?}", $result);
    };
    ($result:expr, $msg:expr) => {
        assert!($result.is_ok(), "期望 Ok，得到 {:?} - {}", $result, $msg);
    };
}

#[macro_export]
macro_rules! assert_err {
    ($result:expr) => {
        assert!($result.is_err(), "期望 Err，得到 {:?}", $result);
    };
    ($result:expr, $msg:expr) => {
        assert!($result.is_err(), "期望 Err，得到 {:?} - {}", $result, $msg);
    };
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_until_success() {
        let mut counter = 0;
        let result = wait_until(
            || {
                counter += 1;
                counter >= 3
            },
            Duration::from_secs(1),
        );
        assert!(result);
        assert_eq!(counter, 3);
    }

    #[test]
    fn test_wait_until_timeout() {
        let result = wait_until(
            || false,
            Duration::from_millis(100),
        );
        assert!(!result);
    }

    #[test]
    fn test_retry_success() {
        let mut attempts = 0;
        let result: Result<(), String> = retry(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err("失败".to_string())
                } else {
                    Ok(())
                }
            },
            5,
        );
        assert!(result.is_ok());
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_retry_failure() {
        let result: Result<(), String> = retry(
            || Err("永远失败".to_string()),
            3,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_temp_file() {
        let temp = TempFile::new("test content");
        assert!(temp.path().exists());
        assert_eq!(temp.content().unwrap(), "test content");
    }

    #[test]
    fn test_temp_file_cleanup() {
        let path = {
            let temp = TempFile::new("test");
            temp.path().clone()
        };
        // 临时文件应该在 drop 后被清理
        assert!(!path.exists());
    }

    #[test]
    fn test_create_test_client() {
        let client = create_test_client();
        // 验证客户端可以工作
        let result = client.get("https://httpbin.org/get")
            .timeout(Duration::from_secs(5))
            .send();
        // 不验证结果，只验证不 panic
        let _ = result;
    }
}
