//! 搜索模块错误类型
//!
//! 定义搜索相关的专用错误类型

use thiserror::Error;
use crate::tools::network::error::NetworkError;

/// 搜索错误类型
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("网络请求失败：{0}")]
    Network(#[from] reqwest::Error),

    #[error("搜索 API 返回错误：{status} - {message}")]
    ApiError {
        status: u16,
        message: String,
    },

    #[error("未找到搜索结果")]
    NoResults,

    #[error("搜索超时：{0}")]
    Timeout(#[from] std::io::Error),

    #[allow(dead_code)]
    #[error("URL 验证失败：{0}")]
    UrlValidation(String),

    #[error("搜索引擎不可用：{engine}")]
    EngineUnavailable {
        engine: String,
    },

    #[allow(dead_code)]
    #[error("解析搜索结果失败：{0}")]
    ParseFailed(String),

    #[allow(dead_code)]
    #[error("缓存错误：{0}")]
    CacheError(String),

    #[error("查询无效：{0}")]
    InvalidQuery(String),

    #[allow(dead_code)]
    #[error("速率限制：{0}")]
    RateLimited(String),

    #[allow(dead_code)]
    #[error("认证失败：{0}")]
    AuthenticationFailed(String),

    #[allow(dead_code)]
    #[error("引擎初始化失败：{0}")]
    EngineInitFailed(String),
}

impl From<SearchError> for NetworkError {
    fn from(err: SearchError) -> Self {
        use crate::tools::network::error::SearchError as NetSearchError;
        match err {
            SearchError::Network(e) => NetworkError::Search(NetSearchError::Network(e.to_string())),
            SearchError::ApiError { status, message } => {
                NetworkError::Search(NetSearchError::ApiError { status, message })
            },
            SearchError::NoResults => {
                NetworkError::Search(NetSearchError::NoResults)
            },
            SearchError::Timeout(e) => NetworkError::Search(NetSearchError::Timeout(e.to_string())),
            SearchError::UrlValidation(msg) => {
                NetworkError::Search(NetSearchError::UrlValidation(msg))
            },
            SearchError::EngineUnavailable { engine } => {
                NetworkError::Search(NetSearchError::EngineUnavailable { engine })
            },
            SearchError::ParseFailed(msg) => {
                NetworkError::Search(NetSearchError::ParseFailed(msg))
            },
            SearchError::CacheError(msg) => {
                NetworkError::Search(NetSearchError::CacheError(msg))
            },
            SearchError::InvalidQuery(msg) => {
                NetworkError::Search(NetSearchError::InvalidQuery(msg))
            },
            SearchError::RateLimited(msg) => {
                NetworkError::Search(NetSearchError::InvalidQuery(msg))
            },
            SearchError::AuthenticationFailed(msg) => {
                NetworkError::Search(NetSearchError::InvalidQuery(msg))
            },
            SearchError::EngineInitFailed(msg) => {
                NetworkError::Search(NetSearchError::InvalidQuery(msg))
            },
        }
    }
}

/// 搜索模块结果类型
#[allow(dead_code)]
pub type SearchResult<T> = Result<T, SearchError>;
