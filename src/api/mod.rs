use axum::{Router, routing::{any, get, post}};

pub mod aliases;
pub mod config;
pub mod diagnostics;
pub mod error;
pub mod health;
pub mod instances;
pub mod opencode;
pub mod settings;
pub mod state;
pub mod static_fallback;
pub mod templates;
pub mod traces;

use state::AppState;

/// 构造完整的 axum Router。
/// - `/api/*` 路由（S0-T4+ 接入真实 handler）
/// - `/ys-proxy/{alias}/**` 代理路由
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
        .route(
            "/api/settings",
            get(settings::get).put(settings::put),
        )
        .route("/api/diagnostics", get(diagnostics::get))
        .route(
            "/api/traces/sessions",
            get(traces::list_sessions).delete(traces::clear_all_sessions),
        )
        .route(
            "/api/traces/sessions/:id",
            get(traces::get_session).delete(traces::delete_session),
        )
        .route(
            "/api/traces/sessions/:id/records",
            get(traces::get_records),
        )
        .route(
            "/api/traces/sessions/:id/export/jsonl",
            get(traces::export_session),
        )
        .route(
            "/ys-proxy/:alias/*path",
            any(crate::proxy::handler::proxy_handler),
        )
        .with_state(state)
        .fallback(static_fallback::spa_fallback)
}
