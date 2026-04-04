use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// REPL 全屏布局 — 对齐 CC-Haha FullscreenLayout
///
/// 布局结构：
/// ```
/// ┌─────────────────────────────────┐
/// │ Header (1 line)                 │
/// ├─────────────────────────────────┤
/// │                                 │
/// │  Message Area (flex, grows)     │
/// │                                 │
/// ├─────────────────────────────────┤
/// │ Prompt Input (3 lines)          │
/// ├─────────────────────────────────┤
/// │ Footer (1 line)                 │
/// └─────────────────────────────────┘
/// ```
pub struct ReplLayout {
    pub header: Rect,
    pub messages: Rect,
    pub prompt: Rect,
    pub footer: Rect,
}

pub fn build_layout(area: Rect) -> ReplLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(4),     // Messages (flex)
            Constraint::Length(3),  // Prompt Input
            Constraint::Length(1),  // Footer
        ])
        .split(area);

    ReplLayout {
        header: chunks[0],
        messages: chunks[1],
        prompt: chunks[2],
        footer: chunks[3],
    }
}
