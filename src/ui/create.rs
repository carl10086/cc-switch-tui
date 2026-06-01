use crate::app::state::{App, AppState};
use crate::dao::Dao;
use crate::domain::ModelTemplate;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// 根据当前 AppState 渲染新建向导的对应页面
pub fn draw_create<D: Dao>(frame: &mut Frame, app: &App<D>) {
    match &app.state {
        AppState::CreateProvider => draw_provider_select(frame, app),
        AppState::CreateModel { .. } | AppState::EditModel { .. } => {
            draw_model_select(frame, app)
        }
        AppState::CreateApiKey { .. } => draw_api_key_input(frame, app),
        AppState::CreateOpencodeModel { .. } | AppState::EditOpencodeModel { .. } => {
            draw_opencode_model_select(frame, app)
        }
        AppState::CreateAlias { .. } => draw_alias_input(frame, app),
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
    let items: Vec<ListItem> = templates
        .iter()
        .enumerate()
        .map(|(i, template)| {
            let style = if i == app.provider_index {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            ListItem::new(template.name.clone()).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("选择 Provider")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn draw_model_select<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = centered_rect(frame, 40, 12);
    frame.render_widget(Clear, area);

    // CreateModel 用全局 provider_index；EditModel 必须从 instance_id 反查 instance.template_id
    // （不能用 provider_index，编辑场景下两者无任何关联）
    let (template_name, models): (String, Vec<ModelTemplate>) = match &app.state {
        AppState::EditModel { instance_id } => app
            .dao
            .get_instance(instance_id)
            .and_then(|i| app.dao.get_template(&i.template_id))
            .map(|t| (t.name.clone(), t.models.clone()))
            .unwrap_or_else(|| ("Unknown".to_string(), vec![])),
        _ => app
            .current_provider()
            .map(|t| (t.name.clone(), t.models.clone()))
            .unwrap_or_else(|| ("Unknown".to_string(), vec![])),
    };

    let t = theme::theme();
    // 防御性 clamp：models 为空或 model_index 越界时无高亮（避免无意义下标匹配）
    let highlight = if app.model_index < models.len() {
        app.model_index
    } else {
        usize::MAX
    };
    let items: Vec<ListItem> = models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = if i == highlight {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            ListItem::new(m.name.clone()).style(style)
        })
        .collect();

    // models 为空时插入一行提示（避免无内容空框让用户困惑）
    let items = if items.is_empty() {
        vec![ListItem::new(Span::styled(
            "(无 model 可选，按 Esc 退出)",
            Style::default().fg(t.muted()),
        ))]
    } else {
        items
    };

    let list = List::new(items).block(
        Block::default()
            .title(format!("选择 Model - {}", template_name))
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn draw_opencode_model_select<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = centered_rect(frame, 40, 12);
    frame.render_widget(Clear, area);

    let t = theme::theme();
    let models = match &app.state {
        AppState::EditOpencodeModel { instance_id } => app
            .dao
            .get_instance(instance_id)
            .map(|i| app.get_opencode_models_for_provider_id(&i.template_id))
            .unwrap_or_default(),
        _ => app.get_opencode_models_for_current_provider(),
    };
    let items: Vec<ListItem> = models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            // 防御性 clamp：opencode_model_index 越界时无高亮
            let highlight = if app.opencode_model_index < models.len() {
                app.opencode_model_index
            } else {
                usize::MAX
            };
            let style = if i == highlight {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            ListItem::new(m.clone()).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("选择 OpenCode Model")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
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

    let paragraph =
        Paragraph::new(text).block(Block::default().title("输入 API Key").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn draw_alias_input<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let area = centered_rect(frame, 50, 7);
    frame.render_widget(Clear, area);

    let t = theme::theme();
    let text = vec![
        Line::from("请输入别名（必须以 cl- 开头）："),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::raw(app.edit_input.value.clone()),
            Span::styled("_", Style::default().fg(t.warning())),
        ]),
    ];

    let paragraph =
        Paragraph::new(text).block(Block::default().title("输入别名").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
