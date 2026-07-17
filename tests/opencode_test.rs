//! /api/opencode-config 集成测试 (S6-T2)

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
    stream
        .write_all(req.as_bytes())
        .await
        .expect("write failed");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read failed");
    String::from_utf8_lossy(&buf).into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_opencode_config_returns_json() {
    let addr = api::spawn_app().await;
    // 创建一条带 minimax 的 instance
    let create_body =
        r#"{"templateId":"minimax","alias":"cl-oc","modelId":"MiniMax-M3[1m]","apiKey":"sk-x"}"#;
    http_request(addr, "POST", "/api/instances", Some(create_body)).await;

    let raw = http_request(addr, "GET", "/api/opencode-config/minimax-cl-oc", None).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(
        raw.contains("application/json"),
        "expected JSON, got:\n{raw}"
    );

    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"$schema\""),
        "expected $schema, got:\n{body}"
    );
    assert!(
        body.contains("opencode.ai/config.json"),
        "expected opencode schema, got:\n{body}"
    );
    assert!(
        body.contains("minimax-cn"),
        "expected provider id, got:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_opencode_config_not_found_returns_404() {
    let addr = api::spawn_app().await;
    let raw = http_request(addr, "GET", "/api/opencode-config/nonexistent", None).await;
    assert!(raw.starts_with("HTTP/1.0 404"), "expected 404, got:\n{raw}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_apply_opencode_config_returns_path() {
    let addr = api::spawn_app().await;
    let create_body =
        r#"{"templateId":"minimax","alias":"cl-oc","modelId":"MiniMax-M3[1m]","apiKey":"sk-x"}"#;
    http_request(addr, "POST", "/api/instances", Some(create_body)).await;

    let raw = http_request(
        addr,
        "POST",
        "/api/opencode-config/minimax-cl-oc/apply",
        Some("{}"),
    )
    .await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"path\""),
        "expected path field, got:\n{body}"
    );
    assert!(
        body.contains("cl-oc.json"),
        "expected file name, got:\n{body}"
    );
}
