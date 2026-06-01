//! 公共测试 helper：spawn_app() 启动 axum server 在 127.0.0.1 随机端口。

use std::net::SocketAddr;
use tokio::net::TcpListener;

/// 启动 cc-switch-tui 的 axum server，绑定 127.0.0.1 随机端口。
/// 返回 server 实际监听的地址。
pub async fn spawn_app() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind random port");
    let addr = listener.local_addr().expect("failed to get local_addr");

    let app = cc_switch_tui::api::router();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("axum server failed");
    });

    // 给 server 一点 warmup 时间（multi-thread runtime + axum 启动需要)
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    addr
}
