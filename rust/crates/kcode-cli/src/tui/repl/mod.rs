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
use ratatui::Terminal;

use self::dialog::{handle_dialog_key, render_dialog, DialogAction, DialogState, DialogType};
use self::diff_viewer::{render_diff_viewer, DiffViewer};
use self::footer::footer;
use self::footer_pills::FooterPills;
use self::header::header;
use self::input_enhance::{highlight_input, HistorySearch, HistorySearchAction, InputStash};
use self::layout::build_layout;
use self::message_row::render_message;
use self::messages::{auto_scroll_to_bottom, render_messages};
use self::notification_render::render_notifications;
use self::notifications::{NotificationPriority, NotificationQueue};
use self::permission::{render_permission_dialog, PermissionAction};
use self::permission_enhanced::{EnhancedPermissionAction, EnhancedPermissionRequest, PermissionMode as PermMode};
use self::prompt::{
    render_prompt_input, render_slash_command_picker, PromptAction, SlashPickerAction,
};
use self::state::{
    PermissionDecision, PermissionRequest, RenderableMessage, SessionState, SysLevel, ToolStatus,
};
use self::theme::{TerminalType, ThemePalette, ThemePreset};
use self::tool_group::group_tool_calls;
use self::virtual_scroll::VirtualWindow;

pub use self::prompt::{InputMode, PromptInput, SlashCommandEntry, SlashCommandPicker};

/// REPL 应用状态 — 全面对齐 CC-Haha
pub struct ReplApp {
    pub messages: Vec<RenderableMessage>,
    pub state: SessionState,
    pub input: PromptInput,
    pub picker: SlashCommandPicker,
    pub model: String,
    pub profile: String,
    pub session_id: String,
    pub permission_mode: PermMode,
    pub scroll_offset: usize,
    pub pending_command: Option<String>,
    pub permission_pending: bool,
    pub quit: bool,
    // Phase 1: 通知系统
    pub notifications: NotificationQueue,
    // Phase 2: 对话框系统
    pub dialog: DialogState,
    // Phase 3: 虚拟滚动
    pub virtual_window: VirtualWindow,
    // Phase 4: 输入增强
    pub history_search: HistorySearch,
    pub input_stash: InputStash,
    // Phase 5: 工具折叠
    pub tools_collapsed: bool,
    // Phase 6: Footer Pills
    pub footer_pills: FooterPills,
    // Phase 7: Diff 查看器
    pub diff_viewer: DiffViewer,
    // Phase 8: 权限增强
    pub enhanced_permission: Option<EnhancedPermissionRequest>,
    // Phase 9: 主题系统
    pub theme: ThemePreset,
    pub palette: ThemePalette,
    // 流式响应缓冲
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
    ) -> Self {
        let term_type = TerminalType::detect();
        let theme = term_type.recommended_theme();
        let palette = theme.palette();

        let perm_mode = match permission_mode.to_lowercase().as_str() {
            "allow" | "dangerfullaccess" | "danger" => PermMode::BypassDanger,
            "auto" => PermMode::Auto,
            "plan" => PermMode::Plan,
            _ => PermMode::Prompt,
        };

        let perm_str = permission_mode.clone();
        let footer_pills = FooterPills::new(
            model.clone(),
            perm_str,
            session_id.clone(),
        );

        Self {
            messages: vec![RenderableMessage::System {
                message: "Kcode REPL 已启动。输入消息开始对话，或按 / 查看命令，F1 帮助。".to_string(),
                level: SysLevel::Info,
            }],
            state: SessionState::Idle,
            input: PromptInput::new(),
            picker: SlashCommandPicker::new(),
            model,
            profile,
            session_id,
            permission_mode: perm_mode,
            scroll_offset: 0,
            pending_command: None,
            permission_pending: false,
            quit: false,
            notifications: NotificationQueue::new(),
            dialog: DialogState::new(),
            virtual_window: VirtualWindow::new(20),
            history_search: HistorySearch::new(),
            input_stash: InputStash::new(),
            tools_collapsed: true,
            footer_pills,
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
        // 自动滚动到底部
        self.scroll_offset = auto_scroll_to_bottom(&self.messages, 20);
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

    pub fn notify_error(&mut self, message: String) {
        self.notify(message, NotificationPriority::Immediate);
    }

    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    /// 切换主题
    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.cycle();
        self.palette = self.theme.palette();
        self.notify_info(format!("主题切换为: {}", self.theme.name()));
    }

    /// 切换工具折叠
    pub fn toggle_tools_collapsed(&mut self) {
        self.tools_collapsed = !self.tools_collapsed;
    }

    /// 处理按键 — 完整的事件分发
    fn handle_key(&mut self, key: KeyEvent) {
        // Diff 查看器优先
        if self.diff_viewer.visible {
            self.diff_viewer.handle_key(key);
            return;
        }

        // 对话框优先
        if self.dialog.is_active() {
            match handle_dialog_key(&mut self.dialog, key) {
                DialogAction::Close => {}
                DialogAction::SelectModel(model) => {
                    self.model = model.clone();
                    self.notify_success(format!("模型已切换: {}", model));
                    self.footer_pills.model = model;
                }
                DialogAction::SelectSession(session) => {
                    self.notify_info(format!("会话切换: {}", session));
                }
                DialogAction::None => {}
            }
            return;
        }

        // 历史搜索
        if self.history_search.active {
            match self.history_search.handle_key(key, &self.input.history) {
                HistorySearchAction::Select(entry) => {
                    self.input.text = entry.clone();
                    self.input.cursor = entry.len();
                    self.notify_info("已从历史恢复".to_string());
                }
                HistorySearchAction::Cancel => {}
                HistorySearchAction::Updated => {}
                HistorySearchAction::None => {}
            }
            return;
        }

        // 全局退出
        if key.code == KeyCode::Char('d')
            && key.modifiers == KeyModifiers::CONTROL
            && self.input.text.is_empty()
        {
            self.quit = true;
            return;
        }

        // F1: 帮助
        if key.code == KeyCode::F(1) {
            self.dialog.show(DialogType::Help);
            return;
        }

        // F2: 模型选择器
        if key.code == KeyCode::F(2) {
            let models = vec![
                "gpt-4.1".to_string(),
                "gpt-4o".to_string(),
                "claude-sonnet-4-20250514".to_string(),
                "claude-opus-4-20250514".to_string(),
                "qwen-plus".to_string(),
                "qwen-max".to_string(),
            ];
            self.dialog
                .show(DialogType::ModelPicker { models, selected: 0 });
            return;
        }

        // F3: 切换主题
        if key.code == KeyCode::F(3) {
            self.cycle_theme();
            return;
        }

        // F4: 切换工具折叠
        if key.code == KeyCode::F(4) {
            self.toggle_tools_collapsed();
            self.notify_info("工具调用展示已切换".to_string());
            return;
        }

        // Ctrl+S: 暂存输入
        if key.code == KeyCode::Char('s')
            && key.modifiers == KeyModifiers::CONTROL
            && !self.input.text.is_empty()
        {
            self.input_stash
                .stash(&self.input.text, self.input.cursor);
            self.input.text.clear();
            self.input.cursor = 0;
            self.notify_info("输入已暂存".to_string());
            return;
        }

        // Ctrl+S: 恢复暂存
        if key.code == KeyCode::Char('s')
            && key.modifiers == KeyModifiers::CONTROL
            && self.input.text.is_empty()
            && self.input_stash.has_stash()
        {
            let (text, cursor) = self.input_stash.restore();
            self.input.text = text;
            self.input.cursor = cursor;
            self.input_stash.clear();
            self.notify_info("输入已恢复".to_string());
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
                self.notify_warning("请求已中断".to_string());
            }
            PromptAction::HistorySearch => {
                self.history_search.activate();
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
    let mut frame_count: u64 = 0;

    while !app.quit {
        frame_count += 1;

        terminal.draw(|frame| {
            let layout = build_layout(frame.area());

            // Header
            let state_label = app.state.label();
            frame.render_widget(
                header(
                    &app.model,
                    &app.profile,
                    &app.session_id,
                    app.permission_mode.label(),
                    state_label,
                ),
                layout.header,
            );

            // 通知（在 header 下方）
            render_notifications(frame, &mut app.notifications, frame.area());

            // 消息列表（应用工具折叠）
            let display_messages = if app.tools_collapsed {
                group_tool_calls(&app.messages)
            } else {
                app.messages.clone()
            };
            render_messages(
                frame,
                &display_messages,
                layout.messages,
                &mut app.scroll_offset,
            );

            // Prompt Input
            let is_active = !app.permission_pending
                && !app.picker.visible
                && !app.dialog.is_active()
                && !app.history_search.active
                && !app.diff_viewer.visible;
            render_prompt_input(frame, &app.input, layout.prompt, is_active);

            // Slash Command Picker
            render_slash_command_picker(frame, &app.picker, frame.area());

            // 对话框
            render_dialog(frame, &app.dialog, frame.area());

            // Diff 查看器
            render_diff_viewer(frame, &app.diff_viewer, frame.area());

            // 权限弹窗
            if app.permission_pending {
                let dummy_req = PermissionRequest::new(
                    "example_tool".to_string(),
                    "这是一个演示权限请求".to_string(),
                );
                render_permission_dialog(frame, &dummy_req, frame.area(), 0);
            }

            // Footer Pills
            app.footer_pills.token_usage = if app.usage_input_tokens > 0
                || app.usage_output_tokens > 0
            {
                Some(self::footer_pills::TokenUsage {
                    input_tokens: app.usage_input_tokens,
                    output_tokens: app.usage_output_tokens,
                })
            } else {
                None
            };
            app.footer_pills.has_active_query = app.state.is_active();
            app.footer_pills.has_pending_permission = app.permission_pending;
            app.footer_pills.has_notifications = !app.notifications.is_empty();
            frame.render_widget(app.footer_pills.render(), layout.footer);
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
                    app.notify_success("权限已授予".to_string());
                    app.set_state(SessionState::Idle);
                } else if cmd == "__permission_deny" {
                    app.add_message(RenderableMessage::System {
                        message: "权限已拒绝".to_string(),
                        level: SysLevel::Warning,
                    });
                    app.notify_warning("权限已拒绝".to_string());
                    app.set_state(SessionState::Idle);
                } else if cmd.starts_with('/') {
                    let cmd_text = &cmd[1..];
                    app.add_message(RenderableMessage::System {
                        message: format!("执行命令: /{}", cmd_text),
                        level: SysLevel::Info,
                    });
                    app.notify_info(format!("执行: /{}", cmd_text));
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

                    // 模拟完成
                    app.set_state(SessionState::Completed {
                        summary: "演示模式：模拟完成".to_string(),
                    });
                    app.notify_success("演示响应完成".to_string());

                    // 模拟用量
                    app.usage_input_tokens += 150;
                    app.usage_output_tokens += 280;
                    app.footer_pills.token_usage =
                        Some(self::footer_pills::TokenUsage {
                            input_tokens: app.usage_input_tokens,
                            output_tokens: app.usage_output_tokens,
                        });
                }
            }
        }
    }

    Ok(submitted)
}
