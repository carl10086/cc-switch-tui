pub mod create;
pub mod edit;
pub mod list;
pub mod popup;

use crate::app::state::{App, AppState};
use ratatui::Frame;

/// 根据 App 状态分发渲染逻辑
pub fn draw(frame: &mut Frame, app: &App) {
    tracing::trace!("render frame, state={:?}", app.state);
    match &app.state {
        AppState::List => list::draw_list(frame, app),
        AppState::CreateProvider
        | AppState::CreateModel { .. }
        | AppState::CreateApiKey { .. } => {
            list::draw_list(frame, app);
            create::draw_create(frame, app);
        }
        AppState::Edit { .. } => {
            list::draw_list(frame, app);
            edit::draw_edit(frame, app);
        }
        AppState::DeleteConfirm { .. } => {
            list::draw_list(frame, app);
            popup::draw_delete_confirm(frame, app);
        }
    }
}
