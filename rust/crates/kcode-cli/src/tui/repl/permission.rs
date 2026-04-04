use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::PermissionRequest;

/// 权限弹窗渲染 — 对齐 CC-Haha PermissionDialog
pub fn render_permission_dialog(
    frame: &mut Frame<'_>,
    request: &PermissionRequest,
    area: Rect,
    focused_button: usize,
) {
    let lines = vec![
        Line::from(vec![
            Span::styled(
                " ⚠ Permission Required",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Tool: ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &request.tool_name,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Input:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]),
    ];

    // 添加工具输入预览
    let input_lines: Vec<Line> = request
        .input_summary
        .lines()
        .take(4)
        .map(|l| {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(l.to_string(), Style::default().fg(Color::Gray)),
            ])
        })
        .collect();

    let mut all_lines = lines;
    all_lines.extend(input_lines);

    if request.input_summary.lines().count() > 4 {
        all_lines.push(Line::from(vec![Span::styled(
            "  ...",
            Style::default().fg(Color::DarkGray),
        )]));
    }

    all_lines.push(Line::from(""));

    // 按钮行
    let buttons = vec![
        ("[Allow]", 0),
        ("[Allow Always]", 1),
        ("[Deny]", 2),
        ("[Deny Always]", 3),
    ];

    let button_line = Line::from(
        buttons
            .iter()
            .flat_map(|(label, idx)| {
                let style = if *idx == focused_button {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                vec![
                    Span::styled(label.to_string(), style),
                    Span::raw("  "),
                ]
            })
            .collect::<Vec<_>>(),
    );
    all_lines.push(button_line);

    all_lines.push(Line::from(vec![Span::styled(
        "  Tab/←→切换 · Enter确认",
        Style::default().fg(Color::DarkGray),
    )]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(25, 20, 8)));

    let dialog_area = centered_rect(60, 18, area);
    let paragraph = Paragraph::new(all_lines).block(block);

    frame.render_widget(Clear, dialog_area);
    frame.render_widget(paragraph, dialog_area);
}

pub fn handle_permission_key(
    key: KeyEvent,
    focused_button: &mut usize,
) -> Option<PermissionAction> {
    match key.code {
        KeyCode::Enter => {
            let action = match *focused_button {
                0 => PermissionAction::Allow,
                1 => PermissionAction::AllowAlways,
                2 => PermissionAction::Deny,
                3 => PermissionAction::DenyAlways,
                _ => PermissionAction::Allow,
            };
            Some(action)
        }
        KeyCode::Tab | KeyCode::Right => {
            *focused_button = (*focused_button + 1) % 4;
            None
        }
        KeyCode::Left => {
            *focused_button = (*focused_button + 3) % 4;
            None
        }
        _ => None,
    }
}

pub enum PermissionAction {
    Allow,
    AllowAlways,
    Deny,
    DenyAlways,
}

fn centered_rect(width_pct: u16, height_pct: u16, area: Rect) -> Rect {
    let w = (area.width * width_pct) / 100;
    let h = (area.height * height_pct) / 100;
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}
