use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::theme::ThemePalette;

pub fn header(
    width: u16,
    model: &str,
    profile: &str,
    session_id: &str,
    permission_mode: &str,
    state_label: &str,
    palette: ThemePalette,
) -> Paragraph<'static> {
    let mut spans = vec![Span::styled(
        " Kcode",
        Style::default()
            .fg(palette.brand)
            .add_modifier(Modifier::BOLD),
    )];

    spans.push(separator(palette));
    spans.push(mode_status(permission_mode, palette));

    if width >= 44 {
        spans.push(separator(palette));
        spans.push(meta("model", model, palette.info));
    }

    if width >= 72 {
        spans.push(separator(palette));
        spans.push(meta(
            "session",
            &short_session_id(session_id),
            palette.text_muted,
        ));
    }

    if width >= 104 {
        spans.push(separator(palette));
        spans.push(meta("profile", profile, palette.text_muted));
    }

    if width >= 132 && state_label != "idle" {
        spans.push(separator(palette));
        spans.push(meta("state", state_label, palette.accent));
    }

    Paragraph::new(vec![Line::from(spans)]).style(Style::default().bg(palette.panel_bg))
}

fn meta(label: &str, value: &str, color: Color) -> Span<'static> {
    Span::styled(format!("{label}:{value}"), Style::default().fg(color))
}

fn separator(palette: ThemePalette) -> Span<'static> {
    Span::styled("  ·  ", Style::default().fg(palette.text_muted))
}

fn mode_status(permission_mode: &str, palette: ThemePalette) -> Span<'static> {
    let (label, color) = match permission_mode {
        "danger-full-access" | "allow" | "danger" => ("⏵⏵ accept edits on", palette.warning),
        "workspace-write" => ("workspace write", palette.success),
        "plan" => ("plan mode", palette.accent),
        "read-only" | "default" | "prompt" => ("ask before edits", palette.text_muted),
        other => (other, palette.warning),
    };
    Span::styled(label.to_string(), Style::default().fg(color))
}

fn short_session_id(session_id: &str) -> String {
    const MAX_CHARS: usize = 20;
    let count = session_id.chars().count();
    if count <= MAX_CHARS {
        return session_id.to_string();
    }

    let mut short = session_id.chars().take(MAX_CHARS - 1).collect::<String>();
    short.push('…');
    short
}
