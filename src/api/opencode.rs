use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use serde_json::Value;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::opencode_config;

#[derive(Serialize)]
pub struct ApplyResponse {
    pub path: String,
}

/// GET /api/opencode-config/:instance_id
/// 渲染 OpenCode 配置 JSON（不写盘）
pub async fn get(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let (instance, template) = {
        let dao = state.dao.lock().await;
        let instance = dao
            .get_instance(&instance_id)
            .ok_or_else(|| ApiError::not_found(format!("instance {instance_id} not found")))?
            .clone();
        let template = dao
            .get_template(&instance.template_id)
            .ok_or_else(|| {
                ApiError::not_found(format!("template {} not found", instance.template_id))
            })?
            .clone();
        (instance, template)
    };

    let config = opencode_config::render_opencode_config(&instance, &template)
        .ok_or_else(|| ApiError::internal("instance has no opencode config (missing fields)"))?;
    Ok(Json(config))
}

/// POST /api/opencode-config/:instance_id/apply
/// 写入 ~/.cc-switch-tui/opencode/{alias}.json
pub async fn apply(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<ApplyResponse>, ApiError> {
    let home =
        dirs::home_dir().ok_or_else(|| ApiError::internal("could not determine home directory"))?;
    let dir = home.join(".cc-switch-tui");

    let (instance, template) = {
        let dao = state.dao.lock().await;
        let instance = dao
            .get_instance(&instance_id)
            .ok_or_else(|| ApiError::not_found(format!("instance {instance_id} not found")))?
            .clone();
        let template = dao
            .get_template(&instance.template_id)
            .ok_or_else(|| {
                ApiError::not_found(format!("template {} not found", instance.template_id))
            })?
            .clone();
        (instance, template)
    };

    let path = opencode_config::write_opencode_config(&dir, &instance, &template)
        .map_err(|e| ApiError::internal(format!("failed to write opencode config: {e}")))?
        .ok_or_else(|| ApiError::internal("instance has no opencode config (missing fields)"))?;

    Ok(Json(ApplyResponse {
        path: path.to_string_lossy().into_owned(),
    }))
}
