use std::fmt;

use runtime::Session;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    Help,
    Status,
    Sandbox,
    Compact,
    Bughunter {
        scope: Option<String>,
    },
    Commit,
    Pr {
        context: Option<String>,
    },
    Issue {
        context: Option<String>,
    },
    DebugToolCall,
    Model {
        model: Option<String>,
    },
    Permissions {
        mode: Option<String>,
    },
    Clear {
        confirm: bool,
    },
    Cost,
    Resume {
        session_path: Option<String>,
    },
    Config {
        section: Option<String>,
    },
    Mcp {
        action: Option<String>,
        target: Option<String>,
    },
    Memory,
    Init,
    Diff,
    Version,
    Export {
        path: Option<String>,
    },
    Session {
        action: Option<String>,
        target: Option<String>,
    },
    Plugins {
        action: Option<String>,
        target: Option<String>,
    },
    Agents {
        args: Option<String>,
    },
    Skills {
        args: Option<String>,
    },
    Powerup,
    Btw {
        question: Option<String>,
    },
    Doctor,
    Login,
    Logout,
    Vim,
    Upgrade,
    Stats,
    Share,
    Feedback,
    Files,
    Fast,
    Exit,
    Summary,
    Desktop,
    Schedule {
        args: Option<String>,
    },
    Loop {
        args: Option<String>,
    },
    Brief,
    Advisor,
    Stickers,
    Insights,
    Thinkback,
    ReleaseNotes,
    SecurityReview,
    Keybindings,
    PrivacySettings,
    Plan {
        mode: Option<String>,
    },
    Review {
        scope: Option<String>,
    },
    Tasks {
        args: Option<String>,
    },
    Theme {
        name: Option<String>,
    },
    Voice {
        mode: Option<String>,
    },
    Usage {
        scope: Option<String>,
    },
    Rename {
        name: Option<String>,
    },
    Copy {
        target: Option<String>,
    },
    Hooks {
        args: Option<String>,
    },
    Context {
        action: Option<String>,
    },
    Color {
        scheme: Option<String>,
    },
    Effort {
        level: Option<String>,
    },
    Branch {
        name: Option<String>,
    },
    Rewind {
        steps: Option<String>,
    },
    Ide {
        target: Option<String>,
    },
    Tag {
        label: Option<String>,
    },
    OutputStyle {
        style: Option<String>,
    },
    AddDir {
        path: Option<String>,
    },
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandParseError {
    message: String,
}

impl SlashCommandParseError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SlashCommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SlashCommandParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandResult {
    pub message: String,
    pub session: Session,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginsCommandResult {
    pub message: String,
    pub reload_runtime: bool,
}
