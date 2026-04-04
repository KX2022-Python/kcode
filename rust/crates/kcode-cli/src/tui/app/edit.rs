use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use runtime::FilesystemIsolationMode;

use super::core::{FieldId, TuiApp};

const PERMISSION_MODES: &[&str] = &["read-only", "workspace-write", "danger-full-access"];
const KEYBINDING_PRESETS: &[&str] = &["default", "vim", "compact"];

impl TuiApp {
    pub(super) fn activate_current_field(&mut self) {
        let Some(field) = self.rows().get(self.field_index).map(|row| row.id) else {
            return;
        };
        if let Some((title, value)) = self.editor_seed(field) {
            self.editor = Some(super::core::EditorState {
                target: field,
                title: title.to_string(),
                cursor: value.chars().count(),
                value,
            });
            self.status = "编辑模式：Enter 应用，Esc 取消。".to_string();
            return;
        }

        match field {
            FieldId::ProviderSupportsTools => {
                self.settings.provider.supports_tools = !self.settings.provider.supports_tools;
            }
            FieldId::ProviderSupportsStreaming => {
                self.settings.provider.supports_streaming =
                    !self.settings.provider.supports_streaming;
            }
            FieldId::RuntimePermissionMode => {
                self.settings.runtime.permission_mode =
                    next_choice(&self.settings.runtime.permission_mode, PERMISSION_MODES);
            }
            FieldId::SandboxEnabled => {
                self.settings.sandbox.enabled = !self.settings.sandbox.enabled
            }
            FieldId::SandboxNamespaceRestrictions => {
                self.settings.sandbox.namespace_restrictions =
                    !self.settings.sandbox.namespace_restrictions;
            }
            FieldId::SandboxNetworkIsolation => {
                self.settings.sandbox.network_isolation = !self.settings.sandbox.network_isolation;
            }
            FieldId::SandboxFilesystemMode => {
                self.settings.sandbox.filesystem_mode =
                    next_filesystem_mode(self.settings.sandbox.filesystem_mode);
            }
            FieldId::McpTransport => {
                if let Some(server) = self.current_mcp_mut() {
                    server.transport = server.transport.next();
                }
            }
            FieldId::McpOauthXaa => {
                if let Some(server) = self.current_mcp_mut() {
                    server.oauth_xaa = !server.oauth_xaa;
                }
            }
            FieldId::AppearanceTheme => {
                self.settings.appearance.theme = self.settings.appearance.theme.next();
            }
            FieldId::AppearanceRedactSecrets => {
                self.settings.appearance.redact_secrets = !self.settings.appearance.redact_secrets;
            }
            FieldId::AppearanceKeybindingsHint => {
                self.settings.appearance.keybindings_hint = next_choice(
                    &self.settings.appearance.keybindings_hint,
                    KEYBINDING_PRESETS,
                );
            }
            FieldId::ReadOnly => self.status = "当前条目为只读。".to_string(),
            _ => {}
        }
        self.clamp_field_index();
    }

    fn editor_seed(&self, field: FieldId) -> Option<(&'static str, String)> {
        match field {
            FieldId::ProviderActiveProfile => Some((
                "Active profile",
                self.settings.provider.active_profile.clone(),
            )),
            FieldId::ProviderDefaultModel => Some((
                "Default model",
                self.settings.provider.default_model.clone(),
            )),
            FieldId::ProviderBaseUrl => Some(("Base URL", self.settings.provider.base_url.clone())),
            FieldId::ProviderApiKeyEnv => {
                Some(("API key env", self.settings.provider.api_key_env.clone()))
            }
            FieldId::ProviderRequestTimeoutMs => Some((
                "Timeout ms",
                self.settings.provider.request_timeout_ms.clone(),
            )),
            FieldId::ProviderMaxRetries => {
                Some(("Max retries", self.settings.provider.max_retries.clone()))
            }
            FieldId::RuntimeSessionDir => {
                Some(("Session dir", self.settings.runtime.session_dir.clone()))
            }
            FieldId::RuntimePermissionAllow => Some((
                "Allow rules",
                self.settings.runtime.permission_allow.clone(),
            )),
            FieldId::RuntimePermissionDeny => {
                Some(("Deny rules", self.settings.runtime.permission_deny.clone()))
            }
            FieldId::RuntimePermissionAsk => {
                Some(("Ask rules", self.settings.runtime.permission_ask.clone()))
            }
            FieldId::SandboxAllowedMounts => Some((
                "Allowed mounts",
                self.settings.sandbox.allowed_mounts.clone(),
            )),
            FieldId::ExtensionsPreToolUse => {
                Some(("Hook preToolUse", self.settings.hooks.pre_tool_use.clone()))
            }
            FieldId::ExtensionsPostToolUse => Some((
                "Hook postToolUse",
                self.settings.hooks.post_tool_use.clone(),
            )),
            FieldId::ExtensionsPostToolUseFailure => Some((
                "Hook postToolUseFailure",
                self.settings.hooks.post_tool_use_failure.clone(),
            )),
            FieldId::ExtensionsEnabledPlugins => Some((
                "Enabled plugins",
                self.settings.plugins.enabled_plugins.clone(),
            )),
            FieldId::ExtensionsDisabledPlugins => Some((
                "Disabled plugins",
                self.settings.plugins.disabled_plugins.clone(),
            )),
            FieldId::ExtensionsExternalDirectories => Some((
                "Plugin dirs",
                self.settings.plugins.external_directories.clone(),
            )),
            FieldId::ExtensionsInstallRoot => {
                Some(("Install root", self.settings.plugins.install_root.clone()))
            }
            FieldId::ExtensionsRegistryPath => {
                Some(("Registry path", self.settings.plugins.registry_path.clone()))
            }
            FieldId::ExtensionsBundledRoot => {
                Some(("Bundled root", self.settings.plugins.bundled_root.clone()))
            }
            FieldId::McpServerName => self
                .current_mcp()
                .map(|server| ("MCP name", server.name.clone())),
            FieldId::McpCommandOrUrl => self
                .current_mcp()
                .map(|server| ("MCP command / URL", server.command_or_url.clone())),
            FieldId::McpArgs => self
                .current_mcp()
                .map(|server| ("MCP args", server.args.clone())),
            FieldId::McpEnvOrHeaders => self
                .current_mcp()
                .map(|server| ("MCP env / headers", server.env_or_headers.clone())),
            FieldId::McpTimeoutMs => self
                .current_mcp()
                .map(|server| ("MCP timeout ms", server.timeout_ms.clone())),
            FieldId::McpHelperOrName => self
                .current_mcp()
                .map(|server| ("MCP helper / name", server.helper_or_name.clone())),
            FieldId::McpOauthClientId => self
                .current_mcp()
                .map(|server| ("OAuth client id", server.oauth_client_id.clone())),
            FieldId::McpOauthCallbackPort => self
                .current_mcp()
                .map(|server| ("OAuth callback port", server.oauth_callback_port.clone())),
            FieldId::McpOauthMetadataUrl => self
                .current_mcp()
                .map(|server| ("OAuth metadata URL", server.oauth_metadata_url.clone())),
            FieldId::BridgeTelegramBotToken => Some((
                "Telegram bot token",
                self.settings.bridge.telegram_bot_token.clone(),
            )),
            FieldId::BridgeWebhookUrl => {
                Some(("Webhook URL", self.settings.bridge.webhook_url.clone()))
            }
            FieldId::BridgeWebhookVerifyToken => Some((
                "Webhook verify token",
                self.settings.bridge.webhook_verify_token.clone(),
            )),
            FieldId::BridgeWhatsappPhoneId => Some((
                "WhatsApp phone id",
                self.settings.bridge.whatsapp_phone_id.clone(),
            )),
            FieldId::BridgeWhatsappToken => Some((
                "WhatsApp token",
                self.settings.bridge.whatsapp_token.clone(),
            )),
            FieldId::BridgeWhatsappAppSecret => Some((
                "WhatsApp app secret",
                self.settings.bridge.whatsapp_app_secret.clone(),
            )),
            FieldId::BridgeFeishuAppId => {
                Some(("Feishu app id", self.settings.bridge.feishu_app_id.clone()))
            }
            FieldId::BridgeFeishuAppSecret => Some((
                "Feishu app secret",
                self.settings.bridge.feishu_app_secret.clone(),
            )),
            _ => None,
        }
    }

    fn apply_editor_value(&mut self, target: FieldId, value: String) {
        match target {
            FieldId::ProviderActiveProfile => self.settings.provider.active_profile = value,
            FieldId::ProviderDefaultModel => self.settings.provider.default_model = value,
            FieldId::ProviderBaseUrl => self.settings.provider.base_url = value,
            FieldId::ProviderApiKeyEnv => self.settings.provider.api_key_env = value,
            FieldId::ProviderRequestTimeoutMs => self.settings.provider.request_timeout_ms = value,
            FieldId::ProviderMaxRetries => self.settings.provider.max_retries = value,
            FieldId::RuntimeSessionDir => self.settings.runtime.session_dir = value,
            FieldId::RuntimePermissionAllow => self.settings.runtime.permission_allow = value,
            FieldId::RuntimePermissionDeny => self.settings.runtime.permission_deny = value,
            FieldId::RuntimePermissionAsk => self.settings.runtime.permission_ask = value,
            FieldId::SandboxAllowedMounts => self.settings.sandbox.allowed_mounts = value,
            FieldId::ExtensionsPreToolUse => self.settings.hooks.pre_tool_use = value,
            FieldId::ExtensionsPostToolUse => self.settings.hooks.post_tool_use = value,
            FieldId::ExtensionsPostToolUseFailure => {
                self.settings.hooks.post_tool_use_failure = value;
            }
            FieldId::ExtensionsEnabledPlugins => self.settings.plugins.enabled_plugins = value,
            FieldId::ExtensionsDisabledPlugins => self.settings.plugins.disabled_plugins = value,
            FieldId::ExtensionsExternalDirectories => {
                self.settings.plugins.external_directories = value;
            }
            FieldId::ExtensionsInstallRoot => self.settings.plugins.install_root = value,
            FieldId::ExtensionsRegistryPath => self.settings.plugins.registry_path = value,
            FieldId::ExtensionsBundledRoot => self.settings.plugins.bundled_root = value,
            FieldId::McpServerName => {
                if let Some(server) = self.current_mcp_mut() {
                    server.name = value;
                }
            }
            FieldId::McpCommandOrUrl => {
                if let Some(server) = self.current_mcp_mut() {
                    server.command_or_url = value;
                }
            }
            FieldId::McpArgs => {
                if let Some(server) = self.current_mcp_mut() {
                    server.args = value;
                }
            }
            FieldId::McpEnvOrHeaders => {
                if let Some(server) = self.current_mcp_mut() {
                    server.env_or_headers = value;
                }
            }
            FieldId::McpTimeoutMs => {
                if let Some(server) = self.current_mcp_mut() {
                    server.timeout_ms = value;
                }
            }
            FieldId::McpHelperOrName => {
                if let Some(server) = self.current_mcp_mut() {
                    server.helper_or_name = value;
                }
            }
            FieldId::McpOauthClientId => {
                if let Some(server) = self.current_mcp_mut() {
                    server.oauth_client_id = value;
                }
            }
            FieldId::McpOauthCallbackPort => {
                if let Some(server) = self.current_mcp_mut() {
                    server.oauth_callback_port = value;
                }
            }
            FieldId::McpOauthMetadataUrl => {
                if let Some(server) = self.current_mcp_mut() {
                    server.oauth_metadata_url = value;
                }
            }
            FieldId::BridgeTelegramBotToken => self.settings.bridge.telegram_bot_token = value,
            FieldId::BridgeWebhookUrl => self.settings.bridge.webhook_url = value,
            FieldId::BridgeWebhookVerifyToken => self.settings.bridge.webhook_verify_token = value,
            FieldId::BridgeWhatsappPhoneId => self.settings.bridge.whatsapp_phone_id = value,
            FieldId::BridgeWhatsappToken => self.settings.bridge.whatsapp_token = value,
            FieldId::BridgeWhatsappAppSecret => self.settings.bridge.whatsapp_app_secret = value,
            FieldId::BridgeFeishuAppId => self.settings.bridge.feishu_app_id = value,
            FieldId::BridgeFeishuAppSecret => self.settings.bridge.feishu_app_secret = value,
            _ => {}
        }
    }

    pub(super) fn handle_editor_key(&mut self, key: KeyEvent) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.editor = None;
                self.status = "已取消编辑。".to_string();
            }
            KeyCode::Enter => {
                let editor = self.editor.take().expect("editor should exist");
                self.apply_editor_value(editor.target, editor.value);
                self.status = format!("已更新 {}。", editor.title);
            }
            KeyCode::Backspace => {
                if editor.cursor > 0 {
                    editor.cursor -= 1;
                    editor.value.remove(editor.cursor);
                }
            }
            KeyCode::Delete => {
                if editor.cursor < editor.value.len() {
                    editor.value.remove(editor.cursor);
                }
            }
            KeyCode::Left => editor.cursor = editor.cursor.saturating_sub(1),
            KeyCode::Right => editor.cursor = (editor.cursor + 1).min(editor.value.len()),
            KeyCode::Home => editor.cursor = 0,
            KeyCode::End => editor.cursor = editor.value.len(),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                editor.value.clear();
                editor.cursor = 0;
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                editor.value.insert(editor.cursor, ch);
                editor.cursor += ch.len_utf8();
            }
            _ => {}
        }
    }
}

fn next_choice(current: &str, choices: &[&str]) -> String {
    let index = choices
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);
    choices[(index + 1) % choices.len()].to_string()
}

fn next_filesystem_mode(mode: FilesystemIsolationMode) -> FilesystemIsolationMode {
    match mode {
        FilesystemIsolationMode::Off => FilesystemIsolationMode::WorkspaceOnly,
        FilesystemIsolationMode::WorkspaceOnly => FilesystemIsolationMode::AllowList,
        FilesystemIsolationMode::AllowList => FilesystemIsolationMode::Off,
    }
}
