use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

use super::theme::ThemePreset;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BootstrapState {
    pub theme: ThemePreset,
    pub theme_onboarded: bool,
    pub workspace_root: PathBuf,
    pub trusted_workspace: bool,
}

pub(crate) fn load_bootstrap_state(cwd: &Path) -> Result<BootstrapState, Box<dyn Error>> {
    load_bootstrap_state_with_config_home(cwd, None)
}

pub(crate) fn persist_bootstrap_theme(
    cwd: &Path,
    theme: ThemePreset,
) -> Result<BootstrapState, Box<dyn Error>> {
    persist_bootstrap_theme_with_config_home(cwd, theme, None)
}

pub(crate) fn persist_workspace_trust(cwd: &Path) -> Result<BootstrapState, Box<dyn Error>> {
    persist_workspace_trust_with_config_home(cwd, None)
}

fn load_bootstrap_state_with_config_home(
    cwd: &Path,
    config_home: Option<&Path>,
) -> Result<BootstrapState, Box<dyn Error>> {
    let config_path = config_path_for(cwd, config_home);
    let workspace_root = workspace_root_for(cwd);
    let raw = load_toml_table(&config_path)?;
    let ui = raw.get("ui").and_then(Value::as_table);
    let bootstrap = raw.get("bootstrap").and_then(Value::as_table);

    let theme_value = ui
        .and_then(|table| table.get("theme"))
        .and_then(Value::as_str);
    let theme = theme_value
        .map(ThemePreset::parse)
        .unwrap_or(ThemePreset::Default);
    let theme_onboarded = bootstrap
        .and_then(|table| table.get("theme_onboarded"))
        .and_then(Value::as_bool)
        .unwrap_or(theme_value.is_some());
    let trusted_workspace = bootstrap
        .and_then(|table| table.get("trusted_workspaces"))
        .and_then(Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .as_str()
                    .is_some_and(|value| Path::new(value) == workspace_root)
            })
        });

    Ok(BootstrapState {
        theme,
        theme_onboarded,
        workspace_root: workspace_root.to_path_buf(),
        trusted_workspace,
    })
}

fn persist_bootstrap_theme_with_config_home(
    cwd: &Path,
    theme: ThemePreset,
    config_home: Option<&Path>,
) -> Result<BootstrapState, Box<dyn Error>> {
    let config_path = config_path_for(cwd, config_home);
    let mut raw = load_toml_table(&config_path)?;
    let ui = ensure_table_mut(&mut raw, &["ui"]);
    ui.insert(
        "theme".to_string(),
        Value::String(theme.label().to_string()),
    );

    let bootstrap = ensure_table_mut(&mut raw, &["bootstrap"]);
    bootstrap.insert("theme_onboarded".to_string(), Value::Boolean(true));

    write_toml_table(&config_path, &raw)?;
    load_bootstrap_state_with_config_home(cwd, config_home)
}

fn persist_workspace_trust_with_config_home(
    cwd: &Path,
    config_home: Option<&Path>,
) -> Result<BootstrapState, Box<dyn Error>> {
    let config_path = config_path_for(cwd, config_home);
    let workspace_root = workspace_root_for(cwd);
    let mut raw = load_toml_table(&config_path)?;
    let bootstrap = ensure_table_mut(&mut raw, &["bootstrap"]);
    let mut trusted = bootstrap
        .get("trusted_workspaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let workspace_value = workspace_root.display().to_string();
    if !trusted
        .iter()
        .any(|entry| entry.as_str().is_some_and(|value| value == workspace_value))
    {
        trusted.push(Value::String(workspace_value));
    }
    bootstrap.insert("trusted_workspaces".to_string(), Value::Array(trusted));

    write_toml_table(&config_path, &raw)?;
    load_bootstrap_state_with_config_home(cwd, config_home)
}

fn config_path_for(cwd: &Path, config_home: Option<&Path>) -> PathBuf {
    config_home
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            runtime::ConfigLoader::default_for(cwd)
                .config_home()
                .to_path_buf()
        })
        .join(CONFIG_FILE_NAME)
}

fn workspace_root_for(cwd: &Path) -> PathBuf {
    crate::find_git_root_in(cwd).unwrap_or_else(|_| cwd.to_path_buf())
}

fn load_toml_table(path: &Path) -> Result<toml::map::Map<String, Value>, Box<dyn Error>> {
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

fn write_toml_table(
    path: &Path,
    table: &toml::map::Map<String, Value>,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", toml::to_string(table)?))?;
    Ok(())
}

fn ensure_table_mut<'a>(
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
    let child = entry.as_table_mut().expect("table entry expected");
    if tail.is_empty() {
        child
    } else {
        ensure_table_mut(child, tail)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        load_bootstrap_state_with_config_home, persist_bootstrap_theme_with_config_home,
        persist_workspace_trust_with_config_home,
    };
    use crate::tui::repl::theme::ThemePreset;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("kcode-bootstrap-store-{label}-{nanos}"))
    }

    #[test]
    fn theme_write_marks_bootstrap_onboarded() {
        let root = temp_dir("theme");
        fs::create_dir_all(&root).expect("root should exist");
        let config_home = root.join(".kcode");

        let state = persist_bootstrap_theme_with_config_home(
            &root,
            ThemePreset::CatppuccinMocha,
            Some(&config_home),
        )
        .expect("theme should save");

        assert_eq!(state.theme, ThemePreset::CatppuccinMocha);
        assert!(state.theme_onboarded);

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn trust_write_marks_current_workspace_as_trusted() {
        let root = temp_dir("trust");
        fs::create_dir_all(root.join(".git")).expect("git dir should exist");
        let config_home = root.join(".kcode");

        let state = persist_workspace_trust_with_config_home(&root, Some(&config_home))
            .expect("trust should save");

        assert!(state.trusted_workspace);
        assert_eq!(state.workspace_root, root);
        assert!(
            load_bootstrap_state_with_config_home(&root, Some(&config_home))
                .expect("state should reload")
                .trusted_workspace
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }
}
