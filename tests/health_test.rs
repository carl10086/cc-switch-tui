//! /api/health endpoint 集成测试。
//!
//! 验证：
//! - 返 200
//! - 返 application/json
//! - body 包含 status=ok, version, db_path

mod api;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
async fn test_health_returns_json() {
    let addr = api::spawn_app().await;
    let raw = http_get(addr, "/api/health").await;

    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(
        raw.contains("application/json"),
        "expected application/json content-type, got:\n{raw}"
    );
    assert!(
        raw.contains("\"status\":\"ok\""),
        "expected status=ok, got:\n{raw}"
    );
    assert!(
        raw.contains("\"version\""),
        "expected version field, got:\n{raw}"
    );
    assert!(
        raw.contains("\"dbPath\""),
        "expected dbPath field, got:\n{raw}"
    );
}
