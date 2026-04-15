use cc_switch_tui::app::state::App;
use cc_switch_tui::app::templates::register_templates;
use cc_switch_tui::dao::sqlite_impl::SqliteDaoImpl;
use cc_switch_tui::shell;
use cc_switch_tui::ui;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

fn main() -> io::Result<()> {
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("app.log")
        .expect("无法创建日志文件");

    tracing_subscriber::fmt()
        .with_writer(move || log_file.try_clone().unwrap())
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("INFO"))
        )
        .with_ansi(false)
        .with_target(true)
        .init();

    tracing::info!("cc-switch-tui starting");

    let zshrc_modified = shell::ensure_zshrc_source(
        &dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".zshrc"),
    )
    .unwrap_or(false);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db_path = ".cc-switch-tui/db.sqlite";
    let templates = register_templates();
    let dao = SqliteDaoImpl::new(db_path, templates).expect("无法初始化数据库");
    let mut app = App::new_with_dao(dao);
    app.zshrc_modified = zshrc_modified;

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }

    tracing::info!("cc-switch-tui exiting");
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App<SqliteDaoImpl>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    app.on_key(key);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
