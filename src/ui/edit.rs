use crate::app::state::{App, EditField};
use crate::dao::Dao;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// 渲染编辑弹窗（支持 API Key 和 Alias）
pub fn draw_edit<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = frame.size();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 3,
        width: area.width / 2,
        height: 7,
    };
    frame.render_widget(Clear, popup_area);

    let t = theme::theme();
    let title = match &app.state {
        crate::app::state::AppState::EditField { field: EditField::Alias, .. } => "编辑别名",
        _ => "编辑",
    };
    let prompt = match &app.state {
        crate::app::state::AppState::EditField { field: EditField::Alias, .. } => "修改别名：",
        _ => "修改 API Key：",
    };
    let text = vec![
        Line::from(prompt),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::raw(app.edit_input.value.clone()),
            Span::styled("_", Style::default().fg(t.warning())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(paragraph, popup_area);
}
