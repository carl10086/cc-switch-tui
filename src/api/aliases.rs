use axum::{Json, extract::State, http::header};
use serde::Serialize;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::shell;

/// GET /api/aliases
/// 渲染 aliases.zsh 内容（text/plain），不写盘
pub async fn get(
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let dao = state.dao.lock().await;
    let instances: Vec<_> = dao.list_instances().into_iter().cloned().collect();
    let templates: Vec<_> = dao.get_templates().into_iter().cloned().collect();
    drop(dao);

    let content = shell::render_aliases(&instances, &templates);

    Ok(axum::response::Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(axum::body::Body::from(content))
        .expect("building aliases response"))
}

#[derive(Serialize)]
pub struct ApplyResponse {
    pub path: String,
}

/// POST /api/aliases/apply
/// 写入 aliases.zsh + opencode 配置文件到 ~/.cc-switch-tui/
pub async fn apply(State(state): State<AppState>) -> Result<Json<ApplyResponse>, ApiError> {
    let home = dirs::home_dir()
        .ok_or_else(|| ApiError::internal("could not determine home directory"))?;
    let dir = home.join(".cc-switch-tui");

    let dao = state.dao.lock().await;
    let instances: Vec<_> = dao.list_instances().into_iter().cloned().collect();
    let templates: Vec<_> = dao.get_templates().into_iter().cloned().collect();
    drop(dao);

    shell::generate_aliases(&dir, &instances, &templates)
        .map_err(|e| ApiError::internal(format!("failed to write aliases: {e}")))?;

    Ok(Json(ApplyResponse {
        path: dir.join("aliases.zsh").to_string_lossy().into_owned(),
    }))
}
