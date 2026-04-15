use crate::app::state::App;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// 渲染编辑 API Key 的弹窗
pub fn draw_edit(frame: &mut Frame, app: &App) {
    let area = frame.size();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 3,
        width: area.width / 2,
        height: 7,
    };
    frame.render_widget(Clear, popup_area);

    let t = theme::theme();
    let text = vec![
        Line::from("修改 API Key:"),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::raw(app.edit_input.value.clone()),
            Span::styled("_", Style::default().fg(t.warning())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("编辑").borders(Borders::ALL));
    frame.render_widget(paragraph, popup_area);
}
