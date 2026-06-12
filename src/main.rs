use cc_switch_tui::api;
use cc_switch_tui::api::state::AppState;
use cc_switch_tui::dao::SqliteDaoImpl;
use cc_switch_tui::port;
use cc_switch_tui::templates::register_templates;
use std::io;
use std::path::PathBuf;

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

    // 初始化 DAO + AppState
    let templates = register_templates();
    let db_path = ".cc-switch-tui/db.sqlite";
    let dao = SqliteDaoImpl::new(db_path, templates).expect("无法初始化数据库");
    let trace_store =
        cc_switch_tui::trace::store::TraceStore::new(".cc-switch-tui/traces.sqlite")
            .expect("无法初始化 trace 数据库");
    let state = AppState::new(dao, trace_store);

    // 固定端口 7480，失败直接报错（与 ys-proxy 硬编码保持一致）
    let cc_dir = cc_switch_tui_home();
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

fn cc_switch_tui_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".cc-switch-tui")
}
