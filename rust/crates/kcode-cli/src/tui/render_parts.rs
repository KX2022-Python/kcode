use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::app::{FieldId, FieldRow, TuiApp};
use super::state::{Section, ThemePreset};

#[derive(Debug, Clone, Copy)]
pub(super) struct Palette {
    pub(super) accent: Color,
    pub(super) accent_alt: Color,
    pub(super) accent_soft: Color,
    pub(super) border: Color,
    pub(super) border_active: Color,
    pub(super) surface: Color,
    pub(super) panel: Color,
    pub(super) panel_alt: Color,
    pub(super) muted: Color,
    pub(super) success: Color,
    pub(super) warning: Color,
    pub(super) text: Color,
    pub(super) text_dim: Color,
}

pub(super) fn detail_panel(
    app: &TuiApp,
    selected_row: Option<&FieldRow>,
    palette: Palette,
) -> Paragraph<'static> {
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "What This Section Controls",
            Style::default()
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(section_description(app.section())),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Current Focus",
            Style::default()
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    if let Some(row) = selected_row {
        lines.push(Line::from(vec![
            Span::styled("Field  ", Style::default().fg(palette.text_dim)),
            Span::styled(row.label.clone(), Style::default().fg(palette.text)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Mode   ", Style::default().fg(palette.text_dim)),
            Span::styled(
                if row.editable { "editable" } else { "readonly" },
                Style::default().fg(if row.editable {
                    palette.success
                } else {
                    palette.muted
                }),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Value  ", Style::default().fg(palette.text_dim)),
            Span::styled(
                summarize_value(&display_value(row)),
                value_style(row, palette),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Guidance",
            Style::default()
                .fg(palette.accent_alt)
                .add_modifier(Modifier::BOLD),
        )]));
        for line in field_guidance(app.section(), &row.label) {
            lines.push(Line::from(line));
        }
    } else {
        lines.push(Line::from("No field selected."));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Theme Preview",
        Style::default()
            .fg(palette.accent_alt)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::styled("Accent ", Style::default().fg(palette.accent)),
        Span::styled("Success ", Style::default().fg(palette.success)),
        Span::styled("Warning ", Style::default().fg(palette.warning)),
        Span::styled("Muted", Style::default().fg(palette.muted)),
    ]));

    Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Context ")
                .borders(Borders::ALL)
                .style(Style::default().bg(palette.surface))
                .border_style(Style::default().fg(palette.border)),
        )
        .wrap(Wrap { trim: true })
}

pub(super) fn display_value(row: &FieldRow) -> String {
    if row.value.trim().is_empty() && row.id != FieldId::ReadOnly {
        "<unset>".to_string()
    } else {
        row.value.clone()
    }
}

pub(super) fn value_style(row: &FieldRow, palette: Palette) -> Style {
    let value = display_value(row);
    if row.id == FieldId::ReadOnly {
        return Style::default().fg(palette.text);
    }
    if value == "<unset>" {
        return Style::default().fg(palette.warning);
    }
    Style::default().fg(palette.text)
}

pub(super) fn section_badge(section: Section) -> &'static str {
    match section {
        Section::Overview => "◎",
        Section::Provider => "◉",
        Section::Runtime => "⌘",
        Section::Sandbox => "⛶",
        Section::Extensions => "✦",
        Section::Mcp => "⇄",
        Section::Bridge => "☍",
        Section::Appearance => "◌",
        Section::Review => "✓",
    }
}

pub(super) fn palette(theme: ThemePreset) -> Palette {
    match theme {
        ThemePreset::Default => Palette {
            accent: Color::Rgb(150, 179, 255),
            accent_alt: Color::Rgb(216, 228, 255),
            accent_soft: Color::Rgb(85, 106, 150),
            border: Color::Rgb(70, 79, 96),
            border_active: Color::Rgb(150, 179, 255),
            surface: Color::Rgb(19, 22, 29),
            panel: Color::Rgb(14, 17, 24),
            panel_alt: Color::Rgb(28, 34, 45),
            muted: Color::Rgb(118, 126, 141),
            success: Color::Rgb(113, 211, 163),
            warning: Color::Rgb(255, 191, 87),
            text: Color::Rgb(235, 239, 247),
            text_dim: Color::Rgb(162, 170, 186),
        },
        ThemePreset::Amber => Palette {
            accent: Color::Yellow,
            accent_alt: Color::Rgb(255, 220, 120),
            accent_soft: Color::Rgb(210, 160, 30),
            border: Color::Rgb(120, 90, 22),
            border_active: Color::Rgb(255, 208, 72),
            surface: Color::Rgb(26, 20, 8),
            panel: Color::Rgb(42, 30, 8),
            panel_alt: Color::Rgb(58, 42, 12),
            muted: Color::Rgb(168, 158, 138),
            success: Color::Rgb(135, 224, 164),
            warning: Color::Rgb(255, 210, 92),
            text: Color::Rgb(255, 248, 231),
            text_dim: Color::Rgb(214, 198, 162),
        },
        ThemePreset::Ocean => Palette {
            accent: Color::Cyan,
            accent_alt: Color::Rgb(176, 248, 255),
            accent_soft: Color::Rgb(40, 150, 170),
            border: Color::Rgb(34, 103, 115),
            border_active: Color::Rgb(114, 232, 246),
            surface: Color::Rgb(7, 24, 31),
            panel: Color::Rgb(8, 32, 42),
            panel_alt: Color::Rgb(10, 47, 58),
            muted: Color::Rgb(138, 162, 168),
            success: Color::Rgb(114, 226, 181),
            warning: Color::Rgb(255, 205, 102),
            text: Color::Rgb(234, 249, 255),
            text_dim: Color::Rgb(172, 207, 214),
        },
        ThemePreset::CatppuccinMocha => Palette {
            accent: Color::Rgb(137, 180, 250),
            accent_alt: Color::Rgb(180, 190, 254),
            accent_soft: Color::Rgb(88, 91, 112),
            border: Color::Rgb(108, 112, 134),
            border_active: Color::Rgb(137, 180, 250),
            surface: Color::Rgb(30, 30, 46),
            panel: Color::Rgb(24, 24, 37),
            panel_alt: Color::Rgb(49, 50, 68),
            muted: Color::Rgb(166, 173, 200),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(148, 156, 187),
        },
        ThemePreset::DarkHighContrast => Palette {
            accent: Color::Rgb(153, 209, 255),
            accent_alt: Color::White,
            accent_soft: Color::Rgb(99, 120, 150),
            border: Color::Rgb(120, 140, 170),
            border_active: Color::Rgb(153, 209, 255),
            surface: Color::Rgb(6, 8, 12),
            panel: Color::Rgb(10, 12, 18),
            panel_alt: Color::Rgb(20, 24, 34),
            muted: Color::Rgb(180, 190, 205),
            success: Color::Rgb(140, 235, 170),
            warning: Color::Rgb(255, 220, 120),
            text: Color::Rgb(245, 248, 255),
            text_dim: Color::Rgb(200, 208, 220),
        },
        ThemePreset::Light => Palette {
            accent: Color::Rgb(48, 104, 196),
            accent_alt: Color::Rgb(12, 64, 140),
            accent_soft: Color::Rgb(180, 198, 226),
            border: Color::Rgb(190, 202, 222),
            border_active: Color::Rgb(48, 104, 196),
            surface: Color::Rgb(246, 248, 252),
            panel: Color::Rgb(255, 255, 255),
            panel_alt: Color::Rgb(236, 240, 247),
            muted: Color::Rgb(106, 116, 134),
            success: Color::Rgb(45, 140, 87),
            warning: Color::Rgb(176, 118, 8),
            text: Color::Rgb(25, 32, 46),
            text_dim: Color::Rgb(86, 96, 112),
        },
    }
}

fn summarize_value(value: &str) -> String {
    if value.len() > 48 {
        format!("{}…", &value[..48])
    } else {
        value.to_string()
    }
}

fn section_description(section: Section) -> &'static str {
    match section {
        Section::Overview => "Read the current runtime posture, loaded config files, and doctor summary.",
        Section::Provider => "Choose a provider profile and fill endpoint, API key env name, and default model.",
        Section::Runtime => "Define permission behavior and where sessions are persisted on disk.",
        Section::Sandbox => "Control filesystem and network isolation for tool execution.",
        Section::Extensions => "Manage hooks, plugin directories, and plugin activation state.",
        Section::Mcp => "Register MCP servers and edit their transport-specific connection details.",
        Section::Bridge => "Store Telegram, WhatsApp, and Feishu bridge secrets outside source control.",
        Section::Appearance => "Change theme, redaction behavior, and keybinding presentation for both the setup deck and the REPL.",
        Section::Review => "Inspect the flattened save result before leaving the configuration deck.",
    }
}

fn field_guidance(section: Section, label: &str) -> Vec<&'static str> {
    match (section, label) {
        (Section::Provider, "Active profile") => vec![
            "Use `custom` for any OpenAI-compatible endpoint.",
            "Switch only after you know which provider contract you want.",
        ],
        (Section::Provider, "Base URL") => vec![
            "Point this to the provider's OpenAI-compatible base URL.",
            "Do not place secrets in this field.",
        ],
        (Section::Provider, "API key env") => vec![
            "This must be an environment variable name, not the key itself.",
            "Examples: KCODE_API_KEY, OPENAI_API_KEY.",
        ],
        (Section::Provider, "Default model") => vec![
            "Use a model string that your provider really exposes.",
            "This becomes the launch default for `kcode`.",
        ],
        (Section::Bridge, _) => vec![
            "Bridge secrets are written to bridge.env, not to source control.",
            "Keep redaction enabled unless you are actively verifying values.",
        ],
        (Section::Appearance, _) => vec![
            "Theme changes should carry into the chat REPL and status output, not only this deck.",
            "Use Appearance before editing secrets or bridge values.",
        ],
        _ => vec![
            "Move with arrows, edit with Enter, and save with `s`.",
            "Use Review before quitting if you changed multiple sections.",
        ],
    }
}
