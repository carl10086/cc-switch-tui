use axum::{Json, extract::State};
use serde::Serialize;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::domain::ProviderInstance;

/// 列表响应：不包含 apiKey（敏感信息只在详情里返回）。
/// 字段匹配 Rust `ProviderInstance`，字段名驼峰（serde rename_all）。
///
/// 注：spec 原本列出 `baseUrl` / `envOverrides` / `isDefault` 三个字段，
/// 但当前 `ProviderInstance` 还没有这三个字段（它们在 ModelTemplate / 待办）。
/// 后续如果要在 instance 上加这些字段，会同步扩展这里。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    pub id: String,
    pub template_id: String,
    pub alias: String,
    pub model_id: String,
    pub opencode_model_id: String,
    pub kv_cache_enabled: bool,
}

impl From<&ProviderInstance> for InstanceSummary {
    fn from(i: &ProviderInstance) -> Self {
        Self {
            id: i.id.clone(),
            template_id: i.template_id.clone(),
            alias: i.alias.clone(),
            model_id: i.model_id.clone(),
            opencode_model_id: i.opencode_model_id.clone(),
            kv_cache_enabled: i.kv_cache_enabled,
        }
    }
}

/// GET /api/instances
pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<InstanceSummary>>, ApiError> {
    let dao = state.dao.lock().await;
    let summaries: Vec<InstanceSummary> =
        dao.list_instances().into_iter().map(Into::into).collect();
    Ok(Json(summaries))
}
