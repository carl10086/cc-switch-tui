use crate::app::state::{App, AppState};
use crate::dao::Dao;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// 根据当前 AppState 渲染新建向导的对应页面
pub fn draw_create<D: Dao>(frame: &mut Frame, app: &App<D>) {
    match &app.state {
        AppState::CreateProvider => draw_provider_select(frame, app),
        AppState::CreateModel { .. } => draw_model_select(frame, app),
        AppState::CreateApiKey { .. } => draw_api_key_input(frame, app),
        _ => {}
    }
}

fn centered_rect(frame: &Frame, width: u16, height: u16) -> Rect {
    let area = frame.size();
    Rect {
        x: area.width.saturating_sub(width) / 2,
        y: area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn draw_provider_select<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = centered_rect(frame, 40, 12);
    frame.render_widget(Clear, area);

    let t = theme::theme();
    let templates = app.dao.get_templates();
    let items: Vec<ListItem> = templates.iter().enumerate().map(|(i, template)| {
        let style = if i == app.provider_index {
            Style::default().bg(t.selection_bg()).fg(t.selection_fg())
        } else {
            Style::default()
        };
        ListItem::new(template.name.clone()).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().title("选择 Provider").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn draw_model_select<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = centered_rect(frame, 40, 12);
    frame.render_widget(Clear, area);

    if let Some(template) = app.current_provider() {
        let t = theme::theme();
        let items: Vec<ListItem> = template.models.iter().enumerate().map(|(i, m)| {
            let style = if i == app.model_index {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            ListItem::new(m.name.clone()).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().title(format!("选择 Model - {}", template.name)).borders(Borders::ALL));
        frame.render_widget(list, area);
    }
}

fn draw_api_key_input<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = centered_rect(frame, 50, 7);
    frame.render_widget(Clear, area);

    let t = theme::theme();
    let text = vec![
        Line::from("请输入 API Key:"),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::raw(app.api_key_input.value.clone()),
            Span::styled("_", Style::default().fg(t.warning())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("输入 API Key").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
