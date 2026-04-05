use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::text_cursor::{clamp_cursor_to_boundary, next_char_boundary, previous_char_boundary};
use super::text_layout::display_line_count;
use super::theme::ThemePalette;

/// Prompt 输入框状态
#[derive(Debug, Clone)]
pub struct PromptInput {
    pub text: String,
    pub cursor: usize,
    pub mode: InputMode,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Bash,
}

impl PromptInput {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            mode: InputMode::Normal,
            history: Vec::new(),
            history_index: None,
        }
    }

    pub fn submit(&mut self) -> Option<String> {
        let submitted = self.text.trim().to_string();
        if submitted.is_empty() {
            return None;
        }

        self.history.push(submitted.clone());
        self.history_index = None;
        self.text.clear();
        self.cursor = 0;
        Some(submitted)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PromptAction {
        self.cursor = clamp_cursor_to_boundary(&self.text, self.cursor);
        match key.code {
            KeyCode::Enter if key.modifiers.is_empty() => PromptAction::Submit,
            KeyCode::Enter
                if key.modifiers.contains(KeyModifiers::SHIFT)
                    || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.text.insert(self.cursor, '\n');
                self.cursor += '\n'.len_utf8();
                PromptAction::Edited
            }
            KeyCode::Char('j') if key.modifiers == KeyModifiers::CONTROL => {
                self.text.insert(self.cursor, '\n');
                self.cursor += '\n'.len_utf8();
                PromptAction::Edited
            }
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                PromptAction::Edited
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let previous = previous_char_boundary(&self.text, self.cursor);
                    self.text.drain(previous..self.cursor);
                    self.cursor = previous;
                }
                PromptAction::Edited
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    let next = next_char_boundary(&self.text, self.cursor);
                    self.text.drain(self.cursor..next);
                }
                PromptAction::Edited
            }
            KeyCode::Left if key.modifiers == KeyModifiers::NONE => {
                self.cursor = previous_char_boundary(&self.text, self.cursor);
                PromptAction::Moved
            }
            KeyCode::Right if key.modifiers == KeyModifiers::NONE => {
                self.cursor = next_char_boundary(&self.text, self.cursor);
                PromptAction::Moved
            }
            KeyCode::Home => {
                self.cursor = 0;
                PromptAction::Moved
            }
            KeyCode::End => {
                self.cursor = self.text.len();
                PromptAction::Moved
            }
            KeyCode::Up if key.modifiers == KeyModifiers::NONE => {
                if self.history.is_empty() {
                    return PromptAction::None;
                }
                match self.history_index {
                    None => {
                        self.history_index = Some(self.history.len() - 1);
                        self.text = self.history.last().cloned().unwrap_or_default();
                    }
                    Some(index) if index > 0 => {
                        self.history_index = Some(index - 1);
                        self.text = self.history[index - 1].clone();
                    }
                    Some(_) => {}
                }
                self.cursor = self.text.len();
                PromptAction::Edited
            }
            KeyCode::Down if key.modifiers == KeyModifiers::NONE => {
                match self.history_index {
                    Some(index) if index + 1 < self.history.len() => {
                        self.history_index = Some(index + 1);
                        self.text = self.history[index + 1].clone();
                    }
                    Some(_) => {
                        self.history_index = None;
                        self.text.clear();
                    }
                    None => {}
                }
                self.cursor = self.text.len();
                PromptAction::Edited
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.text.clear();
                self.cursor = 0;
                PromptAction::Interrupt
            }
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
                PromptAction::HistorySearch
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                self.text.clear();
                self.cursor = 0;
                PromptAction::Edited
            }
            KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
                self.text.drain(self.cursor..);
                PromptAction::Edited
            }
            _ => PromptAction::None,
        }
    }
}

pub enum PromptAction {
    Submit,
    Edited,
    Moved,
    Interrupt,
    HistorySearch,
    None,
}

pub fn prompt_height(input: &PromptInput, available_width: u16) -> u16 {
    let content_width = prompt_content_width(available_width);
    let visible_text = if input.text.is_empty() {
        "给 Kcode 下达任务，或输入 / 查看命令..."
    } else {
        &input.text
    };
    let line_count = display_line_count(visible_text, content_width);
    (line_count as u16 + 2).clamp(3, 8)
}

/// 渲染 Prompt 输入框
pub fn render_prompt_input(
    frame: &mut Frame<'_>,
    input: &PromptInput,
    area: ratatui::layout::Rect,
    is_active: bool,
    palette: ThemePalette,
) {
    let (prompt_prefix, active_color) = match input.mode {
        InputMode::Normal => ("> ", palette.accent),
        InputMode::Bash => ("! ", palette.warning),
    };
    let border_color = match (is_active, input.mode.clone()) {
        (true, InputMode::Normal) => palette.accent,
        (true, InputMode::Bash) => palette.warning,
        (false, _) => palette.border,
    };

    let cursor = clamp_cursor_to_boundary(&input.text, input.cursor);
    let (before_cursor, after_cursor) = input.text.split_at(cursor);
    let cursor_char = after_cursor.chars().next().unwrap_or(' ');
    let after_visible = after_cursor
        .chars()
        .skip(if after_cursor.is_empty() { 0 } else { 1 })
        .collect::<String>();
    let placeholder = input.text.is_empty();

    let mut spans = vec![Span::styled(
        prompt_prefix,
        Style::default().fg(if is_active {
            active_color
        } else {
            palette.text_muted
        }),
    )];

    if placeholder {
        spans.push(Span::styled(
            "给 Kcode 下达任务，或输入 / 查看命令…",
            Style::default().fg(palette.text_muted),
        ));
    } else {
        spans.push(Span::raw(before_cursor.to_string()));
        if is_active {
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default()
                    .fg(palette.inverse_text)
                    .bg(active_color)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if !after_visible.is_empty() {
            spans.push(Span::raw(after_visible));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(palette.input_bg));
    let paragraph = Paragraph::new(vec![Line::from(spans)])
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(if is_active {
            palette.text
        } else {
            palette.text_muted
        }));

    frame.render_widget(paragraph, area);
}

fn prompt_content_width(available_width: u16) -> usize {
    available_width.saturating_sub(2).max(1) as usize
}

#[cfg(test)]
mod tests {
    use super::{prompt_height, PromptAction, PromptInput};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn prompt_height_grows_for_multiline_content() {
        let mut input = PromptInput::new();
        input.text = "0123456789abcdefghij0123456789".to_string();
        input.cursor = input.text.len();

        assert!(prompt_height(&input, 20) > prompt_height(&input, 80));
    }

    #[test]
    fn prompt_height_accounts_for_wide_characters() {
        let mut input = PromptInput::new();
        input.text = "你好世界你好世界".to_string();
        input.cursor = input.text.len();

        assert!(prompt_height(&input, 12) > prompt_height(&input, 24));
    }

    #[test]
    fn shift_enter_inserts_a_newline_instead_of_submitting() {
        let mut input = PromptInput::new();
        let action = input.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

        assert!(matches!(action, PromptAction::Edited));
        assert_eq!(input.text, "\n");
    }

    #[test]
    fn plain_enter_submits() {
        let mut input = PromptInput::new();
        input.text = "hello".to_string();
        input.cursor = input.text.len();

        let action = input.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(action, PromptAction::Submit));
    }

    #[test]
    fn utf8_input_uses_char_boundaries_for_cursor_movement_and_delete() {
        let mut input = PromptInput::new();

        assert!(matches!(
            input.handle_key(KeyEvent::new(KeyCode::Char('你'), KeyModifiers::NONE)),
            PromptAction::Edited
        ));
        assert_eq!(input.text, "你");
        assert_eq!(input.cursor, "你".len());

        assert!(matches!(
            input.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            PromptAction::Moved
        ));
        assert_eq!(input.cursor, 0);

        assert!(matches!(
            input.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            PromptAction::Moved
        ));
        assert_eq!(input.cursor, "你".len());

        assert!(matches!(
            input.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            PromptAction::Edited
        ));
        assert!(input.text.is_empty());
        assert_eq!(input.cursor, 0);
    }
}
