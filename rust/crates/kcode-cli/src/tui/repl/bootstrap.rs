use std::error::Error;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;

use super::theme::{ThemePalette, ThemePreset};
use super::{
    load_bootstrap_state, persist_bootstrap_theme, persist_workspace_trust, BootstrapState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BootstrapDecision {
    Continue { theme: ThemePreset },
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootstrapStep {
    Theme,
    Trust,
}

#[derive(Debug, Clone)]
struct BootstrapApp {
    state: BootstrapState,
    step: BootstrapStep,
    theme_index: usize,
    trust_index: usize,
}

impl BootstrapApp {
    fn new(state: BootstrapState) -> Option<Self> {
        let step = if !state.theme_onboarded {
            BootstrapStep::Theme
        } else if !state.trusted_workspace {
            BootstrapStep::Trust
        } else {
            return None;
        };
        let theme_index = ThemePreset::ALL
            .iter()
            .position(|theme| *theme == state.theme)
            .unwrap_or(0);
        Some(Self {
            state,
            step,
            theme_index,
            trust_index: 0,
        })
    }

    fn selected_theme(&self) -> ThemePreset {
        ThemePreset::ALL[self.theme_index]
    }

    fn selected_theme_palette(&self) -> ThemePalette {
        self.selected_theme().palette()
    }
}

pub(crate) fn run_bootstrap_flow(cwd: &Path) -> Result<BootstrapDecision, Box<dyn Error>> {
    let state = load_bootstrap_state(cwd)?;
    let Some(mut app) = BootstrapApp::new(state) else {
        return Ok(BootstrapDecision::Continue {
            theme: load_bootstrap_state(cwd)?.theme,
        });
    };

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(BootstrapDecision::Continue {
            theme: app.state.theme,
        });
    }
    ensure_bootstrap_terminal_size()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let result = run_bootstrap_loop(&mut terminal, cwd, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn ensure_bootstrap_terminal_size() -> Result<(), Box<dyn Error>> {
    let (width, height) = crossterm::terminal::size()?;
    if bootstrap_area_usable(Rect::new(0, 0, width, height)) {
        Ok(())
    } else {
        Err("kcode bootstrap requires a terminal with non-zero size".into())
    }
}

fn bootstrap_area_usable(area: Rect) -> bool {
    area.width > 1 && area.height > 1
}

fn run_bootstrap_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    cwd: &Path,
    app: &mut BootstrapApp,
) -> Result<BootstrapDecision, Box<dyn Error>> {
    loop {
        terminal.draw(|frame| draw_bootstrap(frame, app))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        match handle_bootstrap_key(app, key) {
            BootstrapAction::None => {}
            BootstrapAction::ConfirmTheme | BootstrapAction::SkipTheme => {
                app.state = persist_bootstrap_theme(cwd, app.selected_theme())?;
                if app.state.trusted_workspace {
                    return Ok(BootstrapDecision::Continue {
                        theme: app.state.theme,
                    });
                }
                app.step = BootstrapStep::Trust;
            }
            BootstrapAction::ConfirmTrust => {
                if app.trust_index == 0 {
                    app.state = persist_workspace_trust(cwd)?;
                    return Ok(BootstrapDecision::Continue {
                        theme: app.state.theme,
                    });
                }
                return Ok(BootstrapDecision::Exit);
            }
            BootstrapAction::Exit => return Ok(BootstrapDecision::Exit),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootstrapAction {
    None,
    ConfirmTheme,
    SkipTheme,
    ConfirmTrust,
    Exit,
}

fn handle_bootstrap_key(app: &mut BootstrapApp, key: KeyEvent) -> BootstrapAction {
    if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
        return BootstrapAction::Exit;
    }

    match app.step {
        BootstrapStep::Theme => match key.code {
            KeyCode::Up => {
                app.theme_index = app.theme_index.saturating_sub(1);
                BootstrapAction::None
            }
            KeyCode::Down => {
                app.theme_index =
                    (app.theme_index + 1).min(ThemePreset::ALL.len().saturating_sub(1));
                BootstrapAction::None
            }
            KeyCode::Enter => BootstrapAction::ConfirmTheme,
            KeyCode::Esc => BootstrapAction::SkipTheme,
            _ => BootstrapAction::None,
        },
        BootstrapStep::Trust => match key.code {
            KeyCode::Up | KeyCode::Down => {
                app.trust_index = 1_usize.saturating_sub(app.trust_index);
                BootstrapAction::None
            }
            KeyCode::Enter => BootstrapAction::ConfirmTrust,
            KeyCode::Esc => BootstrapAction::Exit,
            _ => BootstrapAction::None,
        },
    }
}

fn draw_bootstrap(frame: &mut ratatui::Frame<'_>, app: &BootstrapApp) {
    if !bootstrap_area_usable(frame.area()) {
        return;
    }
    match app.step {
        BootstrapStep::Theme => render_theme_bootstrap(frame, app),
        BootstrapStep::Trust => render_trust_bootstrap(frame, app),
    }
}

fn render_theme_bootstrap(frame: &mut ratatui::Frame<'_>, app: &BootstrapApp) {
    let palette = app.selected_theme_palette();
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let welcome = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Welcome to Kcode ",
                Style::default()
                    .fg(palette.brand)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(palette.text_muted),
            ),
        ]),
        Line::from(""),
        Line::from("Let's get started."),
        Line::from("To change this later, run `kcode tui appearance`."),
    ])
    .block(
        Block::default()
            .title("Bootstrap")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent_soft))
            .style(Style::default().bg(palette.panel_bg)),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(welcome, layout[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(28)])
        .split(layout[1]);
    render_theme_list(frame, body[0], app);
    render_theme_preview(frame, body[1], app);

    frame.render_widget(
        Paragraph::new("↑/↓ 选择主题 · Enter 确认 · Esc 跳过")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.accent_soft)),
            )
            .wrap(Wrap { trim: true }),
        layout[2],
    );
}

fn render_theme_list(frame: &mut ratatui::Frame<'_>, area: Rect, app: &BootstrapApp) {
    let palette = app.selected_theme_palette();
    let items = ThemePreset::ALL
        .iter()
        .map(|theme| {
            let prefix = if *theme == app.selected_theme() {
                "❯ "
            } else {
                "  "
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(palette.accent)),
                Span::styled(theme.display_name(), Style::default().fg(palette.text)),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.theme_index));
    frame.render_stateful_widget(
        List::new(items).block(
            Block::default()
                .title("Theme")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.accent_soft)),
        ),
        area,
        &mut state,
    );
}

fn render_theme_preview(frame: &mut ratatui::Frame<'_>, area: Rect, app: &BootstrapApp) {
    let theme = app.selected_theme();
    let palette = theme.palette();
    let preview = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            theme.display_name(),
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(theme.helper_text()),
        Line::from(""),
        Line::from(vec![Span::styled(
            "function greet() {",
            Style::default().fg(palette.accent),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("console.log", Style::default().fg(palette.accent)),
            Span::styled("(\"Hello, Kcode!\");", Style::default().fg(palette.info)),
        ]),
        Line::from(vec![Span::styled("}", Style::default().fg(palette.accent))]),
    ])
    .block(
        Block::default()
            .title("Preview")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.border))
            .style(Style::default().bg(palette.dialog_bg)),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area);
    frame.render_widget(preview, area);
}

fn render_trust_bootstrap(frame: &mut ratatui::Frame<'_>, app: &BootstrapApp) {
    let palette = app.state.theme.palette();
    let area = centered_rect(frame.area(), 78, 18);
    let options = ["Yes, I trust this workspace", "No, exit"];
    let option_lines = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let prefix = if index == app.trust_index {
                "❯ "
            } else {
                "  "
            };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(palette.accent)),
                Span::styled(*option, Style::default().fg(palette.text)),
            ])
        })
        .collect::<Vec<_>>();
    let content = vec![
        Line::from(vec![Span::styled(
            "Accessing workspace:",
            Style::default()
                .fg(palette.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(app.state.workspace_root.display().to_string()),
        Line::from(""),
        Line::from("Quick safety check: confirm this is a project you created or one you trust."),
        Line::from("Kcode will be able to read, edit, and execute files here."),
        Line::from(""),
    ];

    let mut lines = content;
    lines.extend(option_lines);
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓ 选择 · Enter 确认 · Esc 退出"));

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Workspace Trust")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.warning))
                    .style(Style::default().bg(palette.dialog_bg)),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(height.min(area.height.saturating_sub(2))),
            Constraint::Min(1),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}
