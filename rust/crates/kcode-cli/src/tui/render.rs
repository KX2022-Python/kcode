use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{EditorState, FieldId, TuiApp};
use super::state::{Section, ThemePreset};

#[derive(Debug, Clone, Copy)]
struct Palette {
    accent: Color,
    accent_soft: Color,
    panel: Color,
    muted: Color,
    text: Color,
}

pub(crate) fn draw(frame: &mut Frame<'_>, app: &TuiApp) {
    let palette = palette(app.settings().appearance.theme);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
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
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(20)])
        .split(area);

    let items = Section::ALL
        .iter()
        .map(|section| {
            let mut label = section.title().to_string();
            if *section == app.section() {
                label.push_str("  •");
            }
            ListItem::new(Line::from(label))
        })
        .collect::<Vec<_>>();
    let mut section_state = ListState::default();
    section_state.select(
        Section::ALL
            .iter()
            .position(|section| *section == app.section()),
    );
    let section_list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(palette.accent)
                .bg(palette.panel)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title("Sections")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.accent_soft)),
        );
    frame.render_stateful_widget(section_list, sections[0], &mut section_state);

    let rows = app.rows();
    let items = rows
        .iter()
        .map(|row| {
            let label_style = if row.editable {
                Style::default().fg(palette.accent)
            } else {
                Style::default().fg(palette.muted)
            };
            let value = if row.value.trim().is_empty() && row.id != FieldId::ReadOnly {
                "<unset>".to_string()
            } else {
                row.value.clone()
            };
            let line = if row.id == FieldId::ReadOnly {
                Line::from(vec![
                    Span::styled(format!("{:<16}", row.label), label_style),
                    Span::styled(value, Style::default().fg(palette.text)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!("{:<16}", row.label), label_style),
                    Span::styled(value, Style::default().fg(palette.text)),
                ])
            };
            ListItem::new(line)
        })
        .collect::<Vec<_>>();
    let mut field_state = ListState::default();
    if !rows.is_empty() {
        field_state.select(Some(app.field_index()));
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
    let row_list = List::new(items)
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(palette.text)
                .bg(palette.panel)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.accent_soft)),
        );
    frame.render_stateful_widget(row_list, sections[1], &mut field_state);
}

fn header(app: &TuiApp, palette: Palette) -> Paragraph<'static> {
    let state = if app.is_dirty() { "modified" } else { "saved" };
    let line = Line::from(vec![
        Span::styled(
            "Kcode Configure",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("scope: {}", app.settings().scope.label()),
            Style::default().fg(palette.text),
        ),
        Span::raw("  "),
        Span::styled(
            format!("state: {state}"),
            Style::default().fg(palette.muted),
        ),
    ]);
    Paragraph::new(vec![
        line,
        Line::from("完整终端设置界面，保存后直接作用于 config.toml 与 bridge.env。"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent_soft)),
    )
    .wrap(Wrap { trim: true })
}

fn footer(app: &TuiApp, palette: Palette) -> Paragraph<'_> {
    let hint = "←/→ 切页 · ↑/↓ 选项 · Enter 编辑/切换 · s 保存 · g 作用域 · r 重载 · q 退出";
    Paragraph::new(vec![
        Line::from(vec![Span::styled(
            app.status().to_string(),
            Style::default().fg(palette.text),
        )]),
        Line::from(vec![Span::styled(hint, Style::default().fg(palette.muted))]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent_soft)),
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
            "Enter 应用，Esc 取消，Ctrl+U 清空。",
            Style::default().fg(palette.muted),
        )]),
    ])
    .block(
        Block::default()
            .title("Edit")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent)),
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

fn palette(theme: ThemePreset) -> Palette {
    match theme {
        ThemePreset::Amber => Palette {
            accent: Color::Yellow,
            accent_soft: Color::Rgb(210, 160, 30),
            panel: Color::Rgb(42, 30, 8),
            muted: Color::Gray,
            text: Color::White,
        },
        ThemePreset::Ocean => Palette {
            accent: Color::Cyan,
            accent_soft: Color::Rgb(40, 150, 170),
            panel: Color::Rgb(8, 32, 42),
            muted: Color::Gray,
            text: Color::White,
        },
        ThemePreset::Default => Palette {
            accent: Color::Green,
            accent_soft: Color::Rgb(40, 120, 70),
            panel: Color::Rgb(18, 28, 20),
            muted: Color::Gray,
            text: Color::White,
        },
    }
}
