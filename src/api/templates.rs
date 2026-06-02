use axum::{Json, extract::State};
use serde::Serialize;

use crate::api::state::AppState;
use crate::dao::Dao;
use crate::domain::ProviderTemplate;

/// Template summary for the Web UI.
/// 注：ProviderTemplate 没有 `default_base_url` / `default_model` 顶层字段
/// （这些在 ModelTemplate / default_env 里）。UI 主要用 `available_models`
/// 填 model 下拉，其他字段保留供未来扩展。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub id: String,
    pub display_name: String,
    pub opencode_provider_id: String,
    pub opencode_base_url: String,
    /// 该 template 下所有可用的 model 列表（来自 models[].id）
    pub available_models: Vec<String>,
}

impl From<&ProviderTemplate> for TemplateSummary {
    fn from(t: &ProviderTemplate) -> Self {
        Self {
            id: t.id.clone(),
            display_name: t.name.clone(),
            opencode_provider_id: t.opencode_provider_id.clone(),
            opencode_base_url: t.opencode_base_url.clone(),
            available_models: t.models.iter().map(|m| m.id.clone()).collect(),
        }
    }
}

/// GET /api/templates
pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<TemplateSummary>>, crate::api::error::ApiError> {
    let dao = state.dao.lock().await;
    let templates: Vec<TemplateSummary> =
        dao.get_templates().into_iter().map(Into::into).collect();
    Ok(Json(templates))
}
