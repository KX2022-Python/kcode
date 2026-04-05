use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::message_row::render_message;
use super::state::RenderableMessage;
use super::theme::{ThemePalette, ThemePreset};

pub fn render_messages(
    frame: &mut Frame<'_>,
    messages: &[RenderableMessage],
    area: Rect,
    scroll_offset: &mut usize,
    palette: ThemePalette,
) {
    if messages.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from("  输入消息开始对话，或输入 / 查看命令"),
        ])
        .style(Style::default().fg(palette.text_muted).bg(palette.panel_bg));
        frame.render_widget(empty, area);
        return;
    }

    let lines = message_lines(messages, area.width, palette);
    let max_offset = lines.len().saturating_sub(area.height as usize);
    *scroll_offset = (*scroll_offset).min(max_offset);

    let paragraph = Paragraph::new(lines)
        .scroll(((*scroll_offset).min(u16::MAX as usize) as u16, 0))
        .style(Style::default().bg(palette.panel_bg));
    frame.render_widget(paragraph, area);
}

pub fn auto_scroll_to_bottom(
    messages: &[RenderableMessage],
    area_height: u16,
    area_width: u16,
) -> usize {
    line_count(messages, area_width).saturating_sub(area_height as usize)
}

fn line_count(messages: &[RenderableMessage], width: u16) -> usize {
    message_lines(messages, width, ThemePreset::Default.palette())
        .len()
        .max(1)
}

fn message_lines(
    messages: &[RenderableMessage],
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let content_width = width.saturating_sub(1).max(1);
    let mut lines = Vec::new();

    for (index, message) in messages.iter().enumerate() {
        if index > 0 {
            lines.push(Line::from(""));
        }
        lines.extend(render_message(message, content_width, palette));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::auto_scroll_to_bottom;
    use crate::tui::repl::state::RenderableMessage;

    #[test]
    fn auto_scroll_changes_with_terminal_width() {
        let messages = vec![RenderableMessage::AssistantText {
            text: "abcdefghijklmnopqrstuvwxyz0123456789".to_string(),
            streaming: false,
        }];

        let narrow = auto_scroll_to_bottom(&messages, 3, 12);
        let wide = auto_scroll_to_bottom(&messages, 3, 80);

        assert!(narrow >= wide);
    }
}
