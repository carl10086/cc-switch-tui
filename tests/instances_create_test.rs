//! POST /api/instances 集成测试 (S2-T1)

mod api;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn http_post(addr: std::net::SocketAddr, path: &str, body: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect failed");
    let req = format!(
        "POST {path} HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .await
        .expect("write failed");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read failed");
    String::from_utf8_lossy(&buf).into_owned()
}

const VALID_BODY: &str =
    r#"{"templateId":"minimax","alias":"cl-test","modelId":"MiniMax-M3","apiKey":"sk-test-123"}"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_instance_returns_201() {
    let addr = api::spawn_app().await;
    let raw = http_post(addr, "/api/instances", VALID_BODY).await;
    assert!(raw.starts_with("HTTP/1.0 201"), "expected 201, got:\n{raw}");
    assert!(
        raw.contains("application/json"),
        "expected JSON, got:\n{raw}"
    );

    // body should contain the created instance with apiKey + id
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"id\":\"minimax-cl-test\""),
        "missing id, got:\n{body}"
    );
    assert!(
        body.contains("\"alias\":\"cl-test\""),
        "missing alias, got:\n{body}"
    );
    assert!(
        body.contains("\"apiKey\":\"sk-test-123\""),
        "missing apiKey in detail, got:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_duplicate_alias_returns_409() {
    let addr = api::spawn_app().await;
    // First create succeeds
    let r1 = http_post(addr, "/api/instances", VALID_BODY).await;
    assert!(
        r1.starts_with("HTTP/1.0 201"),
        "first create should succeed, got:\n{r1}"
    );

    // Second create with same alias should 409
    let r2 = http_post(addr, "/api/instances", VALID_BODY).await;
    assert!(
        r2.starts_with("HTTP/1.0 409"),
        "second create should 409, got:\n{r2}"
    );
    let body_start = r2.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &r2[body_start..];
    assert!(
        body.contains("\"field\":\"alias\""),
        "expected field=alias, got:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_invalid_alias_returns_400() {
    let addr = api::spawn_app().await;
    let bad =
        r#"{"templateId":"minimax","alias":"CL-Test","modelId":"MiniMax-M3","apiKey":"sk-test"}"#;
    let raw = http_post(addr, "/api/instances", bad).await;
    assert!(raw.starts_with("HTTP/1.0 400"), "expected 400, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("VALIDATION_ERROR"),
        "expected VALIDATION_ERROR code, got:\n{body}"
    );
}
