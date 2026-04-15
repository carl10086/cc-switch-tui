pub mod create;
pub mod edit;
pub mod list;
pub mod popup;
pub mod theme;

use crate::app::state::{App, AppState};
use crate::dao::Dao;
use ratatui::Frame;

/// 根据 App 状态分发渲染逻辑
pub fn draw<D: Dao>(frame: &mut Frame, app: &App<D>) {
    tracing::trace!("render frame, state={:?}", app.state);
    match &app.state {
        AppState::List => list::draw_list(frame, app),
        AppState::CreateProvider
        | AppState::CreateModel { .. }
        | AppState::CreateApiKey { .. } => {
            list::draw_list(frame, app);
            create::draw_create(frame, app);
        }
        AppState::Edit { .. }
        | AppState::EditInfoPanel { .. }
        | AppState::EditField { .. } => {
            list::draw_list(frame, app);
            edit::draw_edit(frame, app);
        }
        AppState::CreateAlias { .. } => {
            list::draw_list(frame, app);
            create::draw_create(frame, app);
        }
        AppState::DeleteConfirm { .. } => {
            list::draw_list(frame, app);
            popup::draw_delete_confirm(frame, app);
        }
    }
}
