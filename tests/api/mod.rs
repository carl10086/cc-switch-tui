//! 公共测试 helper：spawn_app() 启动 axum server 在 127.0.0.1 随机端口。
//! 用 :memory: SQLite 让每个 test 独立。

use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn spawn_app() -> SocketAddr {
    let templates = cc_switch_tui::templates::register_templates();
    let dao = cc_switch_tui::dao::SqliteDaoImpl::new(":memory:", templates)
        .expect("failed to create in-memory DB");
    let state = cc_switch_tui::api::state::AppState::new(dao);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind random port");
    let addr = listener.local_addr().expect("failed to get local_addr");

    let app = cc_switch_tui::api::router(state);
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("axum server failed");
    });

    // 给 server 一点 warmup 时间
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    addr
}
