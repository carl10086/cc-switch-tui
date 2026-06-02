use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::state::AppState;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub auto_open_browser: bool,
    pub default_template: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_open_browser: true,
            default_template: None,
        }
    }
}

/// GET /api/settings
/// 返当前 settings（in-memory；重启 reset 为默认）
pub async fn get(State(state): State<AppState>) -> Result<Json<Settings>, ApiError> {
    let current = state.settings.read().await;
    Ok(Json(current.clone()))
}

/// PUT /api/settings
/// 替换 settings（in-memory）
pub async fn put(
    State(state): State<AppState>,
    Json(new_settings): Json<Settings>,
) -> Result<Json<Settings>, ApiError> {
    let mut current = state.settings.write().await;
    *current = new_settings;
    Ok(Json(current.clone()))
}
