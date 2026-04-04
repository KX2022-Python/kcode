use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Footer Pills — 对齐 CC-Haha PromptInputFooter 状态指示器系统
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
    pub fn new(
        model: String,
        permission_mode: String,
        session_id: String,
    ) -> Self {
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

    pub fn render(&self) -> Paragraph<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();

        // Model pill
        spans.push(Span::styled(
            format!(" 🤖 {} ", self.model),
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(15, 25, 30)),
        ));

        spans.push(Span::raw(" "));

        // Permission mode pill
        let perm_color = match self.permission_mode.as_str() {
            "allow" | "Allow" => Color::Green,
            "prompt" | "Prompt" => Color::Yellow,
            "danger" => Color::Red,
            _ => Color::Gray,
        };
        spans.push(Span::styled(
            format!(" 🔒 {} ", self.permission_mode),
            Style::default()
                .fg(perm_color)
                .bg(Color::Rgb(25, 22, 8)),
        ));

        spans.push(Span::raw(" "));

        // Session pill
        spans.push(Span::styled(
            format!(" 📋 {} ", self.session_id.chars().take(12).collect::<String>()),
            Style::default()
                .fg(Color::Gray)
                .bg(Color::Rgb(20, 20, 20)),
        ));

        // Token usage pill (if available)
        if let Some(ref usage) = self.token_usage {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!(
                    " ⚡ {}in/{}out ",
                    usage.input_tokens, usage.output_tokens
                ),
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Rgb(25, 22, 8)),
            ));
        }

        // Active query indicator
        if self.has_active_query {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                " ● processing ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Permission pending indicator
        if self.has_pending_permission {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                " ⚠ permission ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Notification indicator
        if self.has_notifications {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                " 🔔 ",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        Paragraph::new(vec![Line::from(spans)])
            .style(Style::default().bg(Color::Rgb(18, 28, 20)))
    }
}
