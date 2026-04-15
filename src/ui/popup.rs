use crate::app::state::App;
use crate::dao::Dao;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// 渲染删除确认对话框
pub fn draw_delete_confirm<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = frame.size();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 3,
        width: area.width / 2,
        height: 7,
    };
    frame.render_widget(Clear, popup_area);

    let instance_id = match &app.state {
        crate::app::state::AppState::DeleteConfirm { instance_id } => instance_id.clone(),
        _ => return,
    };

    let t = theme::theme();
    let text = vec![
        Line::from(format!("确定删除 {} 的实例吗？", instance_id)),
        Line::from(""),
        Line::from("[Y] 确认    [N] 取消"),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("确认删除").borders(Borders::ALL))
        .style(Style::default().fg(t.error()));
    frame.render_widget(paragraph, popup_area);
}
