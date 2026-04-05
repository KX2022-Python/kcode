use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::markdown_render::render_markdown_lines;
use super::state::{RenderableMessage, SysLevel, ToolStatus};
use super::text_layout::wrap_display_text;
use super::theme::ThemePalette;

pub fn render_message(
    msg: &RenderableMessage,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    match msg {
        RenderableMessage::User { text } => render_prefixed_message(
            "> ",
            Style::default()
                .fg(palette.accent)
                .bg(palette.user_msg_bg)
                .add_modifier(Modifier::BOLD),
            text,
            width,
            Style::default().fg(palette.text).bg(palette.user_msg_bg),
        ),
        RenderableMessage::AssistantText { text, streaming } => {
            let mut lines = render_assistant_body(text, *streaming, width, palette);
            if *streaming {
                if let Some(last_line) = lines.last_mut() {
                    last_line.spans.push(Span::styled(
                        " █",
                        Style::default()
                            .fg(palette.inverse_text)
                            .bg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
            lines
        }
        RenderableMessage::AssistantThinking { text } => render_prefixed_message(
            "thinking ",
            Style::default()
                .fg(palette.info)
                .bg(palette.assistant_msg_bg)
                .add_modifier(Modifier::DIM),
            text,
            width,
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.assistant_msg_bg)
                .add_modifier(Modifier::DIM),
        ),
        RenderableMessage::ToolCall {
            name,
            input,
            status,
        } => render_tool_call(name, input, status, width, palette),
        RenderableMessage::ToolResult {
            name,
            output,
            is_error,
        } => render_tool_result(name, output, *is_error, width, palette),
        RenderableMessage::System { message, level } => {
            render_system(message, level, width, palette)
        }
        RenderableMessage::CompactBoundary => vec![Line::from(vec![Span::styled(
            "──── context compacted ────",
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::DIM),
        )])],
        RenderableMessage::Error { message } => {
            render_system(message, &SysLevel::Error, width, palette)
        }
        RenderableMessage::Usage {
            input_tokens,
            output_tokens,
            cost,
        } => render_usage(*input_tokens, *output_tokens, cost, width, palette),
    }
}

fn render_prefixed_message(
    prefix: &str,
    prefix_style: Style,
    body: &str,
    width: u16,
    body_style: Style,
) -> Vec<Line<'static>> {
    let prefix_width = prefix.chars().count().max(1);
    let available = width.saturating_sub(prefix_width as u16).max(1) as usize;
    let wrapped = wrap_display_text(body, available);
    let continuation = " ".repeat(prefix_width);
    let mut lines = Vec::new();

    for (index, line) in wrapped.into_iter().enumerate() {
        let mut spans = Vec::new();
        if index == 0 {
            spans.push(Span::styled(prefix.to_string(), prefix_style));
        } else {
            spans.push(Span::styled(
                continuation.clone(),
                Style::default().bg(prefix_style.bg.unwrap_or(Color::Reset)),
            ));
        }
        spans.push(Span::styled(line, body_style));
        lines.push(Line::from(spans));
    }

    lines
}

fn render_assistant_body(
    text: &str,
    streaming: bool,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let content = if text.is_empty() && streaming {
        "..."
    } else {
        text
    };
    render_markdown_lines(content, width, palette)
}

fn render_tool_call(
    name: &str,
    input: &str,
    status: &ToolStatus,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let (symbol, color) = match status {
        ToolStatus::Pending | ToolStatus::Running => ("● ", palette.warning),
        ToolStatus::Completed => ("✓ ", palette.success),
        ToolStatus::Denied => ("✕ ", palette.error),
    };
    let mut lines = render_prefixed_message(
        symbol,
        Style::default()
            .fg(color)
            .bg(palette.assistant_msg_bg)
            .add_modifier(Modifier::BOLD),
        name,
        width,
        Style::default()
            .fg(palette.text)
            .bg(palette.assistant_msg_bg),
    );
    lines.extend(render_body(
        input,
        width,
        "  ",
        Style::default()
            .fg(palette.text_muted)
            .bg(palette.assistant_msg_bg),
    ));
    lines
}

fn render_tool_result(
    name: &str,
    output: &str,
    is_error: bool,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let (symbol, color) = if is_error {
        ("✕ ", palette.error)
    } else {
        ("✓ ", palette.success)
    };
    let mut lines = render_prefixed_message(
        symbol,
        Style::default()
            .fg(color)
            .bg(palette.assistant_msg_bg)
            .add_modifier(Modifier::BOLD),
        name,
        width,
        Style::default()
            .fg(palette.text)
            .bg(palette.assistant_msg_bg),
    );
    lines.extend(render_body(
        output,
        width,
        "  ",
        Style::default()
            .fg(palette.text_muted)
            .bg(palette.assistant_msg_bg),
    ));
    lines
}

fn render_system(
    message: &str,
    level: &SysLevel,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let color = match level {
        SysLevel::Info => palette.info,
        SysLevel::Warning => palette.warning,
        SysLevel::Error => palette.error,
        SysLevel::Success => palette.success,
    };
    render_prefixed_message(
        "· ",
        Style::default().fg(color).add_modifier(Modifier::DIM),
        message,
        width,
        Style::default().fg(color).add_modifier(Modifier::DIM),
    )
}

fn render_usage(
    input_tokens: u64,
    output_tokens: u64,
    cost: &str,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    render_prefixed_message(
        "· ",
        Style::default()
            .fg(palette.info)
            .add_modifier(Modifier::DIM),
        &format!(
            "tokens: {} in / {} out  cost: {}",
            input_tokens, output_tokens, cost
        ),
        width,
        Style::default()
            .fg(palette.info)
            .add_modifier(Modifier::DIM),
    )
}

fn render_body(text: &str, width: u16, prefix: &str, style: Style) -> Vec<Line<'static>> {
    let available = width.saturating_sub(prefix.chars().count() as u16).max(1) as usize;
    wrap_display_text(text, available)
        .into_iter()
        .map(|line| {
            let mut spans = Vec::new();
            if !prefix.is_empty() {
                spans.push(Span::styled(
                    prefix.to_string(),
                    Style::default().bg(style.bg.unwrap_or(Color::Reset)),
                ));
            }
            spans.push(Span::styled(line, style));
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::render_message;
    use crate::tui::repl::state::RenderableMessage;
    use crate::tui::repl::theme::ThemePreset;

    #[test]
    fn assistant_messages_keep_full_content_when_wrapped() {
        let text = "abcdefghijklmnopqrstuvwxyz0123456789";
        let lines = render_message(
            &RenderableMessage::AssistantText {
                text: text.to_string(),
                streaming: false,
            },
            12,
            ThemePreset::Default.palette(),
        );

        let rendered = lines
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|span| span.content.into_owned()))
            .collect::<Vec<_>>()
            .join("")
            .replace(char::is_whitespace, "");

        assert!(rendered.contains(&text.replace(char::is_whitespace, "")));
    }

    #[test]
    fn fenced_code_blocks_keep_visible_styling() {
        let lines = render_message(
            &RenderableMessage::AssistantText {
                text: "```rust\nfn main() {}\n```".to_string(),
                streaming: false,
            },
            40,
            ThemePreset::Default.palette(),
        );

        let visible = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("");

        assert!(visible.contains("fn main() {}"));
        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.style.bg.is_some()));
    }
}
