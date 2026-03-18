//! 网络工具模块
//!
//! 提供 HTTP 请求、搜索、下载、网络诊断等功能
//!
//! # 模块结构
//! - `error`: 统一错误类型（结构化）
//! - `ssrf_protection`: SSRF 防护统一模块（支持热更新配置）
//! - `http_client`: HTTP 客户端工具（支持 SSRF 防护）
//! - `search`: 统一搜索工具（多引擎智能路由，已拆分为子模块）
//! - `download`: 下载工具（支持断点续传）
//! - `network_tools`: 网络诊断工具（Ping、端口扫描等）
//! - `wikipedia`: 维基百科搜索工具
//! - `request_monitor`: 请求监控和统计

pub mod error;
pub mod ssrf_protection;
pub mod http_client;
pub mod search;
pub mod download;
pub mod network_tools;
pub mod request_monitor;
pub mod wikipedia;

// ============================================================================
// 重新导出 - 工具类
// ============================================================================

pub use http_client::HttpClientTools;
pub use search::SearchTools;
pub use download::DownloadTools;
pub use network_tools::NetworkTools;
pub use wikipedia::WikipediaTools;

// ============================================================================
// 重新导出 - 错误类型（方便使用）
// ============================================================================

pub use error::NetworkResult;

// ============================================================================
// 模块版本信息
// ============================================================================

/// 网络工具模块版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 获取模块信息
pub fn get_module_info() -> &'static str {
    "网络工具模块 v0.2.0 - 重构版"
}
