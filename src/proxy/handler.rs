//! Proxy handler for `/ys-proxy/{alias}/**` routes.

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::StreamExt;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::proxy::filter::filter_headers;
use crate::proxy::sse::SseParser;
use crate::proxy::upstream::UpstreamClient;
use crate::trace::models::TraceDirection;

const ALLOWED_PATHS: &[&str] = &["/v1/messages", "/v1/complete", "/v1/models"];

/// Detect whether a request body asks for SSE streaming.
fn is_streaming_request(body: &[u8]) -> bool {
    let text = String::from_utf8_lossy(body);
    text.contains("\"stream\":true") || text.contains("\"stream\": true")
}

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
    let upstream_url = format!("{}{}", upstream_url, path);

    if is_streaming_request(&body_bytes) {
        // --- 流式路径 ---
        let stream_resp = client
            .forward_streaming(method, &upstream_url, filtered_headers, body_bytes, &api_key)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

        // 启动 trace 记录后台任务
        let (trace_tx, mut trace_rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
        let trace_store = state.trace_store.clone();
        let session_id_clone = session_id.clone();

        tokio::spawn(async move {
            let mut parser = SseParser::new();
            while let Some(chunk) = trace_rx.recv().await {
                let events = parser.feed(&chunk);
                for event in events {
                    let payload = if let Some(ref ty) = event.event_type {
                        format!("{{\"event_type\":\"{}\",\"data\":{}}}", ty, event.data)
                    } else {
                        event.data
                    };
                    let store = trace_store.lock().await;
                    let _ = store.append_record(
                        &session_id_clone,
                        Some(1),
                        TraceDirection::Response,
                        &payload,
                    );
                }
            }
            let final_events = parser.flush();
            for event in final_events {
                let payload = if let Some(ref ty) = event.event_type {
                    format!("{{\"event_type\":\"{}\",\"data\":{}}}", ty, event.data)
                } else {
                    event.data
                };
                let store = trace_store.lock().await;
                let _ = store.append_record(
                    &session_id_clone,
                    Some(1),
                    TraceDirection::Response,
                    &payload,
                );
            }
            let store = trace_store.lock().await;
            let _ = store.finalize_session(&session_id_clone, "complete", None);
        });

        // 构建客户端流：转发同时克隆到 trace 通道
        let client_stream = stream_resp.response.bytes_stream().map(move |result| {
            if let Ok(ref bytes) = result {
                let _ = trace_tx.send(bytes.clone());
            }
            result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        });

        let body = Body::from_stream(client_stream);
        let mut builder = Response::builder().status(stream_resp.status);
        for (key, value) in stream_resp.headers {
            if let Some(key) = key {
                builder = builder.header(key, value);
            }
        }

        Ok(builder.body(body).unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "build response failed").into_response()
        }))
    } else {
        // --- 非流式路径（保持原有逻辑） ---
        let upstream_resp = client
            .forward(method, &upstream_url, filtered_headers, body_bytes, &api_key)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_detection_true() {
        assert!(is_streaming_request(b"{\"stream\":true}"));
        assert!(is_streaming_request(b"{\"stream\": true, \"model\":\"test\"}"));
    }

    #[test]
    fn test_streaming_detection_false() {
        assert!(!is_streaming_request(b"{\"stream\":false}"));
        assert!(!is_streaming_request(b"{\"stream\": false}"));
        assert!(!is_streaming_request(b"{}"));
        assert!(!is_streaming_request(b"{\"model\":\"test\"}"));
    }
}
