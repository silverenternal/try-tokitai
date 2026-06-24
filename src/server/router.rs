//! axum Router 组装
//!
//! 把所有子路由拼接成顶层 `axum::Router`，并挂上 trace / cors / timeout 中间件。

use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::server::routes;
use crate::server::state::AppState;

/// 健康检查（顶层 /healthz，不依赖 AppState）
async fn healthz() -> &'static str {
    "ok"
}

/// 构建完整的 HTTP server router
pub fn build_router(state: AppState) -> Router {
    // /v1 子树：所有依赖 AppState 的路由都挂在这里
    let v1: Router<AppState> = Router::new()
        .merge(routes::health::router())
        .merge(routes::tools::router())
        .merge(routes::chat::router())
        .merge(routes::providers::router())
        .merge(routes::orchestrator::router())
        .merge(routes::dialogue::router())
        .merge(routes::workflows::router())
        .merge(routes::sessions::router());
    // .merge(routes::context::router()) // Commit 5
    // .merge(routes::autonomy::router()) // Commit 5
    // .merge(routes::tool_market::router()) // Commit 5
    // .merge(routes::mcp::router()) // Commit 5
    // .merge(routes::cli::router()) // Commit 5

    // 顶层 Router：基础设施 + v1 子树
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/v1", v1)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
