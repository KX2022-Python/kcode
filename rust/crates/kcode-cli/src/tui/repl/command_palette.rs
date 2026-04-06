use std::path::Path;

use commands::{validate_slash_command_input, CommandSource};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod context;

use self::context::{
    context_title, extract_palette_filter, palette_entries, search_entries, slash_command_entries,
};
use super::theme::ThemePalette;

const MAX_PICKER_ROWS: usize = 7;
const PICKER_PAGE_STEP: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandEntryAction {
    Insert(String),
    Navigate(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandEntry {
    pub name: String,
    pub usage: String,
    pub action: SlashCommandEntryAction,
    pub aliases: Vec<String>,
    pub description: String,
    pub detail: String,
    pub argument_hint: Option<String>,
    pub source: CommandSource,
}

impl SlashCommandEntry {
    fn insert_text(&self) -> Option<String> {
        match &self.action {
            SlashCommandEntryAction::Insert(insert_text) => Some(insert_text.clone()),
            SlashCommandEntryAction::Navigate(_) => None,
        }
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
        let mut reset_selection = false;
        match next_filter {
            Some((context_command, filter)) => {
                reset_selection =
                    !self.visible || self.filter != filter || self.context_command != context_command;
                self.visible = true;
                self.filter = filter;
                self.context_command = context_command;
            }
            None => self.close(),
        }
        let filtered = self.filtered();
        if reset_selection {
            self.selected = default_selected_index(&filtered);
        }
        self.selected = self.selected.min(filtered.len().saturating_sub(1));
    }

    pub fn filtered(&self) -> Vec<SlashCommandEntry> {
        let entries = if self.filter.is_empty() {
            palette_entries(
                &self.commands,
                self.context_command.as_deref(),
                &self.available_models,
            )
        } else {
            search_entries(&self.commands, &self.available_models)
        };
        if self.filter.is_empty() {
            return entries;
        }

        let needle = self.filter.to_ascii_lowercase();
        let mut filtered = entries
            .into_iter()
            .filter(|entry| entry_matches(entry, &needle))
            .collect::<Vec<_>>();
        filtered.sort_by_key(|entry| entry_match_rank(entry, &needle));
        filtered
    }

    pub fn selected_insert_text(&self) -> Option<String> {
        self.filtered()
            .get(self.selected)
            .and_then(|entry| entry.insert_text())
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

    pub fn handle_key(&mut self, key: KeyEvent, _current_input: &str) -> SlashPickerAction {
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
                let Some(entry) = self.filtered().get(self.selected).cloned() else {
                    return SlashPickerAction::None;
                };
                match entry.action {
                    SlashCommandEntryAction::Navigate(context_command) => {
                        self.context_command = context_command;
                        self.filter.clear();
                        self.selected = default_selected_index(&self.filtered());
                        SlashPickerAction::None
                    }
                    SlashCommandEntryAction::Insert(command) => {
                        self.close();
                        if key.code == KeyCode::Enter && command_ready_for_submit(&command) {
                            SlashPickerAction::Submit(command.trim_end().to_string())
                        } else {
                            SlashPickerAction::Select(command)
                        }
                    }
                }
            }
            _ => SlashPickerAction::None,
        }
    }
}

fn command_ready_for_submit(command: &str) -> bool {
    matches!(validate_slash_command_input(command.trim_end()), Ok(Some(_)))
}
fn default_selected_index(entries: &[SlashCommandEntry]) -> usize {
    usize::from(matches!(entries.first(), Some(entry) if entry.usage == "Back") && entries.len() > 1)
}
fn truncate_display(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + w > width { break; }
        out.push(ch);
        used += w;
    }
    out
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
        .max(6);
    let row_count = filtered.len().max(1).min(MAX_PICKER_ROWS) as u16;
    let height = (row_count + 4).min(available_height);
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

    let display_rows = height.saturating_sub(4) as usize;
    let (start, end) = visible_window_bounds(filtered.len(), picker.selected, display_rows);
    let mut status_suffix = if picker.filter.is_empty() {
        "  type to filter".to_string()
    } else {
        format!("  filter: {}", picker.filter)
    };
    if !filtered.is_empty() {
        status_suffix.push_str(&format!("  {}-{}/{}", start + 1, end, filtered.len()));
    }
    let selected_detail = filtered
        .get(picker.selected)
        .map(|entry| format!("已选中: {}  · Enter 执行 · Tab 补全", entry.usage))
        .unwrap_or_else(|| "输入命令名搜索，Enter 执行当前选中项，Tab 只补全，Esc 关闭。".to_string());

    let title = context_title(picker.context_command.as_deref());
    let inner_width = picker_rect.width.saturating_sub(2) as usize;
    let status = truncate_display(
        &status_suffix,
        inner_width.saturating_sub(UnicodeWidthStr::width(title.as_str())),
    );
    let mut lines = vec![Line::from(vec![
        Span::styled(title, Style::default().fg(palette.brand).add_modifier(Modifier::BOLD)),
        Span::styled(status, Style::default().fg(palette.text_muted)),
    ])];
    lines.push(Line::from(vec![Span::styled(
        truncate_display(&format!("  {selected_detail}"), picker_rect.width.saturating_sub(2) as usize),
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
    )]));

    for (offset, entry) in filtered[start..end].iter().enumerate() {
        let index = start + offset;
        let is_selected = index == picker.selected;
        let prefix = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            Style::default().fg(palette.inverse_text).bg(palette.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text)
        };
        let row = format!("{prefix}{}  {}", entry.usage, entry.description);
        lines.push(Line::from(vec![Span::styled(
            truncate_display(&row, picker_rect.width.saturating_sub(2) as usize),
            style,
        )]));
    }

    if filtered.is_empty() {
        lines.push(Line::from(vec![Span::styled("  No matching commands", Style::default().fg(palette.text_muted))]));
    }

    let block = Block::default()
        .title(" / ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent))
        .style(Style::default().bg(palette.dialog_bg));
    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(Clear, picker_rect);
    frame.render_widget(paragraph, picker_rect);
    frame.set_cursor_position(Position::new(
        picker_rect.x + 2,
        picker_rect.y + if filtered.is_empty() { 2 } else { 3 + (picker.selected - start) as u16 },
    ));
}

fn visible_window_bounds(total: usize, selected: usize, rows: usize) -> (usize, usize) {
    if total == 0 || rows == 0 {
        return (0, 0);
    }
    let start = selected.saturating_sub(rows.min(4).saturating_sub(1)).min(total.saturating_sub(rows));
    let end = (start + rows).min(total);
    (start, end)
}

fn entry_matches(entry: &SlashCommandEntry, needle: &str) -> bool {
    entry.usage.to_ascii_lowercase().contains(needle)
        || entry.name.to_ascii_lowercase().contains(needle)
        || entry
            .aliases
            .iter()
            .any(|alias| alias.to_ascii_lowercase().contains(needle))
        || entry.description.to_ascii_lowercase().contains(needle)
        || entry.detail.to_ascii_lowercase().contains(needle)
}

fn entry_match_rank(entry: &SlashCommandEntry, needle: &str) -> u8 {
    let usage = entry.usage.to_ascii_lowercase();
    let name = entry.name.to_ascii_lowercase();
    let slash_needle = format!("/{needle}");
    let alias_exact = entry.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(needle));
    let alias_prefix = entry
        .aliases
        .iter()
        .any(|alias| alias.to_ascii_lowercase().starts_with(needle));
    if usage == slash_needle || name == needle {
        0
    } else if name.starts_with(needle) {
        1
    } else if usage.starts_with(&slash_needle) {
        2
    } else if alias_exact {
        3
    } else if alias_prefix {
        4
    } else if usage.contains(needle) {
        5
    } else if entry.description.to_ascii_lowercase().contains(needle) {
        6
    } else {
        7
    }
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
        assert_eq!(entries[0].usage, "Back");
        assert_eq!(entries[1].usage, "/model");
        assert_eq!(entries[2].usage, "/model gpt-5.4-mini");
        assert_eq!(entries[3].usage, "/model gpt-5.4");
    }
    #[test]
    fn exact_dream_command_opens_status_and_toggle_choices() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd, &[]);
        picker.sync_with_input("/dream");
        let entries = picker.filtered();
        assert_eq!(entries[1].usage, "/dream");
        assert_eq!(entries[2].usage, "/dream status");
        assert_eq!(entries[3].usage, "/dream on");
        assert_eq!(entries[4].usage, "/dream off");
    }
    #[test]
    fn root_palette_shows_top_level_groups_before_filtering() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd, &[]);
        picker.sync_with_input("/");
        let entries = picker.filtered();
        assert_eq!(entries[0].usage, "Session");
        assert_eq!(entries[1].usage, "Runtime");
        assert_eq!(entries[2].usage, "Workspace");
        assert_eq!(entries[3].usage, "Integrations");
        assert_eq!(entries[4].usage, "Automation");
    }
    #[test]
    fn filtering_searches_across_the_full_command_surface() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd, &[]);
        picker.sync_with_input("/danger");
        let entries = picker.filtered();
        assert!(entries.iter().any(|entry| entry.usage == "/permissions danger-full-access"));
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
    fn enter_submits_the_selected_command_from_prefix_filter() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd, &[]);
        picker.sync_with_input("/sta");
        assert!(matches!(
            picker.handle_key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Enter,
                    crossterm::event::KeyModifiers::NONE
                ),
                "/sta"
            ),
            super::SlashPickerAction::Submit(command) if command == "/status"
        ));
    }
    #[test]
    fn visible_window_tracks_the_selected_row() {
        assert_eq!(visible_window_bounds(0, 0, 8), (0, 0));
        assert_eq!(visible_window_bounds(3, 0, 8), (0, 3));
        assert_eq!(visible_window_bounds(20, 0, 8), (0, 8));
        assert_eq!(visible_window_bounds(20, 3, 8), (0, 8));
        assert_eq!(visible_window_bounds(20, 4, 8), (1, 9));
        assert_eq!(visible_window_bounds(20, 8, 8), (5, 13));
        assert_eq!(visible_window_bounds(20, 19, 8), (12, 20));
    }
}
