pub mod http_client;
pub mod web_search;
pub mod download;
pub mod download_enhanced;
pub mod network_tools;
pub mod browser;
pub mod request_monitor;
pub mod search_engine;
pub mod ssrf_protection;
pub mod error;

pub use http_client::HttpClientTools;
pub use web_search::WebSearchTools;
pub use download::DownloadTools;
pub use download_enhanced::{DownloadToolsEnhanced, Downloader, DownloadConfig};
pub use network_tools::NetworkTools;
pub use browser::{BrowserTools, BrowserConfig};
pub use request_monitor::{RequestMonitor, RequestStats, RequestLog};
pub use search_engine::{SearchEngineManager, SearchEngine, SearchResult, SearchError};
pub use ssrf_protection::{SsrfConfig, SsrfError, UrlSafety};
pub use error::{NetworkError, NetworkResult, ErrorContext};

// 导出 SSRF 防护工具函数
pub use ssrf_protection::{
    validate_url,
    validate_url_with_config,
    check_ip_safety,
    check_ip_safety_with_config,
    validate_save_path,
    validate_save_path_with_config,
    is_url_safe,
    is_ip_safe,
    is_path_safe,
};
