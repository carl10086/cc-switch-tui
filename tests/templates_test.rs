//! GET /api/templates 集成测试 (S4-T1)

mod api;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn http_get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect failed");
    let req = format!("GET {path} HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.expect("write failed");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read failed");
    String::from_utf8_lossy(&buf).into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_templates_returns_built_in_list() {
    let addr = api::spawn_app().await;
    let raw = http_get(addr, "/api/templates").await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    // 至少包含 minimax + kimi
    assert!(body.contains("\"id\":\"minimax\""), "expected minimax template, got:\n{body}");
    assert!(body.contains("\"id\":\"kimi\""), "expected kimi template, got:\n{body}");
    // 字段名驼峰
    assert!(body.contains("\"displayName\""), "expected displayName field, got:\n{body}");
    assert!(body.contains("\"availableModels\""), "expected availableModels field, got:\n{body}");
    // minimax template 至少有一个 model
    assert!(body.contains("MiniMax-M3") || body.contains("M2.7"), "expected at least one model, got:\n{body}");
    // B2 fix: models 数组 + 每项含 opencodeModelId
    assert!(body.contains("\"models\""), "expected models array field (B2), got:\n{body}");
    assert!(
        body.contains("\"opencodeModelId\""),
        "expected opencodeModelId field per model (B2), got:\n{body}"
    );
}
