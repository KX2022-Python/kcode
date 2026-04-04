use std::error::Error;
use std::io::{self, IsTerminal};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
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
use super::tool_group::group_tool_calls;
use super::ReplApp;

const KCODE_BANNER: &str = "Kcode";

pub(crate) fn default_welcome_messages(
    model: &str,
    profile: &str,
    permission_mode: &str,
    session_id: &str,
) -> Vec<RenderableMessage> {
    vec![
        RenderableMessage::AssistantText {
            text: format!(
                "{KCODE_BANNER}\n\
  Model        {model}\n\
  Profile      {profile}\n\
  Permissions  {permission_mode}\n\
  Session      {session_id}"
            ),
            streaming: false,
        },
        RenderableMessage::System {
            message: "输入消息开始对话 · `/` 打开命令面板 · Shift+Enter 换行 · PgUp/PgDn 滚动"
                .to_string(),
            level: SysLevel::Info,
        },
    ]
}

pub(crate) fn run_repl<F>(
    model: String,
    profile: String,
    session_id: String,
    permission_mode: String,
    profile_supports_tools: bool,
    welcome_messages: Vec<RenderableMessage>,
    mut executor: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(SubmittedCommand) -> Result<BackendResult, String>,
{
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("kcode repl requires an interactive terminal".into());
    }

    let mut app = ReplApp::new(
        model,
        profile,
        session_id,
        permission_mode,
        profile_supports_tools,
        welcome_messages,
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
        _ => {
            app.add_message(RenderableMessage::User {
                text: command.clone(),
            });
            app.set_state(SessionState::Requesting {
                start: std::time::Instant::now(),
            });
            match executor(SubmittedCommand::Prompt(command)) {
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
            }
        }
    }
}

fn apply_backend_result(app: &mut ReplApp, result: BackendResult) {
    for message in result.messages {
        app.add_message(message);
    }

    if let Some(input_tokens) = result.input_tokens {
        app.usage_input_tokens += input_tokens;
    }
    if let Some(output_tokens) = result.output_tokens {
        app.usage_output_tokens += output_tokens;
    }

    if result.input_tokens.is_some() || result.output_tokens.is_some() {
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
            ),
            layout.header,
        );
    }

    render_notifications(frame, &mut app.notifications, frame.area());
    render_messages(
        frame,
        &display_messages,
        layout.messages,
        &mut app.scroll_offset,
    );

    let input_active = !app.permission_pending
        && !app.dialog.is_active()
        && !app.history_search.active
        && !app.diff_viewer.visible;
    render_prompt_input(frame, &app.input, layout.prompt, input_active);
    render_slash_command_picker(frame, &app.picker, layout.prompt, frame.area());
    render_dialog(frame, &app.dialog, frame.area());
    render_diff_viewer(frame, &app.diff_viewer, frame.area());

    if app.permission_pending {
        let request = PermissionRequest::new(
            "example_tool".to_string(),
            "这是一个演示权限请求".to_string(),
        );
        render_permission_dialog(frame, &request, frame.area(), 0);
    }

    app.footer_pills.has_active_query = app.state.is_active();
    app.footer_pills.has_pending_permission = app.permission_pending;
    app.footer_pills.has_notifications = !app.notifications.is_empty();
    if layout.footer.height > 0 {
        frame.render_widget(app.footer_pills.render(layout.footer.width), layout.footer);
    }
}

#[cfg(test)]
mod tests {
    use super::ReplApp;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn slash_palette_follows_the_current_input() {
        let mut app = ReplApp::new(
            "gpt-4.1".to_string(),
            "default".to_string(),
            "session-1".to_string(),
            "workspace-write".to_string(),
            true,
            Vec::new(),
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
}
