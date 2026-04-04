use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn header(
    width: u16,
    model: &str,
    profile: &str,
    session_id: &str,
    permission_mode: &str,
    state_label: &str,
) -> Paragraph<'static> {
    let mut spans = vec![Span::styled(
        " Kcode ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    spans.push(Span::raw(" "));
    spans.push(pill("model", model, Color::Green));
    spans.push(Span::raw(" "));
    spans.push(pill("mode", permission_mode, Color::Yellow));

    if width >= 90 {
        spans.push(Span::raw(" "));
        spans.push(pill("state", state_label, Color::Magenta));
    }

    if width >= 120 {
        spans.push(Span::raw(" "));
        spans.push(pill("profile", profile, Color::Gray));
    }

    if width >= 150 {
        let short_session = session_id.chars().take(12).collect::<String>();
        spans.push(Span::raw(" "));
        spans.push(pill("session", &short_session, Color::Gray));
    }

    Paragraph::new(vec![Line::from(spans)]).style(Style::default().bg(Color::Rgb(18, 28, 20)))
}

fn pill(label: &str, value: &str, color: Color) -> Span<'static> {
    Span::styled(format!("{label}:{value}"), Style::default().fg(color))
}
