use axum::{Json, extract::State};
use serde::Serialize;

use crate::api::state::AppState;
use crate::dao::Dao;
use crate::domain::ProviderTemplate;

/// Per-model summary for the Web UI.
/// 用于 OpenCode Model ID 下拉：UI 列出 name，存的值是 opencode_model_id。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateModelSummary {
    pub id: String,
    pub name: String,
    pub opencode_model_id: String,
    pub context_window: Option<u64>,
}

/// Template summary for the Web UI.
/// 注：ProviderTemplate 没有 `default_base_url` / `default_model` 顶层字段
/// （这些在 ModelTemplate / default_env 里）。UI 主要用 `models` 数组
/// 填 model 下拉和 opencode_model_id 下拉；`available_models` 保留
/// 向后兼容（旧前端）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub id: String,
    pub display_name: String,
    pub opencode_provider_id: String,
    pub opencode_base_url: String,
    /// 兼容字段：旧前端仅消费此列表。**Deprecated**，新代码应使用 `models`。
    #[serde(default)]
    pub available_models: Vec<String>,
    /// 该 template 下所有可用的 model（带 name + opencode_model_id）。
    pub models: Vec<TemplateModelSummary>,
    /// OpenCode 支持的模型 ID 列表（硬编码在内存中）。
    pub opencode_models: Vec<String>,
}

impl From<&ProviderTemplate> for TemplateSummary {
    fn from(t: &ProviderTemplate) -> Self {
        Self {
            id: t.id.clone(),
            display_name: t.name.clone(),
            opencode_provider_id: t.opencode_provider_id.clone(),
            opencode_base_url: t.opencode_base_url.clone(),
            available_models: t.models.iter().map(|m| m.id.clone()).collect(),
            models: t
                .models
                .iter()
                .map(|m| TemplateModelSummary {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    opencode_model_id: m.opencode_model_id.clone(),
                    context_window: m.context_window,
                })
                .collect(),
            opencode_models: t.opencode_models.clone(),
        }
    }
}

/// GET /api/templates
pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<TemplateSummary>>, crate::api::error::ApiError> {
    let dao = state.dao.lock().await;
    let templates: Vec<TemplateSummary> = dao.get_templates().into_iter().map(Into::into).collect();
    Ok(Json(templates))
}
