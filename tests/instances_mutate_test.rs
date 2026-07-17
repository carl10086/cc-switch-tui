//! PATCH/DELETE/duplicate 集成测试 (S3-T1)

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

async fn create_one(addr: std::net::SocketAddr) -> String {
    let body = r#"{"templateId":"minimax","alias":"cl-edit","modelId":"MiniMax-M3","apiKey":"sk-original"}"#;
    let raw = http_request(addr, "POST", "/api/instances", Some(body)).await;
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    raw[body_start..].to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_patch_instance_updates_model_and_apikey() {
    let addr = api::spawn_app().await;
    let _ = create_one(addr).await;

    let body = r#"{"modelId":"MiniMax-M2.7","apiKey":"sk-updated"}"#;
    let raw = http_request(addr, "PATCH", "/api/instances/minimax-cl-edit", Some(body)).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"modelId\":\"MiniMax-M2.7\""),
        "model not updated:\n{body}"
    );
    assert!(
        body.contains("\"apiKey\":\"sk-updated\""),
        "apiKey not updated:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_patch_instance_not_found_returns_404() {
    let addr = api::spawn_app().await;
    let raw = http_request(
        addr,
        "PATCH",
        "/api/instances/nonexistent",
        Some(r#"{"modelId":"x"}"#),
    )
    .await;
    assert!(raw.starts_with("HTTP/1.0 404"), "expected 404, got:\n{raw}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_delete_instance_returns_204() {
    let addr = api::spawn_app().await;
    let _ = create_one(addr).await;

    let raw = http_request(addr, "DELETE", "/api/instances/minimax-cl-edit", None).await;
    assert!(raw.starts_with("HTTP/1.0 204"), "expected 204, got:\n{raw}");

    // Verify it's gone
    let list_raw = http_request(addr, "GET", "/api/instances", None).await;
    let body_start = list_raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    assert!(
        !list_raw[body_start..].contains("cl-edit"),
        "instance should be deleted"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_duplicate_instance_adds_copy_suffix() {
    let addr = api::spawn_app().await;
    let _ = create_one(addr).await;

    let raw = http_request(
        addr,
        "POST",
        "/api/instances/minimax-cl-edit/duplicate",
        None,
    )
    .await;
    assert!(raw.starts_with("HTTP/1.0 201"), "expected 201, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"alias\":\"cl-edit-copy\""),
        "expected alias=cl-edit-copy, got:\n{body}"
    );
    assert!(
        body.contains("\"id\":\"minimax-cl-edit-copy\""),
        "wrong id, got:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_duplicate_collision_returns_409() {
    let addr = api::spawn_app().await;
    let _ = create_one(addr).await;
    // First duplicate succeeds
    let _ = http_request(
        addr,
        "POST",
        "/api/instances/minimax-cl-edit/duplicate",
        None,
    )
    .await;
    // Second duplicate collides on -copy suffix
    let raw = http_request(
        addr,
        "POST",
        "/api/instances/minimax-cl-edit/duplicate",
        None,
    )
    .await;
    assert!(raw.starts_with("HTTP/1.0 409"), "expected 409, got:\n{raw}");
}
