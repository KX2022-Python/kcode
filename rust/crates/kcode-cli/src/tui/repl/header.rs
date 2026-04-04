use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Header 状态栏 — 对齐 CC-Haha 顶部信息条
pub fn header(
    model: &str,
    profile: &str,
    session_id: &str,
    permission_mode: &str,
    state_label: &str,
) -> ratatui::widgets::Paragraph<'static> {
    let model_line = Line::from(vec![
        Span::styled(
            " Kcode REPL",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("model: {}", model),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  "),
        Span::styled(
            format!("profile: {}", profile),
            Style::default().fg(Color::Gray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("session: {}", session_id),
            Style::default().fg(Color::Gray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("perm: {}", permission_mode),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", state_label),
            Style::default()
                .fg(if state_label == "idle" {
                    Color::Gray
                } else {
                    Color::Magenta
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    ratatui::widgets::Paragraph::new(vec![model_line])
        .style(Style::default().bg(Color::Rgb(18, 28, 20)))
}
