use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Footer 快捷键提示 — 对齐 CC-Haha 底部状态栏
pub fn footer(
    is_active: bool,
    has_picker: bool,
    state_label: &str,
) -> ratatui::widgets::Paragraph<'static> {
    let hints = if has_picker {
        "↑↓选择 · Enter执行 · Esc取消 · Tab补全"
    } else if is_active {
        "Enter发送 · Ctrl+C中断 · ↑↓历史 · Ctrl+R搜索 · /命令 · Ctrl+D退出"
    } else {
        "输入消息开始对话"
    };

    let state_indicator = match state_label {
        "requesting" | "thinking" | "responding" | "tool_use" | "tool_running" => {
            Span::styled(
                format!(" ● {}", state_label),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
        }
        "waiting_permission" => Span::styled(
            " ⚠ waiting_permission",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        "error" => Span::styled(
            " ✗ error",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        _ => Span::styled(
            " · ready",
            Style::default().fg(Color::Gray),
        ),
    };

    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                hints,
                Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
            ),
            state_indicator,
        ]),
    ])
    .style(ratatui::style::Style::default().bg(Color::Rgb(18, 28, 20)))
}
