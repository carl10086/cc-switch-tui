//! Upstream HTTP client for forwarding requests to provider APIs.

use axum::http::{HeaderMap, Method};
use bytes::Bytes;
use reqwest::{Client, StatusCode};

use crate::domain::AppError;

/// Response from an upstream provider after forwarding.
pub struct ForwardedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

/// Streaming response from an upstream provider.
///
/// The caller must consume `response.bytes_stream()` to read the body.
pub struct ForwardedStream {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub response: reqwest::Response,
}

/// Client for forwarding requests to upstream providers.
pub struct UpstreamClient {
    client: Client,
}

impl Default for UpstreamClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UpstreamClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Forward a request to the upstream provider and buffer the full response.
    pub async fn forward(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
        api_key: &str,
    ) -> Result<ForwardedResponse, AppError> {
        let response = self
            .send_request(method, url, headers, body, api_key)
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .bytes()
            .await
            .map_err(|e| AppError::Database(format!("upstream body read failed: {}", e)))?;

        Ok(ForwardedResponse {
            status,
            headers,
            body,
        })
    }

    /// Forward a request and return a streaming response without buffering the body.
    pub async fn forward_streaming(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
        api_key: &str,
    ) -> Result<ForwardedStream, AppError> {
        let response = self
            .send_request(method, url, headers, body, api_key)
            .await?;

        let status = response.status();
        let headers = response.headers().clone();

        Ok(ForwardedStream {
            status,
            headers,
            response,
        })
    }

    async fn send_request(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
        api_key: &str,
    ) -> Result<reqwest::Response, AppError> {
        let mut request = self.client.request(method, url).body(body);

        // 先注入 api_key 的 Authorization，再添加其他 headers（跳过原始 Authorization 避免重复）
        request = request.header("Authorization", format!("Bearer {}", api_key));
        for (key, value) in headers {
            if let Some(key) = key {
                if key.as_str().eq_ignore_ascii_case("authorization") {
                    continue;
                }
                request = request.header(key.as_str(), value.as_bytes());
            }
        }

        request
            .send()
            .await
            .map_err(|e| AppError::Database(format!("upstream request failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, routing::post};
    use futures::StreamExt;

    async fn mock_handler(body: Body) -> axum::response::Response<String> {
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        axum::response::Response::builder()
            .status(200)
            .header("x-custom", "test")
            .body(format!("echo: {}", String::from_utf8_lossy(&bytes)))
            .unwrap()
    }

    async fn mock_streaming_handler() -> axum::response::Response<Body> {
        let stream = futures::stream::iter(vec![
            Ok::<_, std::convert::Infallible>(Bytes::from("chunk1")),
            Ok(Bytes::from("chunk2")),
        ]);

        axum::response::Response::builder()
            .status(200)
            .header("x-custom", "stream")
            .body(Body::from_stream(stream))
            .unwrap()
    }

    #[tokio::test]
    async fn test_forward_post() {
        let app = Router::new().route("/v1/messages", post(mock_handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let client = UpstreamClient::new();
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        let resp = client
            .forward(
                Method::POST,
                &format!("http://127.0.0.1:{}/v1/messages", port),
                headers,
                Bytes::from(r#"{"model":"test"}"#),
                "test-key",
            )
            .await
            .unwrap();

        assert_eq!(resp.status, 200);
        assert!(resp.headers.contains_key("x-custom"));
        let body_str = String::from_utf8_lossy(&resp.body);
        assert!(body_str.contains("echo:"));
        assert!(body_str.contains("test"));
    }

    #[tokio::test]
    async fn test_forward_streaming() {
        let app = Router::new().route("/v1/messages", post(mock_streaming_handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let client = UpstreamClient::new();
        let stream_resp = client
            .forward_streaming(
                Method::POST,
                &format!("http://127.0.0.1:{}/v1/messages", port),
                HeaderMap::new(),
                Bytes::from(r#"{"model":"test"}"#),
                "test-key",
            )
            .await
            .unwrap();

        assert_eq!(stream_resp.status, 200);
        assert!(stream_resp.headers.contains_key("x-custom"));

        let mut chunks = Vec::new();
        let mut body_stream = stream_resp.response.bytes_stream();
        while let Some(chunk) = body_stream.next().await {
            chunks.push(chunk.unwrap());
        }

        let all_bytes = chunks.concat();
        let body = String::from_utf8_lossy(&all_bytes);
        assert!(body.contains("chunk1"));
        assert!(body.contains("chunk2"));
    }
}
