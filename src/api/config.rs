use axum::{Json, extract::State, http::header};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::domain::ProviderInstance;

/// 当前导出格式版本号。未来 schema 变更时递增。
pub const EXPORT_VERSION: u32 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub instances: Vec<ExportedInstance>,
}

/// 单个 instance 的导出格式。
/// **不含 apiKey**（安全考虑 — 导出的 JSON 可能分享到不安全渠道）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedInstance {
    pub id: String,
    pub template_id: String,
    pub alias: String,
    pub model_id: String,
    pub opencode_model_id: String,
    pub kv_cache_enabled: bool,
}

impl From<&ProviderInstance> for ExportedInstance {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPayload {
    pub version: u32,
    #[serde(default)]
    pub instances: Vec<ImportedInstance>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedInstance {
    pub id: String,
    pub template_id: String,
    pub alias: String,
    pub model_id: String,
    #[serde(default)]
    pub opencode_model_id: String,
    #[serde(default)]
    pub kv_cache_enabled: bool,
    /// **导入时可选**。若没提供，导入后用户需手动补填 apiKey。
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResponse {
    pub created: usize,
    pub skipped: usize,
    pub skipped_aliases: Vec<String>,
}

impl From<ImportedInstance> for ProviderInstance {
    fn from(i: ImportedInstance) -> Self {
        let id = if i.id.is_empty() {
            format!("{}-{}", i.template_id, i.alias)
        } else {
            i.id
        };
        ProviderInstance {
            id,
            template_id: i.template_id,
            alias: i.alias,
            model_id: i.model_id,
            api_key: i.api_key.unwrap_or_default(),
            created_at: Utc::now(),
            opencode_model_id: i.opencode_model_id,
            kv_cache_enabled: i.kv_cache_enabled,
        }
    }
}

/// GET /api/config/export
/// 导出当前所有 instances 为 JSON（不含 apiKey）。
/// Content-Disposition: attachment 触发浏览器下载。
pub async fn export(State(state): State<AppState>) -> Result<axum::response::Response, ApiError> {
    let dao = state.dao.lock().await;
    let instances: Vec<ExportedInstance> =
        dao.list_instances().into_iter().map(Into::into).collect();
    drop(dao);

    let payload = ExportPayload {
        version: EXPORT_VERSION,
        exported_at: Utc::now(),
        instances,
    };

    let json = serde_json::to_string_pretty(&payload)
        .map_err(|e| ApiError::internal(format!("serialize: {e}")))?;

    Ok(axum::response::Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=cc-switch-config-{}.json",
                Utc::now().format("%Y%m%d")
            ),
        )
        .body(axum::body::Body::from(json))
        .expect("building export response"))
}

/// POST /api/config/import
/// 接受 ImportPayload，按 alias 跳过已存在的 instance。
/// **不**删除现有 instance — 这是 merge 模式。
pub async fn import(
    State(state): State<AppState>,
    Json(payload): Json<ImportPayload>,
) -> Result<Json<ImportResponse>, ApiError> {
    if payload.version != EXPORT_VERSION {
        return Err(ApiError::validation(
            "version",
            format!(
                "unsupported version {}, expected {}",
                payload.version, EXPORT_VERSION
            ),
        ));
    }

    let mut dao = state.dao.lock().await;
    let mut created = 0;
    let mut skipped = 0;
    let mut skipped_aliases = Vec::new();

    for imp in payload.instances {
        let alias_for_report = imp.alias.clone();
        let new_instance: ProviderInstance = imp.into();
        match dao.create_instance(new_instance) {
            Ok(()) => created += 1,
            Err(crate::domain::AppError::InstanceAlreadyExists(_)) => {
                skipped += 1;
                skipped_aliases.push(alias_for_report);
            }
            Err(crate::domain::AppError::InvalidAlias(msg)) => {
                tracing::warn!("import: skipping invalid alias: {msg}");
                skipped += 1;
            }
            Err(e) => return Err(ApiError::internal(format!("import failed: {e}"))),
        }
    }

    Ok(Json(ImportResponse {
        created,
        skipped,
        skipped_aliases,
    }))
}
