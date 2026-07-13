use axum::{Json, extract::State};
use serde::Serialize;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    /// 状态: ok | warn | error
    pub status: &'static str,
    pub db_path: String,
    pub db_writable: bool,
    pub aliases_path: String,
    pub aliases_writable: bool,
    pub zshrc_path: String,
    pub zshrc_writable: bool,
    pub opencode_dir: String,
    pub opencode_dir_writable: bool,
    pub instance_count: usize,
    pub template_count: usize,
}

/// GET /api/diagnostics
/// 返回系统级诊断信息（路径、可写性、计数）。
pub async fn get(State(state): State<AppState>) -> Result<Json<Diagnostics>, ApiError> {
    let home = dirs::home_dir()
        .ok_or_else(|| ApiError::internal("could not determine home directory"))?;
    let cc_dir = home.join(".cc-switch-tui");
    let db_path = cc_dir.join("db.sqlite");
    let aliases_path = cc_dir.join("aliases.zsh");
    let opencode_dir = cc_dir.join("opencode");
    let zshrc = home.join(".zshrc");

    let dao = state.dao.lock().await;
    let instance_count = dao.list_instances().len();
    let template_count = dao.get_templates().len();
    drop(dao);

    let db_writable = check_writable(&db_path);
    let aliases_writable = check_writable(&aliases_path);
    let zshrc_writable = check_writable(&zshrc);
    let opencode_dir_writable = check_dir_writable(&opencode_dir);

    let status = if db_writable && aliases_writable && zshrc_writable {
        "ok"
    } else if !db_writable {
        "error"
    } else {
        "warn"
    };

    Ok(Json(Diagnostics {
        status,
        db_path: db_path.to_string_lossy().into_owned(),
        db_writable,
        aliases_path: aliases_path.to_string_lossy().into_owned(),
        aliases_writable,
        zshrc_path: zshrc.to_string_lossy().into_owned(),
        zshrc_writable,
        opencode_dir: opencode_dir.to_string_lossy().into_owned(),
        opencode_dir_writable,
        instance_count,
        template_count,
    }))
}

fn check_writable(path: &std::path::Path) -> bool {
    if let Some(parent) = path.parent()
        && !parent.exists() {
            return std::fs::create_dir_all(parent).is_ok();
        }
    // 尝试在路径存在时测写
    if path.exists() {
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .is_ok()
    } else if let Some(parent) = path.parent() {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(parent.join(".write_test"))
            .map(|_| {
                let _ = std::fs::remove_file(parent.join(".write_test"));
                true
            })
            .unwrap_or(false)
    } else {
        false
    }
}

fn check_dir_writable(path: &std::path::Path) -> bool {
    if !path.exists() {
        return std::fs::create_dir_all(path).is_ok();
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path.join(".write_test"))
        .map(|_| {
            let _ = std::fs::remove_file(path.join(".write_test"));
            true
        })
        .unwrap_or(false)
}
