//! SPA fallback handler 的集成测试。
//!
//! 用 raw TCP 发 HTTP/1.0 请求，绕开 reqwest 看到稳定的响应。
//! S0-T3 验证 SPA fallback 工作：GET / 和 GET /unknown-path 都返 web-dist/index.html。

mod api;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 发送 HTTP/1.0 GET 请求，返回完整 raw 响应
async fn http_get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect failed");
    let req = format!("GET {path} HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .await
        .expect("write failed");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read failed");
    String::from_utf8_lossy(&buf).into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_root_returns_index_html() {
    let addr = api::spawn_app().await;
    let raw = http_get(addr, "/").await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(raw.contains("text/html"), "expected text/html, got:\n{raw}");
    assert!(
        raw.contains("cc-switch"),
        "body should contain 'cc-switch', got:\n{raw}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_spa_fallback_for_unknown_path() {
    let addr = api::spawn_app().await;
    let raw = http_get(addr, "/some/random/path").await;
    assert!(
        raw.starts_with("HTTP/1.0 200"),
        "SPA fallback should serve 200, got:\n{raw}"
    );
    assert!(
        raw.contains("cc-switch"),
        "fallback should serve index.html, got:\n{raw}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_api_health_responds() {
    let addr = api::spawn_app().await;
    let raw = http_get(addr, "/api/health").await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(
        raw.contains("\"status\":\"ok\""),
        "expected status=ok, got:\n{raw}"
    );
}
