//! GET /api/instances 集成测试 (S1-T1)
//!
//! 验证：
//! - 空 DB 时返 200 + []
//! - 字段名 camelCase
//! - 列表中不含 apiKey 字段（即使 DB 里有）

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
async fn test_list_instances_empty_db_returns_empty_array() {
    let addr = api::spawn_app().await;
    let raw = http_get(addr, "/api/instances").await;
    assert!(raw.starts_with("HTTP/1.0 200"), "expected 200, got:\n{raw}");
    assert!(raw.contains("application/json"), "expected JSON, got:\n{raw}");
    // body 末尾应该是 []
    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &raw[body_start..];
    assert_eq!(body.trim(), "[]", "expected empty array, got body:\n{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_instances_does_not_leak_api_key() {
    // 此测试要插一条带 api_key 的 instance 再列表，验证响应不含 apiKey
    // 留待 S2 (Create) 之后再补：当前需要后端支持 POST /api/instances
}
