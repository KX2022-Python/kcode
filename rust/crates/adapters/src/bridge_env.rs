use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const BRIDGE_ENV_PATH_ENV: &str = "KCODE_BRIDGE_ENV_FILE";
const KCODE_CONFIG_HOME_ENV: &str = "KCODE_CONFIG_HOME";
const CLAW_CONFIG_HOME_ENV: &str = "CLAW_CONFIG_HOME";

const KNOWN_BRIDGE_KEYS: &[&str] = &[
    "KCODE_API_KEY",
    "KCODE_TELEGRAM_BOT_TOKEN",
    "KCODE_WEBHOOK_URL",
    "KCODE_WEBHOOK_VERIFY_TOKEN",
    "KCODE_WHATSAPP_PHONE_ID",
    "KCODE_WHATSAPP_TOKEN",
    "KCODE_WHATSAPP_APP_SECRET",
    "KCODE_FEISHU_APP_ID",
    "KCODE_FEISHU_APP_SECRET",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeEnvSnapshot {
    pub path: PathBuf,
    pub values: BTreeMap<String, String>,
}

impl BridgeEnvSnapshot {
    #[must_use]
    pub fn resolve(&self, key: &str) -> Option<String> {
        env::var(key)
            .ok()
            .and_then(non_empty_owned)
            .or_else(|| self.values.get(key).cloned().and_then(non_empty_owned))
    }
}

#[must_use]
pub fn bridge_env_path() -> PathBuf {
    env::var_os(BRIDGE_ENV_PATH_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os(KCODE_CONFIG_HOME_ENV)
                .map(PathBuf::from)
                .map(|path| path.join("bridge.env"))
        })
        .or_else(|| {
            env::var_os(CLAW_CONFIG_HOME_ENV)
                .map(PathBuf::from)
                .map(|path| path.join("bridge.env"))
        })
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".kcode").join("bridge.env"))
        })
        .unwrap_or_else(|| PathBuf::from(".kcode").join("bridge.env"))
}

pub fn load_bridge_env_snapshot() -> io::Result<BridgeEnvSnapshot> {
    let path = bridge_env_path();
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    Ok(BridgeEnvSnapshot {
        path,
        values: parse_bridge_env(&contents),
    })
}

pub fn apply_bridge_env_defaults_to_process() -> io::Result<BridgeEnvSnapshot> {
    let snapshot = load_bridge_env_snapshot()?;
    for (key, value) in &snapshot.values {
        if env::var_os(key).is_none() {
            env::set_var(key, value);
        }
    }
    Ok(snapshot)
}

pub fn write_bridge_env_file(path: &Path, values: &BTreeMap<String, String>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut lines = vec![
        "# Kcode bridge credentials".to_string(),
        "# This file is loaded as a local fallback when the process environment is unset."
            .to_string(),
        "# Keep permissions private (0600 recommended).".to_string(),
        String::new(),
    ];

    for key in KNOWN_BRIDGE_KEYS {
        match values
            .get(*key)
            .and_then(|value| non_empty_owned(value.clone()))
        {
            Some(value) => lines.push(format!("{key}={}", shell_escape(&value))),
            None => lines.push(format!("# {key}=")),
        }
    }

    let extra_keys = values
        .keys()
        .filter(|key| !KNOWN_BRIDGE_KEYS.contains(&key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !extra_keys.is_empty() {
        lines.push(String::new());
        lines.push("# Extra keys".to_string());
        for key in extra_keys {
            if let Some(value) = values.get(&key) {
                lines.push(format!("{key}={}", shell_escape(value)));
            }
        }
    }

    fs::write(path, format!("{}\n", lines.join("\n")))?;
    set_private_permissions(path)?;
    Ok(())
}

#[must_use]
pub fn known_bridge_keys() -> &'static [&'static str] {
    KNOWN_BRIDGE_KEYS
}

#[must_use]
pub fn parse_bridge_env(source: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = unquote(value);
        if let Some(value) = non_empty_owned(value.to_string()) {
            values.insert(key.to_string(), value);
        }
    }
    values
}

fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        "\"\"".to_string()
    } else if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn unquote(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn non_empty_owned(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn set_private_permissions(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_bridge_env, write_bridge_env_file};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("adapters-bridge-env-{name}-{nanos}.env"))
    }

    #[test]
    fn parses_shell_style_env_lines() {
        let parsed = parse_bridge_env(
            r#"
export KCODE_API_KEY="abc 123"
KCODE_TELEGRAM_BOT_TOKEN=123:abc
# comment
INVALID
"#,
        );

        assert_eq!(
            parsed.get("KCODE_API_KEY").map(String::as_str),
            Some("abc 123")
        );
        assert_eq!(
            parsed.get("KCODE_TELEGRAM_BOT_TOKEN").map(String::as_str),
            Some("123:abc")
        );
    }

    #[test]
    fn writes_known_keys_and_comments_for_missing_values() {
        let path = temp_path("write");
        let mut values = BTreeMap::new();
        values.insert("KCODE_API_KEY".to_string(), "secret".to_string());
        values.insert("KCODE_FEISHU_APP_ID".to_string(), "app-id".to_string());

        write_bridge_env_file(&path, &values).expect("bridge env should write");
        let rendered = fs::read_to_string(&path).expect("bridge env should exist");

        assert!(rendered.contains("KCODE_API_KEY=secret"));
        assert!(rendered.contains("KCODE_FEISHU_APP_ID=app-id"));
        assert!(rendered.contains("# KCODE_TELEGRAM_BOT_TOKEN="));

        let _ = fs::remove_file(path);
    }
}
