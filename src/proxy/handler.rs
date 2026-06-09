//! Proxy handler for `/ys-proxy/{alias}/**` routes.

use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::proxy::filter::filter_headers;
use crate::proxy::upstream::UpstreamClient;
use crate::trace::models::TraceDirection;

const ALLOWED_PATHS: &[&str] = &["/v1/messages", "/v1/complete", "/v1/models"];

/// Handle all proxied requests.
pub async fn proxy_handler(
    Path((alias, path)): Path<(String, String)>,
    State(state): State<AppState>,
    req: Request,
) -> Result<Response, ApiError> {
    // 路径白名单检查
    if !ALLOWED_PATHS.iter().any(|p| path.starts_with(p)) {
        return Ok((StatusCode::NOT_FOUND, "Not Found").into_response());
    }

    // 解构请求，先取 headers/method，再消费 body
    let (parts, body) = req.into_parts();
    let headers = parts.headers;
    let method = parts.method;

    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| ApiError::internal(format!("body read failed: {}", e)))?;

    // 获取 instance 配置
    let (upstream_url, api_key, provider, model) = {
        let dao = state.dao.lock().await;
        let instances = dao.list_instances();
        let instance = instances
            .iter()
            .find(|i| i.alias == alias)
            .ok_or_else(|| ApiError::not_found(format!("instance not found: {}", alias)))?;

        let templates = dao.get_templates();
        let template = templates
            .iter()
            .find(|t| t.id == instance.template_id)
            .ok_or_else(|| ApiError::internal(format!("template not found: {}", instance.template_id)))?;

        let upstream_url = template
            .default_env
            .get("ANTHROPIC_BASE_URL")
            .cloned()
            .unwrap_or_default();

        let model_name = template
            .models
            .iter()
            .find(|m| m.id == instance.model_id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| instance.model_id.clone());

        (
            upstream_url,
            instance.api_key.clone(),
            template.name.clone(),
            model_name,
        )
    };

    // 创建 trace session
    let session_id = {
        let store = state.trace_store.lock().await;
        store
            .create_session(&alias, &provider, &model)
            .map_err(|e| ApiError::internal(e.to_string()))?
    };

    // 记录 request
    {
        let store = state.trace_store.lock().await;
        store
            .append_record(
                &session_id,
                Some(1),
                TraceDirection::Request,
                &String::from_utf8_lossy(&body_bytes),
            )
            .map_err(|e| ApiError::internal(e.to_string()))?;
    }

    // 过滤 headers 并转发
    let filtered_headers = filter_headers(&headers, true);
    let client = UpstreamClient::new();
    let upstream_resp = client
        .forward(
            method,
            &format!("{}{}", upstream_url, path),
            filtered_headers,
            body_bytes,
            &api_key,
        )
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // 记录 response
    {
        let store = state.trace_store.lock().await;
        store
            .append_record(
                &session_id,
                Some(1),
                TraceDirection::Response,
                &String::from_utf8_lossy(&upstream_resp.body),
            )
            .map_err(|e| ApiError::internal(e.to_string()))?;
        store
            .finalize_session(&session_id, "complete", None)
            .map_err(|e| ApiError::internal(e.to_string()))?;
    }

    // 构建 axum response
    let mut builder = Response::builder().status(upstream_resp.status);
    for (key, value) in upstream_resp.headers {
        if let Some(key) = key {
            builder = builder.header(key, value);
        }
    }

    Ok(builder
        .body(upstream_resp.body.into())
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "build response failed").into_response()))
}
