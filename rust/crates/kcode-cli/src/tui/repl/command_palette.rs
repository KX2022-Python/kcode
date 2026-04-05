use std::path::Path;

use commands::CommandSource;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

mod context;

use self::context::{extract_palette_filter, palette_entries, slash_command_entries};
use super::theme::ThemePalette;

const MAX_PICKER_ROWS: usize = 8;
const PICKER_PAGE_STEP: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandEntry {
    pub name: String,
    pub usage: String,
    pub insert_text: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub argument_hint: Option<String>,
    pub source: CommandSource,
}

impl SlashCommandEntry {
    fn insert_text(&self) -> String {
        self.insert_text.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SlashCommandPicker {
    pub visible: bool,
    pub filter: String,
    pub context_command: Option<String>,
    pub selected: usize,
    pub commands: Vec<SlashCommandEntry>,
    pub available_models: Vec<String>,
}

impl SlashCommandPicker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh_commands(
        &mut self,
        profile_supports_tools: bool,
        cwd: &Path,
        available_models: &[String],
    ) {
        self.available_models = available_models.to_vec();
        self.commands = slash_command_entries(profile_supports_tools, cwd);
        self.selected = self.selected.min(self.filtered().len().saturating_sub(1));
    }

    pub fn sync_with_input(&mut self, input: &str) {
        let next_filter = extract_palette_filter(input, &self.available_models);
        match next_filter {
            Some((context_command, filter)) => {
                if !self.visible || self.filter != filter || self.context_command != context_command
                {
                    self.selected = 0;
                }
                self.visible = true;
                self.filter = filter;
                self.context_command = context_command;
            }
            None => self.close(),
        }
        self.selected = self.selected.min(self.filtered().len().saturating_sub(1));
    }

    pub fn filtered(&self) -> Vec<SlashCommandEntry> {
        let entries = palette_entries(
            &self.commands,
            self.context_command.as_deref(),
            &self.available_models,
        );
        if self.filter.is_empty() {
            return entries;
        }

        let needle = self.filter.to_ascii_lowercase();
        entries
            .into_iter()
            .filter(|entry| {
                entry.name.to_ascii_lowercase().contains(&needle)
                    || entry
                        .aliases
                        .iter()
                        .any(|alias| alias.to_ascii_lowercase().contains(&needle))
                    || entry.description.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    }

    pub fn selected_insert_text(&self) -> Option<String> {
        self.filtered()
            .get(self.selected)
            .map(|entry| entry.insert_text())
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.filter.clear();
        self.context_command = None;
        self.selected = 0;
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        let last_index = self.filtered().len().saturating_sub(1);
        self.selected = (self.selected + 1).min(last_index);
    }

    pub fn handle_key(&mut self, key: KeyEvent, current_input: &str) -> SlashPickerAction {
        match key.code {
            KeyCode::Esc => {
                self.close();
                SlashPickerAction::Cancel
            }
            KeyCode::Up => {
                self.select_previous();
                SlashPickerAction::None
            }
            KeyCode::Down => {
                self.select_next();
                SlashPickerAction::None
            }
            KeyCode::Home => {
                self.selected = 0;
                SlashPickerAction::None
            }
            KeyCode::End => {
                self.selected = self.filtered().len().saturating_sub(1);
                SlashPickerAction::None
            }
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(PICKER_PAGE_STEP);
                SlashPickerAction::None
            }
            KeyCode::PageDown => {
                let last_index = self.filtered().len().saturating_sub(1);
                self.selected = (self.selected + PICKER_PAGE_STEP).min(last_index);
                SlashPickerAction::None
            }
            KeyCode::Enter | KeyCode::Tab => {
                let Some(command) = self.selected_insert_text() else {
                    return SlashPickerAction::None;
                };
                self.close();
                if key.code == KeyCode::Enter && current_input == command {
                    SlashPickerAction::Submit(command)
                } else {
                    SlashPickerAction::Select(command)
                }
            }
            _ => SlashPickerAction::None,
        }
    }
}

pub enum SlashPickerAction {
    Select(String),
    Submit(String),
    Cancel,
    None,
}

pub fn render_slash_command_picker(
    frame: &mut Frame<'_>,
    picker: &SlashCommandPicker,
    prompt_area: Rect,
    area: Rect,
    palette: ThemePalette,
) {
    if !picker.visible {
        return;
    }

    let filtered = picker.filtered();
    let available_height = prompt_area
        .y
        .saturating_sub(area.y)
        .saturating_sub(1)
        .max(4);
    let row_count = filtered.len().max(1).min(MAX_PICKER_ROWS) as u16;
    let height = (row_count + 2).min(available_height);
    let width = area.width.saturating_sub(4).clamp(36, 80);
    let x = if prompt_area.width > width {
        prompt_area.x
    } else {
        area.x + (area.width.saturating_sub(width)) / 2
    };
    let y = prompt_area.y.saturating_sub(height).max(area.y + 1);
    let picker_rect = Rect {
        x,
        y,
        width,
        height,
    };

    let display_rows = height.saturating_sub(2) as usize;
    let (start, end) = visible_window_bounds(filtered.len(), picker.selected, display_rows);
    let mut status_suffix = if picker.filter.is_empty() {
        "  type to filter".to_string()
    } else {
        format!("  filter: {}", picker.filter)
    };
    if !filtered.is_empty() {
        status_suffix.push_str(&format!("  {}-{}/{}", start + 1, end, filtered.len()));
    }

    let mut lines = vec![Line::from(vec![
        Span::styled(
            picker
                .context_command
                .as_deref()
                .map_or("Commands".to_string(), |command| format!("/{command}")),
            Style::default()
                .fg(palette.brand)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(status_suffix, Style::default().fg(palette.text_muted)),
    ])];

    for (offset, entry) in filtered[start..end].iter().enumerate() {
        let index = start + offset;
        let is_selected = index == picker.selected;
        let usage_style = if is_selected {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.brand)
        };
        let description_style = if is_selected {
            Style::default().fg(palette.text)
        } else {
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::DIM)
        };
        let prefix = if is_selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(prefix, usage_style),
            Span::styled(entry.usage.clone(), usage_style),
            Span::raw("  "),
            Span::styled(entry.description.clone(), description_style),
        ]));
    }

    if filtered.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No matching commands",
            Style::default().fg(palette.text_muted),
        )]));
    }

    let block = Block::default()
        .title(" / ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent))
        .style(Style::default().bg(palette.dialog_bg));
    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(Clear, picker_rect);
    frame.render_widget(paragraph, picker_rect);
}

fn visible_window_bounds(total: usize, selected: usize, rows: usize) -> (usize, usize) {
    if total == 0 || rows == 0 {
        return (0, 0);
    }
    let start = selected
        .saturating_add(1)
        .saturating_sub(rows)
        .min(total.saturating_sub(rows));
    let end = (start + rows).min(total);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::{extract_palette_filter, visible_window_bounds, SlashCommandPicker};

    #[test]
    fn extracts_filter_only_for_the_command_name_segment() {
        assert_eq!(
            extract_palette_filter("/", &[]),
            Some((None, String::new()))
        );
        assert_eq!(
            extract_palette_filter("/re", &[]),
            Some((None, "re".to_string()))
        );
        assert_eq!(
            extract_palette_filter("/resume", &[]),
            Some((None, "resume".to_string()))
        );
        assert_eq!(
            extract_palette_filter("/permissions danger", &[]),
            Some((Some("permissions".to_string()), "danger".to_string()))
        );
        assert_eq!(
            extract_palette_filter("/model", &["gpt-5.4".to_string()]),
            Some((Some("model".to_string()), String::new()))
        );
        assert_eq!(extract_palette_filter("/resume latest", &[]), None);
        assert_eq!(extract_palette_filter("hello", &[]), None);
    }

    #[test]
    fn selected_command_inserts_a_trailing_space_when_arguments_are_expected() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd, &[]);
        picker.sync_with_input("/mod");

        assert_eq!(picker.selected_insert_text(), Some("/model ".to_string()));
    }

    #[test]
    fn exact_model_command_opens_the_model_context_palette() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(
            true,
            &cwd,
            &["gpt-5.4-mini".to_string(), "gpt-5.4".to_string()],
        );
        picker.sync_with_input("/model");

        let entries = picker.filtered();
        assert_eq!(entries[0].usage, "/model");
        assert_eq!(entries[1].usage, "/model gpt-5.4-mini");
        assert_eq!(entries[2].usage, "/model gpt-5.4");
    }

    #[test]
    fn enter_submits_when_the_exact_palette_command_is_already_present() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd, &[]);
        picker.sync_with_input("/status");

        assert!(matches!(
            picker.handle_key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Enter,
                    crossterm::event::KeyModifiers::NONE
                ),
                "/status"
            ),
            super::SlashPickerAction::Submit(command) if command == "/status"
        ));
    }

    #[test]
    fn visible_window_tracks_the_selected_row() {
        assert_eq!(visible_window_bounds(0, 0, 8), (0, 0));
        assert_eq!(visible_window_bounds(3, 0, 8), (0, 3));
        assert_eq!(visible_window_bounds(12, 0, 8), (0, 8));
        assert_eq!(visible_window_bounds(12, 8, 8), (1, 9));
        assert_eq!(visible_window_bounds(12, 11, 8), (4, 12));
    }
}
