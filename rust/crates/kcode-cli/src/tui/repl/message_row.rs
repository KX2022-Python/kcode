use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::state::{RenderableMessage, SysLevel, ToolStatus};

/// 渲染单条消息 — 对齐 CC-Haha MessageRow
pub fn render_message(msg: &RenderableMessage) -> Vec<Line<'static>> {
    match msg {
        RenderableMessage::User { text } => render_user_message(text),
        RenderableMessage::AssistantText { text, streaming } => {
            render_assistant_text(text, *streaming)
        }
        RenderableMessage::AssistantThinking { text } => render_thinking(text),
        RenderableMessage::ToolCall {
            name,
            input,
            status,
        } => render_tool_call(name, input, status),
        RenderableMessage::ToolResult {
            name,
            output,
            is_error,
        } => render_tool_result(name, output, *is_error),
        RenderableMessage::System { message, level } => render_system(message, level),
        RenderableMessage::CompactBoundary => render_compact_boundary(),
        RenderableMessage::Error { message } => render_error(message),
        RenderableMessage::Usage {
            input_tokens,
            output_tokens,
            cost,
        } => render_usage(*input_tokens, *output_tokens, cost),
    }
}

fn render_user_message(text: &str) -> Vec<Line<'static>> {
    let preview = truncate_with_ellipsis(text, 120);
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "▌ You",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", preview),
                Style::default().fg(Color::White),
            ),
        ]),
    ]
}

fn render_assistant_text(text: &str, streaming: bool) -> Vec<Line<'static>> {
    if text.is_empty() && streaming {
        return vec![Line::from(vec![Span::styled(
            "▌ Assistant ...",
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
        )])];
    }
    let preview = truncate_with_ellipsis(text, 200);
    let mut lines = vec![Line::from("")];
    for line in preview.lines() {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                line.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    if streaming {
        if let Some(last) = lines.last_mut() {
            last.spans.push(Span::styled(
                " █",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ));
        }
    }
    lines
}

fn render_thinking(text: &str) -> Vec<Line<'static>> {
    let preview = truncate_with_ellipsis(text, 100);
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "▌ 💭 thinking",
                Style::default()
                    .fg(Color::Rgb(150, 120, 200))
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                format!(" {}", preview),
                Style::default()
                    .fg(Color::Rgb(150, 120, 200))
                    .add_modifier(Modifier::DIM),
            ),
        ]),
    ]
}

fn render_tool_call(name: &str, input: &str, status: &ToolStatus) -> Vec<Line<'static>> {
    let (icon, style) = match status {
        ToolStatus::Pending => ("○", Style::default().fg(Color::Yellow)),
        ToolStatus::Running => (
            "◉",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        ToolStatus::Completed => ("●", Style::default().fg(Color::Green)),
        ToolStatus::Denied => ("✗", Style::default().fg(Color::Red)),
    };
    let input_preview = truncate_with_ellipsis(input, 80);
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} {}", icon, name),
                style.add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", input_preview),
                Style::default().fg(Color::Gray),
            ),
        ]),
    ]
}

fn render_tool_result(name: &str, output: &str, is_error: bool) -> Vec<Line<'static>> {
    let (icon, color) = if is_error {
        ("✗", Color::Red)
    } else {
        ("✓", Color::Green)
    };
    let preview = truncate_with_ellipsis(output, 100);
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} {} →", icon, name),
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", preview),
                Style::default().fg(Color::Gray),
            ),
        ]),
    ]
}

fn render_system(message: &str, level: &SysLevel) -> Vec<Line<'static>> {
    let color = match level {
        SysLevel::Info => Color::Cyan,
        SysLevel::Warning => Color::Yellow,
        SysLevel::Error => Color::Red,
        SysLevel::Success => Color::Green,
    };
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("⚙ {}", message),
                Style::default().fg(color).add_modifier(Modifier::DIM),
            ),
        ]),
    ]
}

fn render_compact_boundary() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "───── context compacted ─────",
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
        )]),
    ]
}

fn render_error(message: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("✗ {}", message),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ]
}

fn render_usage(input_tokens: u64, output_tokens: u64, cost: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(
                    "⚡ tokens: {} in / {} out",
                    input_tokens, output_tokens
                ),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            ),
            Span::raw("  "),
            Span::styled(
                format!("cost: {}", cost),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM),
            ),
        ]),
    ]
}

fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}
