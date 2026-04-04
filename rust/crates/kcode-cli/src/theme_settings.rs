use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::render_theme::ThemeName;

#[must_use]
pub fn current_theme_name() -> ThemeName {
    theme_name_from_sources().unwrap_or(ThemeName::Graphite)
}

fn theme_name_from_sources() -> Option<ThemeName> {
    let cwd = env::current_dir().ok();
    for path in config_candidates(cwd.as_deref()) {
        if let Some(theme) = read_theme_from_path(&path) {
            return Some(theme);
        }
    }
    None
}

fn config_candidates(cwd: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(cwd) = cwd {
        candidates.push(cwd.join(".kcode").join("config.toml"));
    }
    candidates.push(config_home().join("config.toml"));
    candidates
}

fn config_home() -> PathBuf {
    if let Some(explicit) = env::var_os("KCODE_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(explicit);
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".kcode")
}

fn read_theme_from_path(path: &Path) -> Option<ThemeName> {
    let raw = fs::read_to_string(path).ok()?;
    let value = raw.parse::<toml::Value>().ok()?;
    let theme = value
        .get("ui")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("theme"))
        .and_then(toml::Value::as_str)?;
    Some(ThemeName::parse(theme))
}
