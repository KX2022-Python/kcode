use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{EditorState, FieldId, FieldRow, TuiApp};
use super::render_parts::{
    detail_panel, display_value, palette, section_badge, value_style, Palette,
};
use super::state::{Section, ThemePreset};

pub(crate) fn draw(frame: &mut Frame<'_>, app: &TuiApp) {
    let palette = palette(app.settings().appearance.theme);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(16),
            Constraint::Length(3),
        ])
        .split(frame.area());

    frame.render_widget(header(app, palette), layout[0]);
    render_body(frame, app, palette, layout[1]);
    frame.render_widget(footer(app, palette), layout[2]);
    if let Some(editor) = app.editor() {
        render_editor(frame, editor, palette);
    }
}

fn render_body(frame: &mut Frame<'_>, app: &TuiApp, palette: Palette, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(36),
            Constraint::Length(34),
        ])
        .split(area);

    render_sections(frame, app, palette, columns[0]);
    let selected_row = render_fields(frame, app, palette, columns[1]);
    frame.render_widget(
        detail_panel(app, selected_row.as_ref(), palette),
        columns[2],
    );
}

fn render_sections(frame: &mut Frame<'_>, app: &TuiApp, palette: Palette, area: Rect) {
    let items = Section::ALL
        .iter()
        .map(|section| {
            let label = if *section == app.section() {
                format!("{}  •", section.title())
            } else {
                section.title().to_string()
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    section_badge(*section),
                    Style::default()
                        .fg(if *section == app.section() {
                            palette.accent
                        } else {
                            palette.muted
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(label, Style::default().fg(palette.text)),
            ]))
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    state.select(
        Section::ALL
            .iter()
            .position(|section| *section == app.section()),
    );

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(palette.text)
                .bg(palette.panel_alt)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(" Navigate ")
                .borders(Borders::ALL)
                .style(Style::default().bg(palette.surface))
                .border_style(Style::default().fg(palette.border)),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_fields(
    frame: &mut Frame<'_>,
    app: &TuiApp,
    palette: Palette,
    area: Rect,
) -> Option<FieldRow> {
    let rows = app.rows();
    let selected_row = rows.get(app.field_index()).cloned();
    let items = rows
        .iter()
        .map(|row| {
            let label_style = if row.editable {
                Style::default().fg(palette.accent)
            } else {
                Style::default().fg(palette.text_dim)
            };
            let value = display_value(row);
            let line = Line::from(vec![
                Span::styled(format!("{:<16}", row.label), label_style),
                Span::styled(value, value_style(row, palette)),
            ]);
            ListItem::new(line)
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(app.field_index()));
    }

    let title = match app.section() {
        Section::Mcp if !app.settings().mcp.servers.is_empty() => format!(
            "{}  [{}/{}]  n:add x:del [ ]:switch",
            app.section().title(),
            app.settings().mcp.selected + 1,
            app.settings().mcp.servers.len()
        ),
        _ => app.section().title().to_string(),
    };

    let list = List::new(items)
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(palette.text)
                .bg(palette.panel_alt)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .borders(Borders::ALL)
                .style(Style::default().bg(palette.surface))
                .border_style(Style::default().fg(palette.border_active)),
        );
    frame.render_stateful_widget(list, area, &mut state);
    selected_row
}

fn header(app: &TuiApp, palette: Palette) -> Paragraph<'static> {
    let state = if app.is_dirty() { "modified" } else { "saved" };
    let runtime = if app.settings().overview.runtime_ready {
        "ready"
    } else {
        "needs setup"
    };
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Kcode Control Deck",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                format!("section {}", app.section().title()),
                Style::default().fg(palette.text),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("scope {}", app.settings().scope.label()),
                Style::default().fg(palette.accent_alt),
            ),
            Span::raw("   "),
            Span::styled(
                format!("state {state}"),
                Style::default().fg(if app.is_dirty() {
                    palette.warning
                } else {
                    palette.success
                }),
            ),
            Span::raw("   "),
            Span::styled(
                format!("runtime {runtime}"),
                Style::default().fg(if app.settings().overview.runtime_ready {
                    palette.success
                } else {
                    palette.warning
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "First run starts in Provider.",
                Style::default().fg(palette.text),
            ),
            Span::raw(" "),
            Span::styled(
                "Fill endpoint, API key env, and default model before chatting.",
                Style::default().fg(palette.text_dim),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(palette.panel))
            .border_style(Style::default().fg(palette.border_active)),
    )
    .wrap(Wrap { trim: true })
}

fn footer(app: &TuiApp, palette: Palette) -> Paragraph<'_> {
    Paragraph::new(vec![
        Line::from(vec![Span::styled(
            app.status().to_string(),
            Style::default().fg(palette.text),
        )]),
        Line::from(vec![Span::styled(
            "←/→ section  ↑/↓ field  Enter edit/toggle  s save  g scope  r reload  q quit",
            Style::default().fg(palette.text_dim),
        )]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(palette.panel))
            .border_style(Style::default().fg(palette.border)),
    )
    .wrap(Wrap { trim: true })
}

fn render_editor(frame: &mut Frame<'_>, editor: &EditorState, palette: Palette) {
    let area = centered_rect(70, 20, frame.area());
    let text = line_with_cursor(&editor.value, editor.cursor);
    let popup = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            editor.title.clone(),
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        text,
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter apply  Esc cancel  Ctrl+U clear",
            Style::default().fg(palette.text_dim),
        )]),
    ])
    .block(
        Block::default()
            .title(" Edit ")
            .borders(Borders::ALL)
            .style(Style::default().bg(palette.panel))
            .border_style(Style::default().fg(palette.border_active)),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn line_with_cursor(value: &str, cursor: usize) -> Line<'static> {
    let cursor = cursor.min(value.len());
    let (head, tail) = value.split_at(cursor);
    if tail.is_empty() {
        Line::from(vec![
            Span::raw(head.to_string()),
            Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)),
        ])
    } else {
        let (focus, rest) = tail.split_at(1);
        Line::from(vec![
            Span::raw(head.to_string()),
            Span::styled(
                focus.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(rest.to_string()),
        ])
    }
}
