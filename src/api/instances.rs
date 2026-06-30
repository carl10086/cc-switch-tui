use axum::{
    Json,
    extract::{Path, State},
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

/// 详情响应：包含 apiKey（仅在 GET /:id 和 POST/PATCH 响应里返回）。
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

// ===== List =====

/// GET /api/instances
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<InstanceSummary>>, ApiError> {
    let dao = state.dao.lock().await;
    let summaries: Vec<InstanceSummary> =
        dao.list_instances().into_iter().map(Into::into).collect();
    Ok(Json(summaries))
}

// ===== Create =====

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
        Err(AppError::InstanceAlreadyExists(_)) => {
            Err(ApiError::conflict("alias", format!("alias already exists")))
        }
        Err(AppError::InvalidAlias(msg)) => Err(ApiError::validation("alias", msg)),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

// ===== Get detail =====

/// GET /api/instances/:id
pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<InstanceDetail>, ApiError> {
    let dao = state.dao.lock().await;
    let instance = dao
        .get_instance(&id)
        .ok_or_else(|| ApiError::not_found(format!("instance {id} not found")))?;
    Ok(Json(InstanceDetail::from(instance)))
}

// ===== Patch =====

/// PATCH /api/instances/:id
/// 支持修改 modelId, apiKey, opencodeModelId, kvCacheEnabled。
/// alias 通过 PATCH 修改暂不支持（id 与 alias 绑定，需要 rename_instance；M1 留作 TODO）。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchInstanceRequest {
    #[serde(default)]
    pub alias: Option<String>,
    pub model_id: Option<String>,
    pub api_key: Option<String>,
    pub opencode_model_id: Option<String>,
    pub kv_cache_enabled: Option<bool>,
}

pub async fn patch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PatchInstanceRequest>,
) -> Result<Json<InstanceDetail>, ApiError> {
    if req.alias.is_some() {
        return Err(ApiError::validation(
            "alias",
            "alias cannot be changed via PATCH (delete + recreate to change alias)",
        ));
    }

    let mut dao = state.dao.lock().await;
    let existing = dao
        .get_instance(&id)
        .ok_or_else(|| ApiError::not_found(format!("instance {id} not found")))?
        .clone();

    let new_model_id = req.model_id.unwrap_or_else(|| existing.model_id.clone());
    let new_api_key = req.api_key.unwrap_or_else(|| existing.api_key.clone());

    dao.update_instance(&id, new_model_id, existing.alias.clone(), new_api_key)
        .map_err(|e| match e {
            AppError::InstanceNotFound(_) => ApiError::not_found("instance not found"),
            AppError::AliasAlreadyExists(_) => ApiError::conflict("alias", "alias conflict"),
            AppError::InvalidAlias(msg) => ApiError::validation("alias", msg),
            other => ApiError::internal(other.to_string()),
        })?;

    if let Some(ocm) = req.opencode_model_id {
        dao.set_opencode_model_id(&id, ocm)
            .map_err(|e| ApiError::internal(e.to_string()))?;
    }
    if let Some(kv) = req.kv_cache_enabled {
        dao.set_kv_cache_enabled(&id, kv)
            .map_err(|e| ApiError::internal(e.to_string()))?;
    }

    let updated = dao
        .get_instance(&id)
        .ok_or_else(|| ApiError::internal("updated instance not found"))?;
    Ok(Json(InstanceDetail::from(updated)))
}

// ===== Delete =====

/// DELETE /api/instances/:id
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut dao = state.dao.lock().await;
    match dao.delete_instance(&id) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(AppError::InstanceNotFound(_)) => {
            Err(ApiError::not_found(format!("instance {id} not found")))
        }
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

// ===== Duplicate =====

/// POST /api/instances/:id/duplicate
/// 复制 instance，alias 加 "-copy" 后缀；冲突返 409
pub async fn duplicate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<InstanceDetail>), ApiError> {
    let mut dao = state.dao.lock().await;
    let original = dao
        .get_instance(&id)
        .ok_or_else(|| ApiError::not_found(format!("instance {id} not found")))?
        .clone();

    let new_alias = format!("{}-copy", original.alias);
    let new_id = format!("{}-{}", original.template_id, new_alias);

    let new_instance = ProviderInstance {
        id: new_id.clone(),
        template_id: original.template_id,
        model_id: original.model_id,
        api_key: original.api_key,
        created_at: Utc::now(),
        alias: new_alias,
        opencode_model_id: original.opencode_model_id,
        kv_cache_enabled: original.kv_cache_enabled,
    };

    dao.create_instance(new_instance).map_err(|e| match e {
        AppError::InstanceAlreadyExists(_) => {
            ApiError::conflict("alias", "copy alias already exists")
        }
        AppError::InvalidAlias(msg) => ApiError::validation("alias", msg),
        other => ApiError::internal(other.to_string()),
    })?;

    let created = dao
        .get_instance(&new_id)
        .ok_or_else(|| ApiError::internal("just-created instance not found"))?;
    Ok((StatusCode::CREATED, Json(InstanceDetail::from(created))))
}
