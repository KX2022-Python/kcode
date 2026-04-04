mod app;
mod config_store;
mod render;
mod render_parts;
pub(crate) mod repl;
pub(crate) mod state;

use std::error::Error;
use std::io::{self, IsTerminal};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use self::app::TuiApp;
use self::state::Section;

pub(crate) fn run(section: Option<&str>) -> Result<(), Box<dyn Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "kcode tui requires an interactive terminal; use `kcode config show` or `kcode doctor` instead"
                .into(),
        );
    }
    let cwd = std::env::current_dir()?;
    let initial_section = section
        .map(parse_section)
        .transpose()?
        .unwrap_or(Section::Overview);
    let mut app = TuiApp::load(cwd, initial_section)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
) -> Result<(), Box<dyn Error>> {
    while !app.should_quit() {
        terminal.draw(|frame| render::draw(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
    }
    Ok(())
}

fn parse_section(value: &str) -> Result<Section, Box<dyn Error>> {
    match value.trim().to_ascii_lowercase().as_str() {
        "overview" => Ok(Section::Overview),
        "provider" | "profile" | "model" => Ok(Section::Provider),
        "runtime" | "permissions" => Ok(Section::Runtime),
        "sandbox" => Ok(Section::Sandbox),
        "extensions" | "plugins" | "hooks" => Ok(Section::Extensions),
        "mcp" => Ok(Section::Mcp),
        "bridge" | "telegram" | "whatsapp" | "feishu" => Ok(Section::Bridge),
        "appearance" | "theme" | "color" | "privacy" => Ok(Section::Appearance),
        "review" => Ok(Section::Review),
        other => Err(format!(
            "unsupported TUI section `{other}`; expected overview, provider, runtime, sandbox, extensions, mcp, bridge, appearance, or review"
        )
        .into()),
    }
}

/// REPL TUI 入口 — 全屏 AI 编程会话界面
pub fn run_repl<F>(
    model: String,
    profile: String,
    session_id: String,
    permission_mode: String,
    profile_supports_tools: bool,
    welcome_messages: Vec<repl::RenderableMessage>,
    executor: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(repl::SubmittedCommand) -> Result<repl::BackendResult, String>,
{
    repl::run_repl(
        model,
        profile,
        session_id,
        permission_mode,
        profile_supports_tools,
        welcome_messages,
        executor,
    )
}
