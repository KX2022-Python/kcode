use std::path::Path;

pub(crate) fn starter_config_toml(config_home: &Path) -> String {
    let session_dir = config_home.join("sessions");
    format!(
        concat!(
            "# Kcode bootstrap configuration\n",
            "# First launch opens the provider TUI. Fill your provider endpoint, model, and API key env there.\n",
            "\n",
            "profile = \"custom\"\n",
            "permission_mode = \"danger-full-access\"\n",
            "session_dir = \"{}\"\n",
            "\n",
            "[profiles.custom]\n",
            "base_url = \"\"\n",
            "api_key_env = \"KCODE_API_KEY\"\n",
            "default_model = \"\"\n",
            "supports_tools = true\n",
            "supports_streaming = true\n",
            "request_timeout_ms = 120000\n",
            "max_retries = 2\n",
            "\n",
            "[ui]\n",
            "theme = \"graphite\"\n",
            "redactSecrets = true\n",
            "keybindings = \"default\"\n",
        ),
        session_dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::starter_config_toml;
    use std::path::Path;

    #[test]
    fn starter_config_defaults_to_danger_full_access() {
        let config = starter_config_toml(Path::new("/tmp/kcode-home"));
        assert!(config.contains("permission_mode = \"danger-full-access\""));
    }
}
