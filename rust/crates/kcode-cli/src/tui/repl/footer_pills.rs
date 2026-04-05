use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::theme::ThemePalette;

pub struct FooterPills {
    pub model: String,
    pub permission_mode: String,
    pub token_usage: Option<TokenUsage>,
    pub session_id: String,
    pub has_active_query: bool,
    pub has_pending_permission: bool,
    pub has_notifications: bool,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl FooterPills {
    pub fn new(model: String, permission_mode: String, session_id: String) -> Self {
        Self {
            model,
            permission_mode,
            token_usage: None,
            session_id,
            has_active_query: false,
            has_pending_permission: false,
            has_notifications: false,
        }
    }

    pub fn render(&self, width: u16, palette: ThemePalette) -> Paragraph<'static> {
        let mut spans = vec![hint("Enter", "send", palette.text_muted)];
        spans.push(separator(palette));
        spans.push(hint("Shift+Enter", "newline", palette.text_muted));

        if width >= 84 {
            spans.push(separator(palette));
            spans.push(hint("/", "commands", palette.info));
        }

        if width >= 114 {
            spans.push(separator(palette));
            spans.push(hint("Ctrl+R", "history", palette.text_muted));
        }

        if width >= 142 {
            spans.push(separator(palette));
            spans.push(hint("PgUp/PgDn", "scroll", palette.text_muted));
        }

        if width >= 166 {
            spans.push(separator(palette));
            spans.push(hint("Ctrl+D", "exit", palette.text_muted));
        }

        if self.has_active_query {
            spans.push(separator(palette));
            spans.push(Span::styled(
                "processing",
                Style::default().fg(palette.accent),
            ));
        }

        if self.has_pending_permission {
            spans.push(separator(palette));
            spans.push(Span::styled(
                "waiting permission",
                Style::default().fg(palette.warning),
            ));
        }
        if self.has_notifications {
            spans.push(separator(palette));
            spans.push(Span::styled("notice", Style::default().fg(palette.error)));
        }

        if let Some(usage) = &self.token_usage {
            if width >= 170 {
                spans.push(separator(palette));
                spans.push(hint(
                    "tokens",
                    &format!("{} in / {} out", usage.input_tokens, usage.output_tokens),
                    palette.text_muted,
                ));
            }
        }

        Paragraph::new(vec![Line::from(spans)]).style(Style::default().bg(palette.panel_bg))
    }
}

fn hint(label: &str, value: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!("{label} {value}"),
        Style::default().fg(color).add_modifier(Modifier::DIM),
    )
}

fn separator(palette: ThemePalette) -> Span<'static> {
    Span::styled("  ·  ", Style::default().fg(palette.text_muted))
}
