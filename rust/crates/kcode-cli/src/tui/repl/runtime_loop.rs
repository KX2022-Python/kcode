use std::error::Error;
use std::io::{self, IsTerminal};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use super::command_palette::render_slash_command_picker;
use super::dialog::render_dialog;
use super::diff_viewer::render_diff_viewer;
use super::header::header;
use super::layout::build_layout;
use super::messages::{auto_scroll_to_bottom, render_messages};
use super::notification_render::render_notifications;
use super::permission::render_permission_dialog;
use super::prompt::{prompt_height, render_prompt_input};
use super::state::{
    BackendResult, PermissionRequest, RenderableMessage, SessionState, SubmittedCommand, SysLevel,
};
use super::theme::ThemePreset;
use super::tool_group::group_tool_calls;
use super::ReplApp;

pub(crate) fn default_welcome_messages(
    model: &str,
    profile: &str,
    permission_mode: &str,
    session_id: &str,
) -> Vec<RenderableMessage> {
    vec![
        RenderableMessage::AssistantText {
            text: render_welcome_banner(),
            streaming: false,
        },
        RenderableMessage::System {
            message: format!(
                "{model} · {profile} · {permission_mode} · {}",
                short_session_id(session_id)
            ),
            level: SysLevel::Info,
        },
        RenderableMessage::System {
            message:
                "Enter 发送消息 · Shift+Enter/Ctrl+J 换行 · PgUp/PgDn 或 Ctrl+↑/↓ 滚动 · Ctrl+D 退出"
                    .to_string(),
            level: SysLevel::Info,
        },
    ]
}

pub(crate) fn render_welcome_banner() -> String {
    format!(
        "Kcode v{}\n  Start a task, inspect the workspace, or press `/` to browse commands.\n  /help shows the full command surface and /resume latest jumps back into recent work.",
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn short_session_id(session_id: &str) -> String {
    const MAX_CHARS: usize = 24;
    let count = session_id.chars().count();
    if count <= MAX_CHARS {
        return session_id.to_string();
    }

    let mut short = session_id.chars().take(MAX_CHARS - 1).collect::<String>();
    short.push('…');
    short
}

pub(crate) fn run_repl<F>(
    model: String,
    profile: String,
    session_id: String,
    permission_mode: String,
    profile_supports_tools: bool,
    available_models: Vec<String>,
    welcome_messages: Vec<RenderableMessage>,
    initial_theme: Option<ThemePreset>,
    mut executor: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(SubmittedCommand) -> Result<BackendResult, String>,
{
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("kcode repl requires an interactive terminal".into());
    }
    ensure_terminal_size("kcode repl")?;

    let mut app = ReplApp::new(
        model,
        profile,
        session_id,
        permission_mode,
        profile_supports_tools,
        available_models,
        welcome_messages,
        initial_theme,
    );

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    while event::poll(Duration::from_millis(10)).unwrap_or(false) {
        let _ = event::read();
    }

    let run_result = run_repl_loop(&mut terminal, &mut app, &mut executor);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    run_result
}

fn run_repl_loop<F>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReplApp,
    executor: &mut F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(SubmittedCommand) -> Result<BackendResult, String>,
{
    terminal.draw(|frame| draw_frame(frame, app))?;

    while !app.quit {
        let has_event = event::poll(Duration::from_millis(200)).unwrap_or(false);
        if has_event {
            match event::read() {
                Ok(Event::Key(key)) => {
                    app.handle_key(key);
                    if app.pending_command.is_some() {
                        terminal.draw(|frame| draw_frame(frame, app))?;
                    }
                    process_pending_command(app, executor);
                }
                Ok(Event::Mouse(mouse)) => {
                    app.handle_mouse(mouse);
                    process_pending_command(app, executor);
                }
                Ok(Event::Resize(_, _)) => {
                    let _ = terminal.autoresize();
                }
                Ok(_) | Err(_) => {}
            }
        }
        terminal.draw(|frame| draw_frame(frame, app))?;
    }

    Ok(())
}

fn ensure_terminal_size(context: &str) -> Result<(), Box<dyn Error>> {
    let (width, height) = crossterm::terminal::size()?;
    if terminal_area_usable(Rect::new(0, 0, width, height)) {
        Ok(())
    } else {
        Err(format!("{context} requires a terminal with non-zero size").into())
    }
}

fn terminal_area_usable(area: Rect) -> bool {
    area.width > 1 && area.height > 1
}

fn process_pending_command<F>(app: &mut ReplApp, executor: &mut F)
where
    F: FnMut(SubmittedCommand) -> Result<BackendResult, String>,
{
    let Some(command) = app.pending_command.take() else {
        return;
    };

    match command.as_str() {
        "__permission_allow" => {
            app.add_message(RenderableMessage::System {
                message: "权限已授予".to_string(),
                level: SysLevel::Success,
            });
            app.notify_success("权限已授予".to_string());
            app.set_state(SessionState::Idle);
        }
        "__permission_deny" => {
            app.add_message(RenderableMessage::System {
                message: "权限已拒绝".to_string(),
                level: SysLevel::Warning,
            });
            app.notify_warning("权限已拒绝".to_string());
            app.set_state(SessionState::Idle);
        }
        _ if command.starts_with('/') => {
            if matches!(command.as_str(), "/exit" | "/quit") {
                app.quit = true;
                return;
            }
            app.add_message(RenderableMessage::System {
                message: format!("执行命令: {}", command),
                level: SysLevel::Info,
            });
            app.notify_info(format!("执行: {}", command));
            match executor(SubmittedCommand::Slash(command)) {
                Ok(result) => apply_backend_result(app, result),
                Err(error) => {
                    app.add_message(RenderableMessage::Error {
                        message: error.clone(),
                    });
                    app.notify_warning(error);
                    app.set_state(SessionState::Error {
                        message: "slash-command-error".to_string(),
                    });
                }
            }
        }
        _ => match executor(SubmittedCommand::Prompt(command)) {
            Ok(result) => apply_backend_result(app, result),
            Err(error) => {
                app.add_message(RenderableMessage::Error {
                    message: error.clone(),
                });
                app.notify_warning(error);
                app.set_state(SessionState::Error {
                    message: "prompt-error".to_string(),
                });
            }
        },
    }
}

fn apply_backend_result(app: &mut ReplApp, result: BackendResult) {
    let BackendResult {
        messages,
        input_tokens,
        output_tokens,
        ui_state,
    } = result;

    for message in messages {
        app.add_message(message);
    }

    if let Some(ui_state) = ui_state {
        app.sync_runtime_ui_state(ui_state);
    }

    if let Some(input_tokens) = input_tokens {
        app.usage_input_tokens += input_tokens;
    }
    if let Some(output_tokens) = output_tokens {
        app.usage_output_tokens += output_tokens;
    }

    if input_tokens.is_some() || output_tokens.is_some() {
        app.footer_pills.token_usage = Some(super::footer_pills::TokenUsage {
            input_tokens: app.usage_input_tokens,
            output_tokens: app.usage_output_tokens,
        });
    }

    if matches!(
        app.state,
        SessionState::Requesting { .. } | SessionState::Thinking { .. }
    ) {
        app.set_state(SessionState::Completed {
            summary: "turn-complete".to_string(),
        });
    } else if matches!(app.state, SessionState::Error { .. }) {
    } else {
        app.set_state(SessionState::Idle);
    }
}

fn draw_frame(frame: &mut ratatui::Frame<'_>, app: &mut ReplApp) {
    if !terminal_area_usable(frame.area()) {
        return;
    }
    let prompt_h = prompt_height(&app.input, frame.area().width);
    let layout = build_layout(frame.area(), prompt_h);
    app.set_message_viewport(layout.messages);
    let display_messages = if app.tools_collapsed {
        group_tool_calls(&app.messages)
    } else {
        app.messages.clone()
    };

    if layout.header.height > 0 {
        frame.render_widget(
            header(
                layout.header.width,
                &app.model,
                &app.profile,
                &app.session_id,
                &app.permission_mode_label,
                app.state.label(),
                app.palette,
            ),
            layout.header,
        );
    }

    render_notifications(frame, &mut app.notifications, frame.area(), app.palette);
    render_messages(
        frame,
        &display_messages,
        layout.messages,
        &mut app.scroll_offset,
        app.palette,
    );

    let input_active = !app.permission_pending
        && !app.dialog.is_active()
        && !app.history_search.active
        && !app.diff_viewer.visible;
    render_prompt_input(frame, &app.input, layout.prompt, input_active, app.palette);
    render_slash_command_picker(frame, &app.picker, layout.prompt, frame.area(), app.palette);
    render_dialog(frame, &app.dialog, frame.area(), app.palette);
    render_diff_viewer(frame, &app.diff_viewer, frame.area(), app.palette);

    if app.permission_pending {
        let request = PermissionRequest::new(
            "example_tool".to_string(),
            "这是一个演示权限请求".to_string(),
        );
        render_permission_dialog(frame, &request, frame.area(), 0, app.palette);
    }

    app.footer_pills.has_active_query = app.state.is_active();
    app.footer_pills.has_pending_permission = app.permission_pending;
    app.footer_pills.has_notifications = !app.notifications.is_empty();
    if layout.footer.height > 0 {
        frame.render_widget(
            app.footer_pills.render(layout.footer.width, app.palette),
            layout.footer,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_backend_result, ReplApp};
    use crate::tui::repl::messages::auto_scroll_to_bottom;
    use crate::tui::repl::state::{BackendResult, RuntimeUiState};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;

    #[test]
    fn terminal_area_usable_rejects_zero_sized_frames() {
        assert!(!super::terminal_area_usable(Rect::new(0, 0, 0, 24)));
        assert!(!super::terminal_area_usable(Rect::new(0, 0, 80, 0)));
        assert!(super::terminal_area_usable(Rect::new(0, 0, 80, 24)));
    }

    #[test]
    fn slash_palette_follows_the_current_input() {
        let mut app = ReplApp::new(
            "gpt-4.1".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

        assert!(app.picker.visible);
        assert_eq!(app.picker.filter, "r");
    }

    #[test]
    fn paging_up_breaks_follow_mode_until_the_user_reaches_bottom_again() {
        let mut app = ReplApp::new(
            "gpt-4.1".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );
        app.message_area_height = 4;
        app.message_area_width = 24;

        for index in 0..10 {
            app.add_message(crate::tui::repl::RenderableMessage::AssistantText {
                text: format!("message-{index}"),
                streaming: false,
            });
        }

        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        assert!(!app.stick_to_bottom);

        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        app.scroll_to_bottom();
        assert!(app.stick_to_bottom);
    }

    #[test]
    fn ctrl_arrow_keys_scroll_messages() {
        let mut app = ReplApp::new(
            "gpt-4.1".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );
        app.message_area_height = 4;
        app.message_area_width = 24;

        for index in 0..12 {
            app.add_message(crate::tui::repl::RenderableMessage::AssistantText {
                text: format!("message-{index}"),
                streaming: false,
            });
        }

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
        assert!(!app.stick_to_bottom);

        let before = app.scroll_offset;
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
        assert!(app.scroll_offset >= before);
    }

    #[test]
    fn backend_results_refresh_runtime_metadata_in_the_tui() {
        let mut app = ReplApp::new(
            "gpt-5.4-mini".to_string(),
            "cliproxyapi".to_string(),
            "session-1".to_string(),
            "danger-full-access".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );

        apply_backend_result(
            &mut app,
            BackendResult {
                ui_state: Some(RuntimeUiState {
                    model: "gpt-5.4".to_string(),
                    profile: "cliproxyapi".to_string(),
                    session_id: "session-2".to_string(),
                    permission_mode_label: "workspace-write".to_string(),
                    profile_supports_tools: false,
                }),
                ..BackendResult::default()
            },
        );

        assert_eq!(app.model, "gpt-5.4");
        assert_eq!(app.session_id, "session-2");
        assert_eq!(app.permission_mode_label, "workspace-write");
        assert!(!app.profile_supports_tools);
        assert_eq!(app.footer_pills.model, "gpt-5.4");
        assert_eq!(app.footer_pills.session_id, "session-2");
        assert_eq!(app.footer_pills.permission_mode, "workspace-write");
    }

    #[test]
    fn resize_preserves_bottom_gap_for_manual_scroll_position() {
        let mut app = ReplApp::new(
            "gpt-4.1".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );

        app.set_message_viewport(Rect {
            x: 0,
            y: 0,
            width: 48,
            height: 6,
        });

        for index in 0..12 {
            app.add_message(crate::tui::repl::RenderableMessage::AssistantText {
                text: format!("message-{index} abcdefghijklmnopqrstuvwxyz0123456789 你好世界"),
                streaming: false,
            });
        }

        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        let initial_offset = app.scroll_offset;
        let initial_gap = auto_scroll_to_bottom(
            &app.messages,
            app.message_area_height,
            app.message_area_width,
        )
        .saturating_sub(app.scroll_offset);

        app.set_message_viewport(Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 6,
        });
        let narrow_gap = auto_scroll_to_bottom(
            &app.messages,
            app.message_area_height,
            app.message_area_width,
        )
        .saturating_sub(app.scroll_offset);
        assert_eq!(narrow_gap, initial_gap);

        app.set_message_viewport(Rect {
            x: 0,
            y: 0,
            width: 48,
            height: 6,
        });
        assert_eq!(app.scroll_offset, initial_offset);
    }

    #[test]
    fn enter_queues_user_prompt_before_executor_runs() {
        let mut app = ReplApp::new(
            "gpt-4.1".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
            Vec::new(),
            None,
        );

        for ch in "hello".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(
            app.state,
            crate::tui::repl::SessionState::Requesting { .. }
        ));
        assert_eq!(app.pending_command.as_deref(), Some("hello"));
        assert!(matches!(
            app.messages.last(),
            Some(crate::tui::repl::RenderableMessage::User { text }) if text == "hello"
        ));
    }
}
