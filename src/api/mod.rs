use axum::{Router, routing::get};

pub mod error;
pub mod static_fallback;

use error::ApiError;

/// 占位 health handler — S0-T4 会替换为真实实现 + 测试
async fn placeholder_health() -> Result<&'static str, ApiError> {
    Ok("placeholder")
}

/// 构造完整的 axum Router。
/// - `/api/*` 路由（S0-T4+ 接入真实 handler）
/// - 其他所有路径走 SPA fallback（返回 web-dist/index.html 或静态文件）
pub fn router() -> Router {
    Router::new()
        .route("/api/health", get(placeholder_health))
        .fallback(static_fallback::spa_fallback)
}
