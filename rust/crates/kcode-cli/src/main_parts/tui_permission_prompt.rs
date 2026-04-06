#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiPermissionChoice {
    AllowOnce,
    AllowForTurn,
    DenyOnce,
    DenyForTurn,
}

fn permission_memory_key(request: &runtime::PermissionRequest) -> String {
    format!("{}::{}", request.tool_name, request.required_mode.as_str())
}

fn prompt_tui_permission(
    request: &runtime::PermissionRequest,
    current_mode: PermissionMode,
) -> Result<TuiPermissionChoice, String> {
    ensure_permission_terminal_size()?;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(
        io::stdout(),
    ))
    .map_err(|error| error.to_string())?;
    let mut focused_button = 0usize;

    loop {
        terminal
            .draw(|frame| render_tui_permission_prompt(frame, request, current_mode, focused_button))
            .map_err(|error| error.to_string())?;

        match crossterm::event::read().map_err(|error| error.to_string())? {
            crossterm::event::Event::Key(key) => {
                if let Some(choice) = handle_tui_permission_prompt_key(key, &mut focused_button) {
                    return Ok(choice);
                }
            }
            crossterm::event::Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn ensure_permission_terminal_size() -> Result<(), String> {
    let (width, height) = crossterm::terminal::size().map_err(|error| error.to_string())?;
    if permission_prompt_area_usable(ratatui::layout::Rect::new(0, 0, width, height)) {
        Ok(())
    } else {
        Err("TUI permission prompt requires a terminal with non-zero size".to_string())
    }
}

fn permission_prompt_area_usable(area: ratatui::layout::Rect) -> bool {
    area.width > 1 && area.height > 1
}

fn handle_tui_permission_prompt_key(
    key: crossterm::event::KeyEvent,
    focused_button: &mut usize,
) -> Option<TuiPermissionChoice> {
    match key.code {
        crossterm::event::KeyCode::Char('y') => Some(TuiPermissionChoice::AllowOnce),
        crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Esc => {
            Some(TuiPermissionChoice::DenyOnce)
        }
        crossterm::event::KeyCode::Tab | crossterm::event::KeyCode::Right => {
            *focused_button = (*focused_button + 1) % 4;
            None
        }
        crossterm::event::KeyCode::Left => {
            *focused_button = (*focused_button + 3) % 4;
            None
        }
        crossterm::event::KeyCode::Enter => Some(match *focused_button {
            0 => TuiPermissionChoice::AllowOnce,
            1 => TuiPermissionChoice::AllowForTurn,
            2 => TuiPermissionChoice::DenyOnce,
            _ => TuiPermissionChoice::DenyForTurn,
        }),
        _ => None,
    }
}

fn render_tui_permission_prompt(
    frame: &mut ratatui::Frame<'_>,
    request: &runtime::PermissionRequest,
    current_mode: PermissionMode,
    focused_button: usize,
) {
    if !permission_prompt_area_usable(frame.area()) {
        return;
    }
    let preview_lines = request
        .input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(4)
        .map(|line| {
            ratatui::text::Line::from(vec![
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(
                    line.to_string(),
                    ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                ),
            ])
        })
        .collect::<Vec<_>>();

    let mut lines = vec![
        ratatui::text::Line::from(ratatui::text::Span::styled(
            " Permission Required",
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(format!("Tool: {}", request.tool_name)),
        ratatui::text::Line::from(format!("Current mode: {}", current_mode.as_str())),
        ratatui::text::Line::from(format!("Required mode: {}", request.required_mode.as_str())),
    ];

    if let Some(reason) = request.reason.as_deref() {
        lines.push(ratatui::text::Line::from(format!("Reason: {reason}")));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from("Input preview:"));
    if preview_lines.is_empty() {
        lines.push(ratatui::text::Line::from("  <empty>"));
    } else {
        lines.extend(preview_lines);
    }
    lines.push(ratatui::text::Line::from(""));

    let buttons = [
        ("[Allow once]", 0usize),
        ("[Allow turn]", 1usize),
        ("[Deny once]", 2usize),
        ("[Deny turn]", 3usize),
    ];
    let button_line = ratatui::text::Line::from(
        buttons
            .iter()
            .flat_map(|(label, index)| {
                let style = if *index == focused_button {
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Black)
                        .bg(ratatui::style::Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Gray)
                };
                vec![
                    ratatui::text::Span::styled((*label).to_string(), style),
                    ratatui::text::Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>(),
    );
    lines.push(button_line);
    lines.push(ratatui::text::Line::from(
        "  Tab/←→ 切换 · Enter 确认 · y 允许一次 · n 拒绝一次",
    ));

    let dialog_area = centered_tui_permission_rect(frame.area());
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        )
        .title("TUI Permission");
    let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);

    frame.render_widget(ratatui::widgets::Clear, dialog_area);
    frame.render_widget(paragraph, dialog_area);
}

fn centered_tui_permission_rect(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let width = area.width.saturating_mul(3).saturating_div(4).max(60);
    let height = 16;
    ratatui::layout::Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width.saturating_sub(2).max(1)),
        height: height.min(area.height.saturating_sub(2).max(1)),
    }
}

#[cfg(test)]
mod tui_permission_prompt_tests {
    use super::{
        handle_tui_permission_prompt_key, permission_memory_key, TuiPermissionChoice,
        TuiPermissionPrompter,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use runtime::{PermissionMode, PermissionPromptDecision, PermissionRequest};

    #[test]
    fn permission_prompt_keybinds_cover_shortcuts_and_focus_selection() {
        let mut focused = 0;
        assert_eq!(
            handle_tui_permission_prompt_key(
                KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
                &mut focused,
            ),
            None
        );
        assert_eq!(focused, 1);
        assert_eq!(
            handle_tui_permission_prompt_key(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &mut focused,
            ),
            Some(TuiPermissionChoice::AllowForTurn)
        );
        assert_eq!(
            handle_tui_permission_prompt_key(
                KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
                &mut focused,
            ),
            Some(TuiPermissionChoice::AllowOnce)
        );
        assert_eq!(
            handle_tui_permission_prompt_key(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                &mut focused,
            ),
            Some(TuiPermissionChoice::DenyOnce)
        );
    }

    #[test]
    fn cached_tui_permission_decisions_apply_for_rest_of_turn() {
        let request = PermissionRequest {
            tool_name: "write_file".to_string(),
            input: "{\"path\":\"demo.txt\"}".to_string(),
            current_mode: PermissionMode::ReadOnly,
            required_mode: PermissionMode::WorkspaceWrite,
            reason: None,
        };
        let key = permission_memory_key(&request);
        let mut prompter = TuiPermissionPrompter::new(PermissionMode::ReadOnly);

        prompter.allow_for_turn.insert(key.clone());
        assert_eq!(
            prompter.cached_decision(&request),
            Some(PermissionPromptDecision::Allow)
        );

        prompter.allow_for_turn.clear();
        prompter.deny_for_turn.insert(key);
        assert!(matches!(
            prompter.cached_decision(&request),
            Some(PermissionPromptDecision::Deny { .. })
        ));
    }

    #[test]
    fn permission_prompt_area_usable_rejects_zero_sized_frames() {
        assert!(!super::permission_prompt_area_usable(
            ratatui::layout::Rect::new(0, 0, 0, 24)
        ));
        assert!(!super::permission_prompt_area_usable(
            ratatui::layout::Rect::new(0, 0, 80, 0)
        ));
        assert!(super::permission_prompt_area_usable(
            ratatui::layout::Rect::new(0, 0, 80, 24)
        ));
    }
}
