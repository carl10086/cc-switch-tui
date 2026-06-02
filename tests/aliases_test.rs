//! /api/aliases 集成测试 (S5-T2)

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
async fn test_get_aliases_returns_text() {
    let addr = api::spawn_app().await;
    let raw = http_request(addr, "GET", "/api/aliases", None).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(raw.contains("text/plain"), "expected text/plain, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    // 空 instances 时仍返回包含 header 的文本
    assert!(body.contains("# Auto-generated"), "expected header, got:\n{body}");
    assert!(body.contains("emulate zsh"), "expected emulate zsh, got:\n{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_apply_aliases_returns_path() {
    let addr = api::spawn_app().await;
    let raw = http_request(addr, "POST", "/api/aliases/apply", Some("{}")).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(body.contains("\"path\""), "expected path field, got:\n{body}");
    assert!(body.contains("aliases.zsh"), "expected aliases.zsh in path, got:\n{body}");
}
