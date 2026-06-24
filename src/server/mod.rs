//! HTTP REST API Server 模块
//!
//! 提供基于 axum 的 RESTful API，暴露 CLI/TUI 的全部能力。
//!
//! ## 启动方式
//! ```bash
//! cargo run --features server -- --server --port 8080
//! ```
//!
//! ## 仅监听 127.0.0.1
//! 为安全起见，HTTP server **只能**绑定到 loopback 地址，端口可配置。
//!
//! ## 特性开关
//! 整个模块由 `server` cargo feature 控制；不启用时不会拉入 axum/tower 依赖。

pub mod error;
pub mod router;
pub mod state;
pub mod stores;
pub mod tool_set;

mod routes;

#[cfg(feature = "server")]
pub use state::ServerConfig;

use anyhow::Result;
use std::net::Ipv4Addr;
use tracing::info;

/// 启动 HTTP REST API Server
///
/// 该函数会**阻塞**当前线程直到收到 SIGINT 或 SIGTERM。
///
/// # 参数
/// - `port`: 监听端口（1-65535）
/// - `api_key`: 可选的 Bearer token；为 None 时关闭鉴权
/// - `state`: 已构造好的 [`AppState`](state::AppState)
pub async fn run_server(port: u16, api_key: Option<String>, state: state::AppState) -> Result<()> {
    use tokio::net::TcpListener;

    let bind_addr = Ipv4Addr::LOCALHOST;
    let addr = std::net::SocketAddr::from((bind_addr, port));

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("绑定 {} 失败：{}（端口被占用？）", addr, e))?;

    let mut config = state.server_cfg.clone();
    config.port = port;
    config.api_key = api_key;

    let app = router::build_router(state);

    info!("🌐 HTTP Server 监听 http://{}", addr);
    if config.api_key.is_some() {
        info!("🔒 已启用 Bearer token 鉴权");
    } else {
        info!("🔓 未启用鉴权（仅 127.0.0.1 可访问）");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("HTTP server 运行错误：{}", e))?;

    info!("HTTP Server 已停止");
    Ok(())
}

/// 监听 SIGINT / SIGTERM 信号用于 graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("注册 SIGTERM 处理器失败");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("收到 SIGINT，开始 graceful shutdown"); }
        _ = terminate => { info!("收到 SIGTERM，开始 graceful shutdown"); }
    }
}
