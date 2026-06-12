use axum::Json;
use crate::data_migration::default_cc_dir;
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
    default_cc_dir()
        .join("db.sqlite")
        .to_string_lossy()
        .into_owned()
}
