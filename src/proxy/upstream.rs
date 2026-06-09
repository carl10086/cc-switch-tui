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

/// Client for forwarding requests to upstream providers.
pub struct UpstreamClient {
    client: Client,
}

impl UpstreamClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Forward a request to the upstream provider and buffer the full response.
    ///
    /// The `api_key` is injected as the `Authorization` header.
    pub async fn forward(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
        api_key: &str,
    ) -> Result<ForwardedResponse, AppError> {
        let mut request = self
            .client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", api_key))
            .body(body);

        for (key, value) in headers {
            if let Some(key) = key {
                request = request.header(key.as_str(), value.as_bytes());
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::Database(format!("upstream request failed: {}", e)))?;

        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::post, Router};

    async fn mock_handler(body: Body) -> axum::response::Response <String> {
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        axum::response::Response::builder()
            .status(200)
            .header("x-custom", "test")
            .body(format!("echo: {}", String::from_utf8_lossy(&bytes)))
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
}
