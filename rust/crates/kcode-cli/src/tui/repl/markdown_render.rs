use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use crate::render::{SyntaxThemePreference, TerminalRenderer};

use super::theme::ThemePalette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StyledCell {
    ch: char,
    style: Style,
}

#[derive(Clone, Copy, Debug)]
struct AnsiStyleState {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    dim: bool,
    italic: bool,
    underlined: bool,
    crossed_out: bool,
}

impl AnsiStyleState {
    fn new(base_style: Style) -> Self {
        Self {
            fg: base_style.fg,
            bg: base_style.bg,
            bold: false,
            dim: false,
            italic: false,
            underlined: false,
            crossed_out: false,
        }
    }

    fn reset(&mut self, base_style: Style) {
        *self = Self::new(base_style);
    }

    fn to_style(self) -> Style {
        let mut style = Style::default();
        if let Some(fg) = self.fg {
            style = style.fg(fg);
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }

        let mut modifiers = Modifier::empty();
        if self.bold {
            modifiers |= Modifier::BOLD;
        }
        if self.dim {
            modifiers |= Modifier::DIM;
        }
        if self.italic {
            modifiers |= Modifier::ITALIC;
        }
        if self.underlined {
            modifiers |= Modifier::UNDERLINED;
        }
        if self.crossed_out {
            modifiers |= Modifier::CROSSED_OUT;
        }

        style.add_modifier(modifiers)
    }
}

fn renderer(prefers_light_code: bool) -> &'static TerminalRenderer {
    static DARK_RENDERER: OnceLock<TerminalRenderer> = OnceLock::new();
    static LIGHT_RENDERER: OnceLock<TerminalRenderer> = OnceLock::new();

    if prefers_light_code {
        LIGHT_RENDERER
            .get_or_init(|| TerminalRenderer::with_syntax_preference(SyntaxThemePreference::Light))
    } else {
        DARK_RENDERER
            .get_or_init(|| TerminalRenderer::with_syntax_preference(SyntaxThemePreference::Dark))
    }
}

pub(crate) fn render_markdown_lines(
    text: &str,
    width: u16,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let base_style = Style::default()
        .fg(palette.text)
        .bg(palette.assistant_msg_bg);
    let ansi = renderer(palette.prefers_light_code).render_markdown(text);
    let logical_lines = ansi_to_cells(&ansi, base_style);
    let mut wrapped = Vec::new();

    for line in logical_lines {
        wrapped.extend(wrap_cells(line, width.max(1) as usize));
    }

    if wrapped.is_empty() {
        vec![Line::from(vec![Span::styled(String::new(), base_style)])]
    } else {
        wrapped
    }
}

fn ansi_to_cells(input: &str, base_style: Style) -> Vec<Vec<StyledCell>> {
    let mut lines = vec![Vec::new()];
    let mut style = AnsiStyleState::new(base_style);
    let bytes = input.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\x1b' && bytes.get(index + 1) == Some(&b'[') {
            index += 2;
            let start = index;
            while index < bytes.len() && bytes[index] != b'm' {
                index += 1;
            }
            if index >= bytes.len() {
                break;
            }
            apply_sgr(&input[start..index], &mut style, base_style);
            index += 1;
            continue;
        }

        if bytes[index] == b'\n' {
            lines.push(Vec::new());
            index += 1;
            continue;
        }

        if bytes[index] == b'\r' {
            index += 1;
            continue;
        }

        let ch = input[index..]
            .chars()
            .next()
            .expect("index always points at a valid utf-8 boundary");
        lines
            .last_mut()
            .expect("lines always has at least one entry")
            .push(StyledCell {
                ch,
                style: style.to_style(),
            });
        index += ch.len_utf8();
    }

    if lines.is_empty() {
        vec![Vec::new()]
    } else {
        lines
    }
}

fn wrap_cells(cells: Vec<StyledCell>, width: usize) -> Vec<Line<'static>> {
    if cells.is_empty() {
        return vec![Line::default()];
    }

    let width = width.max(1);
    let mut wrapped = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0;

    for cell in cells {
        let ch_width = UnicodeWidthChar::width(cell.ch).unwrap_or(0);
        let would_overflow = current_width > 0 && ch_width > 0 && current_width + ch_width > width;

        if would_overflow {
            wrapped.push(cells_to_line(std::mem::take(&mut current)));
            current_width = 0;
        }

        current.push(cell);
        current_width += ch_width;

        if current_width >= width {
            wrapped.push(cells_to_line(std::mem::take(&mut current)));
            current_width = 0;
        }
    }

    if current.is_empty() {
        if wrapped.is_empty() {
            wrapped.push(Line::default());
        }
    } else {
        wrapped.push(cells_to_line(current));
    }

    wrapped
}

fn cells_to_line(cells: Vec<StyledCell>) -> Line<'static> {
    if cells.is_empty() {
        return Line::default();
    }

    let mut spans = Vec::new();
    let mut current_style = cells[0].style;
    let mut current_text = String::new();

    for cell in cells {
        if cell.style == current_style {
            current_text.push(cell.ch);
            continue;
        }

        spans.push(Span::styled(
            std::mem::take(&mut current_text),
            current_style,
        ));
        current_style = cell.style;
        current_text.push(cell.ch);
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, current_style));
    }

    Line::from(spans)
}

fn apply_sgr(sequence: &str, state: &mut AnsiStyleState, base_style: Style) {
    let params = if sequence.is_empty() {
        vec![0]
    } else {
        sequence
            .split(';')
            .map(|part| part.parse::<u16>().unwrap_or(0))
            .collect::<Vec<_>>()
    };

    let mut index = 0;
    while index < params.len() {
        match params[index] {
            0 => state.reset(base_style),
            1 => state.bold = true,
            2 => state.dim = true,
            3 => state.italic = true,
            4 => state.underlined = true,
            9 => state.crossed_out = true,
            22 => {
                state.bold = false;
                state.dim = false;
            }
            23 => state.italic = false,
            24 => state.underlined = false,
            29 => state.crossed_out = false,
            30..=37 => state.fg = Some(ansi_basic_color((params[index] - 30) as u8, false)),
            39 => state.fg = base_style.fg,
            40..=47 => state.bg = Some(ansi_basic_color((params[index] - 40) as u8, false)),
            49 => state.bg = base_style.bg,
            90..=97 => state.fg = Some(ansi_basic_color((params[index] - 90) as u8, true)),
            100..=107 => state.bg = Some(ansi_basic_color((params[index] - 100) as u8, true)),
            38 | 48 => {
                if let Some((color, consumed)) = parse_extended_color(&params[index..]) {
                    if params[index] == 38 {
                        state.fg = Some(color);
                    } else {
                        state.bg = Some(color);
                    }
                    index += consumed;
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn parse_extended_color(params: &[u16]) -> Option<(Color, usize)> {
    match params.get(1).copied() {
        Some(5) => params
            .get(2)
            .map(|value| (Color::Indexed((*value).min(u8::MAX as u16) as u8), 2)),
        Some(2) => match (params.get(2), params.get(3), params.get(4)) {
            (Some(r), Some(g), Some(b)) => Some((
                Color::Rgb(
                    (*r).min(u8::MAX as u16) as u8,
                    (*g).min(u8::MAX as u16) as u8,
                    (*b).min(u8::MAX as u16) as u8,
                ),
                4,
            )),
            _ => None,
        },
        _ => None,
    }
}

fn ansi_basic_color(index: u8, bright: bool) -> Color {
    match (index, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::{ansi_to_cells, render_markdown_lines};
    use crate::tui::repl::theme::ThemePreset;

    #[test]
    fn parses_truecolor_and_background_sequences() {
        let base = ThemePreset::Default.palette();
        let lines = ansi_to_cells(
            "\u{1b}[38;2;10;20;30mfg\u{1b}[48;5;236mbg\u{1b}[0m",
            ratatui::style::Style::default()
                .fg(base.text)
                .bg(base.assistant_msg_bg),
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0][0].style.fg, Some(Color::Rgb(10, 20, 30)));
        assert_eq!(lines[0][2].style.bg, Some(Color::Indexed(236)));
    }

    #[test]
    fn renders_markdown_code_blocks_into_styled_lines() {
        let lines = render_markdown_lines(
            "```rust\nfn main() {}\n```",
            80,
            ThemePreset::Default.palette(),
        );

        let joined = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("");

        assert!(joined.contains("╭─ rust"));
        assert!(joined.contains("fn main() {}"));
        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.style.bg.is_some()));
    }

    #[test]
    fn light_theme_uses_a_different_code_block_background() {
        let dark = render_markdown_lines(
            "```rust\nfn main() {}\n```",
            80,
            ThemePreset::Default.palette(),
        );
        let light = render_markdown_lines(
            "```rust\nfn main() {}\n```",
            80,
            ThemePreset::Light.palette(),
        );

        let dark_background = dark
            .iter()
            .filter(|line| line.spans.iter().any(|span| span.content.contains("fn")))
            .flat_map(|line| line.spans.iter())
            .filter_map(|span| span.style.bg)
            .find(|bg| *bg != Color::Reset);
        let light_background = light
            .iter()
            .filter(|line| line.spans.iter().any(|span| span.content.contains("fn")))
            .flat_map(|line| line.spans.iter())
            .filter_map(|span| span.style.bg)
            .find(|bg| *bg != Color::Reset);

        assert_ne!(dark_background, light_background);
    }
}
