use std::path::PathBuf;

use runtime::FilesystemIsolationMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    User,
    Project,
}

impl Default for ConfigScope {
    fn default() -> Self {
        Self::User
    }
}

impl ConfigScope {
    pub const ALL: [Self; 2] = [Self::User, Self::Project];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }

    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::User => Self::Project,
            Self::Project => Self::User,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Overview,
    Provider,
    Runtime,
    Sandbox,
    Extensions,
    Mcp,
    Bridge,
    Appearance,
    Review,
}

impl Section {
    pub const ALL: [Self; 9] = [
        Self::Overview,
        Self::Provider,
        Self::Runtime,
        Self::Sandbox,
        Self::Extensions,
        Self::Mcp,
        Self::Bridge,
        Self::Appearance,
        Self::Review,
    ];

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Provider => "Provider",
            Self::Runtime => "Runtime",
            Self::Sandbox => "Sandbox",
            Self::Extensions => "Extensions",
            Self::Mcp => "MCP",
            Self::Bridge => "Bridge",
            Self::Appearance => "Appearance",
            Self::Review => "Review",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    Default,
    Amber,
    Ocean,
    DarkHighContrast,
    CatppuccinMocha,
    Light,
}

impl ThemePreset {
    pub const ALL: [Self; 6] = [
        Self::Default,
        Self::Amber,
        Self::Ocean,
        Self::CatppuccinMocha,
        Self::DarkHighContrast,
        Self::Light,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Amber => "amber",
            Self::Ocean => "ocean",
            Self::DarkHighContrast => "dark-hc",
            Self::CatppuccinMocha => "catppuccin",
            Self::Light => "light",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "amber" => Self::Amber,
            "ocean" => Self::Ocean,
            "dark-hc" => Self::DarkHighContrast,
            "catppuccin" => Self::CatppuccinMocha,
            "light" => Self::Light,
            _ => Self::Default,
        }
    }

    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Amber,
            Self::Amber => Self::Ocean,
            Self::Ocean => Self::CatppuccinMocha,
            Self::CatppuccinMocha => Self::DarkHighContrast,
            Self::DarkHighContrast => Self::Light,
            Self::Light => Self::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransportDraft {
    Stdio,
    Sse,
    Http,
    Ws,
    Sdk,
    ManagedProxy,
}

impl McpTransportDraft {
    pub const ALL: [Self; 6] = [
        Self::Stdio,
        Self::Sse,
        Self::Http,
        Self::Ws,
        Self::Sdk,
        Self::ManagedProxy,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Sse => "sse",
            Self::Http => "http",
            Self::Ws => "ws",
            Self::Sdk => "sdk",
            Self::ManagedProxy => "claudeai-proxy",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "sse" => Self::Sse,
            "http" => Self::Http,
            "ws" => Self::Ws,
            "sdk" => Self::Sdk,
            "claudeai-proxy" => Self::ManagedProxy,
            _ => Self::Stdio,
        }
    }

    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Stdio => Self::Sse,
            Self::Sse => Self::Http,
            Self::Http => Self::Ws,
            Self::Ws => Self::Sdk,
            Self::Sdk => Self::ManagedProxy,
            Self::ManagedProxy => Self::Stdio,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderSettings {
    pub active_profile: String,
    pub default_model: String,
    pub base_url: String,
    pub api_key_env: String,
    pub request_timeout_ms: String,
    pub max_retries: String,
    pub supports_tools: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeSettings {
    pub permission_mode: String,
    pub session_dir: String,
    pub permission_allow: String,
    pub permission_deny: String,
    pub permission_ask: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSettings {
    pub enabled: bool,
    pub namespace_restrictions: bool,
    pub network_isolation: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub allowed_mounts: String,
}

impl Default for SandboxSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            namespace_restrictions: true,
            network_isolation: false,
            filesystem_mode: FilesystemIsolationMode::WorkspaceOnly,
            allowed_mounts: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookSettings {
    pub pre_tool_use: String,
    pub post_tool_use: String,
    pub post_tool_use_failure: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginSettings {
    pub enabled_plugins: String,
    pub disabled_plugins: String,
    pub external_directories: String,
    pub install_root: String,
    pub registry_path: String,
    pub bundled_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerDraft {
    pub name: String,
    pub transport: McpTransportDraft,
    pub command_or_url: String,
    pub args: String,
    pub env_or_headers: String,
    pub timeout_ms: String,
    pub helper_or_name: String,
    pub oauth_client_id: String,
    pub oauth_callback_port: String,
    pub oauth_metadata_url: String,
    pub oauth_xaa: bool,
}

impl Default for McpServerDraft {
    fn default() -> Self {
        Self {
            name: "new-server".to_string(),
            transport: McpTransportDraft::Stdio,
            command_or_url: String::new(),
            args: String::new(),
            env_or_headers: String::new(),
            timeout_ms: String::new(),
            helper_or_name: String::new(),
            oauth_client_id: String::new(),
            oauth_callback_port: String::new(),
            oauth_metadata_url: String::new(),
            oauth_xaa: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpSettings {
    pub selected: usize,
    pub servers: Vec<McpServerDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BridgeSettings {
    pub telegram_bot_token: String,
    pub webhook_url: String,
    pub webhook_verify_token: String,
    pub whatsapp_phone_id: String,
    pub whatsapp_token: String,
    pub whatsapp_app_secret: String,
    pub feishu_app_id: String,
    pub feishu_app_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceSettings {
    pub theme: ThemePreset,
    pub redact_secrets: bool,
    pub keybindings_hint: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreset::Default,
            redact_secrets: true,
            keybindings_hint: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverviewSettings {
    pub cwd: PathBuf,
    pub config_home: PathBuf,
    pub session_dir: PathBuf,
    pub config_path: PathBuf,
    pub bridge_env_path: PathBuf,
    pub loaded_files: Vec<PathBuf>,
    pub runtime_ready: bool,
    pub doctor_report: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TuiSettings {
    pub scope: ConfigScope,
    pub overview: OverviewSettings,
    pub provider: ProviderSettings,
    pub runtime: RuntimeSettings,
    pub sandbox: SandboxSettings,
    pub hooks: HookSettings,
    pub plugins: PluginSettings,
    pub mcp: McpSettings,
    pub bridge: BridgeSettings,
    pub appearance: AppearanceSettings,
}

#[must_use]
pub fn redact(secret: &str, redact_secrets: bool) -> String {
    if !redact_secrets {
        return secret.to_string();
    }
    if secret.trim().is_empty() {
        return "<unset>".to_string();
    }
    let count = secret.chars().count();
    if count <= 4 {
        return "****".to_string();
    }
    let suffix = secret
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("***{suffix}")
}
