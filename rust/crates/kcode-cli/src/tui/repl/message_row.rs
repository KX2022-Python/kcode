use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::state::{RenderableMessage, SysLevel, ToolStatus};

pub fn render_message(msg: &RenderableMessage, width: u16) -> Vec<Line<'static>> {
    match msg {
        RenderableMessage::User { text } => render_titled_block(
            "You",
            Color::Green,
            text,
            width,
            Style::default().fg(Color::White),
        ),
        RenderableMessage::AssistantText { text, streaming } => {
            let mut lines = render_titled_block(
                "Kcode",
                Color::Cyan,
                if text.is_empty() && *streaming {
                    "..."
                } else {
                    text
                },
                width,
                Style::default().fg(Color::White),
            );
            if *streaming {
                if let Some(last_line) = lines.last_mut() {
                    last_line.spans.push(Span::styled(
                        " █",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
            lines
        }
        RenderableMessage::AssistantThinking { text } => render_titled_block(
            "Thinking",
            Color::Magenta,
            text,
            width,
            Style::default()
                .fg(Color::Rgb(180, 160, 220))
                .add_modifier(Modifier::DIM),
        ),
        RenderableMessage::ToolCall {
            name,
            input,
            status,
        } => render_tool_call(name, input, status, width),
        RenderableMessage::ToolResult {
            name,
            output,
            is_error,
        } => render_tool_result(name, output, *is_error, width),
        RenderableMessage::System { message, level } => render_system(message, level, width),
        RenderableMessage::CompactBoundary => vec![Line::from(vec![Span::styled(
            "──── context compacted ────",
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
        )])],
        RenderableMessage::Error { message } => render_system(message, &SysLevel::Error, width),
        RenderableMessage::Usage {
            input_tokens,
            output_tokens,
            cost,
        } => render_usage(*input_tokens, *output_tokens, cost, width),
    }
}

fn render_titled_block(
    title: &str,
    title_color: Color,
    body: &str,
    width: u16,
    body_style: Style,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .fg(title_color)
            .add_modifier(Modifier::BOLD),
    )])];
    lines.extend(render_body(body, width, "  ", body_style));
    lines
}

fn render_tool_call(
    name: &str,
    input: &str,
    status: &ToolStatus,
    width: u16,
) -> Vec<Line<'static>> {
    let (label, color) = match status {
        ToolStatus::Pending => ("tool pending", Color::Yellow),
        ToolStatus::Running => ("tool running", Color::Yellow),
        ToolStatus::Completed => ("tool complete", Color::Green),
        ToolStatus::Denied => ("tool denied", Color::Red),
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(name.to_string(), Style::default().fg(Color::White)),
    ])];
    lines.extend(render_body(
        input,
        width,
        "  ",
        Style::default().fg(Color::Gray),
    ));
    lines
}

fn render_tool_result(name: &str, output: &str, is_error: bool, width: u16) -> Vec<Line<'static>> {
    let (label, color) = if is_error {
        ("tool error", Color::Red)
    } else {
        ("tool result", Color::Green)
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(name.to_string(), Style::default().fg(Color::White)),
    ])];
    lines.extend(render_body(
        output,
        width,
        "  ",
        Style::default().fg(Color::Gray),
    ));
    lines
}

fn render_system(message: &str, level: &SysLevel, width: u16) -> Vec<Line<'static>> {
    let color = match level {
        SysLevel::Info => Color::Cyan,
        SysLevel::Warning => Color::Yellow,
        SysLevel::Error => Color::Red,
        SysLevel::Success => Color::Green,
    };
    render_body(
        message,
        width,
        "",
        Style::default().fg(color).add_modifier(Modifier::DIM),
    )
}

fn render_usage(
    input_tokens: u64,
    output_tokens: u64,
    cost: &str,
    width: u16,
) -> Vec<Line<'static>> {
    render_body(
        &format!(
            "tokens: {} in / {} out  cost: {}",
            input_tokens, output_tokens, cost
        ),
        width,
        "",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
    )
}

fn render_body(text: &str, width: u16, prefix: &str, style: Style) -> Vec<Line<'static>> {
    let available = width.saturating_sub(prefix.chars().count() as u16).max(1) as usize;
    wrap_text(text, available)
        .into_iter()
        .map(|line| {
            let mut spans = Vec::new();
            if !prefix.is_empty() {
                spans.push(Span::raw(prefix.to_string()));
            }
            spans.push(Span::styled(line, style));
            Line::from(spans)
        })
        .collect()
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    for raw_line in text.lines() {
        if raw_line.is_empty() {
            wrapped.push(String::new());
            continue;
        }
        let chars = raw_line.chars().collect::<Vec<_>>();
        for chunk in chars.chunks(width.max(1)) {
            wrapped.push(chunk.iter().collect());
        }
    }
    if wrapped.is_empty() {
        wrapped.push(String::new());
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::render_message;
    use crate::tui::repl::state::RenderableMessage;

    #[test]
    fn assistant_messages_keep_full_content_when_wrapped() {
        let text = "abcdefghijklmnopqrstuvwxyz0123456789";
        let lines = render_message(
            &RenderableMessage::AssistantText {
                text: text.to_string(),
                streaming: false,
            },
            12,
        );

        let rendered = lines
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|span| span.content.into_owned()))
            .collect::<Vec<_>>()
            .join("")
            .replace(char::is_whitespace, "");

        assert!(rendered.contains(&text.replace(char::is_whitespace, "")));
    }
}
