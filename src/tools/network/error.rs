//! 统一网络错误类型
//!
//! 使用 thiserror 提供结构化的错误定义，支持错误链和上下文信息

use thiserror::Error;

// ============================================================================
// 核心错误类型
// ============================================================================

/// 网络操作统一错误类型
#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NetworkError {
    // ========== SSRF 防护错误 ==========
    #[error("SSRF 防护：{0}")]
    Ssrf(#[from] SsrfError),

    // ========== HTTP 请求错误 ==========
    #[error("HTTP 请求失败：{0}")]
    Http(#[from] HttpError),

    // ========== 搜索错误 ==========
    #[error("搜索失败：{0}")]
    Search(#[from] SearchError),

    // ========== 下载错误 ==========
    #[error("下载失败：{0}")]
    Download(#[from] DownloadError),

    // ========== 网络诊断错误 ==========
    #[error("网络诊断失败：{0}")]
    NetworkTool(#[from] NetworkToolError),

    // ========== 浏览器错误 ==========
    #[error("浏览器操作失败：{0}")]
    Browser(String),

    // ========== 其他错误 ==========
    #[error("{0}")]
    Other(String),
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

// 实现 From trait 用于外部依赖错误类型转换
impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::Other(format!("IO 错误：{}", err))
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        NetworkError::Other(format!("JSON 解析错误：{}", err))
    }
}

impl From<url::ParseError> for NetworkError {
    fn from(err: url::ParseError) -> Self {
        NetworkError::Other(format!("URL 解析错误：{}", err))
    }
}

impl From<reqwest::Error> for NetworkError {
    fn from(err: reqwest::Error) -> Self {
        NetworkError::Other(format!("HTTP 客户端错误：{}", err))
    }
}

/// 统一结果类型
pub type NetworkResult<T> = Result<T, NetworkError>;

// ============================================================================
// SSRF 防护错误
// ============================================================================

#[derive(Error, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SsrfError {
    #[error("无效 URL 格式：{0}")]
    InvalidUrl(String),

    #[error("不支持的协议：{0}，仅支持 http/https")]
    UnsupportedScheme(String),

    #[error("URL 缺少主机名")]
    MissingHostname,

    #[error("禁止访问内网地址：{0} (SSRF 防护)")]
    PrivateNetwork(String),

    #[error("禁止访问内网域名：{0} (SSRF 防护)")]
    BlockedDomain(String),

    #[error("URL 过长 ({0} > {1} 字符)")]
    UrlTooLong(usize, usize),

    #[error("禁止写入敏感目录：{0} (安全限制)")]
    SensitivePath(String),

    #[error("禁止写入当前目录外的路径：{0} (安全限制)")]
    OutOfCwd(String),

    #[error("路径过长 ({0} > {1} 字符)")]
    PathTooLong(usize, usize),

    #[error("DNS 解析失败：{0}")]
    DnsResolution(String),
}

// ============================================================================
// HTTP 请求错误
// ============================================================================

#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HttpError {
    #[error("请求超时：{0}")]
    Timeout(String),

    #[error("连接失败：{0}")]
    ConnectionFailed(String),

    #[error("HTTP 状态码错误：{status} - {message}")]
    StatusCode { status: u16, message: String },

    #[error("重定向失败：{0}")]
    RedirectFailed(String),

    #[error("响应解析失败：{0}")]
    ResponseParseFailed(String),

    #[error("请求体过大：{size} bytes，最大允许 {max} bytes")]
    RequestTooLarge { size: usize, max: usize },

    #[error("响应体过大：{size} bytes，最大允许 {max} bytes")]
    ResponseTooLarge { size: usize, max: usize },

    #[error("SSRF 防护拦截：{0}")]
    SsrfBlocked(String),

    #[error("{context}")]
    WithContext { context: String },
}

impl HttpError {
    #[allow(dead_code)]
    pub fn with_context<S: Into<String>>(self, context: S) -> Self {
        match self {
            HttpError::WithContext { .. } => self,
            _ => HttpError::WithContext {
                context: context.into(),
            },
        }
    }
}

// ============================================================================
// 搜索错误
// ============================================================================

#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchError {
    #[error("网络请求失败：{0}")]
    Network(String),

    #[error("搜索 API 返回错误：{status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("未找到搜索结果")]
    NoResults,

    #[error("搜索超时：{0}")]
    Timeout(String),

    #[error("URL 验证失败：{0}")]
    UrlValidation(String),

    #[error("搜索引擎不可用：{engine}")]
    EngineUnavailable { engine: String },

    #[error("解析搜索结果失败：{0}")]
    ParseFailed(String),

    #[error("缓存错误：{0}")]
    CacheError(String),

    #[error("查询无效：{0}")]
    InvalidQuery(String),
}

impl From<reqwest::Error> for SearchError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            SearchError::Timeout(format!("请求超时：{}", err))
        } else if err.is_connect() {
            SearchError::Network(format!("连接失败：{}", err))
        } else {
            SearchError::Network(format!("网络请求失败：{}", err))
        }
    }
}

// ============================================================================
// 下载错误
// ============================================================================

#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DownloadError {
    #[error("HTTP 请求失败：{0}")]
    Http(String),

    #[error("文件 IO 错误：{0}")]
    Io(String),

    #[error("文件过大：{size} MB，最大允许 {max} MB")]
    FileTooLarge { size: usize, max: usize },

    #[error("磁盘空间不足：需要 {needed} bytes，可用 {available} bytes")]
    DiskSpaceInsufficient { needed: u64, available: u64 },

    #[error("不支持的文件类型：{ext}")]
    UnsupportedFileType { ext: String },

    #[error("断点续传失败：{0}")]
    ResumeFailed(String),

    #[error("下载中断：{0}")]
    Interrupted(String),

    #[error("路径验证失败：{0}")]
    PathValidation(String),

    #[error("文件名无效：{0}")]
    InvalidFilename(String),
}

impl From<reqwest::Error> for DownloadError {
    fn from(err: reqwest::Error) -> Self {
        DownloadError::Http(err.to_string())
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(err: std::io::Error) -> Self {
        DownloadError::Io(err.to_string())
    }
}

// ============================================================================
// 网络诊断错误
// ============================================================================

#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NetworkToolError {
    #[error("主机名无效：{0}")]
    InvalidHostname(String),

    #[error("端口无效：{port}，范围应为 1-65535")]
    InvalidPort { port: u16 },

    #[error("IP 地址无效：{0}")]
    InvalidIp(String),

    #[error("连接超时：{host}:{port}")]
    ConnectionTimeout { host: String, port: u16 },

    #[error("连接被拒绝：{host}:{port}")]
    ConnectionRefused { host: String, port: u16 },

    #[error("DNS 解析失败：{0}")]
    DnsResolution(String),

    #[error("权限不足：{0}")]
    PermissionDenied(String),

    #[error("命令执行失败：{cmd} - {error}")]
    CommandFailed { cmd: String, error: String },

    #[error("安全限制：{0}")]
    SecurityRestriction(String),

    #[error("平台不支持：{0}")]
    PlatformNotSupported(String),
}

// ============================================================================
// 错误上下文辅助结构
// ============================================================================

/// 错误上下文信息
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    pub url: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<u16>,
    pub retry_count: Option<u32>,
    pub operation: Option<String>,
    pub details: Option<String>,
}

#[allow(dead_code)]
impl ErrorContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
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

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// 格式化为详细错误信息（用于日志）
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
        if let Some(op) = &self.operation {
            parts.push(format!("操作：{}", op));
        }
        if let Some(details) = &self.details {
            parts.push(format!("详情：{}", details));
        }

        parts.join(" | ")
    }
}

// ============================================================================
// 错误辅助宏
// ============================================================================

/// 快速创建带上下文的错误
#[macro_export]
macro_rules! network_err {
    (Http, $msg:expr) => {
        $crate::tools::network::error::NetworkError::Http(
            $crate::tools::network::error::HttpError::WithContext {
                context: $msg.to_string(),
            }
        )
    };
    (Http, $fmt:expr, $($args:tt)*) => {
        $crate::tools::network::error::NetworkError::Http(
            $crate::tools::network::error::HttpError::WithContext {
                context: format!($fmt, $($args)*),
            }
        )
    };
    (Search, $msg:expr) => {
        $crate::tools::network::error::NetworkError::Search(
            $crate::tools::network::error::SearchError::ParseFailed($msg.to_string())
        )
    };
    (Search, $fmt:expr, $($args:tt)*) => {
        $crate::tools::network::error::NetworkError::Search(
            $crate::tools::network::error::SearchError::ParseFailed(format!($fmt, $($args)*))
        )
    };
    (Download, $msg:expr) => {
        $crate::tools::network::error::NetworkError::Download(
            $crate::tools::network::error::DownloadError::Interrupted($msg.to_string())
        )
    };
    (Download, $fmt:expr, $($args:tt)*) => {
        $crate::tools::network::error::NetworkError::Download(
            $crate::tools::network::error::DownloadError::Interrupted(format!($fmt, $($args)*))
        )
    };
    (NetworkTool, $msg:expr) => {
        $crate::tools::network::error::NetworkError::NetworkTool(
            $crate::tools::network::error::NetworkToolError::SecurityRestriction($msg.to_string())
        )
    };
    (NetworkTool, $fmt:expr, $($args:tt)*) => {
        $crate::tools::network::error::NetworkError::NetworkTool(
            $crate::tools::network::error::NetworkToolError::SecurityRestriction(format!($fmt, $($args)*))
        )
    };
}

/// 快速创建带上下文的 Result
#[macro_export]
macro_rules! ensure_network {
    ($cond:expr, $kind:ident, $msg:expr) => {
        if !($cond) {
            return Err(network_err!($kind, $msg));
        }
    };
    ($cond:expr, $kind:ident, $fmt:expr, $($args:tt)*) => {
        if !($cond) {
            return Err(network_err!($kind, $fmt, $($args)*));
        }
    };
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = HttpError::StatusCode {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(format!("{}", err), "HTTP 状态码错误：404 - Not Found");

        let err = SearchError::NoResults;
        assert_eq!(format!("{}", err), "未找到搜索结果");
    }

    #[test]
    fn test_error_from_conversion() {
        // 测试 From trait - io 错误会被转换为 Other 变体
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "文件未找到");
        let err: NetworkError = io_err.into();
        assert!(matches!(err, NetworkError::Other(_)));

        // serde_json 错误也会被转换为 Other 变体
        let json_err = serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid JSON",
        ));
        let err: NetworkError = json_err.into();
        assert!(matches!(err, NetworkError::Other(_)));
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_url("https://example.com")
            .with_method("GET")
            .with_status_code(404)
            .with_retry_count(3)
            .with_operation("fetch_url")
            .with_details("资源不存在");

        let formatted = ctx.format("请求失败");
        assert!(formatted.contains("URL: https://example.com"));
        assert!(formatted.contains("方法：GET"));
        assert!(formatted.contains("状态码：404"));
        assert!(formatted.contains("重试次数：3"));
        assert!(formatted.contains("操作：fetch_url"));
        assert!(formatted.contains("详情：资源不存在"));
    }

    #[test]
    fn test_ssrf_error_equality() {
        let err1 = SsrfError::PrivateNetwork("192.168.1.1".to_string());
        let err2 = SsrfError::PrivateNetwork("192.168.1.1".to_string());
        let err3 = SsrfError::PrivateNetwork("10.0.0.1".to_string());

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let err = SearchError::Timeout(source.to_string());

        // 验证错误链
        let network_err: NetworkError = err.into();
        assert!(matches!(network_err, NetworkError::Search(_)));
    }

    #[test]
    fn test_macro_helpers() {
        // 测试宏辅助函数
        let err = network_err!(Http, "连接超时");
        assert!(matches!(err, NetworkError::Http(_)));

        let err = network_err!(Search, "搜索失败：{}", "关键词过长");
        assert!(matches!(err, NetworkError::Search(_)));
    }
}
