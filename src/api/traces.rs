//! Trace API handlers.

use axum::body::Body;
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
    pub date: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, serde::Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<TraceSession>,
    pub total: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct GetRecordsResponse {
    pub records: Vec<TraceRecord>,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<ListSessionsResponse>, ApiError> {
    let store = state.trace_store.lock().await;
    let sessions = store
        .list_sessions(query.limit.max(1).min(100), query.offset.max(0), query.date.as_deref())
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let total = store
        .count_sessions(query.date.as_deref())
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(ListSessionsResponse { sessions, total }))
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

#[derive(Debug, Deserialize)]
pub struct GetRecordsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

pub async fn get_records(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetRecordsQuery>,
) -> Result<Json<GetRecordsResponse>, ApiError> {
    let store = state.trace_store.lock().await;
    let records = store
        .get_records(&id, query.limit.max(1).min(100), query.offset.max(0))
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(GetRecordsResponse { records }))
}

pub async fn export_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let store = state.trace_store.lock().await;
    let records = store
        .get_all_records(&id)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let mut lines = Vec::new();
    for record in records {
        let payload: serde_json::Value = serde_json::from_str(&record.payload_json)
            .unwrap_or(serde_json::Value::String(record.payload_json));
        let line = serde_json::json!({
            "record_index": record.record_index,
            "turn": record.turn,
            "timestamp": record.timestamp,
            "direction": record.direction,
            "payload": payload,
        });
        lines.push(serde_json::to_string(&line).unwrap_or_default());
    }

    let body = lines.join("\n");
    let response = axum::response::Response::builder()
        .header("Content-Type", "application/jsonl")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"trace-{}.jsonl\"", id),
        )
        .body(Body::from(body))
        .unwrap_or_else(|_| axum::http::Response::new(Body::empty()));

    Ok(response)
}

pub async fn clear_all_sessions(
    State(state): State<AppState>,
) -> Result<axum::http::StatusCode, ApiError> {
    let store = state.trace_store.lock().await;
    store
        .clear_all()
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
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
