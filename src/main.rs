use cc_switch_tui::app::templates::register_templates;
use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;

fn main() {
    let templates = register_templates();
    let _dao = MemoryDaoImpl::new(templates);

    // TODO: 启动 TUI
    println!("cc-switch-tui initialized");
}
