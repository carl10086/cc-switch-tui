use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub db_path: String,
}

/// GET /api/health
/// 用途：前端启动时探活；监控；build metadata。
/// 当前是 read-only 的纯内存响应（不读 DB），保持 fast & no-side-effect。
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        db_path: db_path_default(),
    })
}

fn db_path_default() -> String {
    // 与 main.rs v0.3.0 旧版保持一致：相对工作目录的 .cc-switch-tui/db.sqlite
    // TODO(S1+): 改为注入 AppState 读取实际配置路径
    ".cc-switch-tui/db.sqlite".to_string()
}
