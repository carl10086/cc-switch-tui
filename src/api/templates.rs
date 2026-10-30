use axum::{Json, extract::State};
use serde::Serialize;

use crate::api::state::AppState;
use crate::dao::Dao;
use crate::domain::ProviderTemplate;

/// Per-model summary for the Web UI.
/// Context window 信息由前端从 model id 推断（如 `[1m]` → "1M context"），
/// 不再通过 API 字段传输。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateModelSummary {
    pub id: String,
    pub name: String,
}

/// Template summary for the Web UI.
/// 注：ProviderTemplate 没有 `default_base_url` / `default_model` 顶层字段
/// （这些在 ModelTemplate / default_env 里）。UI 主要用 `models` 数组
/// 填 model 下拉；`available_models` 保留向后兼容（旧前端）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub id: String,
    pub display_name: String,
    /// 兼容字段：旧前端仅消费此列表。**Deprecated**，新代码应使用 `models`。
    #[serde(default)]
    pub available_models: Vec<String>,
    /// 该 template 下所有可用的 model（带 name）。
    pub models: Vec<TemplateModelSummary>,
}

impl From<&ProviderTemplate> for TemplateSummary {
    fn from(t: &ProviderTemplate) -> Self {
        Self {
            id: t.id.clone(),
            display_name: t.name.clone(),
            available_models: t.models.iter().map(|m| m.id.clone()).collect(),
            models: t
                .models
                .iter()
                .map(|m| TemplateModelSummary {
                    id: m.id.clone(),
                    name: m.name.clone(),
                })
                .collect(),
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
