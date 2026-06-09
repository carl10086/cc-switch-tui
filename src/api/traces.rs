//! Trace API handlers.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::trace::models::{TraceRecord, TraceSession};

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<TraceSession>>, ApiError> {
    let store = state.trace_store.lock().await;
    let sessions = store
        .list_sessions(query.limit.max(1).min(100), query.offset.max(0))
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(sessions))
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TraceSession>, ApiError> {
    let store = state.trace_store.lock().await;
    let session = store
        .get_session(&id)
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("session not found: {}", id)))?;
    Ok(Json(session))
}

pub async fn get_records(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<TraceRecord>>, ApiError> {
    let store = state.trace_store.lock().await;
    let records = store
        .get_records(&id)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(records))
}

pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let store = state.trace_store.lock().await;
    store
        .delete_session(&id)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
