//! Theme system: ThemePalette, default theme, semantic color mapping.

use crate::render_semantic::SemanticRole;
use crate::theme_settings::current_theme_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Graphite,
    Sunset,
    Amber,
    Ocean,
}

impl ThemeName {
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "sunset" => Self::Sunset,
            "amber" => Self::Amber,
            "ocean" => Self::Ocean,
            _ => Self::Graphite,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Graphite => "graphite",
            Self::Sunset => "sunset",
            Self::Amber => "amber",
            Self::Ocean => "ocean",
        }
    }
}

/// A single entry in a theme palette: semantic role → ANSI code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    pub role: SemanticRole,
    /// ANSI escape sequence for foreground color.
    pub ansi_fg: &'static str,
    /// Whether this role should render bold in TTY mode.
    pub bold: bool,
}

/// A named collection of semantic color mappings.
#[derive(Debug, Clone)]
pub struct ThemePalette {
    pub name: &'static str,
    pub entries: &'static [ThemeEntry],
}

impl ThemePalette {
    /// The default terminal theme — optimized for dark backgrounds,
    /// low eye-strain during long REPL sessions.
    pub fn default_terminal() -> &'static Self {
        Self::graphite()
    }

    #[must_use]
    pub fn current_terminal() -> &'static Self {
        Self::for_theme(current_theme_name())
    }

    #[must_use]
    pub fn for_theme(theme: ThemeName) -> &'static Self {
        match theme {
            ThemeName::Graphite => Self::graphite(),
            ThemeName::Sunset => Self::sunset(),
            ThemeName::Amber => Self::amber(),
            ThemeName::Ocean => Self::ocean(),
        }
    }

    fn graphite() -> &'static Self {
        static PALETTE: ThemePalette = ThemePalette {
            name: "graphite",
            entries: &[
                ThemeEntry {
                    role: SemanticRole::User,
                    ansi_fg: "\x1b[36m", // cyan
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Assistant,
                    ansi_fg: "\x1b[0m", // default
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Tool,
                    ansi_fg: "\x1b[33m", // yellow
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::System,
                    ansi_fg: "\x1b[90m", // dark grey
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Warning,
                    ansi_fg: "\x1b[38;5;208m", // orange
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Error,
                    ansi_fg: "\x1b[31m", // red
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Success,
                    ansi_fg: "\x1b[32m", // green
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Memory,
                    ansi_fg: "\x1b[38;5;141m", // purple
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Compact,
                    ansi_fg: "\x1b[38;5;146m", // grey-blue
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Permission,
                    ansi_fg: "\x1b[38;5;172m", // brown-orange
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Diff,
                    ansi_fg: "\x1b[38;5;117m", // light blue
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Progress,
                    ansi_fg: "\x1b[34m", // blue
                    bold: true,
                },
            ],
        };
        &PALETTE
    }

    fn sunset() -> &'static Self {
        static PALETTE: ThemePalette = ThemePalette {
            name: "sunset",
            entries: &[
                ThemeEntry {
                    role: SemanticRole::User,
                    ansi_fg: "\x1b[38;5;216m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Assistant,
                    ansi_fg: "\x1b[0m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Tool,
                    ansi_fg: "\x1b[38;5;209m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::System,
                    ansi_fg: "\x1b[38;5;244m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Warning,
                    ansi_fg: "\x1b[38;5;220m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Error,
                    ansi_fg: "\x1b[38;5;203m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Success,
                    ansi_fg: "\x1b[38;5;151m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Memory,
                    ansi_fg: "\x1b[38;5;181m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Compact,
                    ansi_fg: "\x1b[38;5;180m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Permission,
                    ansi_fg: "\x1b[38;5;215m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Diff,
                    ansi_fg: "\x1b[38;5;223m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Progress,
                    ansi_fg: "\x1b[38;5;209m",
                    bold: true,
                },
            ],
        };
        &PALETTE
    }

    fn amber() -> &'static Self {
        static PALETTE: ThemePalette = ThemePalette {
            name: "amber",
            entries: &[
                ThemeEntry {
                    role: SemanticRole::User,
                    ansi_fg: "\x1b[38;5;221m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Assistant,
                    ansi_fg: "\x1b[0m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Tool,
                    ansi_fg: "\x1b[38;5;214m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::System,
                    ansi_fg: "\x1b[38;5;243m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Warning,
                    ansi_fg: "\x1b[38;5;220m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Error,
                    ansi_fg: "\x1b[31m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Success,
                    ansi_fg: "\x1b[32m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Memory,
                    ansi_fg: "\x1b[38;5;179m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Compact,
                    ansi_fg: "\x1b[38;5;246m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Permission,
                    ansi_fg: "\x1b[38;5;178m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Diff,
                    ansi_fg: "\x1b[38;5;229m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Progress,
                    ansi_fg: "\x1b[33m",
                    bold: true,
                },
            ],
        };
        &PALETTE
    }

    fn ocean() -> &'static Self {
        static PALETTE: ThemePalette = ThemePalette {
            name: "ocean",
            entries: &[
                ThemeEntry {
                    role: SemanticRole::User,
                    ansi_fg: "\x1b[96m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Assistant,
                    ansi_fg: "\x1b[0m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Tool,
                    ansi_fg: "\x1b[94m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::System,
                    ansi_fg: "\x1b[36m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Warning,
                    ansi_fg: "\x1b[38;5;222m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Error,
                    ansi_fg: "\x1b[91m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Success,
                    ansi_fg: "\x1b[92m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Memory,
                    ansi_fg: "\x1b[38;5;123m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Compact,
                    ansi_fg: "\x1b[38;5;110m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Permission,
                    ansi_fg: "\x1b[38;5;87m",
                    bold: true,
                },
                ThemeEntry {
                    role: SemanticRole::Diff,
                    ansi_fg: "\x1b[38;5;159m",
                    bold: false,
                },
                ThemeEntry {
                    role: SemanticRole::Progress,
                    ansi_fg: "\x1b[96m",
                    bold: true,
                },
            ],
        };
        &PALETTE
    }

    /// Look up the ANSI code for a role.
    pub fn ansi_for(&self, role: SemanticRole) -> &'static str {
        self.entries
            .iter()
            .find(|e| e.role == role)
            .map(|e| e.ansi_fg)
            .unwrap_or("")
    }

    pub fn is_bold(&self, role: SemanticRole) -> bool {
        self.entries
            .iter()
            .find(|e| e.role == role)
            .map(|e| e.bold)
            .unwrap_or(false)
    }
}

/// Render a single text span with the palette's color mapping.
pub fn render_with_palette(
    palette: &ThemePalette,
    text: &str,
    role: SemanticRole,
    allow_colors: bool,
    allow_bold: bool,
) -> String {
    if !allow_colors {
        return format!("{}{}", role.prefix_label(), text);
    }
    let color = palette.ansi_for(role);
    let bold = if allow_bold && palette.is_bold(role) {
        "\x1b[1m"
    } else {
        ""
    };
    let reset = if !color.is_empty() || !bold.is_empty() {
        "\x1b[0m"
    } else {
        ""
    };
    format!("{bold}{color}{text}{reset}")
}

/// Render multiple intents as separate lines.
pub fn render_intents(
    palette: &ThemePalette,
    intents: &[crate::render_semantic::RenderIntent],
    allow_colors: bool,
    allow_bold: bool,
) -> String {
    intents
        .iter()
        .map(|i| render_with_palette(palette, &i.text, i.role, allow_colors, allow_bold))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_semantic::RenderIntent;

    #[test]
    fn default_palette_has_all_roles() {
        let palette = ThemePalette::default_terminal();
        let roles = [
            SemanticRole::User,
            SemanticRole::Assistant,
            SemanticRole::Tool,
            SemanticRole::System,
            SemanticRole::Warning,
            SemanticRole::Error,
            SemanticRole::Success,
            SemanticRole::Memory,
            SemanticRole::Compact,
            SemanticRole::Permission,
            SemanticRole::Diff,
            SemanticRole::Progress,
        ];
        for role in roles {
            assert!(
                !palette.ansi_for(role).is_empty() || role == SemanticRole::Assistant,
                "palette missing entry for {:?}",
                role
            );
        }
    }

    #[test]
    fn render_with_palette_no_color_uses_prefix() {
        let palette = ThemePalette::default_terminal();
        let rendered =
            render_with_palette(&palette, "build failed", SemanticRole::Error, false, false);
        assert!(rendered.contains("build failed"));
        assert!(rendered.starts_with("✗ "));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    fn render_with_palette_colored_includes_ansi() {
        let palette = ThemePalette::default_terminal();
        let rendered = render_with_palette(&palette, "done", SemanticRole::Success, true, true);
        assert!(rendered.contains("done"));
        assert!(rendered.contains("\x1b["));
        assert!(rendered.ends_with("\x1b[0m"));
    }

    #[test]
    fn render_intents_joins_with_newlines() {
        let palette = ThemePalette::default_terminal();
        let intents = vec![
            RenderIntent::progress("starting"),
            RenderIntent::success("done"),
        ];
        let rendered = render_intents(&palette, &intents, true, false);
        assert!(rendered.contains("starting"));
        assert!(rendered.contains("done"));
        assert_eq!(rendered.matches('\n').count(), 1);
    }
}
