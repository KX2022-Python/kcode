use std::path::Path;

pub(crate) fn starter_config_toml(config_home: &Path) -> String {
    let session_dir = config_home.join("sessions");
    format!(
        concat!(
            "# Kcode bootstrap configuration\n",
            "# First launch opens the provider TUI. Fill your provider endpoint, model, and API key env there.\n",
            "\n",
            "profile = \"custom\"\n",
            "permission_mode = \"workspace-write\"\n",
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
