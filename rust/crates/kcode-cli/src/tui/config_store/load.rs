use std::error::Error;
use std::path::Path;

use adapters::load_bridge_env_snapshot;
use runtime::{
    ConfigLoader, FilesystemIsolationMode, McpServerConfig, ResolvedProviderProfile,
    ScopedMcpServerConfig, SetupMode,
};
use toml::Value;

use super::helpers::{config_path_for_scope, join_csv, join_key_value_csv, load_toml_table};
use crate::tui::state::{
    AppearanceSettings, BridgeSettings, ConfigScope, HookSettings, McpServerDraft, McpSettings,
    McpTransportDraft, OverviewSettings, PluginSettings, ProviderSettings, RuntimeSettings,
    SandboxSettings, ThemePreset, TuiSettings,
};

pub(crate) fn load_settings(cwd: &Path, scope: ConfigScope) -> Result<TuiSettings, Box<dyn Error>> {
    let runtime_config = ConfigLoader::default_for(cwd).load()?;
    let setup = crate::load_setup_context(
        SetupMode::Config,
        None,
        None,
        crate::default_permission_mode(),
        None,
    )?;
    let config_path = config_path_for_scope(cwd, scope);
    let raw = load_toml_table(&config_path)?;
    let bridge_snapshot = load_bridge_env_snapshot()?;

    let loaded_files = setup
        .resolved_config
        .loaded_entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();

    Ok(TuiSettings {
        scope,
        overview: OverviewSettings {
            cwd: cwd.to_path_buf(),
            config_home: setup.resolved_config.config_home.clone(),
            session_dir: setup.resolved_config.session_dir.clone(),
            config_path,
            bridge_env_path: bridge_snapshot.path.clone(),
            loaded_files,
            runtime_ready: setup.resolved_config.is_runtime_ready(),
            doctor_report: crate::render_doctor_report_from_setup(&setup),
        },
        provider: load_provider_settings(&setup.active_profile),
        runtime: load_runtime_settings(&setup, &runtime_config),
        sandbox: load_sandbox_settings(&runtime_config),
        hooks: load_hook_settings(&runtime_config),
        plugins: load_plugin_settings(&runtime_config),
        mcp: load_mcp_settings(&runtime_config),
        bridge: load_bridge_settings(&bridge_snapshot),
        appearance: load_appearance_settings(&raw),
    })
}

fn load_provider_settings(active_profile: &ResolvedProviderProfile) -> ProviderSettings {
    ProviderSettings {
        active_profile: active_profile.profile_name.clone(),
        default_model: active_profile.model.clone(),
        base_url: active_profile.base_url.clone().unwrap_or_default(),
        api_key_env: active_profile.credential.env_name.clone(),
        request_timeout_ms: active_profile.profile.request_timeout_ms.to_string(),
        max_retries: active_profile.profile.max_retries.to_string(),
        supports_tools: active_profile.profile.supports_tools,
        supports_streaming: active_profile.profile.supports_streaming,
    }
}

fn load_runtime_settings(
    setup: &runtime::SetupContext,
    runtime_config: &runtime::RuntimeConfig,
) -> RuntimeSettings {
    RuntimeSettings {
        permission_mode: setup.trust_policy.permission_mode.clone(),
        session_dir: setup.resolved_config.session_dir.display().to_string(),
        permission_allow: join_csv(runtime_config.permission_rules().allow()),
        permission_deny: join_csv(runtime_config.permission_rules().deny()),
        permission_ask: join_csv(runtime_config.permission_rules().ask()),
    }
}

fn load_sandbox_settings(runtime_config: &runtime::RuntimeConfig) -> SandboxSettings {
    let sandbox = runtime_config.sandbox();
    SandboxSettings {
        enabled: sandbox.enabled.unwrap_or(true),
        namespace_restrictions: sandbox.namespace_restrictions.unwrap_or(true),
        network_isolation: sandbox.network_isolation.unwrap_or(false),
        filesystem_mode: sandbox
            .filesystem_mode
            .unwrap_or(FilesystemIsolationMode::WorkspaceOnly),
        allowed_mounts: join_csv(&sandbox.allowed_mounts),
    }
}

fn load_hook_settings(runtime_config: &runtime::RuntimeConfig) -> HookSettings {
    HookSettings {
        pre_tool_use: join_csv(runtime_config.hooks().pre_tool_use()),
        post_tool_use: join_csv(runtime_config.hooks().post_tool_use()),
        post_tool_use_failure: join_csv(runtime_config.hooks().post_tool_use_failure()),
    }
}

fn load_plugin_settings(runtime_config: &runtime::RuntimeConfig) -> PluginSettings {
    let (enabled, disabled): (Vec<_>, Vec<_>) = runtime_config
        .plugins()
        .enabled_plugins()
        .iter()
        .map(|(id, enabled)| (id.clone(), *enabled))
        .partition(|(_, enabled)| *enabled);

    PluginSettings {
        enabled_plugins: join_csv(&enabled.into_iter().map(|(id, _)| id).collect::<Vec<_>>()),
        disabled_plugins: join_csv(&disabled.into_iter().map(|(id, _)| id).collect::<Vec<_>>()),
        external_directories: join_csv(runtime_config.plugins().external_directories()),
        install_root: runtime_config
            .plugins()
            .install_root()
            .unwrap_or_default()
            .to_string(),
        registry_path: runtime_config
            .plugins()
            .registry_path()
            .unwrap_or_default()
            .to_string(),
        bundled_root: runtime_config
            .plugins()
            .bundled_root()
            .unwrap_or_default()
            .to_string(),
    }
}

fn load_mcp_settings(runtime_config: &runtime::RuntimeConfig) -> McpSettings {
    let mut servers = runtime_config
        .mcp()
        .servers()
        .iter()
        .map(|(name, server)| mcp_server_to_draft(name, server))
        .collect::<Vec<_>>();
    servers.sort_by(|left, right| left.name.cmp(&right.name));
    McpSettings {
        selected: 0,
        servers,
    }
}

fn mcp_server_to_draft(name: &str, server: &ScopedMcpServerConfig) -> McpServerDraft {
    match &server.config {
        McpServerConfig::Stdio(config) => McpServerDraft {
            name: name.to_string(),
            transport: McpTransportDraft::Stdio,
            command_or_url: config.command.clone(),
            args: join_csv(&config.args),
            env_or_headers: join_key_value_csv(&config.env),
            timeout_ms: config
                .tool_call_timeout_ms
                .map(|value| value.to_string())
                .unwrap_or_default(),
            helper_or_name: String::new(),
            oauth_client_id: String::new(),
            oauth_callback_port: String::new(),
            oauth_metadata_url: String::new(),
            oauth_xaa: false,
        },
        McpServerConfig::Sse(config) => remote_mcp_draft(name, McpTransportDraft::Sse, config),
        McpServerConfig::Http(config) => remote_mcp_draft(name, McpTransportDraft::Http, config),
        McpServerConfig::Ws(config) => McpServerDraft {
            name: name.to_string(),
            transport: McpTransportDraft::Ws,
            command_or_url: config.url.clone(),
            args: String::new(),
            env_or_headers: join_key_value_csv(&config.headers),
            timeout_ms: String::new(),
            helper_or_name: config.headers_helper.clone().unwrap_or_default(),
            oauth_client_id: String::new(),
            oauth_callback_port: String::new(),
            oauth_metadata_url: String::new(),
            oauth_xaa: false,
        },
        McpServerConfig::Sdk(config) => McpServerDraft {
            name: name.to_string(),
            transport: McpTransportDraft::Sdk,
            command_or_url: String::new(),
            args: String::new(),
            env_or_headers: String::new(),
            timeout_ms: String::new(),
            helper_or_name: config.name.clone(),
            oauth_client_id: String::new(),
            oauth_callback_port: String::new(),
            oauth_metadata_url: String::new(),
            oauth_xaa: false,
        },
        McpServerConfig::ManagedProxy(config) => McpServerDraft {
            name: name.to_string(),
            transport: McpTransportDraft::ManagedProxy,
            command_or_url: config.url.clone(),
            args: String::new(),
            env_or_headers: String::new(),
            timeout_ms: String::new(),
            helper_or_name: config.id.clone(),
            oauth_client_id: String::new(),
            oauth_callback_port: String::new(),
            oauth_metadata_url: String::new(),
            oauth_xaa: false,
        },
    }
}

fn remote_mcp_draft(
    name: &str,
    transport: McpTransportDraft,
    config: &runtime::McpRemoteServerConfig,
) -> McpServerDraft {
    let oauth = config.oauth.as_ref();
    McpServerDraft {
        name: name.to_string(),
        transport,
        command_or_url: config.url.clone(),
        args: String::new(),
        env_or_headers: join_key_value_csv(&config.headers),
        timeout_ms: String::new(),
        helper_or_name: config.headers_helper.clone().unwrap_or_default(),
        oauth_client_id: oauth
            .and_then(|oauth| oauth.client_id.clone())
            .unwrap_or_default(),
        oauth_callback_port: oauth
            .and_then(|oauth| oauth.callback_port.map(|value| value.to_string()))
            .unwrap_or_default(),
        oauth_metadata_url: oauth
            .and_then(|oauth| oauth.auth_server_metadata_url.clone())
            .unwrap_or_default(),
        oauth_xaa: oauth.and_then(|oauth| oauth.xaa).unwrap_or(false),
    }
}

fn load_bridge_settings(snapshot: &adapters::BridgeEnvSnapshot) -> BridgeSettings {
    let resolved = |key: &str| snapshot.resolve(key).unwrap_or_default();
    BridgeSettings {
        telegram_bot_token: resolved("KCODE_TELEGRAM_BOT_TOKEN"),
        webhook_url: resolved("KCODE_WEBHOOK_URL"),
        webhook_verify_token: resolved("KCODE_WEBHOOK_VERIFY_TOKEN"),
        whatsapp_phone_id: resolved("KCODE_WHATSAPP_PHONE_ID"),
        whatsapp_token: resolved("KCODE_WHATSAPP_TOKEN"),
        whatsapp_app_secret: resolved("KCODE_WHATSAPP_APP_SECRET"),
        feishu_app_id: resolved("KCODE_FEISHU_APP_ID"),
        feishu_app_secret: resolved("KCODE_FEISHU_APP_SECRET"),
    }
}

fn load_appearance_settings(raw: &toml::map::Map<String, Value>) -> AppearanceSettings {
    let ui = raw.get("ui").and_then(Value::as_table);
    AppearanceSettings {
        theme: ui
            .and_then(|table| table.get("theme"))
            .and_then(Value::as_str)
            .map(ThemePreset::parse)
            .unwrap_or(ThemePreset::Graphite),
        redact_secrets: ui
            .and_then(|table| table.get("redactSecrets"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        keybindings_hint: ui
            .and_then(|table| table.get("keybindings"))
            .and_then(Value::as_str)
            .unwrap_or("default")
            .to_string(),
    }
}
