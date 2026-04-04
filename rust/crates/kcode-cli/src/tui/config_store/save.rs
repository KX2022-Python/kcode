use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use adapters::write_bridge_env_file;
use runtime::{builtin_profiles, ProviderProfile};
use toml::Value;

use super::helpers::{
    backup_file_if_exists, bridge_values, csv_items, key_value_items, load_toml_table,
    parse_optional_i64, remove_path, remove_paths, set_array_or_remove, set_bool,
    set_bool_map_or_remove, set_bool_or_remove, set_i64_or_remove, set_string,
    set_string_map_or_remove, set_string_or_remove, write_toml_table,
};
use crate::tui::config_store::load::load_settings;
use crate::tui::state::{
    AppearanceSettings, HookSettings, McpServerDraft, McpSettings, McpTransportDraft,
    PluginSettings, ProviderSettings, RuntimeSettings, SandboxSettings, TuiSettings,
};

pub(crate) struct SaveOutcome {
    pub(crate) config_path: PathBuf,
    pub(crate) bridge_env_path: PathBuf,
    pub(crate) config_backup: Option<PathBuf>,
    pub(crate) bridge_env_backup: Option<PathBuf>,
}

pub(crate) fn save_settings(settings: &mut TuiSettings) -> Result<SaveOutcome, Box<dyn Error>> {
    let config_path = settings.overview.config_path.clone();
    let bridge_env_path = settings.overview.bridge_env_path.clone();

    let mut table = load_toml_table(&config_path)?;
    let config_backup = backup_file_if_exists(&config_path)?;
    let bridge_env_backup = backup_file_if_exists(&bridge_env_path)?;

    persist_provider_settings(&mut table, &settings.provider);
    persist_runtime_settings(&mut table, &settings.runtime);
    persist_sandbox_settings(&mut table, &settings.sandbox);
    persist_hook_settings(&mut table, &settings.hooks);
    persist_plugin_settings(&mut table, &settings.plugins);
    persist_mcp_settings(&mut table, &settings.mcp);
    persist_appearance_settings(&mut table, &settings.appearance);

    write_toml_table(&config_path, &table)?;
    write_bridge_env_file(&bridge_env_path, &bridge_values(&settings.bridge))?;

    let reloaded = load_settings(&settings.overview.cwd, settings.scope)?;
    *settings = reloaded;

    Ok(SaveOutcome {
        config_path,
        bridge_env_path,
        config_backup,
        bridge_env_backup,
    })
}

fn persist_provider_settings(
    table: &mut toml::map::Map<String, Value>,
    settings: &ProviderSettings,
) {
    set_string(table, &["profile"], &settings.active_profile);
    remove_paths(
        table,
        &[
            &["model"],
            &["base_url"],
            &["baseUrl"],
            &["api_key_env"],
            &["apiKeyEnv"],
        ],
    );

    let builtin = builtin_profiles()
        .into_iter()
        .find(|profile| profile.name == settings.active_profile);
    let profiles = super::helpers::ensure_table_mut(table, &["profiles"]);
    let profile_table = super::helpers::ensure_table_mut(profiles, &[&settings.active_profile]);
    persist_profile_block(profile_table, builtin.as_ref(), settings);
}

fn persist_profile_block(
    profile_table: &mut toml::map::Map<String, Value>,
    builtin: Option<&ProviderProfile>,
    settings: &ProviderSettings,
) {
    set_string_or_remove(profile_table, &["base_url"], &settings.base_url);
    set_string_or_remove(profile_table, &["api_key_env"], &settings.api_key_env);
    set_string_or_remove(profile_table, &["default_model"], &settings.default_model);

    let timeout = settings
        .request_timeout_ms
        .trim()
        .parse::<i64>()
        .unwrap_or(120_000);
    let retries = settings.max_retries.trim().parse::<i64>().unwrap_or(2);
    let builtin_timeout = builtin.map_or(120_000, |profile| profile.request_timeout_ms as i64);
    let builtin_retries = builtin.map_or(2, |profile| profile.max_retries as i64);
    let builtin_tools = builtin
        .map(|profile| profile.supports_tools)
        .unwrap_or(true);
    let builtin_stream = builtin
        .map(|profile| profile.supports_streaming)
        .unwrap_or(true);

    set_i64_or_remove(
        profile_table,
        &["request_timeout_ms"],
        (timeout != builtin_timeout).then_some(timeout),
    );
    set_i64_or_remove(
        profile_table,
        &["max_retries"],
        (retries != builtin_retries).then_some(retries),
    );
    set_bool_or_remove(
        profile_table,
        &["supports_tools"],
        (settings.supports_tools != builtin_tools).then_some(settings.supports_tools),
    );
    set_bool_or_remove(
        profile_table,
        &["supports_streaming"],
        (settings.supports_streaming != builtin_stream).then_some(settings.supports_streaming),
    );
}

fn persist_runtime_settings(table: &mut toml::map::Map<String, Value>, settings: &RuntimeSettings) {
    set_string(table, &["session_dir"], &settings.session_dir);
    remove_paths(table, &[&["permission_mode"], &["permissionMode"]]);

    let permissions = super::helpers::ensure_table_mut(table, &["permissions"]);
    set_string(permissions, &["defaultMode"], &settings.permission_mode);
    set_array_or_remove(
        permissions,
        &["allow"],
        csv_items(&settings.permission_allow),
    );
    set_array_or_remove(permissions, &["deny"], csv_items(&settings.permission_deny));
    set_array_or_remove(permissions, &["ask"], csv_items(&settings.permission_ask));
}

fn persist_sandbox_settings(table: &mut toml::map::Map<String, Value>, settings: &SandboxSettings) {
    let sandbox = super::helpers::ensure_table_mut(table, &["sandbox"]);
    set_bool(sandbox, &["enabled"], settings.enabled);
    set_bool(
        sandbox,
        &["namespaceRestrictions"],
        settings.namespace_restrictions,
    );
    set_bool(sandbox, &["networkIsolation"], settings.network_isolation);
    set_string(
        sandbox,
        &["filesystemMode"],
        settings.filesystem_mode.as_str(),
    );
    set_array_or_remove(
        sandbox,
        &["allowedMounts"],
        csv_items(&settings.allowed_mounts),
    );
}

fn persist_hook_settings(table: &mut toml::map::Map<String, Value>, settings: &HookSettings) {
    let hooks = super::helpers::ensure_table_mut(table, &["hooks"]);
    set_array_or_remove(hooks, &["PreToolUse"], csv_items(&settings.pre_tool_use));
    set_array_or_remove(hooks, &["PostToolUse"], csv_items(&settings.post_tool_use));
    set_array_or_remove(
        hooks,
        &["PostToolUseFailure"],
        csv_items(&settings.post_tool_use_failure),
    );
}

fn persist_plugin_settings(table: &mut toml::map::Map<String, Value>, settings: &PluginSettings) {
    let plugins = super::helpers::ensure_table_mut(table, &["plugins"]);
    let enabled = csv_items(&settings.enabled_plugins);
    let disabled = csv_items(&settings.disabled_plugins);
    let mut states = BTreeMap::new();
    for plugin in enabled {
        states.insert(plugin, true);
    }
    for plugin in disabled {
        states.insert(plugin, false);
    }
    set_bool_map_or_remove(plugins, &["enabled"], states);
    set_array_or_remove(
        plugins,
        &["externalDirectories"],
        csv_items(&settings.external_directories),
    );
    set_string_or_remove(plugins, &["installRoot"], &settings.install_root);
    set_string_or_remove(plugins, &["registryPath"], &settings.registry_path);
    set_string_or_remove(plugins, &["bundledRoot"], &settings.bundled_root);
}

fn persist_mcp_settings(table: &mut toml::map::Map<String, Value>, settings: &McpSettings) {
    let mut servers = toml::map::Map::new();
    for server in settings
        .servers
        .iter()
        .filter(|server| !server.name.trim().is_empty())
    {
        servers.insert(server.name.clone(), draft_to_toml(server));
    }
    if servers.is_empty() {
        remove_path(table, &["mcpServers"]);
    } else {
        table.insert("mcpServers".to_string(), Value::Table(servers));
    }
}

fn draft_to_toml(server: &McpServerDraft) -> Value {
    let mut table = toml::map::Map::new();
    if server.transport != McpTransportDraft::Stdio {
        table.insert(
            "type".to_string(),
            Value::String(server.transport.label().to_string()),
        );
    }

    match server.transport {
        McpTransportDraft::Stdio => {
            set_string(&mut table, &["command"], &server.command_or_url);
            set_array_or_remove(&mut table, &["args"], csv_items(&server.args));
            set_string_map_or_remove(
                &mut table,
                &["env"],
                key_value_items(&server.env_or_headers),
            );
            set_i64_or_remove(
                &mut table,
                &["toolCallTimeoutMs"],
                parse_optional_i64(&server.timeout_ms),
            );
        }
        McpTransportDraft::Sse | McpTransportDraft::Http => {
            set_string(&mut table, &["url"], &server.command_or_url);
            set_string_map_or_remove(
                &mut table,
                &["headers"],
                key_value_items(&server.env_or_headers),
            );
            set_string_or_remove(&mut table, &["headersHelper"], &server.helper_or_name);
            let oauth = oauth_table(server);
            if !oauth.is_empty() {
                table.insert("oauth".to_string(), Value::Table(oauth));
            }
        }
        McpTransportDraft::Ws => {
            set_string(&mut table, &["url"], &server.command_or_url);
            set_string_map_or_remove(
                &mut table,
                &["headers"],
                key_value_items(&server.env_or_headers),
            );
            set_string_or_remove(&mut table, &["headersHelper"], &server.helper_or_name);
        }
        McpTransportDraft::Sdk => set_string(&mut table, &["name"], &server.helper_or_name),
        McpTransportDraft::ManagedProxy => {
            set_string(&mut table, &["url"], &server.command_or_url);
            set_string(&mut table, &["id"], &server.helper_or_name);
        }
    }

    Value::Table(table)
}

fn oauth_table(server: &McpServerDraft) -> toml::map::Map<String, Value> {
    let mut oauth = toml::map::Map::new();
    set_string_or_remove(&mut oauth, &["clientId"], &server.oauth_client_id);
    set_i64_or_remove(
        &mut oauth,
        &["callbackPort"],
        parse_optional_i64(&server.oauth_callback_port),
    );
    set_string_or_remove(
        &mut oauth,
        &["authServerMetadataUrl"],
        &server.oauth_metadata_url,
    );
    set_bool_or_remove(
        &mut oauth,
        &["xaa"],
        server.oauth_xaa.then_some(server.oauth_xaa),
    );
    oauth
}

fn persist_appearance_settings(
    table: &mut toml::map::Map<String, Value>,
    settings: &AppearanceSettings,
) {
    let ui = super::helpers::ensure_table_mut(table, &["ui"]);
    set_string(ui, &["theme"], settings.theme.label());
    set_bool(ui, &["redactSecrets"], settings.redact_secrets);
    set_string(ui, &["keybindings"], &settings.keybindings_hint);
}
