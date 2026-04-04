use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandManifestEntry {
    pub name: String,
    pub source: CommandSource,
    pub scope: CommandScope,
    pub kind: CommandKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandSource {
    Builtin,
    Skills,
    Plugins,
    Workflow,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Prompt,
    Local,
    LocalUi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    Process,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandAvailability {
    pub cli_visible: bool,
    pub bridge_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: String,
    pub name: String,
    pub kind: CommandKind,
    pub source: CommandSource,
    pub scope: CommandScope,
    pub availability: CommandAvailability,
    pub enabled: bool,
    pub remote_safe: bool,
    pub channel_safe: bool,
    pub aliases: Vec<String>,
    pub description: String,
    pub argument_hint: Option<String>,
    pub resume_supported: bool,
    pub visibility_tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSurface {
    CliLocal,
    Bridge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilteredCommand {
    pub id: String,
    pub scope: CommandScope,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandRegistrySnapshot {
    pub process_commands: Vec<CommandDescriptor>,
    pub session_commands: Vec<CommandDescriptor>,
    pub filtered_out_commands: Vec<FilteredCommand>,
    pub source_breakdown: BTreeMap<CommandSource, usize>,
    pub safety_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRegistryContext {
    pub surface: CommandSurface,
    pub include_local_ui: bool,
    pub profile_supports_tools: bool,
    pub denied_commands: Vec<String>,
}

impl CommandRegistryContext {
    #[must_use]
    pub fn for_surface(surface: CommandSurface, profile_supports_tools: bool) -> Self {
        Self {
            surface,
            include_local_ui: matches!(surface, CommandSurface::CliLocal),
            profile_supports_tools,
            denied_commands: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_denied_commands(mut self, patterns: Vec<String>) -> Self {
        self.denied_commands = patterns;
        self
    }

    #[must_use]
    pub fn cli_local() -> Self {
        Self::for_surface(CommandSurface::CliLocal, true)
    }

    #[must_use]
    pub fn bridge_safe() -> Self {
        Self::for_surface(CommandSurface::Bridge, true)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandRegistry {
    descriptors: Vec<CommandDescriptor>,
}

impl CommandRegistry {
    #[must_use]
    pub fn new(descriptors: Vec<CommandDescriptor>) -> Self {
        Self { descriptors }
    }

    #[must_use]
    pub fn descriptors(&self) -> &[CommandDescriptor] {
        &self.descriptors
    }
}
