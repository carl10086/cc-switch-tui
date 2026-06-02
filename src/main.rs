use cc_switch_tui::api;
use cc_switch_tui::api::state::AppState;
use cc_switch_tui::app::templates::register_templates;
use cc_switch_tui::dao::SqliteDaoImpl;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main]
async fn main() -> io::Result<()> {
    // 日志初始化（沿用 v0.3.0 模式）
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("app.log")
        .expect("无法创建日志文件");
    tracing_subscriber::fmt()
        .with_writer(move || log_file.try_clone().unwrap())
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("INFO")),
        )
        .with_ansi(false)
        .with_target(true)
        .init();
    tracing::info!("cc-switch-tui starting (web mode)");

    // 初始化 DAO + AppState
    let templates = register_templates();
    let db_path = ".cc-switch-tui/db.sqlite";
    let dao = SqliteDaoImpl::new(db_path, templates).expect("无法初始化数据库");
    let state = AppState::new(dao);

    // 绑定 127.0.0.1:7480（端口策略留到 S10 实现，目前硬编码）
    let addr: SocketAddr = "127.0.0.1:7480".parse().unwrap();
    let listener = TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;
    tracing::info!("listening on http://{}", actual_addr);

    // 自动开浏览器
    let url = format!("http://{}", actual_addr);
    if let Err(e) = webbrowser::open(&url) {
        tracing::warn!("无法自动打开浏览器: {}。请手动访问 {}", e, url);
    }

    // 启动 axum server
    let app = api::router(state);
    tracing::info!("server ready, open {} in your browser", url);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("cc-switch-tui exiting");
    Ok(())
}

/// 监听 Ctrl-C / SIGTERM，触发 graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl-C"),
        _ = terminate => tracing::info!("received SIGTERM"),
    }
}
