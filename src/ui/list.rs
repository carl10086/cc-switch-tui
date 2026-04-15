use crate::app::state::App;
use crate::dao::Dao;
use crate::ui::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// 渲染主界面：左侧实例列表 + 右侧信息面板 + 底部帮助栏
pub fn draw_list<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.size());

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[0]);

    draw_instance_list(frame, content_layout[0], app);
    draw_info_panel(frame, content_layout[1], app);
    draw_help_bar(frame, main_layout[1], app);

    if let Some(ref msg) = app.error_message {
        draw_error_popup(frame, msg);
    }
}

fn draw_instance_list<D: Dao>(frame: &mut Frame, area: ratatui::layout::Rect, app: &App<D>) {
    let t = theme::theme();
    let templates = app.dao.get_templates();
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_index = 0;

    for template in templates {
        let group_instances: Vec<_> = template.models.iter()
            .filter_map(|m| {
                let id = format!("{}-{}", template.id, m.id);
                app.dao.get_instance(&id)
            })
            .collect();

        if group_instances.is_empty() {
            continue;
        }

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("[{}]", template.name),
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ])));

        for instance in group_instances {
            let model = template.models.iter()
                .find(|m| m.id == instance.model_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Unknown");

            let is_selected = flat_index == app.list_index;
            let style = if is_selected {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::raw(model),
            ])).style(style));
            flat_index += 1;
        }
    }

    let list = List::new(items)
        .block(Block::default().title("实例列表").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn draw_info_panel<D: Dao>(frame: &mut Frame, area: ratatui::layout::Rect, app: &App<D>) {
    let t = theme::theme();
    let mut text = vec![];

    let focus_index = match &app.state {
        crate::app::state::AppState::EditInfoPanel { focus_index, .. } => Some(*focus_index),
        _ => None,
    };

    if let Some(instance) = app.current_instance() {
        if let Some(template) = app.dao.get_template(&instance.template_id) {
            let model = template.models.iter()
                .find(|m| m.id == instance.model_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Unknown");

            text.push(Line::from(vec![Span::styled(
                "实例详情",
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            )]));
            text.push(Line::from(""));
            text.push(Line::from(format!("ID: {}", instance.id)));
            text.push(Line::from(format!("Provider: {}", template.name)));
            text.push(Line::from(format!("Model: {}", model)));
            text.push(Line::from(""));

            // Alias 字段（可编辑）
            let alias_display = if instance.alias.is_empty() {
                "(未设置)".to_string()
            } else {
                instance.alias.clone()
            };
            let alias_style = if focus_index == Some(0) {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            text.push(Line::from(vec![
                Span::raw("Alias: "),
                Span::styled(alias_display, alias_style),
            ]));

            // API Key 字段（可编辑）
            let api_key_masked = format!("{}*******",
                &instance.api_key.chars().take(3).collect::<String>());
            let api_style = if focus_index == Some(1) {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            text.push(Line::from(vec![
                Span::raw("API Key: "),
                Span::styled(api_key_masked, api_style),
            ]));

            text.push(Line::from(""));
            text.push(Line::from(vec![Span::styled(
                "环境变量",
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            )]));
            text.push(Line::from(""));

            let mut env = template.default_env.clone();
            if let Some(m) = template.models.iter().find(|m| m.id == instance.model_id) {
                env.extend(m.env_overrides.clone());
            }
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), instance.api_key.clone());

            let mut keys: Vec<_> = env.keys().collect();
            keys.sort();
            for key in keys {
                let value = if key == "ANTHROPIC_AUTH_TOKEN" {
                    format!("{}*******", &env.get(key).unwrap().chars().take(3).collect::<String>())
                } else {
                    env.get(key).unwrap().clone()
                };
                text.push(Line::from(format!("{}={}", key, value)));
            }
        }
    } else {
        text.push(Line::from("暂无实例，按 n 新建"));
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("信息面板").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_help_bar<D: Dao>(frame: &mut Frame, area: ratatui::layout::Rect, app: &App<D>) {
    let t = theme::theme();
    let help = match &app.state {
        crate::app::state::AppState::EditInfoPanel { .. } => {
            "↑↓:切换字段  Enter:编辑  Esc:退出编辑"
        }
        _ => "↑↓:移动  Enter:激活  n:新建  e:编辑详情  d:删除  q:退出",
    };
    let paragraph = Paragraph::new(help)
        .style(Style::default().bg(t.muted()).fg(t.selection_fg()));
    frame.render_widget(paragraph, area);
}

fn draw_error_popup(frame: &mut Frame, msg: &str) {
    let t = theme::theme();
    let area = frame.size();
    let popup_area = ratatui::layout::Rect {
        x: area.width / 4,
        y: area.height / 2 - 2,
        width: area.width / 2,
        height: 5,
    };
    frame.render_widget(Clear, popup_area);
    let paragraph = Paragraph::new(msg)
        .block(Block::default().title("错误").borders(Borders::ALL))
        .style(Style::default().fg(t.error()));
    frame.render_widget(paragraph, popup_area);
}
