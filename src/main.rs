use cc_switch_tui::api;
use cc_switch_tui::api::state::AppState;
use cc_switch_tui::dao::SqliteDaoImpl;
use cc_switch_tui::data_migration::{default_cc_dir, ensure_data_migrated};
use cc_switch_tui::port;
use cc_switch_tui::templates::register_templates;
use std::io;

const DEFAULT_PORT: u16 = 7480;

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

    // 统一数据目录：~/.cc-switch-tui
    let cc_dir = default_cc_dir();
    let project_dir = std::env::current_dir()?;
    ensure_data_migrated(&cc_dir, &project_dir)?;

    let db_path = cc_dir.join("db.sqlite");
    let trace_path = cc_dir.join("traces.sqlite");
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid db path"))?;
    let trace_path_str = trace_path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid trace path"))?;

    // 初始化 DAO + AppState
    let templates = register_templates();
    let dao = SqliteDaoImpl::new(db_path_str, templates).expect("无法初始化数据库");
    let trace_store =
        cc_switch_tui::trace::store::TraceStore::new(trace_path_str)
            .expect("无法初始化 trace 数据库");
    let state = AppState::new(dao, trace_store);

    // 固定端口 7480，失败直接报错（与 ys-proxy 硬编码保持一致）
    let port_file = cc_dir.join("port");
    let (listener, actual_port) = port::try_bind(DEFAULT_PORT, 1)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::AddrInUse, e))?;
    let actual_addr = listener.local_addr()?;
    tracing::info!("listening on http://{}", actual_addr);

    // 写 port 到文件
    if let Err(e) = port::write_port_file(&port_file, actual_port) {
        tracing::warn!("failed to write port file: {e}");
    }

    // 自动开浏览器（尊重 settings.autoOpenBrowser）
    let url = format!("http://{}", actual_addr);
    let auto_open = {
        let s = state.settings.read().await;
        s.auto_open_browser
    };
    if auto_open {
        if let Err(e) = webbrowser::open(&url) {
            tracing::warn!("无法自动打开浏览器: {}。请手动访问 {}", e, url);
        }
    } else {
        tracing::info!("auto_open_browser disabled; manually visit {}", url);
    }

    // 启动 axum server + graceful shutdown
    let app = api::router(state);
    tracing::info!("server ready, open {} in your browser", url);
    let port_file_for_cleanup = port_file.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(port::wait_for_shutdown(move || {
            port::clear_port_file(&port_file_for_cleanup);
        }))
        .await?;

    tracing::info!("cc-switch-tui exiting");
    Ok(())
}
