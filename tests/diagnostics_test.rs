//! /api/settings + /api/diagnostics 集成测试 (S8)

mod api;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn http_request(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> String {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect failed");
    let body_str = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
        body_str.len()
    );
    stream.write_all(req.as_bytes()).await.expect("write failed");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read failed");
    String::from_utf8_lossy(&buf).into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_settings_returns_defaults() {
    let addr = api::spawn_app().await;
    let raw = http_request(addr, "GET", "/api/settings", None).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(body.contains("\"autoOpenBrowser\":true"), "expected autoOpenBrowser:true, got:\n{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_put_settings_persists() {
    let addr = api::spawn_app().await;
    let put_body = r#"{"autoOpenBrowser":false,"defaultTemplate":"minimax"}"#;
    let raw = http_request(addr, "PUT", "/api/settings", Some(put_body)).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");

    let get_raw = http_request(addr, "GET", "/api/settings", None).await;
    let body_start = get_raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &get_raw[body_start..];
    assert!(body.contains("\"autoOpenBrowser\":false"), "expected autoOpenBrowser:false, got:\n{body}");
    assert!(body.contains("\"defaultTemplate\":\"minimax\""), "expected defaultTemplate:minimax, got:\n{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_diagnostics_returns_paths_and_status() {
    let addr = api::spawn_app().await;
    let raw = http_request(addr, "GET", "/api/diagnostics", None).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(body.contains("\"dbPath\""), "expected dbPath, got:\n{body}");
    assert!(body.contains("\"status\""), "expected status, got:\n{body}");
    assert!(body.contains("\"instanceCount\":0"), "expected instanceCount:0, got:\n{body}");
    assert!(body.contains("aliases.zsh"), "expected aliases.zsh path, got:\n{body}");
}
