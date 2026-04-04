use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 对话框类型 — 对齐 CC-Haha ModalContext
#[derive(Debug, Clone)]
pub enum DialogType {
    /// 帮助菜单
    Help,
    /// 模型选择器
    ModelPicker {
        models: Vec<String>,
        selected: usize,
    },
    /// 快捷键参考
    Keybindings,
    /// 会话选择器
    SessionPicker {
        sessions: Vec<String>,
        selected: usize,
    },
    /// 通用信息对话框
    Info { title: String, content: String },
}

/// 对话框状态
#[derive(Debug, Clone)]
pub struct DialogState {
    pub visible: bool,
    pub dialog_type: Option<DialogType>,
}

impl DialogState {
    pub fn new() -> Self {
        Self {
            visible: false,
            dialog_type: None,
        }
    }

    pub fn show(&mut self, dialog_type: DialogType) {
        self.visible = true;
        self.dialog_type = Some(dialog_type);
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.dialog_type = None;
    }

    pub fn is_active(&self) -> bool {
        self.visible && self.dialog_type.is_some()
    }
}

/// 渲染对话框 — 对齐 CC-Haha Overlay 系统
pub fn render_dialog(frame: &mut Frame<'_>, dialog: &DialogState, area: Rect) {
    if !dialog.is_active() {
        return;
    }

    let Some(ref dialog_type) = dialog.dialog_type else {
        return;
    };

    let lines = match dialog_type {
        DialogType::Help => build_help_dialog(),
        DialogType::ModelPicker { models, selected } => build_model_picker(models, *selected),
        DialogType::Keybindings => build_keybindings_dialog(),
        DialogType::SessionPicker { sessions, selected } => {
            build_session_picker(sessions, *selected)
        }
        DialogType::Info { title, content } => build_info_dialog(title, content),
    };

    let width = 64.min(area.width.saturating_sub(4));
    let height = (lines.len() as u16 + 4).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let dialog_rect = Rect {
        x,
        y,
        width,
        height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(12, 16, 12)));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(Clear, dialog_rect);
    frame.render_widget(paragraph, dialog_rect);
}

/// 处理对话框按键
pub fn handle_dialog_key(dialog: &mut DialogState, key: KeyEvent) -> DialogAction {
    if !dialog.is_active() {
        return DialogAction::None;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            dialog.hide();
            DialogAction::Close
        }
        KeyCode::Up => {
            if let Some(ref mut dt) = dialog.dialog_type {
                match dt {
                    DialogType::ModelPicker { selected, .. } => {
                        if *selected > 0 {
                            *selected -= 1;
                        }
                    }
                    DialogType::SessionPicker { selected, .. } => {
                        if *selected > 0 {
                            *selected -= 1;
                        }
                    }
                    _ => {}
                }
            }
            DialogAction::None
        }
        KeyCode::Down => {
            if let Some(ref mut dt) = dialog.dialog_type {
                match dt {
                    DialogType::ModelPicker { models, selected } => {
                        if *selected + 1 < models.len() {
                            *selected += 1;
                        }
                    }
                    DialogType::SessionPicker { sessions, selected } => {
                        if *selected + 1 < sessions.len() {
                            *selected += 1;
                        }
                    }
                    _ => {}
                }
            }
            DialogAction::None
        }
        KeyCode::Enter => {
            if let Some(ref dt) = dialog.dialog_type {
                match dt {
                    DialogType::ModelPicker { models, selected } => {
                        let model = models.get(*selected).cloned();
                        dialog.hide();
                        if let Some(m) = model {
                            return DialogAction::SelectModel(m);
                        }
                    }
                    DialogType::SessionPicker { sessions, selected } => {
                        let session = sessions.get(*selected).cloned();
                        dialog.hide();
                        if let Some(s) = session {
                            return DialogAction::SelectSession(s);
                        }
                    }
                    _ => {}
                }
            }
            DialogAction::None
        }
        _ => DialogAction::None,
    }
}

pub enum DialogAction {
    Close,
    SelectModel(String),
    SelectSession(String),
    None,
}

fn build_help_dialog() -> Vec<Line<'static>> {
    let lines = vec![
        Line::from(vec![Span::styled(
            " ⌨ Kcode REPL 快捷键",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Green)),
            Span::raw("          发送消息"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C", Style::default().fg(Color::Red)),
            Span::raw("         中断请求"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+D", Style::default().fg(Color::Red)),
            Span::raw("         退出 REPL"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+R", Style::default().fg(Color::Yellow)),
            Span::raw("         历史搜索"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+U", Style::default().fg(Color::Yellow)),
            Span::raw("         清空输入"),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw("            历史导航"),
        ]),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Magenta)),
            Span::raw("              命令选择框"),
        ]),
        Line::from(vec![
            Span::styled("  Tab", Style::default().fg(Color::Magenta)),
            Span::raw("            命令补全"),
        ]),
        Line::from(vec![
            Span::styled("  F1", Style::default().fg(Color::Magenta)),
            Span::raw("             显示此帮助"),
        ]),
        Line::from(vec![
            Span::styled("  F2", Style::default().fg(Color::Magenta)),
            Span::raw("             模型选择器"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  按 Esc 关闭",
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
        )]),
    ];
    lines
}

fn build_keybindings_dialog() -> Vec<Line<'static>> {
    build_help_dialog()
}

fn build_model_picker(models: &[String], selected: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![Span::styled(
        " 🤖 选择模型",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])];
    lines.push(Line::from(""));

    for (i, model) in models.iter().enumerate() {
        let style = if i == selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if i == selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![Span::styled(
            format!("{}{}", prefix, model),
            style,
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ↑↓选择 · Enter确认 · Esc关闭",
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    )]));
    lines
}

fn build_session_picker(sessions: &[String], selected: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![Span::styled(
        " 📋 选择会话",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])];
    lines.push(Line::from(""));

    for (i, session) in sessions.iter().enumerate() {
        let style = if i == selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if i == selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![Span::styled(
            format!("{}{}", prefix, session),
            style,
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ↑↓选择 · Enter确认 · Esc关闭",
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    )]));
    lines
}

fn build_info_dialog(title: &str, content: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![Span::styled(
        format!(" ℹ {}", title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])];
    lines.push(Line::from(""));

    for line in content.lines().take(15) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::Gray)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  按 Esc 关闭",
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    )]));
    lines
}
