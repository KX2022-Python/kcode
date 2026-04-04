//! Semantic rendering layer: RenderIntent, SemanticRole, RenderPolicy.
//! This is the unified semantic layer shared by CLI, logs, and bridge.

use std::fmt;

/// Semantic role for a text span.
/// Colors serve semantics, not decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRole {
    /// User input or user context reference
    User,
    /// Assistant's formal response
    Assistant,
    /// Tool invocation and tool results
    Tool,
    /// System state and runtime hints
    System,
    /// Recoverable risk
    Warning,
    /// Failure or blocking condition
    Error,
    /// Completion or passing condition
    Success,
    /// Memory read/write/injection prompts
    Memory,
    /// Compaction boundary and result summaries
    Compact,
    /// Permission decisions and denials
    Permission,
    /// Patch / file diff / edit summaries
    Diff,
    /// Stage progression and streaming state
    Progress,
}

impl SemanticRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::System => "system",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Success => "success",
            Self::Memory => "memory",
            Self::Compact => "compact",
            Self::Permission => "permission",
            Self::Diff => "diff",
            Self::Progress => "progress",
        }
    }

    /// Prefix label for no-color / degraded rendering.
    pub fn prefix_label(self) -> &'static str {
        match self {
            Self::User => "▸ ",
            Self::Assistant => "  ",
            Self::Tool => "⚙ ",
            Self::System => "ℹ ",
            Self::Warning => "⚠ ",
            Self::Error => "✗ ",
            Self::Success => "✓ ",
            Self::Memory => "⌘ ",
            Self::Compact => "◈ ",
            Self::Permission => "⊘ ",
            Self::Diff => "± ",
            Self::Progress => "⟳ ",
        }
    }
}

/// A block of text with a single semantic role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderIntent {
    pub role: SemanticRole,
    pub text: String,
}

impl RenderIntent {
    pub fn new(role: SemanticRole, text: impl Into<String>) -> Self {
        Self {
            role,
            text: text.into(),
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::User, text)
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Assistant, text)
    }
    pub fn tool(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Tool, text)
    }
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::System, text)
    }
    pub fn warning(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Warning, text)
    }
    pub fn error(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Error, text)
    }
    pub fn success(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Success, text)
    }
    pub fn memory(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Memory, text)
    }
    pub fn compact(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Compact, text)
    }
    pub fn permission(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Permission, text)
    }
    pub fn diff(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Diff, text)
    }
    pub fn progress(text: impl Into<String>) -> Self {
        Self::new(SemanticRole::Progress, text)
    }
}

/// Rendering policy controlling color, formatting, and width.
#[derive(Debug, Clone)]
pub struct RenderPolicy {
    pub allow_colors: bool,
    pub allow_bold: bool,
    pub allow_dim: bool,
    pub max_width: Option<usize>,
}

impl RenderPolicy {
    /// Detect from environment and TTY state.
    pub fn detect(is_tty: bool) -> Self {
        let no_color = std::env::var("NO_COLOR").is_ok();
        Self {
            allow_colors: is_tty && !no_color,
            allow_bold: is_tty,
            allow_dim: is_tty,
            max_width: None,
        }
    }

    pub fn no_color() -> Self {
        Self {
            allow_colors: false,
            allow_bold: false,
            allow_dim: false,
            max_width: None,
        }
    }

    /// Render a single RenderIntent to a string respecting policy.
    pub fn render_intent(&self, intent: &RenderIntent) -> String {
        if self.allow_colors {
            self.render_colored(intent)
        } else {
            self.render_plain(intent)
        }
    }

    fn render_colored(&self, intent: &RenderIntent) -> String {
        let color = intent.role.ansi_fg();
        let prefix = if self.allow_bold && intent.role.needs_bold() {
            "\x1b[1m"
        } else {
            ""
        };
        let reset = if !color.is_empty() || !prefix.is_empty() {
            "\x1b[0m"
        } else {
            ""
        };
        format!("{prefix}{color}{}{reset}", intent.text)
    }

    fn render_plain(&self, intent: &RenderIntent) -> String {
        let prefix = intent.role.prefix_label();
        format!("{prefix}{}", intent.text)
    }
}

impl SemanticRole {
    /// ANSI foreground color code for this role.
    fn ansi_fg(self) -> &'static str {
        match self {
            Self::User => "\x1b[36m",             // cyan
            Self::Assistant => "\x1b[0m",         // default
            Self::Tool => "\x1b[33m",             // yellow
            Self::System => "\x1b[90m",           // dark grey
            Self::Warning => "\x1b[38;5;208m",    // orange
            Self::Error => "\x1b[31m",            // red
            Self::Success => "\x1b[32m",          // green
            Self::Memory => "\x1b[38;5;141m",     // purple
            Self::Compact => "\x1b[38;5;146m",    // grey-blue
            Self::Permission => "\x1b[38;5;172m", // brown-orange
            Self::Diff => "\x1b[38;5;117m",       // light blue
            Self::Progress => "\x1b[34m",         // blue
        }
    }

    fn needs_bold(self) -> bool {
        matches!(
            self,
            Self::Error | Self::Warning | Self::Success | Self::Progress
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_role_as_str_is_stable() {
        assert_eq!(SemanticRole::User.as_str(), "user");
        assert_eq!(SemanticRole::Error.as_str(), "error");
        assert_eq!(SemanticRole::Progress.as_str(), "progress");
    }

    #[test]
    fn render_intent_convenience_constructors() {
        let i = RenderIntent::error("something failed");
        assert_eq!(i.role, SemanticRole::Error);
        assert_eq!(i.text, "something failed");
    }

    #[test]
    fn render_policy_no_color_uses_prefix_labels() {
        let policy = RenderPolicy::no_color();
        let intent = RenderIntent::error("build failed");
        let rendered = policy.render_intent(&intent);
        assert!(rendered.contains("build failed"));
        assert!(rendered.starts_with("✗ "));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    fn render_policy_detects_no_color_env() {
        // Save state
        std::env::remove_var("NO_COLOR");
        let policy = RenderPolicy::detect(false);
        assert!(!policy.allow_colors);

        std::env::set_var("NO_COLOR", "1");
        let policy2 = RenderPolicy::detect(true);
        assert!(!policy2.allow_colors);
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    fn all_semantic_roles_have_prefix_and_color() {
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
            assert!(!role.as_str().is_empty(), "role {:?} missing as_str", role);
            assert!(
                !role.prefix_label().is_empty(),
                "role {:?} missing prefix",
                role
            );
        }
    }
}
