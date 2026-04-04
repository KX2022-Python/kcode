mod footer;
mod header;
mod layout;
mod message_row;
mod messages;
mod permission;
mod prompt;
mod state;

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

use self::footer::footer;
use self::header::header;
use self::layout::build_layout;
use self::messages::render_messages;
use self::permission::{handle_permission_key, render_permission_dialog, PermissionAction};
use self::prompt::{
    render_prompt_input, render_slash_command_picker, PromptAction, SlashPickerAction,
};
use self::state::{RenderableMessage, SessionState};

pub use self::prompt::{
    default_slash_commands, InputMode, PromptInput, SlashCommandEntry, SlashCommandPicker,
};
pub use self::state::{PermissionDecision, PermissionRequest, SysLevel, ToolStatus};

/// REPL 应用状态
pub struct ReplApp {
    pub messages: Vec<RenderableMessage>,
    pub state: SessionState,
    pub input: PromptInput,
    pub picker: SlashCommandPicker,
    pub model: String,
    pub profile: String,
    pub session_id: String,
    pub permission_mode: String,
    pub scroll_offset: usize,
    pub pending_command: Option<String>,
    pub permission_pending: bool,
    pub quit: bool,
}

impl ReplApp {
    pub fn new(
        model: String,
        profile: String,
        session_id: String,
        permission_mode: String,
    ) -> Self {
        Self {
            messages: vec![RenderableMessage::System {
                message: "Kcode REPL 已启动。输入消息开始对话，或输入 / 查看命令。".to_string(),
                level: SysLevel::Info,
            }],
            state: SessionState::Idle,
            input: PromptInput::new(),
            picker: SlashCommandPicker::new(),
            model,
            profile,
            session_id,
            permission_mode,
            scroll_offset: 0,
            pending_command: None,
            permission_pending: false,
            quit: false,
        }
    }

    pub fn add_message(&mut self, msg: RenderableMessage) {
        self.messages.push(msg);
        self.scroll_offset = messages::auto_scroll_to_bottom(
            &self.messages,
            20, // 将在渲染时用实际高度更新
        );
    }

    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // 全局退出
        if key.code == KeyCode::Char('d')
            && key.modifiers == KeyModifiers::CONTROL
            && self.input.text.is_empty()
        {
            self.quit = true;
            return;
        }

        // 斜杠命令触发
        if key.code == KeyCode::Char('/')
            && self.input.text.is_empty()
            && !self.picker.visible
        {
            self.picker.show();
            return;
        }

        // Tab 补全触发
        if key.code == KeyCode::Tab
            && !self.picker.visible
            && self.input.text.starts_with('/')
        {
            self.picker.show();
            return;
        }

        // 权限弹窗处理
        if self.permission_pending {
            // 简化：直接 Allow
            if key.code == KeyCode::Enter || key.code == KeyCode::Char('a') {
                self.pending_command = Some("__permission_allow".to_string());
                self.permission_pending = false;
            } else if key.code == KeyCode::Char('d') {
                self.pending_command = Some("__permission_deny".to_string());
                self.permission_pending = false;
            }
            return;
        }

        // 命令选择框处理
        if self.picker.visible {
            match self.picker.handle_key(key) {
                SlashPickerAction::Select(cmd) => {
                    self.input.text = format!("/{}", cmd);
                    self.input.cursor = self.input.text.len();
                }
                SlashPickerAction::Cancel => {}
                SlashPickerAction::None => {}
            }
            return;
        }

        // 正常输入处理
        match self.input.handle_key(key) {
            PromptAction::Submit => {
                if let Some(text) = self.input.submit() {
                    self.pending_command = Some(text);
                }
            }
            PromptAction::Interrupt => {
                self.input.text.clear();
                self.input.cursor = 0;
                self.add_message(RenderableMessage::System {
                    message: "已中断当前请求".to_string(),
                    level: SysLevel::Warning,
                });
            }
            _ => {}
        }
    }
}

/// 运行 REPL TUI
pub fn run_repl(
    model: String,
    profile: String,
    session_id: String,
    permission_mode: String,
) -> Result<Vec<String>, Box<dyn Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("kcode repl requires an interactive terminal".into());
    }

    let mut app = ReplApp::new(model, profile, session_id, permission_mode);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let submitted_commands = run_repl_loop(&mut terminal, &mut app)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(submitted_commands)
}

fn run_repl_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReplApp,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut submitted = Vec::new();

    while !app.quit {
        terminal.draw(|frame| {
            let layout = build_layout(frame.area());

            // Header
            let state_label = app.state.label();
            frame.render_widget(
                header(
                    &app.model,
                    &app.profile,
                    &app.session_id,
                    &app.permission_mode,
                    state_label,
                ),
                layout.header,
            );

            // Messages
            render_messages(frame, &app.messages, layout.messages, &mut app.scroll_offset);

            // Prompt Input
            let is_active = !app.permission_pending && !app.picker.visible;
            render_prompt_input(frame, &app.input, layout.prompt, is_active);

            // Slash Command Picker
            render_slash_command_picker(frame, &app.picker, frame.area());

            // Permission Dialog (简化演示)
            if app.permission_pending {
                let dummy_req = PermissionRequest::new(
                    "example_tool".to_string(),
                    "这是一个演示权限请求".to_string(),
                );
                render_permission_dialog(frame, &dummy_req, frame.area(), 0);
            }

            // Footer
            frame.render_widget(
                footer(is_active, app.picker.visible, state_label),
                layout.footer,
            );
        })?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            app.handle_key(key);

            // 处理待执行命令
            if let Some(cmd) = app.pending_command.take() {
                if cmd == "__permission_allow" {
                    app.add_message(RenderableMessage::System {
                        message: "权限已授予".to_string(),
                        level: SysLevel::Success,
                    });
                    app.set_state(SessionState::Idle);
                } else if cmd == "__permission_deny" {
                    app.add_message(RenderableMessage::System {
                        message: "权限已拒绝".to_string(),
                        level: SysLevel::Warning,
                    });
                    app.set_state(SessionState::Idle);
                } else if cmd.starts_with('/') {
                    // 斜杠命令
                    let cmd_text = &cmd[1..];
                    app.add_message(RenderableMessage::System {
                        message: format!("执行命令: /{}", cmd_text),
                        level: SysLevel::Info,
                    });
                    submitted.push(cmd);
                } else {
                    // 普通消息
                    app.add_message(RenderableMessage::User {
                        text: cmd.clone(),
                    });
                    submitted.push(cmd);

                    // 模拟响应（演示用）
                    app.set_state(SessionState::Thinking {
                        text: "Processing...".to_string(),
                    });
                }
            }
        }
    }

    Ok(submitted)
}
