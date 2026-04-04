use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

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

    pub fn render(&self, width: u16) -> Paragraph<'static> {
        let mut spans = vec![pill("Tab", "commands", Color::Cyan)];
        spans.push(Span::raw(" "));
        spans.push(pill("Shift+Enter", "newline", Color::Gray));

        if width >= 100 {
            spans.push(Span::raw(" "));
            spans.push(pill("PgUp/PgDn", "scroll", Color::Gray));
        }

        if let Some(usage) = &self.token_usage {
            if width >= 130 {
                spans.push(Span::raw(" "));
                spans.push(pill(
                    "tokens",
                    &format!("{} in / {} out", usage.input_tokens, usage.output_tokens),
                    Color::Yellow,
                ));
            }
        }

        if self.has_active_query {
            spans.push(Span::raw(" "));
            spans.push(pill("state", "processing", Color::Magenta));
        }
        if self.has_pending_permission {
            spans.push(Span::raw(" "));
            spans.push(pill("state", "permission", Color::Yellow));
        }
        if self.has_notifications {
            spans.push(Span::raw(" "));
            spans.push(pill("state", "notice", Color::Red));
        }

        Paragraph::new(vec![Line::from(spans)]).style(Style::default().bg(Color::Rgb(18, 28, 20)))
    }
}

fn pill(label: &str, value: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!("{label}:{value}"),
        Style::default().fg(color).add_modifier(Modifier::DIM),
    )
}
