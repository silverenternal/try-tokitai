//! 统一网络错误类型
//!
//! 整合所有网络相关工具的错误类型，提供统一的错误处理
//! TODO: Phase 5 集成到统一错误处理系统

use std::fmt;

/// 网络操作统一错误类型
/// TODO: Phase 5 集成到统一错误处理系统
#[derive(Debug)]
#[allow(dead_code)]
pub enum NetworkError {
    /// SSRF 防护错误
    Ssrf(crate::tools::network::ssrf_protection::SsrfError),

    /// HTTP 请求错误
    Http(String),

    /// 搜索错误
    Search(String),

    /// 下载错误
    Download(String),

    /// 浏览器错误
    Browser(String),

    /// 网络诊断错误
    NetworkTool(String),

    /// IO 错误
    Io(std::io::Error),
    
    /// JSON 解析错误
    Json(serde_json::Error),
    
    /// URL 解析错误
    Url(url::ParseError),
    
    /// 其他错误
    Other(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::Ssrf(err) => write!(f, "SSRF 防护：{}", err),
            NetworkError::Http(err) => write!(f, "HTTP 请求：{}", err),
            NetworkError::Search(err) => write!(f, "搜索：{}", err),
            NetworkError::Download(err) => write!(f, "下载：{}", err),
            NetworkError::Browser(err) => write!(f, "浏览器：{}", err),
            NetworkError::NetworkTool(err) => write!(f, "网络诊断：{}", err),
            NetworkError::Io(err) => write!(f, "IO: {}", err),
            NetworkError::Json(err) => write!(f, "JSON 解析：{}", err),
            NetworkError::Url(err) => write!(f, "URL 解析：{}", err),
            NetworkError::Other(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for NetworkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NetworkError::Ssrf(err) => Some(err),
            NetworkError::Io(err) => Some(err),
            NetworkError::Json(err) => Some(err),
            NetworkError::Url(err) => Some(err),
            _ => None,
        }
    }
}

// From 特质实现，支持自动转换
impl From<crate::tools::network::ssrf_protection::SsrfError> for NetworkError {
    fn from(err: crate::tools::network::ssrf_protection::SsrfError) -> Self {
        NetworkError::Ssrf(err)
    }
}

impl From<crate::tools::network::web_search::SearchError> for NetworkError {
    fn from(err: crate::tools::network::web_search::SearchError) -> Self {
        NetworkError::Search(err.to_string())
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::Io(err)
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        NetworkError::Json(err)
    }
}

impl From<url::ParseError> for NetworkError {
    fn from(err: url::ParseError) -> Self {
        NetworkError::Url(err)
    }
}

impl From<String> for NetworkError {
    fn from(err: String) -> Self {
        NetworkError::Other(err)
    }
}

impl From<&str> for NetworkError {
    fn from(err: &str) -> Self {
        NetworkError::Other(err.to_string())
    }
}

/// 统一结果类型
/// TODO: Phase 5 集成到统一错误处理系统
#[allow(dead_code)]
pub type NetworkResult<T> = Result<T, NetworkError>;

/// 错误辅助信息
/// TODO: Phase 5 集成到统一错误处理系统
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ErrorContext {
    pub url: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<u16>,
    pub retry_count: Option<u32>,
    pub details: Option<String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }

    pub fn with_retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = Some(retry_count);
        self
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// 格式化为详细错误信息
    pub fn format(&self, base_error: &str) -> String {
        let mut parts = vec![base_error.to_string()];

        if let Some(url) = &self.url {
            parts.push(format!("URL: {}", url));
        }

        if let Some(method) = &self.method {
            parts.push(format!("方法：{}", method));
        }

        if let Some(status) = &self.status_code {
            parts.push(format!("状态码：{}", status));
        }

        if let Some(retry) = &self.retry_count {
            parts.push(format!("重试次数：{}", retry));
        }

        if let Some(details) = &self.details {
            parts.push(format!("详情：{}", details));
        }

        parts.join(" | ")
    }
}

/// 错误辅助宏
#[macro_export]
macro_rules! network_err {
    ($kind:ident, $msg:expr) => {
        $crate::tools::network::error::NetworkError::$kind($msg.to_string())
    };
    ($kind:ident, $fmt:expr, $($args:tt)*) => {
        $crate::tools::network::error::NetworkError::$kind(format!($fmt, $($args)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = NetworkError::Http("连接超时".to_string());
        assert_eq!(format!("{}", err), "HTTP 请求：连接超时");

        let err = NetworkError::Download("文件不存在".to_string());
        assert_eq!(format!("{}", err), "下载：文件不存在");
    }

    #[test]
    fn test_error_from_string() {
        let err: NetworkError = "自定义错误".into();
        assert!(matches!(err, NetworkError::Other(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "文件未找到");
        let err: NetworkError = io_err.into();
        assert!(matches!(err, NetworkError::Io(_)));
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_url("https://example.com".to_string())
            .with_method("GET".to_string())
            .with_status_code(404)
            .with_details("资源不存在".to_string());

        let formatted = ctx.format("请求失败");
        assert!(formatted.contains("URL: https://example.com"));
        assert!(formatted.contains("方法：GET"));
        assert!(formatted.contains("状态码：404"));
        assert!(formatted.contains("详情：资源不存在"));
    }

    #[test]
    fn test_error_context_default() {
        let ctx = ErrorContext::default();
        assert!(ctx.url.is_none());
        assert!(ctx.method.is_none());
        assert!(ctx.status_code.is_none());
    }
}
