use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::text_cursor::{clamp_cursor_to_boundary, next_char_boundary, previous_char_boundary};

/// 权限模式 — 对齐 CC-Haha 权限模式轮播
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    /// 默认模式：每次工具调用都询问
    Prompt,
    /// 计划模式：先规划再执行
    Plan,
    /// 自动模式：自动允许安全工具
    Auto,
    /// 完全绕过权限（危险）
    BypassDanger,
}

impl PermissionMode {
    pub fn label(&self) -> &str {
        match self {
            PermissionMode::Prompt => "prompt",
            PermissionMode::Plan => "plan",
            PermissionMode::Auto => "auto",
            PermissionMode::BypassDanger => "danger",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            PermissionMode::Prompt => "❓",
            PermissionMode::Plan => "📋",
            PermissionMode::Auto => "⚡",
            PermissionMode::BypassDanger => "⚠️",
        }
    }

    /// 轮播到下一个模式
    pub fn next(&self) -> PermissionMode {
        match self {
            PermissionMode::Prompt => PermissionMode::Plan,
            PermissionMode::Plan => PermissionMode::Auto,
            PermissionMode::Auto => PermissionMode::BypassDanger,
            PermissionMode::BypassDanger => PermissionMode::Prompt,
        }
    }

    pub fn prev(&self) -> PermissionMode {
        match self {
            PermissionMode::Prompt => PermissionMode::BypassDanger,
            PermissionMode::Plan => PermissionMode::Prompt,
            PermissionMode::Auto => PermissionMode::Plan,
            PermissionMode::BypassDanger => PermissionMode::Auto,
        }
    }
}

/// 增强的权限请求 — 对齐 CC-Haha PermissionPrompt Tab-to-amend
#[derive(Debug, Clone)]
pub struct EnhancedPermissionRequest {
    pub tool_name: String,
    pub input_summary: String,
    pub rule_explanation: String,
    pub focused_option: PermissionOption,
    pub show_feedback_input: bool,
    pub feedback_text: String,
    pub feedback_cursor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionOption {
    Allow,
    AllowAlways,
    Deny,
    DenyAlways,
}

impl EnhancedPermissionRequest {
    pub fn new(tool_name: String, input_summary: String, rule_explanation: String) -> Self {
        Self {
            tool_name,
            input_summary,
            rule_explanation,
            focused_option: PermissionOption::Allow,
            show_feedback_input: false,
            feedback_text: String::new(),
            feedback_cursor: 0,
        }
    }

    pub fn focus_next(&mut self) {
        self.focused_option = match &self.focused_option {
            PermissionOption::Allow => PermissionOption::AllowAlways,
            PermissionOption::AllowAlways => PermissionOption::Deny,
            PermissionOption::Deny => PermissionOption::DenyAlways,
            PermissionOption::DenyAlways => PermissionOption::Allow,
        };
        self.show_feedback_input = false;
    }

    pub fn focus_prev(&mut self) {
        self.focused_option = match &self.focused_option {
            PermissionOption::Allow => PermissionOption::DenyAlways,
            PermissionOption::AllowAlways => PermissionOption::Allow,
            PermissionOption::Deny => PermissionOption::AllowAlways,
            PermissionOption::DenyAlways => PermissionOption::Deny,
        };
        self.show_feedback_input = false;
    }

    pub fn toggle_feedback(&mut self) {
        self.show_feedback_input = !self.show_feedback_input;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<EnhancedPermissionAction> {
        // 如果正在输入反馈
        if self.show_feedback_input {
            self.feedback_cursor =
                clamp_cursor_to_boundary(&self.feedback_text, self.feedback_cursor);
            match key.code {
                KeyCode::Esc => {
                    self.show_feedback_input = false;
                    return None;
                }
                KeyCode::Enter => {
                    self.show_feedback_input = false;
                    return None;
                }
                KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                    self.feedback_text.insert(self.feedback_cursor, c);
                    self.feedback_cursor += c.len_utf8();
                    return None;
                }
                KeyCode::Backspace => {
                    if self.feedback_cursor > 0 {
                        let previous =
                            previous_char_boundary(&self.feedback_text, self.feedback_cursor);
                        self.feedback_text.drain(previous..self.feedback_cursor);
                        self.feedback_cursor = previous;
                    }
                    return None;
                }
                KeyCode::Left => {
                    self.feedback_cursor =
                        previous_char_boundary(&self.feedback_text, self.feedback_cursor);
                    return None;
                }
                KeyCode::Right => {
                    self.feedback_cursor =
                        next_char_boundary(&self.feedback_text, self.feedback_cursor);
                    return None;
                }
                _ => return None,
            }
        }

        match key.code {
            KeyCode::Enter => {
                let action = match &self.focused_option {
                    PermissionOption::Allow => EnhancedPermissionAction::Allow,
                    PermissionOption::AllowAlways => EnhancedPermissionAction::AllowAlways,
                    PermissionOption::Deny => EnhancedPermissionAction::Deny,
                    PermissionOption::DenyAlways => EnhancedPermissionAction::DenyAlways,
                };
                Some(action)
            }
            KeyCode::Tab | KeyCode::Right => {
                self.focus_next();
                None
            }
            KeyCode::Left => {
                self.focus_prev();
                None
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                self.toggle_feedback();
                None
            }
            _ => None,
        }
    }
}

pub enum EnhancedPermissionAction {
    Allow,
    AllowAlways,
    Deny,
    DenyAlways,
}
