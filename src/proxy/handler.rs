//! Proxy handler for `/ys-proxy/{alias}/**` routes.

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::StreamExt;
use serde_json::json;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::dao::Dao;
use crate::proxy::filter::filter_headers;
use crate::proxy::parser::{AnthropicParser, StreamingAccumulator};
use crate::proxy::session_extractor::{extract_claude_session_id, redact_user_id_pii};
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
    // axum 的 *path 提取不带前导斜杠，补回来
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{}", path)
    };

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

    // 提前解析 request body（后续 trace 需要）
    let body_str = String::from_utf8_lossy(&body_bytes.clone()).to_string();
    let request_summary = AnthropicParser::new().parse_request(&body_str);

    // 提取 claude_session_id 并脱敏 PII（仅 POST /v1/messages）
    let (claude_session_id, body_str_for_trace) = if method == axum::http::Method::POST
        && path.starts_with("/v1/messages")
    {
        if let Ok(mut body_json) = serde_json::from_str::<serde_json::Value>(&body_str) {
            let sid = extract_claude_session_id(&body_json);
            let _ = redact_user_id_pii(&mut body_json);
            let redacted = serde_json::to_string(&body_json)
                .unwrap_or_else(|_| body_str.clone());
            (sid, redacted)
        } else {
            (None, body_str.clone())
        }
    } else {
        (None, body_str.clone())
    };

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

    // 过滤 headers 并转发（redact=false：upstream 需要完整的 Authorization）
    let filtered_headers = filter_headers(&headers, false);
    let client = UpstreamClient::new();
    let upstream_url = format!("{}{}", upstream_url, path);

    if is_streaming_request(&body_bytes) {
        // --- 流式路径：延迟到 message_start 创建 session ---
        let stream_resp = client
            .forward_streaming(method, &upstream_url, filtered_headers, body_bytes, &api_key)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

        // 启动 trace 记录后台任务
        let (trace_tx, mut trace_rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
        let trace_store = state.trace_store.clone();
        let request_summary_clone = request_summary.clone();
        let body_str_clone = body_str_for_trace;
        let claude_session_id_clone = claude_session_id.clone();
        let alias_clone = alias.clone();
        let provider_clone = provider.clone();
        let model_clone = model.clone();

        tokio::spawn(async move {
            let mut parser = SseParser::new();
            let mut accumulator = StreamingAccumulator::default();
            let anthropic_parser = AnthropicParser::new();
            let mut session_id: Option<String> = None;

            while let Some(chunk) = trace_rx.recv().await {
                let events = parser.feed(&chunk);
                for event in events {
                    // 第一个 message_start 时创建 session 并记录 request
                    if event.event_type.as_deref() == Some("message_start") && session_id.is_none() {
                        let store = trace_store.lock().await;
                        if let Ok(sid) = store.create_session(&alias_clone, &provider_clone, &model_clone) {
                            let _ = store.append_record(
                                &sid,
                                Some(1),
                                TraceDirection::Request,
                                &body_str_clone,
                                claude_session_id_clone.as_deref(),
                            );
                            session_id = Some(sid);
                        }
                    }
                    if session_id.is_some() {
                        anthropic_parser.apply_streaming_event(&mut accumulator, &event);
                    }
                }
            }
            // 处理最后残留的 bytes
            let last_events = parser.flush();
            for event in last_events {
                if session_id.is_some() {
                    anthropic_parser.apply_streaming_event(&mut accumulator, &event);
                }
            }

            // 保存 response
            if let Some(ref sid) = session_id {
                let response_summary = accumulator.into_response();
                let response_json = serde_json::to_string(&response_summary)
                    .unwrap_or_else(|_| "{}".to_string());
                let summary = serde_json::to_string(&json!({
                    "request": request_summary_clone,
                    "response": &response_summary,
                })).unwrap_or_default();

                let store = trace_store.lock().await;
                let _ = store.append_record(
                    sid,
                    Some(1),
                    TraceDirection::Response,
                    &response_json,
                    claude_session_id_clone.as_deref(),
                );
                let _ = store.update_summary(sid, &summary);
                let _ = store.finalize_session(sid, "complete", Some(&summary));
            }
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
        // --- 非流式路径：MVP 仅流式，不记录 trace ---
        let upstream_resp = client
            .forward(method, &upstream_url, filtered_headers, body_bytes, &api_key)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

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
