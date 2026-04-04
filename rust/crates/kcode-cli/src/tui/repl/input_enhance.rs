use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 历史搜索状态 — 对齐 CC-Haha HistorySearchDialog
#[derive(Debug, Clone)]
pub struct HistorySearch {
    pub active: bool,
    pub query: String,
    pub cursor: usize,
    pub selected_index: Option<usize>,
    pub matches: Vec<usize>, // 原始历史数组中的索引
}

impl HistorySearch {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            cursor: 0,
            selected_index: None,
            matches: Vec::new(),
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.query.clear();
        self.cursor = 0;
        self.selected_index = None;
        self.matches.clear();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// 在历史中搜索
    pub fn search(&mut self, history: &[String]) {
        if self.query.is_empty() {
            self.matches.clear();
            self.selected_index = None;
            return;
        }
        let q = self.query.to_lowercase();
        self.matches = history
            .iter()
            .enumerate()
            .rev() // 从新到旧
            .filter(|(_, entry)| entry.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.selected_index = if self.matches.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn handle_key(&mut self, key: KeyEvent, history: &[String]) -> HistorySearchAction {
        match key.code {
            KeyCode::Esc => {
                self.deactivate();
                HistorySearchAction::Cancel
            }
            KeyCode::Enter => {
                if let Some(idx) = self.selected_index {
                    if idx < self.matches.len() {
                        let history_idx = self.matches[idx];
                        let entry = history[history_idx].clone();
                        self.deactivate();
                        return HistorySearchAction::Select(entry);
                    }
                }
                self.deactivate();
                HistorySearchAction::Cancel
            }
            KeyCode::Up => {
                if let Some(idx) = self.selected_index {
                    if idx > 0 {
                        self.selected_index = Some(idx - 1);
                    }
                }
                HistorySearchAction::None
            }
            KeyCode::Down => {
                if let Some(idx) = self.selected_index {
                    if idx + 1 < self.matches.len() {
                        self.selected_index = Some(idx + 1);
                    }
                }
                HistorySearchAction::None
            }
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                self.query.insert(self.cursor, c);
                self.cursor += 1;
                self.search(history);
                HistorySearchAction::Updated
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.query.remove(self.cursor - 1);
                    self.cursor -= 1;
                    self.search(history);
                }
                HistorySearchAction::Updated
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                HistorySearchAction::None
            }
            KeyCode::Right => {
                if self.cursor < self.query.len() {
                    self.cursor += 1;
                }
                HistorySearchAction::None
            }
            _ => HistorySearchAction::None,
        }
    }
}

pub enum HistorySearchAction {
    Select(String),
    Cancel,
    Updated,
    None,
}

/// 输入暂存 — 对齐 CC-Haha PromptInputStashNotice
#[derive(Debug, Clone)]
pub struct InputStash {
    pub text: String,
    pub cursor: usize,
}

impl InputStash {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn stash(&mut self, text: &str, cursor: usize) {
        self.text = text.to_string();
        self.cursor = cursor;
    }

    pub fn restore(&self) -> (String, usize) {
        (self.text.clone(), self.cursor)
    }

    pub fn has_stash(&self) -> bool {
        !self.text.is_empty()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }
}

/// 输入语法高亮 — 对齐 CC-Haha 输入关键字高亮
pub fn highlight_input(text: &str) -> Vec<(String, InputHighlightType)> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_type = InputHighlightType::Normal;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 检测斜杠命令
        if chars[i] == '/' && (i == 0 || chars[i - 1] == ' ') {
            if !current.is_empty() {
                segments.push((current.clone(), current_type));
                current.clear();
            }
            current_type = InputHighlightType::SlashCommand;
            current.push(chars[i]);
        } else if current_type == InputHighlightType::SlashCommand
            && (chars[i] == ' ' || i == chars.len() - 1)
        {
            if i == chars.len() - 1 {
                current.push(chars[i]);
            }
            segments.push((current.clone(), current_type));
            current.clear();
            current_type = InputHighlightType::Normal;
            if i < chars.len() - 1 || chars[i] != ' ' {
                current.push(chars[i]);
            }
        } else {
            current.push(chars[i]);
        }
        i += 1;
    }

    if !current.is_empty() {
        segments.push((current, current_type));
    }

    segments
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputHighlightType {
    Normal,
    SlashCommand,
}
