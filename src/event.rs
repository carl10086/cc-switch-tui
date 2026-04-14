use crossterm::event::{Event, KeyEvent, KeyEventKind};

/// 应用级别的键盘事件，屏蔽掉重复的 KeyRelease 事件
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// 键盘按下事件
    Key(KeyEvent),
    /// 终端大小改变事件
    Resize(u16, u16),
    /// 无事件（轮询超时）
    Tick,
}

/// 从 crossterm 的 Event 转换为 AppEvent
pub fn parse_event(event: Event) -> Option<AppEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
        Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
        _ => None,
    }
}
