use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::domain::{AppError, ProviderInstance, instance::validate_alias};

/// 列表响应：不包含 apiKey（敏感信息只在详情/创建响应里返回）。
/// 字段匹配 Rust `ProviderInstance`，字段名驼峰（serde rename_all）。
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

/// 详情响应：包含 apiKey（仅在 GET /:id 和 POST 响应里返回）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDetail {
    pub id: String,
    pub template_id: String,
    pub alias: String,
    pub api_key: String,
    pub model_id: String,
    pub opencode_model_id: String,
    pub kv_cache_enabled: bool,
    pub created_at: String,
}

impl From<&ProviderInstance> for InstanceDetail {
    fn from(i: &ProviderInstance) -> Self {
        Self {
            id: i.id.clone(),
            template_id: i.template_id.clone(),
            alias: i.alias.clone(),
            api_key: i.api_key.clone(),
            model_id: i.model_id.clone(),
            opencode_model_id: i.opencode_model_id.clone(),
            kv_cache_enabled: i.kv_cache_enabled,
            created_at: i.created_at.to_rfc3339(),
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

/// POST /api/instances request body
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceRequest {
    pub template_id: String,
    pub alias: String,
    pub model_id: String,
    pub api_key: String,
    pub opencode_model_id: Option<String>,
    pub kv_cache_enabled: Option<bool>,
}

/// POST /api/instances
/// 201 + 完整 instance；409 if alias 冲突；400 if 校验失败
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateInstanceRequest>,
) -> Result<(StatusCode, Json<InstanceDetail>), ApiError> {
    if let Err(e) = validate_alias(&req.alias) {
        return Err(ApiError::validation("alias", e.to_string()));
    }

    let id = format!("{}-{}", req.template_id, req.alias);
    let new_instance = ProviderInstance {
        id: id.clone(),
        template_id: req.template_id,
        model_id: req.model_id,
        api_key: req.api_key,
        created_at: Utc::now(),
        alias: req.alias,
        opencode_model_id: req.opencode_model_id.unwrap_or_default(),
        kv_cache_enabled: req.kv_cache_enabled.unwrap_or(false),
    };

    let mut dao = state.dao.lock().await;
    match dao.create_instance(new_instance) {
        Ok(()) => {
            let instance = dao
                .get_instance(&id)
                .ok_or_else(|| ApiError::internal("just-created instance not found"))?;
            Ok((StatusCode::CREATED, Json(InstanceDetail::from(instance))))
        }
        Err(AppError::InstanceAlreadyExists(_)) => Err(ApiError::conflict(
            "alias",
            format!("alias '{}' already exists", instance_alias_from_id(&id)),
        )),
        Err(AppError::InvalidAlias(msg)) => Err(ApiError::validation("alias", msg)),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

fn instance_alias_from_id(id: &str) -> &str {
    // id 格式：{template_id}-{alias}，找第一个 '-' 之后的部分
    id.find('-').map(|i| &id[i + 1..]).unwrap_or(id)
}
