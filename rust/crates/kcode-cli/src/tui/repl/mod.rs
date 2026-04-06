mod bootstrap;
mod bootstrap_store;
mod command_palette;
mod dialog;
mod diff_viewer;
mod footer;
mod footer_pills;
mod header;
mod input_enhance;
mod layout;
mod markdown_render;
mod message_row;
mod messages;
mod notification_render;
mod notifications;
mod permission;
mod permission_enhanced;
mod prompt;
mod runtime_loop;
mod state;
mod text_cursor;
mod text_layout;
mod theme;
mod tool_group;
mod virtual_scroll;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use self::command_palette::{SlashCommandEntry, SlashCommandPicker, SlashPickerAction};
use self::dialog::{handle_dialog_key, DialogAction, DialogState, DialogType};
use self::diff_viewer::DiffViewer;
use self::footer_pills::FooterPills;
use self::input_enhance::{HistorySearch, HistorySearchAction, InputStash};
use self::messages::auto_scroll_to_bottom;
use self::notifications::{NotificationPriority, NotificationQueue};
use self::permission_enhanced::{EnhancedPermissionRequest, PermissionMode as PermMode};
use self::prompt::{PromptAction, PromptInput};
use self::state::{PermissionRequest, SessionState};
use self::theme::{TerminalType, ThemePalette, ThemePreset};
use self::virtual_scroll::VirtualWindow;

pub(crate) use self::bootstrap::{run_bootstrap_flow, BootstrapDecision};
pub(crate) use self::bootstrap_store::{
    load_bootstrap_state, persist_bootstrap_theme, persist_workspace_trust, BootstrapState,
};
pub use self::prompt::InputMode;
pub(crate) use self::runtime_loop::{
    default_welcome_messages, render_welcome_banner, run_repl, short_session_id,
};
pub use self::state::{
    BackendResult, RenderableMessage, RuntimeUiState, SubmittedCommand, SysLevel, ToolStatus,
};

pub struct ReplApp {
    pub messages: Vec<RenderableMessage>,
    pub state: SessionState,
    pub input: PromptInput,
    pub picker: SlashCommandPicker,
    pub available_models: Vec<String>,
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
        available_models: Vec<String>,
        welcome_messages: Vec<RenderableMessage>,
        initial_theme: Option<ThemePreset>,
    ) -> Self {
        let term_type = TerminalType::detect();
        let theme = initial_theme.unwrap_or_else(|| term_type.recommended_theme());
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
        picker.refresh_commands(profile_supports_tools, &cwd, &available_models);

        let messages = if welcome_messages.is_empty() {
            default_welcome_messages(&model, &profile, &permission_mode_label, &session_id)
        } else {
            welcome_messages
        };

        Self {
            messages,
            state: SessionState::Idle,
            input: PromptInput::new(),
            picker,
            available_models,
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

    pub fn sync_runtime_ui_state(&mut self, ui_state: RuntimeUiState) {
        let cwd = std::env::current_dir().unwrap_or_default();
        self.model = ui_state.model.clone();
        self.profile = ui_state.profile.clone();
        self.session_id = ui_state.session_id.clone();
        self.permission_mode_label = ui_state.permission_mode_label.clone();
        self.profile_supports_tools = ui_state.profile_supports_tools;
        self.permission_mode = match ui_state.permission_mode_label.to_ascii_lowercase().as_str() {
            "auto" => PermMode::Auto,
            "plan" => PermMode::Plan,
            "allow" | "danger" | "danger-full-access" => PermMode::BypassDanger,
            _ => PermMode::Prompt,
        };
        self.footer_pills.model = ui_state.model;
        self.footer_pills.permission_mode = ui_state.permission_mode_label;
        self.footer_pills.session_id = ui_state.session_id;
        self.picker
            .refresh_commands(self.profile_supports_tools, &cwd, &self.available_models);
        if self.picker.visible {
            self.sync_picker_with_input();
        }
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
        let next_height = area.height.max(1);
        let next_width = area.width.max(1);
        let resized =
            next_height != self.message_area_height || next_width != self.message_area_width;

        let bottom_gap = if resized && !self.stick_to_bottom {
            self.current_max_scroll().saturating_sub(self.scroll_offset)
        } else {
            0
        };

        self.message_area_height = next_height;
        self.message_area_width = next_width;
        if self.stick_to_bottom {
            self.scroll_to_bottom();
        } else if resized {
            self.scroll_offset = self.current_max_scroll().saturating_sub(bottom_gap);
        } else {
            self.clamp_scroll();
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.current_max_scroll();
        self.stick_to_bottom = true;
    }

    fn clamp_scroll(&mut self) {
        let max_offset = self.current_max_scroll();
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }

    fn current_max_scroll(&self) -> usize {
        auto_scroll_to_bottom(
            &self.messages,
            self.message_area_height,
            self.message_area_width,
        )
    }

    fn sync_picker_with_input(&mut self) {
        self.picker.sync_with_input(&self.input.text);
    }

    fn enqueue_prompt_submission(&mut self, text: String) {
        self.add_message(RenderableMessage::User { text: text.clone() });
        self.set_state(SessionState::Requesting {
            start: std::time::Instant::now(),
        });
        self.pending_command = Some(text);
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
                    self.pending_command = Some(format!("/model {}", model));
                    self.notify_info(format!("准备切换模型: {}", model));
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
            KeyCode::Up if key.modifiers == KeyModifiers::CONTROL => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                self.stick_to_bottom = false;
                return;
            }
            KeyCode::Down if key.modifiers == KeyModifiers::CONTROL => {
                let max_offset = auto_scroll_to_bottom(
                    &self.messages,
                    self.message_area_height,
                    self.message_area_width,
                );
                self.scroll_offset = (self.scroll_offset + 3).min(max_offset);
                self.stick_to_bottom = self.scroll_offset >= max_offset;
                return;
            }
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
            self.dialog.show(DialogType::ModelPicker {
                models: self.available_models.clone(),
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
                KeyCode::Esc
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Enter
                    | KeyCode::Tab
            )
        {
            match self.picker.handle_key(key, &self.input.text) {
                SlashPickerAction::Select(command) => {
                    self.input.text = command;
                    self.input.cursor = self.input.text.len();
                }
                SlashPickerAction::Submit(command) => {
                    self.input.history.push(command.clone());
                    self.input.history_index = None;
                    self.input.text.clear();
                    self.input.cursor = 0;
                    self.pending_command = Some(command);
                }
                SlashPickerAction::Cancel => {}
                SlashPickerAction::None => {}
            }
            return;
        }

        match self.input.handle_key(key) {
            PromptAction::Submit => {
                if let Some(text) = self.input.submit() {
                    self.enqueue_prompt_submission(text);
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

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.dialog.is_active() || self.history_search.active {
            return;
        }

        if self.picker.visible {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.picker.select_previous(),
                MouseEventKind::ScrollDown => self.picker.select_next(),
                _ => {}
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                self.stick_to_bottom = false;
            }
            MouseEventKind::ScrollDown => {
                let max_offset = auto_scroll_to_bottom(
                    &self.messages,
                    self.message_area_height,
                    self.message_area_width,
                );
                self.scroll_offset = (self.scroll_offset + 3).min(max_offset);
                self.stick_to_bottom = self.scroll_offset >= max_offset;
            }
            _ => {}
        }
    }
}
