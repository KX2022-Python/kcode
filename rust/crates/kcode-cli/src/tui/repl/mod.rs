mod command_palette;
mod dialog;
mod diff_viewer;
mod footer;
mod footer_pills;
mod header;
mod input_enhance;
mod layout;
mod message_row;
mod messages;
mod notification_render;
mod notifications;
mod permission;
mod permission_enhanced;
mod prompt;
mod state;
mod theme;
mod tool_group;
mod virtual_scroll;

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

use self::command_palette::{
    render_slash_command_picker, SlashCommandEntry, SlashCommandPicker, SlashPickerAction,
};
use self::dialog::{handle_dialog_key, render_dialog, DialogAction, DialogState, DialogType};
use self::diff_viewer::{render_diff_viewer, DiffViewer};
use self::footer_pills::FooterPills;
use self::header::header;
use self::input_enhance::{HistorySearch, HistorySearchAction, InputStash};
use self::layout::build_layout;
use self::messages::{auto_scroll_to_bottom, render_messages};
use self::notification_render::render_notifications;
use self::notifications::{NotificationPriority, NotificationQueue};
use self::permission::render_permission_dialog;
use self::permission_enhanced::{EnhancedPermissionRequest, PermissionMode as PermMode};
use self::prompt::{prompt_height, render_prompt_input, PromptAction, PromptInput};
use self::state::{PermissionRequest, RenderableMessage, SessionState, SysLevel};
use self::theme::{TerminalType, ThemePalette, ThemePreset};
use self::tool_group::group_tool_calls;
use self::virtual_scroll::VirtualWindow;

pub use self::prompt::InputMode;

const KCODE_BANNER: &str = "Kcode REPL";

pub struct ReplApp {
    pub messages: Vec<RenderableMessage>,
    pub state: SessionState,
    pub input: PromptInput,
    pub picker: SlashCommandPicker,
    pub model: String,
    pub profile: String,
    pub session_id: String,
    pub permission_mode: PermMode,
    pub permission_mode_label: String,
    pub profile_supports_tools: bool,
    pub scroll_offset: usize,
    pub message_area_height: u16,
    pub message_area_width: u16,
    pub stick_to_bottom: bool,
    pub pending_command: Option<String>,
    pub permission_pending: bool,
    pub quit: bool,
    pub notifications: NotificationQueue,
    pub dialog: DialogState,
    pub virtual_window: VirtualWindow,
    pub history_search: HistorySearch,
    pub input_stash: InputStash,
    pub tools_collapsed: bool,
    pub footer_pills: FooterPills,
    pub diff_viewer: DiffViewer,
    pub enhanced_permission: Option<EnhancedPermissionRequest>,
    pub theme: ThemePreset,
    pub palette: ThemePalette,
    pub streaming_text: String,
    pub usage_input_tokens: u64,
    pub usage_output_tokens: u64,
}

impl ReplApp {
    pub fn new(
        model: String,
        profile: String,
        session_id: String,
        permission_mode: String,
        profile_supports_tools: bool,
    ) -> Self {
        let term_type = TerminalType::detect();
        let theme = term_type.recommended_theme();
        let palette = theme.palette();
        let permission_mode_label = permission_mode.clone();
        let permission_mode = match permission_mode.to_ascii_lowercase().as_str() {
            "auto" => PermMode::Auto,
            "plan" => PermMode::Plan,
            "allow" | "danger" | "danger-full-access" => PermMode::BypassDanger,
            _ => PermMode::Prompt,
        };

        let mut picker = SlashCommandPicker::new();
        let cwd = std::env::current_dir().unwrap_or_default();
        picker.refresh_commands(profile_supports_tools, &cwd);

        Self {
            messages: vec![
                RenderableMessage::System {
                    message: KCODE_BANNER.to_string(),
                    level: SysLevel::Info,
                },
                RenderableMessage::System {
                    message: format!(
                        "Model: {}  |  Profile: {}  |  Perm: {}  |  Session: {}",
                        model, profile, permission_mode_label, session_id
                    ),
                    level: SysLevel::Info,
                },
                RenderableMessage::System {
                    message: "输入消息开始对话 · / 命令 · F1 帮助 · F3 换主题".to_string(),
                    level: SysLevel::Info,
                },
            ],
            state: SessionState::Idle,
            input: PromptInput::new(),
            picker,
            model: model.clone(),
            profile,
            session_id: session_id.clone(),
            permission_mode,
            permission_mode_label: permission_mode_label.clone(),
            profile_supports_tools,
            scroll_offset: 0,
            message_area_height: 10,
            message_area_width: 80,
            stick_to_bottom: true,
            pending_command: None,
            permission_pending: false,
            quit: false,
            notifications: NotificationQueue::new(),
            dialog: DialogState::new(),
            virtual_window: VirtualWindow::new(20),
            history_search: HistorySearch::new(),
            input_stash: InputStash::new(),
            tools_collapsed: true,
            footer_pills: FooterPills::new(model, permission_mode_label, session_id),
            diff_viewer: DiffViewer::new(),
            enhanced_permission: None,
            theme,
            palette,
            streaming_text: String::new(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
        }
    }

    pub fn add_message(&mut self, msg: RenderableMessage) {
        self.messages.push(msg);
        if self.stick_to_bottom {
            self.scroll_to_bottom();
        } else {
            self.clamp_scroll();
        }
    }

    pub fn notify(&mut self, message: String, priority: NotificationPriority) {
        self.notifications.push(message, priority);
    }

    pub fn notify_info(&mut self, message: String) {
        self.notify(message, NotificationPriority::Medium);
    }

    pub fn notify_success(&mut self, message: String) {
        self.notify(message, NotificationPriority::High);
    }

    pub fn notify_warning(&mut self, message: String) {
        self.notify(message, NotificationPriority::Immediate);
    }

    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.cycle();
        self.palette = self.theme.palette();
        self.notify_info(format!("主题切换为: {}", self.theme.name()));
    }

    pub fn toggle_tools_collapsed(&mut self) {
        self.tools_collapsed = !self.tools_collapsed;
    }

    pub fn set_message_viewport(&mut self, area: Rect) {
        self.message_area_height = area.height.max(1);
        self.message_area_width = area.width.max(1);
        if self.stick_to_bottom {
            self.scroll_to_bottom();
        } else {
            self.clamp_scroll();
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = auto_scroll_to_bottom(
            &self.messages,
            self.message_area_height,
            self.message_area_width,
        );
        self.stick_to_bottom = true;
    }

    fn clamp_scroll(&mut self) {
        let max_offset = auto_scroll_to_bottom(
            &self.messages,
            self.message_area_height,
            self.message_area_width,
        );
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }

    fn sync_picker_with_input(&mut self) {
        self.picker.sync_with_input(&self.input.text);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.diff_viewer.visible {
            self.diff_viewer.handle_key(key);
            return;
        }

        if self.dialog.is_active() {
            match handle_dialog_key(&mut self.dialog, key) {
                DialogAction::Close | DialogAction::None => {}
                DialogAction::SelectModel(model) => {
                    self.model = model.clone();
                    self.footer_pills.model = model.clone();
                    self.notify_success(format!("模型已切换: {}", model));
                }
                DialogAction::SelectSession(session) => {
                    self.session_id = session.clone();
                    self.footer_pills.session_id = session.clone();
                    self.notify_info(format!("会话切换: {}", session));
                }
            }
            return;
        }

        if self.history_search.active {
            match self.history_search.handle_key(key, &self.input.history) {
                HistorySearchAction::Select(entry) => {
                    self.input.text = entry.clone();
                    self.input.cursor = entry.len();
                    self.sync_picker_with_input();
                    self.notify_info("已从历史恢复".to_string());
                }
                HistorySearchAction::Cancel => {
                    self.sync_picker_with_input();
                }
                HistorySearchAction::Updated | HistorySearchAction::None => {}
            }
            return;
        }

        if key.code == KeyCode::Char('c')
            && key.modifiers == KeyModifiers::CONTROL
            && self.input.text.is_empty()
        {
            self.quit = true;
            return;
        }

        if key.code == KeyCode::Char('d')
            && key.modifiers == KeyModifiers::CONTROL
            && self.input.text.is_empty()
        {
            self.quit = true;
            return;
        }

        match key.code {
            KeyCode::PageUp => {
                self.scroll_offset = self
                    .scroll_offset
                    .saturating_sub((self.message_area_height / 2).max(1) as usize);
                self.stick_to_bottom = false;
                return;
            }
            KeyCode::PageDown => {
                let max_offset = auto_scroll_to_bottom(
                    &self.messages,
                    self.message_area_height,
                    self.message_area_width,
                );
                self.scroll_offset = (self.scroll_offset
                    + (self.message_area_height / 2).max(1) as usize)
                    .min(max_offset);
                self.stick_to_bottom = self.scroll_offset >= max_offset;
                return;
            }
            KeyCode::End if key.modifiers == KeyModifiers::ALT => {
                self.scroll_to_bottom();
                return;
            }
            _ => {}
        }

        if key.code == KeyCode::F(1) {
            self.dialog.show(DialogType::Help);
            return;
        }

        if key.code == KeyCode::F(2) {
            let models = vec![
                "gpt-4.1".to_string(),
                "gpt-4.1-mini".to_string(),
                "gpt-4o".to_string(),
                "qwen-plus".to_string(),
                "qwen-max".to_string(),
            ];
            self.dialog.show(DialogType::ModelPicker {
                models,
                selected: 0,
            });
            return;
        }

        if key.code == KeyCode::F(3) {
            self.cycle_theme();
            return;
        }

        if key.code == KeyCode::F(4) {
            self.toggle_tools_collapsed();
            self.notify_info("工具调用展示已切换".to_string());
            return;
        }

        if key.code == KeyCode::Char('s')
            && key.modifiers == KeyModifiers::CONTROL
            && !self.input.text.is_empty()
        {
            self.input_stash.stash(&self.input.text, self.input.cursor);
            self.input.text.clear();
            self.input.cursor = 0;
            self.sync_picker_with_input();
            self.notify_info("输入已暂存".to_string());
            return;
        }

        if key.code == KeyCode::Char('s')
            && key.modifiers == KeyModifiers::CONTROL
            && self.input.text.is_empty()
            && self.input_stash.has_stash()
        {
            let (text, cursor) = self.input_stash.restore();
            self.input.text = text;
            self.input.cursor = cursor;
            self.input_stash.clear();
            self.sync_picker_with_input();
            self.notify_info("输入已恢复".to_string());
            return;
        }

        if self.permission_pending {
            if key.code == KeyCode::Enter || key.code == KeyCode::Char('a') {
                self.pending_command = Some("__permission_allow".to_string());
                self.permission_pending = false;
            } else if key.code == KeyCode::Char('d') {
                self.pending_command = Some("__permission_deny".to_string());
                self.permission_pending = false;
            }
            return;
        }

        if self.picker.visible
            && matches!(
                key.code,
                KeyCode::Esc | KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Tab
            )
        {
            match self.picker.handle_key(key) {
                SlashPickerAction::Select(command) => {
                    self.input.text = command;
                    self.input.cursor = self.input.text.len();
                    self.sync_picker_with_input();
                }
                SlashPickerAction::Cancel => {
                    self.sync_picker_with_input();
                }
                SlashPickerAction::None => {}
            }
            return;
        }

        match self.input.handle_key(key) {
            PromptAction::Submit => {
                if let Some(text) = self.input.submit() {
                    self.pending_command = Some(text);
                    self.sync_picker_with_input();
                }
            }
            PromptAction::Edited => self.sync_picker_with_input(),
            PromptAction::Interrupt => {
                self.notify_warning("已取消当前输入".to_string());
                self.sync_picker_with_input();
            }
            PromptAction::HistorySearch => self.history_search.activate(),
            PromptAction::Moved | PromptAction::None => {}
        }
    }
}

pub fn run_repl(
    model: String,
    profile: String,
    session_id: String,
    permission_mode: String,
) -> Result<Vec<String>, Box<dyn Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("kcode repl requires an interactive terminal".into());
    }

    let mut app = ReplApp::new(model, profile, session_id, permission_mode, true);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    while event::poll(Duration::from_millis(10)).unwrap_or(false) {
        let _ = event::read();
    }

    let submitted = run_repl_loop(&mut terminal, &mut app)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(submitted)
}

fn run_repl_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReplApp,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut submitted = Vec::new();
    terminal.draw(|frame| draw_frame(frame, app))?;

    while !app.quit {
        let has_event = event::poll(Duration::from_millis(200)).unwrap_or(false);
        if has_event {
            match event::read() {
                Ok(Event::Key(key)) => {
                    app.handle_key(key);
                    process_pending_command(app, &mut submitted);
                }
                Ok(Event::Resize(_, _)) => {
                    let _ = terminal.autoresize();
                }
                Ok(_) | Err(_) => {}
            }
        }
        terminal.draw(|frame| draw_frame(frame, app))?;
    }

    Ok(submitted)
}

fn process_pending_command(app: &mut ReplApp, submitted: &mut Vec<String>) {
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
            app.add_message(RenderableMessage::System {
                message: format!("执行命令: {}", command),
                level: SysLevel::Info,
            });
            app.notify_info(format!("执行: {}", command));
            submitted.push(command);
        }
        _ => {
            app.add_message(RenderableMessage::User {
                text: command.clone(),
            });
            submitted.push(command);
            app.set_state(SessionState::Thinking {
                text: "Processing...".to_string(),
            });
            app.add_message(RenderableMessage::AssistantText {
                text: "TUI 预览模式已记录这条消息。".to_string(),
                streaming: false,
            });
            app.set_state(SessionState::Completed {
                summary: "preview-complete".to_string(),
            });
            app.usage_input_tokens += 150;
            app.usage_output_tokens += 280;
            app.footer_pills.token_usage = Some(self::footer_pills::TokenUsage {
                input_tokens: app.usage_input_tokens,
                output_tokens: app.usage_output_tokens,
            });
        }
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
        );
        app.message_area_height = 4;
        app.message_area_width = 24;

        for index in 0..10 {
            app.add_message(crate::tui::repl::state::RenderableMessage::AssistantText {
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
