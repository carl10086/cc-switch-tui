//! UI 颜色主题，基于 Catppuccin Mocha
//!
//! 参考: https://github.com/catppuccin/catppuccin

use ratatui::style::Color;

/// Catppuccin Mocha 调色板
pub struct Theme;

impl Theme {
    /// 亮色文字
    pub fn fg(&self) -> Color {
        Color::Rgb(205, 214, 244) // text
    }

    /// 暗色背景
    pub fn bg(&self) -> Color {
        Color::Rgb(30, 30, 40) // base
    }

    /// 主强调色 (mauve)
    pub fn accent(&self) -> Color {
        Color::Rgb(203, 166, 247)
    }

    /// 选中项背景 (surface2)
    pub fn selection_bg(&self) -> Color {
        Color::Rgb(88, 91, 112)
    }

    /// 选中项文字
    pub fn selection_fg(&self) -> Color {
        Color::Rgb(205, 214, 244) // text
    }

    /// 错误色 (red)
    pub fn error(&self) -> Color {
        Color::Rgb(243, 139, 168)
    }

    /// 警告色 (yellow)
    pub fn warning(&self) -> Color {
        Color::Rgb(249, 226, 175)
    }

    /// 边框/分割线 (overlay0)
    pub fn border(&self) -> Color {
        Color::Rgb(108, 112, 134)
    }

    /// 次要/静音文字 (overlay1)
    pub fn muted(&self) -> Color {
        Color::Rgb(127, 132, 156)
    }

    /// 标题颜色 (sapphire)
    pub fn heading(&self) -> Color {
        Color::Rgb(116, 199, 236)
    }
}

pub fn theme() -> Theme {
    Theme
}
