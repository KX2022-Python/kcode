use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 斜杠命令条目
#[derive(Debug, Clone)]
pub struct SlashCommandEntry {
    pub name: String,
    pub alias: Option<String>,
    pub description: String,
    pub category: SlashCommandCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandCategory {
    Core,
    Session,
    Tools,
    Config,
    Debug,
}

/// 默认斜杠命令列表 — 对齐 CC-Haha 命令体系
pub fn default_slash_commands() -> Vec<SlashCommandEntry> {
    vec![
        SlashCommandEntry {
            name: "help".into(),
            alias: Some("h".into()),
            description: "显示命令帮助".into(),
            category: SlashCommandCategory::Core,
        },
        SlashCommandEntry {
            name: "clear".into(),
            alias: None,
            description: "清空当前会话".into(),
            category: SlashCommandCategory::Session,
        },
        SlashCommandEntry {
            name: "compact".into(),
            alias: None,
            description: "压缩上下文".into(),
            category: SlashCommandCategory::Session,
        },
        SlashCommandEntry {
            name: "resume".into(),
            alias: None,
            description: "恢复会话 /resume [path|latest]".into(),
            category: SlashCommandCategory::Session,
        },
        SlashCommandEntry {
            name: "model".into(),
            alias: None,
            description: "切换模型 /model <name>".into(),
            category: SlashCommandCategory::Config,
        },
        SlashCommandEntry {
            name: "permissions".into(),
            alias: None,
            description: "权限模式 /permissions <mode>".into(),
            category: SlashCommandCategory::Config,
        },
        SlashCommandEntry {
            name: "config".into(),
            alias: None,
            description: "查看配置 /config [section]".into(),
            category: SlashCommandCategory::Config,
        },
        SlashCommandEntry {
            name: "status".into(),
            alias: None,
            description: "会话状态".into(),
            category: SlashCommandCategory::Core,
        },
        SlashCommandEntry {
            name: "cost".into(),
            alias: None,
            description: "费用统计".into(),
            category: SlashCommandCategory::Core,
        },
        SlashCommandEntry {
            name: "diff".into(),
            alias: None,
            description: "工作区 diff".into(),
            category: SlashCommandCategory::Tools,
        },
        SlashCommandEntry {
            name: "mcp".into(),
            alias: None,
            description: "MCP 管理 /mcp [action] [target]".into(),
            category: SlashCommandCategory::Tools,
        },
        SlashCommandEntry {
            name: "memory".into(),
            alias: None,
            description: "记忆系统".into(),
            category: SlashCommandCategory::Tools,
        },
        SlashCommandEntry {
            name: "doctor".into(),
            alias: None,
            description: "健康诊断".into(),
            category: SlashCommandCategory::Debug,
        },
        SlashCommandEntry {
            name: "bughunter".into(),
            alias: None,
            description: "Bug 扫描 /bughunter [scope]".into(),
            category: SlashCommandCategory::Debug,
        },
        SlashCommandEntry {
            name: "init".into(),
            alias: None,
            description: "初始化 KCODE.md".into(),
            category: SlashCommandCategory::Core,
        },
        SlashCommandEntry {
            name: "version".into(),
            alias: None,
            description: "版本信息".into(),
            category: SlashCommandCategory::Core,
        },
        SlashCommandEntry {
            name: "export".into(),
            alias: None,
            description: "导出会话 /export [path]".into(),
            category: SlashCommandCategory::Session,
        },
        SlashCommandEntry {
            name: "agents".into(),
            alias: None,
            description: "代理管理".into(),
            category: SlashCommandCategory::Tools,
        },
        SlashCommandEntry {
            name: "skills".into(),
            alias: None,
            description: "技能管理".into(),
            category: SlashCommandCategory::Tools,
        },
        SlashCommandEntry {
            name: "plugins".into(),
            alias: None,
            description: "插件管理".into(),
            category: SlashCommandCategory::Tools,
        },
    ]
}

/// 斜杠命令选择框状态
#[derive(Debug, Clone)]
pub struct SlashCommandPicker {
    pub visible: bool,
    pub filter: String,
    pub cursor: usize,
    pub selected: usize,
    pub commands: Vec<SlashCommandEntry>,
}

impl SlashCommandPicker {
    pub fn new() -> Self {
        Self {
            visible: false,
            filter: String::new(),
            cursor: 0,
            selected: 0,
            commands: default_slash_commands(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.filter.clear();
        self.cursor = 0;
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn filtered(&self) -> Vec<&SlashCommandEntry> {
        if self.filter.is_empty() {
            self.commands.iter().collect()
        } else {
            let f = self.filter.to_lowercase();
            self.commands
                .iter()
                .filter(|cmd| {
                    cmd.name.contains(&f)
                        || cmd.alias.as_ref().is_some_and(|a| a.contains(&f))
                        || cmd.description.to_lowercase().contains(&f)
                })
                .collect()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SlashPickerAction {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                SlashPickerAction::Cancel
            }
            KeyCode::Enter => {
                let filtered = self.filtered();
                if !filtered.is_empty() && self.selected < filtered.len() {
                    let cmd = filtered[self.selected].name.clone();
                    self.hide();
                    SlashPickerAction::Select(cmd)
                } else {
                    SlashPickerAction::None
                }
            }
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                SlashPickerAction::None
            }
            KeyCode::Down => {
                let filtered = self.filtered();
                if self.selected + 1 < filtered.len() {
                    self.selected += 1;
                }
                SlashPickerAction::None
            }
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                self.filter.insert(self.cursor, c);
                self.cursor += 1;
                self.selected = 0;
                SlashPickerAction::None
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.filter.remove(self.cursor - 1);
                    self.cursor -= 1;
                    self.selected = 0;
                }
                SlashPickerAction::None
            }
            KeyCode::Delete => {
                if self.cursor < self.filter.len() {
                    self.filter.remove(self.cursor);
                    self.selected = 0;
                }
                SlashPickerAction::None
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                SlashPickerAction::None
            }
            KeyCode::Right => {
                if self.cursor < self.filter.len() {
                    self.cursor += 1;
                }
                SlashPickerAction::None
            }
            KeyCode::Tab => {
                let filtered = self.filtered();
                if !filtered.is_empty() && self.selected < filtered.len() {
                    let cmd = filtered[self.selected].name.clone();
                    self.hide();
                    SlashPickerAction::Select(cmd)
                } else {
                    SlashPickerAction::None
                }
            }
            _ => SlashPickerAction::None,
        }
    }
}

pub enum SlashPickerAction {
    Select(String),
    Cancel,
    None,
}

/// 渲染斜杠命令选择框
pub fn render_slash_command_picker(
    frame: &mut Frame<'_>,
    picker: &SlashCommandPicker,
    area: Rect,
) {
    if !picker.visible {
        return;
    }

    let filtered = picker.filtered();
    let max_height = 12.min(filtered.len() as u16 + 2);

    let picker_area = Rect {
        x: area.width.saturating_sub(52),
        y: area.height.saturating_sub(max_height + 3),
        width: 50.min(area.width),
        height: max_height,
    };

    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(
            " 🔍 Commands",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  (filter: {})", picker.filter)),
    ])];

    let max_display = (max_height as usize).saturating_sub(2);
    for (i, cmd) in filtered.iter().enumerate().take(max_display) {
        let is_selected = i == picker.selected;
        let prefix = if is_selected { "▸ " } else { "  " };
        let name_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let desc_style = if is_selected {
            Style::default()
                .fg(Color::Rgb(200, 255, 200))
                .bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::styled(format!("/{}", cmd.name), name_style),
            Span::raw("  "),
            Span::styled(&cmd.description, desc_style),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(15, 20, 15)));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, picker_area);
}

/// Prompt 输入框状态
#[derive(Debug, Clone)]
pub struct PromptInput {
    pub text: String,
    pub cursor: usize,
    pub mode: InputMode,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub scroll_offset: usize,
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
            scroll_offset: 0,
        }
    }

    pub fn submit(&mut self) -> Option<String> {
        let text = self.text.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.history.push(text.clone());
        self.history_index = None;
        let result = Some(text);
        self.text.clear();
        self.cursor = 0;
        result
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PromptAction {
        match key.code {
            KeyCode::Enter if key.modifiers.is_empty() => {
                PromptAction::Submit
            }
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                self.text.insert(self.cursor, c);
                self.cursor += 1;
                PromptAction::Edited
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.text.remove(self.cursor - 1);
                    self.cursor -= 1;
                }
                PromptAction::Edited
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    self.text.remove(self.cursor);
                }
                PromptAction::Edited
            }
            KeyCode::Left if key.modifiers == KeyModifiers::NONE => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                PromptAction::Moved
            }
            KeyCode::Right if key.modifiers == KeyModifiers::NONE => {
                if self.cursor < self.text.len() {
                    self.cursor += 1;
                }
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
                        self.text = self.history.last().unwrap().clone();
                    }
                    Some(idx) if idx > 0 => {
                        self.history_index = Some(idx - 1);
                        self.text = self.history[idx - 1].clone();
                    }
                    Some(_) => {}
                }
                self.cursor = self.text.len();
                PromptAction::Edited
            }
            KeyCode::Down if key.modifiers == KeyModifiers::NONE => {
                match self.history_index {
                    Some(idx) if idx + 1 < self.history.len() => {
                        self.history_index = Some(idx + 1);
                        self.text = self.history[idx + 1].clone();
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
            KeyCode::Char('b') if key.modifiers == KeyModifiers::CONTROL => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                PromptAction::Moved
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                if self.cursor < self.text.len() {
                    self.cursor += 1;
                }
                PromptAction::Moved
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

/// 渲染 Prompt 输入框
pub fn render_prompt_input(
    frame: &mut Frame<'_>,
    input: &PromptInput,
    area: Rect,
    is_active: bool,
) {
    let mode_label = match input.mode {
        InputMode::Normal => "PROMPT",
        InputMode::Bash => "BASH",
    };
    let mode_color = match input.mode {
        InputMode::Normal => Color::Green,
        InputMode::Bash => Color::Yellow,
    };

    // 构建带光标的文本
    let cursor = input.cursor.min(input.text.len());
    let (before, after) = input.text.split_at(cursor);
    let cursor_char = if after.is_empty() {
        "█"
    } else {
        &after.chars().next().unwrap_or(' ').to_string()
    };
    let after_cursor = if after.is_empty() {
        ""
    } else {
        &after[1.min(after.len())..]
    };

    let mut spans = vec![
        Span::styled(
            format!(" {} ", mode_label),
            Style::default()
                .fg(mode_color)
                .bg(Color::Rgb(30, 30, 30))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(if is_active { "› " } else { "  " }),
        Span::raw(before.to_string()),
    ];

    if is_active {
        spans.push(Span::styled(
            cursor_char.to_string(),
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans.push(Span::raw(after_cursor.to_string()));

    if !is_active && input.text.is_empty() {
        spans.push(Span::styled(
            "输入消息，或 / 查看命令...",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let prompt_style = if is_active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_active {
            Color::Green
        } else {
            Color::DarkGray
        }))
        .style(Style::default().bg(Color::Rgb(12, 18, 12)));

    let paragraph = ratatui::widgets::Paragraph::new(vec![Line::from(spans)])
        .block(block)
        .style(prompt_style);

    frame.render_widget(paragraph, area);
}
