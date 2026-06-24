//! axum Router 组装
//!
//! 把所有子路由拼接成顶层 `axum::Router`，并挂上 trace / cors / timeout 中间件。

use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::server::auth;
use crate::server::routes;
use crate::server::state::AppState;

/// 健康检查（顶层 /healthz，不依赖 AppState）
async fn healthz() -> &'static str {
    "ok"
}

/// 构建完整的 HTTP server router
pub fn build_router(state: AppState, api_key: Option<String>) -> Router {
    // /v1 子树：所有依赖 AppState 的路由都挂在这里
    let v1: Router<AppState> = Router::new()
        .merge(routes::health::router())
        .merge(routes::tools::router())
        .merge(routes::chat::router())
        .merge(routes::providers::router())
        .merge(routes::orchestrator::router())
        .merge(routes::dialogue::router())
        .merge(routes::workflows::router())
        .merge(routes::sessions::router())
        .merge(routes::context::router())
        .merge(routes::autonomy::router())
        .merge(routes::tool_market::router())
        .merge(routes::mcp::router())
        .merge(routes::cli::router());

    // 顶层 Router：基础设施 + v1 子树
    let app = Router::new()
        .route("/healthz", get(healthz))
        .nest("/v1", v1)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    if let Some(api_key) = api_key {
        app.layer(from_fn_with_state(api_key, auth::bearer_auth))
    } else {
        app
    }
}
