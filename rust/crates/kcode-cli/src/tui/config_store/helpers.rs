use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use toml::Value;

use crate::tui::state::{redact, BridgeSettings, ConfigScope, TuiSettings};

pub(super) fn config_path_for_scope(cwd: &Path, scope: ConfigScope) -> PathBuf {
    match scope {
        ConfigScope::User => runtime::ConfigLoader::default_for(cwd)
            .config_home()
            .join("config.toml"),
        ConfigScope::Project => cwd.join(".kcode").join("config.toml"),
    }
}

pub(super) fn load_toml_table(
    path: &Path,
) -> Result<toml::map::Map<String, Value>, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(contents) if contents.trim().is_empty() => Ok(toml::map::Map::new()),
        Ok(contents) => match contents.parse::<Value>()? {
            Value::Table(table) => Ok(table),
            _ => Err(format!("{} must contain a TOML table", path.display()).into()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(toml::map::Map::new()),
        Err(error) => Err(Box::new(error)),
    }
}

pub(super) fn write_toml_table(
    path: &Path,
    table: &toml::map::Map<String, Value>,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = Value::Table(table.clone()).to_string();
    fs::write(path, format!("{rendered}\n"))?;
    Ok(())
}

pub(super) fn backup_file_if_exists(path: &Path) -> Result<Option<PathBuf>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(None);
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let backup = path.with_extension(format!("bak-{timestamp}"));
    fs::copy(path, &backup)?;
    Ok(Some(backup))
}

pub(super) fn ensure_table_mut<'a>(
    table: &'a mut toml::map::Map<String, Value>,
    path: &[&str],
) -> &'a mut toml::map::Map<String, Value> {
    let (head, tail) = path.split_first().expect("path must not be empty");
    let entry = table
        .entry((*head).to_string())
        .or_insert_with(|| Value::Table(toml::map::Map::new()));
    if !entry.is_table() {
        *entry = Value::Table(toml::map::Map::new());
    }
    let table = entry.as_table_mut().expect("table entry expected");
    if tail.is_empty() {
        table
    } else {
        ensure_table_mut(table, tail)
    }
}

fn target_table_mut<'a>(
    table: &'a mut toml::map::Map<String, Value>,
    parent_path: &[&str],
) -> &'a mut toml::map::Map<String, Value> {
    if parent_path.is_empty() {
        table
    } else {
        ensure_table_mut(table, parent_path)
    }
}

pub(super) fn remove_paths(table: &mut toml::map::Map<String, Value>, paths: &[&[&str]]) {
    for path in paths {
        remove_path(table, path);
    }
}

pub(super) fn remove_path(table: &mut toml::map::Map<String, Value>, path: &[&str]) -> bool {
    let Some((head, tail)) = path.split_first() else {
        return false;
    };
    if tail.is_empty() {
        return table.remove(*head).is_some();
    }

    let mut should_prune = false;
    let removed = if let Some(child) = table.get_mut(*head).and_then(Value::as_table_mut) {
        let removed = remove_path(child, tail);
        should_prune = child.is_empty();
        removed
    } else {
        false
    };
    if should_prune {
        table.remove(*head);
    }
    removed
}

pub(super) fn set_string(table: &mut toml::map::Map<String, Value>, path: &[&str], value: &str) {
    let (parent_path, key) = path.split_at(path.len() - 1);
    let target = target_table_mut(table, parent_path);
    target.insert(key[0].to_string(), Value::String(value.trim().to_string()));
}

pub(super) fn set_string_or_remove(
    table: &mut toml::map::Map<String, Value>,
    path: &[&str],
    value: &str,
) {
    if value.trim().is_empty() {
        remove_path(table, path);
    } else {
        set_string(table, path, value);
    }
}

pub(super) fn set_bool(table: &mut toml::map::Map<String, Value>, path: &[&str], value: bool) {
    let (parent_path, key) = path.split_at(path.len() - 1);
    let target = target_table_mut(table, parent_path);
    target.insert(key[0].to_string(), Value::Boolean(value));
}

pub(super) fn set_bool_or_remove(
    table: &mut toml::map::Map<String, Value>,
    path: &[&str],
    value: Option<bool>,
) {
    match value {
        Some(value) => set_bool(table, path, value),
        None => {
            remove_path(table, path);
        }
    }
}

pub(super) fn set_i64_or_remove(
    table: &mut toml::map::Map<String, Value>,
    path: &[&str],
    value: Option<i64>,
) {
    match value {
        Some(value) => {
            let (parent_path, key) = path.split_at(path.len() - 1);
            let target = target_table_mut(table, parent_path);
            target.insert(key[0].to_string(), Value::Integer(value));
        }
        None => {
            remove_path(table, path);
        }
    }
}

pub(super) fn set_array_or_remove(
    table: &mut toml::map::Map<String, Value>,
    path: &[&str],
    values: Vec<String>,
) {
    if values.is_empty() {
        remove_path(table, path);
        return;
    }
    let (parent_path, key) = path.split_at(path.len() - 1);
    let target = target_table_mut(table, parent_path);
    let array = values.into_iter().map(Value::String).collect::<Vec<_>>();
    target.insert(key[0].to_string(), Value::Array(array));
}

pub(super) fn set_bool_map_or_remove(
    table: &mut toml::map::Map<String, Value>,
    path: &[&str],
    values: BTreeMap<String, bool>,
) {
    if values.is_empty() {
        remove_path(table, path);
        return;
    }
    let (parent_path, key) = path.split_at(path.len() - 1);
    let target = target_table_mut(table, parent_path);
    let map = values
        .into_iter()
        .map(|(entry_key, entry_value)| (entry_key, Value::Boolean(entry_value)))
        .collect::<toml::map::Map<_, _>>();
    target.insert(key[0].to_string(), Value::Table(map));
}

pub(super) fn set_string_map_or_remove(
    table: &mut toml::map::Map<String, Value>,
    path: &[&str],
    values: BTreeMap<String, String>,
) {
    if values.is_empty() {
        remove_path(table, path);
        return;
    }
    let (parent_path, key) = path.split_at(path.len() - 1);
    let target = target_table_mut(table, parent_path);
    let map = values
        .into_iter()
        .map(|(entry_key, entry_value)| (entry_key, Value::String(entry_value)))
        .collect::<toml::map::Map<_, _>>();
    target.insert(key[0].to_string(), Value::Table(map));
}

pub(super) fn parse_optional_i64(value: &str) -> Option<i64> {
    value.trim().parse::<i64>().ok()
}

pub(super) fn csv_items(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn key_value_items(value: &str) -> BTreeMap<String, String> {
    value
        .split(',')
        .filter_map(|entry| entry.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .collect()
}

pub(super) fn join_csv<T: AsRef<str>>(values: &[T]) -> String {
    values
        .iter()
        .map(AsRef::as_ref)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn join_key_value_csv(values: &BTreeMap<String, String>) -> String {
    values
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn review_lines(settings: &TuiSettings) -> Vec<String> {
    let mut lines = vec![
        format!("Scope: {}", settings.scope.label()),
        format!("Config file: {}", settings.overview.config_path.display()),
        format!(
            "Bridge env: {}",
            settings.overview.bridge_env_path.display()
        ),
        format!("Profile: {}", settings.provider.active_profile),
        format!("Model: {}", settings.provider.default_model),
        format!("Base URL: {}", settings.provider.base_url),
        format!("API key env: {}", settings.provider.api_key_env),
        format!("Permission mode: {}", settings.runtime.permission_mode),
        format!("Session dir: {}", settings.runtime.session_dir),
        format!(
            "Bridge Telegram: {}",
            redact(
                &settings.bridge.telegram_bot_token,
                settings.appearance.redact_secrets
            )
        ),
        format!(
            "Bridge WhatsApp: {}",
            redact(
                &settings.bridge.whatsapp_phone_id,
                settings.appearance.redact_secrets
            )
        ),
        format!(
            "Bridge Feishu: {}",
            redact(
                &settings.bridge.feishu_app_id,
                settings.appearance.redact_secrets
            )
        ),
    ];
    if !settings.mcp.servers.is_empty() {
        lines.push(format!("MCP servers: {}", settings.mcp.servers.len()));
    }
    lines
}

pub(super) fn bridge_values(settings: &BridgeSettings) -> BTreeMap<String, String> {
    [
        (
            "KCODE_TELEGRAM_BOT_TOKEN",
            settings.telegram_bot_token.as_str(),
        ),
        ("KCODE_WEBHOOK_URL", settings.webhook_url.as_str()),
        (
            "KCODE_WEBHOOK_VERIFY_TOKEN",
            settings.webhook_verify_token.as_str(),
        ),
        (
            "KCODE_WHATSAPP_PHONE_ID",
            settings.whatsapp_phone_id.as_str(),
        ),
        ("KCODE_WHATSAPP_TOKEN", settings.whatsapp_token.as_str()),
        (
            "KCODE_WHATSAPP_APP_SECRET",
            settings.whatsapp_app_secret.as_str(),
        ),
        ("KCODE_FEISHU_APP_ID", settings.feishu_app_id.as_str()),
        (
            "KCODE_FEISHU_APP_SECRET",
            settings.feishu_app_secret.as_str(),
        ),
    ]
    .into_iter()
    .filter_map(|(key, value)| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| (key.to_string(), trimmed.to_string()))
    })
    .collect()
}
