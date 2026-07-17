//! /api/config/export + /api/config/import 集成测试 (S7)

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
async fn test_export_returns_versioned_json() {
    let addr = api::spawn_app().await;
    // 创建一条 instance
    let create_body =
        r#"{"templateId":"minimax","alias":"cl-exp","modelId":"MiniMax-M3","apiKey":"sk-x"}"#;
    http_request(addr, "POST", "/api/instances", Some(create_body)).await;

    let raw = http_request(addr, "GET", "/api/config/export", None).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(raw.contains("application/json"), "expected JSON");
    assert!(
        raw.contains("attachment"),
        "expected Content-Disposition attachment"
    );

    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"version\": 1"),
        "expected version: 1, got:\n{body}"
    );
    assert!(
        body.contains("\"exportedAt\""),
        "expected exportedAt, got:\n{body}"
    );
    assert!(
        body.contains("cl-exp"),
        "expected cl-exp in export, got:\n{body}"
    );
    // 关键：apiKey 不应出现在导出
    assert!(
        !body.contains("sk-x"),
        "apiKey must NOT be exported, got:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_import_creates_new_instances() {
    let addr = api::spawn_app().await;
    let import_body = r#"{
        "version": 1,
        "instances": [
            { "id": "minimax-cl-imp1", "templateId": "minimax", "alias": "cl-imp1", "modelId": "MiniMax-M3" }
        ]
    }"#;
    let raw = http_request(addr, "POST", "/api/config/import", Some(import_body)).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"created\":1"),
        "expected created:1, got:\n{body}"
    );

    // 验证 list 包含
    let list = http_request(addr, "GET", "/api/instances", None).await;
    let list_body_start = list.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    assert!(
        list[list_body_start..].contains("cl-imp1"),
        "imported not in list"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_import_skips_existing_alias() {
    let addr = api::spawn_app().await;
    // 先创建
    let create_body =
        r#"{"templateId":"minimax","alias":"cl-dup","modelId":"MiniMax-M3","apiKey":"sk-x"}"#;
    http_request(addr, "POST", "/api/instances", Some(create_body)).await;

    // 尝试 import 同 alias
    let import_body = r#"{
        "version": 1,
        "instances": [
            { "id": "minimax-cl-dup", "templateId": "minimax", "alias": "cl-dup", "modelId": "MiniMax-M2.7" }
        ]
    }"#;
    let raw = http_request(addr, "POST", "/api/config/import", Some(import_body)).await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("\"created\":0"),
        "expected created:0, got:\n{body}"
    );
    assert!(
        body.contains("\"skipped\":1"),
        "expected skipped:1, got:\n{body}"
    );
    assert!(
        body.contains("cl-dup"),
        "expected alias in skippedAliases, got:\n{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_import_rejects_unknown_version() {
    let addr = api::spawn_app().await;
    let import_body = r#"{ "version": 999, "instances": [] }"#;
    let raw = http_request(addr, "POST", "/api/config/import", Some(import_body)).await;
    assert!(raw.starts_with("HTTP/1.0 400"), "expected 400, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert!(
        body.contains("VALIDATION_ERROR"),
        "expected VALIDATION_ERROR, got:\n{body}"
    );
}
