use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crossterm::event::{KeyCode, KeyEvent};

/// 简单 Diff 查看器 — 对齐 CC-Haha DiffDialog
pub struct DiffViewer {
    pub visible: bool,
    pub file_path: String,
    pub diff_lines: Vec<DiffLine>,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub enum DiffLine {
    Added(String),
    Removed(String),
    Context(String),
    Header(String),
}

impl DiffViewer {
    pub fn new() -> Self {
        Self {
            visible: false,
            file_path: String::new(),
            diff_lines: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn show(&mut self, file_path: String, diff_lines: Vec<DiffLine>) {
        self.visible = true;
        self.file_path = file_path;
        self.diff_lines = diff_lines;
        self.scroll_offset = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = self.diff_lines.len().saturating_sub(20);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down();
                false
            }
            _ => false,
        }
    }
}

pub fn render_diff_viewer(frame: &mut Frame<'_>, viewer: &DiffViewer, area: Rect) {
    if !viewer.visible {
        return;
    }

    let width = 80.min(area.width.saturating_sub(4));
    let height = 24.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let viewer_rect = Rect {
        x,
        y,
        width,
        height,
    };

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            format!(" 📄 {} ", viewer.file_path),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    let max_display = height.saturating_sub(4) as usize;
    let start = viewer.scroll_offset;
    let end = (start + max_display).min(viewer.diff_lines.len());

    for diff_line in &viewer.diff_lines[start..end] {
        let line = match diff_line {
            DiffLine::Added(text) => Line::from(vec![
                Span::styled("+ ", Style::default().fg(Color::Green)),
                Span::styled(text.clone(), Style::default().fg(Color::Green)),
            ]),
            DiffLine::Removed(text) => Line::from(vec![
                Span::styled("- ", Style::default().fg(Color::Red)),
                Span::styled(text.clone(), Style::default().fg(Color::Red)),
            ]),
            DiffLine::Context(text) => Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(text.clone(), Style::default().fg(Color::Gray)),
            ]),
            DiffLine::Header(text) => Line::from(vec![Span::styled(
                text.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
        };
        lines.push(line);
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ↑↓滚动 · Esc关闭",
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    )]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(12, 12, 18)));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(Clear, viewer_rect);
    frame.render_widget(paragraph, viewer_rect);
}
