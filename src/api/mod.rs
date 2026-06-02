use axum::{Router, routing::{get, post}};

pub mod aliases;
pub mod config;
pub mod error;
pub mod health;
pub mod instances;
pub mod opencode;
pub mod state;
pub mod static_fallback;
pub mod templates;

use state::AppState;

/// 构造完整的 axum Router。
/// - `/api/*` 路由（S0-T4+ 接入真实 handler）
/// - 其他所有路径走 SPA fallback（返回 web-dist/index.html 或静态文件）
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .route(
            "/api/instances",
            get(instances::list).post(instances::create),
        )
        .route(
            "/api/instances/:id",
            get(instances::detail)
                .patch(instances::patch)
                .delete(instances::delete),
        )
        .route(
            "/api/instances/:id/duplicate",
            post(instances::duplicate),
        )
        .route("/api/templates", get(templates::list))
        .route("/api/aliases", get(aliases::get))
        .route("/api/aliases/apply", post(aliases::apply))
        .route("/api/opencode-config/:id", get(opencode::get))
        .route("/api/opencode-config/:id/apply", post(opencode::apply))
        .route("/api/config/export", get(config::export))
        .route("/api/config/import", post(config::import))
        .with_state(state)
        .fallback(static_fallback::spa_fallback)
}
