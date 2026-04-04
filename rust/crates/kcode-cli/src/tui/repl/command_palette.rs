use std::path::Path;

use commands::{
    build_command_registry_snapshot_with_cwd, CommandDescriptor, CommandRegistryContext,
    CommandSource, CommandSurface,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

const CC_COMMAND_ORDER: &[&str] = &[
    "help",
    "clear",
    "resume",
    "rename",
    "branch",
    "rewind",
    "compact",
    "config",
    "effort",
    "model",
    "permissions",
    "hooks",
    "init",
    "plugin",
    "agents",
    "powerup",
    "btw",
    "bug",
    "feedback",
    "login",
    "desktop",
    "schedule",
    "loop",
    "mcp",
    "review",
    "status",
    "cost",
    "todos",
    "commit",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandEntry {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub argument_hint: Option<String>,
    pub source: CommandSource,
}

impl SlashCommandEntry {
    fn usage(&self) -> String {
        match &self.argument_hint {
            Some(argument_hint) => format!("/{} {}", self.name, argument_hint),
            None => format!("/{}", self.name),
        }
    }

    fn insert_text(&self) -> String {
        match &self.argument_hint {
            Some(_) => format!("/{} ", self.name),
            None => format!("/{}", self.name),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SlashCommandPicker {
    pub visible: bool,
    pub filter: String,
    pub selected: usize,
    pub commands: Vec<SlashCommandEntry>,
}

impl SlashCommandPicker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh_commands(&mut self, profile_supports_tools: bool, cwd: &Path) {
        self.commands = slash_command_entries(profile_supports_tools, cwd);
        self.selected = self.selected.min(self.filtered().len().saturating_sub(1));
    }

    pub fn sync_with_input(&mut self, input: &str) {
        let next_filter = extract_palette_filter(input);
        match next_filter {
            Some(filter) => {
                if !self.visible || self.filter != filter {
                    self.selected = 0;
                }
                self.visible = true;
                self.filter = filter;
            }
            None => {
                self.visible = false;
                self.filter.clear();
                self.selected = 0;
            }
        }
        self.selected = self.selected.min(self.filtered().len().saturating_sub(1));
    }

    pub fn filtered(&self) -> Vec<&SlashCommandEntry> {
        if self.filter.is_empty() {
            return self.commands.iter().collect();
        }

        let needle = self.filter.to_ascii_lowercase();
        self.commands
            .iter()
            .filter(|entry| {
                entry.name.contains(&needle)
                    || entry.aliases.iter().any(|alias| alias.contains(&needle))
                    || entry.description.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    }

    pub fn selected_insert_text(&self) -> Option<String> {
        self.filtered()
            .get(self.selected)
            .map(|entry| entry.insert_text())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SlashPickerAction {
        match key.code {
            KeyCode::Esc => {
                self.visible = false;
                SlashPickerAction::Cancel
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                SlashPickerAction::None
            }
            KeyCode::Down => {
                let last_index = self.filtered().len().saturating_sub(1);
                self.selected = (self.selected + 1).min(last_index);
                SlashPickerAction::None
            }
            KeyCode::Enter | KeyCode::Tab => self
                .selected_insert_text()
                .map(SlashPickerAction::Select)
                .unwrap_or(SlashPickerAction::None),
            _ => SlashPickerAction::None,
        }
    }
}

pub enum SlashPickerAction {
    Select(String),
    Cancel,
    None,
}

pub fn render_slash_command_picker(
    frame: &mut Frame<'_>,
    picker: &SlashCommandPicker,
    prompt_area: Rect,
    area: Rect,
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
    let row_count = filtered.len().max(1).min(8) as u16;
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

    let mut lines = vec![Line::from(vec![
        Span::styled(
            "Commands",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if picker.filter.is_empty() {
                "  type to filter".to_string()
            } else {
                format!("  filter: {}", picker.filter)
            },
            Style::default().fg(Color::Gray),
        ),
    ])];

    let display_rows = height.saturating_sub(2) as usize;
    for (index, entry) in filtered.iter().take(display_rows).enumerate() {
        let is_selected = index == picker.selected;
        let usage = entry.usage();
        let usage_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let description_style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };
        let prefix = if is_selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(prefix, usage_style),
            Span::styled(usage, usage_style),
            Span::raw("  "),
            Span::styled(entry.description.clone(), description_style),
        ]));
    }

    if filtered.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No matching commands",
            Style::default().fg(Color::Gray),
        )]));
    }

    let block = Block::default()
        .title(" / ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(12, 18, 12)));
    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(Clear, picker_rect);
    frame.render_widget(paragraph, picker_rect);
}

fn slash_command_entries(profile_supports_tools: bool, cwd: &Path) -> Vec<SlashCommandEntry> {
    let snapshot = build_command_registry_snapshot_with_cwd(
        &CommandRegistryContext::for_surface(CommandSurface::CliLocal, profile_supports_tools),
        &[],
        cwd,
    );
    let mut ordered = snapshot
        .session_commands
        .into_iter()
        .enumerate()
        .map(|(index, descriptor)| (command_rank(&descriptor, index), descriptor))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(rank, _)| *rank);

    ordered
        .into_iter()
        .map(|(_, descriptor)| SlashCommandEntry {
            name: descriptor.name,
            aliases: descriptor.aliases,
            description: descriptor.description,
            argument_hint: descriptor.argument_hint,
            source: descriptor.source,
        })
        .collect()
}

fn command_rank(descriptor: &CommandDescriptor, original_index: usize) -> (usize, usize, usize) {
    let cc_rank = CC_COMMAND_ORDER
        .iter()
        .position(|name| *name == descriptor.name)
        .unwrap_or(CC_COMMAND_ORDER.len() + original_index);
    let source_rank = match descriptor.source {
        CommandSource::Builtin => 0,
        CommandSource::Skills => 1,
        CommandSource::Plugins => 2,
        CommandSource::Workflow => 3,
        CommandSource::Mcp => 4,
    };
    (cc_rank, source_rank, original_index)
}

fn extract_palette_filter(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('/') {
        return None;
    }
    let token = trimmed.split_whitespace().next().unwrap_or(trimmed);
    if token.len() < 2 {
        return Some(String::new());
    }
    if trimmed.contains(' ') {
        return None;
    }
    Some(token.trim_start_matches('/').to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{extract_palette_filter, slash_command_entries, SlashCommandPicker};

    #[test]
    fn extracts_filter_only_for_the_command_name_segment() {
        assert_eq!(extract_palette_filter("/"), Some(String::new()));
        assert_eq!(extract_palette_filter("/re"), Some("re".to_string()));
        assert_eq!(
            extract_palette_filter("/resume"),
            Some("resume".to_string())
        );
        assert_eq!(extract_palette_filter("/resume latest"), None);
        assert_eq!(extract_palette_filter("hello"), None);
    }

    #[test]
    fn orders_visible_commands_like_the_cc_palette_subset() {
        let cwd = std::env::current_dir().expect("cwd");
        let names = slash_command_entries(true, &cwd)
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        assert_eq!(
            &names[..13],
            &[
                "help",
                "clear",
                "resume",
                "compact",
                "config",
                "model",
                "permissions",
                "init",
                "plugin",
                "agents",
                "mcp",
                "status",
                "cost",
            ]
        );
    }

    #[test]
    fn selected_command_inserts_a_trailing_space_when_arguments_are_expected() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut picker = SlashCommandPicker::new();
        picker.refresh_commands(true, &cwd);
        picker.sync_with_input("/mod");

        assert_eq!(picker.selected_insert_text(), Some("/model ".to_string()));
    }
}
