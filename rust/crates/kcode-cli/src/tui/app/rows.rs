use crate::tui::config_store::review_lines;
use crate::tui::state::{redact, McpServerDraft, Section};

use super::core::{FieldId, FieldRow, TuiApp};

impl TuiApp {
    pub(crate) fn rows(&self) -> Vec<FieldRow> {
        match self.section {
            Section::Overview => self.overview_rows(),
            Section::Provider => self.provider_rows(),
            Section::Runtime => self.runtime_rows(),
            Section::Sandbox => self.sandbox_rows(),
            Section::Extensions => self.extensions_rows(),
            Section::Mcp => self.mcp_rows(),
            Section::Bridge => self.bridge_rows(),
            Section::Appearance => self.appearance_rows(),
            Section::Review => self.review_rows(),
        }
    }

    pub(super) fn current_mcp(&self) -> Option<&McpServerDraft> {
        self.settings.mcp.servers.get(self.settings.mcp.selected)
    }

    pub(super) fn current_mcp_mut(&mut self) -> Option<&mut McpServerDraft> {
        let len = self.settings.mcp.servers.len();
        if len == 0 {
            return None;
        }
        if self.settings.mcp.selected >= len {
            self.settings.mcp.selected = len - 1;
        }
        self.settings
            .mcp
            .servers
            .get_mut(self.settings.mcp.selected)
    }

    fn overview_rows(&self) -> Vec<FieldRow> {
        let mut rows = vec![
            self.readonly("Scope", self.settings.scope.label()),
            self.readonly(
                "Runtime ready",
                super::core::yes_no(self.settings.overview.runtime_ready),
            ),
            self.readonly(
                "Config home",
                self.settings.overview.config_home.display().to_string(),
            ),
            self.readonly(
                "Session dir",
                self.settings.overview.session_dir.display().to_string(),
            ),
            self.readonly(
                "Config file",
                self.settings.overview.config_path.display().to_string(),
            ),
            self.readonly(
                "Bridge env",
                self.settings.overview.bridge_env_path.display().to_string(),
            ),
        ];
        for path in &self.settings.overview.loaded_files {
            rows.push(self.readonly("Loaded", path.display().to_string()));
        }
        for line in self.settings.overview.doctor_report.lines() {
            rows.push(self.readonly("Doctor", line.to_string()));
        }
        rows
    }

    fn provider_rows(&self) -> Vec<FieldRow> {
        vec![
            self.editable(
                FieldId::ProviderActiveProfile,
                "Active profile",
                self.settings.provider.active_profile.clone(),
            ),
            self.editable(
                FieldId::ProviderDefaultModel,
                "Default model",
                self.settings.provider.default_model.clone(),
            ),
            self.editable(
                FieldId::ProviderBaseUrl,
                "Base URL",
                self.settings.provider.base_url.clone(),
            ),
            self.editable(
                FieldId::ProviderApiKeyEnv,
                "API key env",
                self.settings.provider.api_key_env.clone(),
            ),
            self.editable(
                FieldId::ProviderRequestTimeoutMs,
                "Timeout ms",
                self.settings.provider.request_timeout_ms.clone(),
            ),
            self.editable(
                FieldId::ProviderMaxRetries,
                "Max retries",
                self.settings.provider.max_retries.clone(),
            ),
            self.toggle(
                FieldId::ProviderSupportsTools,
                "Supports tools",
                self.settings.provider.supports_tools,
            ),
            self.toggle(
                FieldId::ProviderSupportsStreaming,
                "Supports streaming",
                self.settings.provider.supports_streaming,
            ),
        ]
    }

    fn runtime_rows(&self) -> Vec<FieldRow> {
        vec![
            self.editable(
                FieldId::RuntimePermissionMode,
                "Permission mode",
                self.settings.runtime.permission_mode.clone(),
            ),
            self.editable(
                FieldId::RuntimeSessionDir,
                "Session dir",
                self.settings.runtime.session_dir.clone(),
            ),
            self.editable(
                FieldId::RuntimePermissionAllow,
                "Allow rules",
                self.settings.runtime.permission_allow.clone(),
            ),
            self.editable(
                FieldId::RuntimePermissionDeny,
                "Deny rules",
                self.settings.runtime.permission_deny.clone(),
            ),
            self.editable(
                FieldId::RuntimePermissionAsk,
                "Ask rules",
                self.settings.runtime.permission_ask.clone(),
            ),
        ]
    }

    fn sandbox_rows(&self) -> Vec<FieldRow> {
        vec![
            self.toggle(
                FieldId::SandboxEnabled,
                "Sandbox enabled",
                self.settings.sandbox.enabled,
            ),
            self.toggle(
                FieldId::SandboxNamespaceRestrictions,
                "Namespace restrictions",
                self.settings.sandbox.namespace_restrictions,
            ),
            self.toggle(
                FieldId::SandboxNetworkIsolation,
                "Network isolation",
                self.settings.sandbox.network_isolation,
            ),
            self.editable(
                FieldId::SandboxFilesystemMode,
                "Filesystem mode",
                self.settings.sandbox.filesystem_mode.as_str(),
            ),
            self.editable(
                FieldId::SandboxAllowedMounts,
                "Allowed mounts",
                self.settings.sandbox.allowed_mounts.clone(),
            ),
        ]
    }

    fn extensions_rows(&self) -> Vec<FieldRow> {
        vec![
            self.editable(
                FieldId::ExtensionsPreToolUse,
                "Hook preToolUse",
                self.settings.hooks.pre_tool_use.clone(),
            ),
            self.editable(
                FieldId::ExtensionsPostToolUse,
                "Hook postToolUse",
                self.settings.hooks.post_tool_use.clone(),
            ),
            self.editable(
                FieldId::ExtensionsPostToolUseFailure,
                "Hook postToolUseFailure",
                self.settings.hooks.post_tool_use_failure.clone(),
            ),
            self.editable(
                FieldId::ExtensionsEnabledPlugins,
                "Enabled plugins",
                self.settings.plugins.enabled_plugins.clone(),
            ),
            self.editable(
                FieldId::ExtensionsDisabledPlugins,
                "Disabled plugins",
                self.settings.plugins.disabled_plugins.clone(),
            ),
            self.editable(
                FieldId::ExtensionsExternalDirectories,
                "Plugin dirs",
                self.settings.plugins.external_directories.clone(),
            ),
            self.editable(
                FieldId::ExtensionsInstallRoot,
                "Install root",
                self.settings.plugins.install_root.clone(),
            ),
            self.editable(
                FieldId::ExtensionsRegistryPath,
                "Registry path",
                self.settings.plugins.registry_path.clone(),
            ),
            self.editable(
                FieldId::ExtensionsBundledRoot,
                "Bundled root",
                self.settings.plugins.bundled_root.clone(),
            ),
        ]
    }

    fn mcp_rows(&self) -> Vec<FieldRow> {
        let Some(server) = self.current_mcp() else {
            return vec![self.readonly("MCP", "No servers configured. Press n to add one.")];
        };
        let index = self.settings.mcp.selected + 1;
        let total = self.settings.mcp.servers.len();
        vec![
            self.readonly("Server", format!("{index}/{total}")),
            self.editable(FieldId::McpServerName, "Name", server.name.clone()),
            self.editable(FieldId::McpTransport, "Transport", server.transport.label()),
            self.editable(
                FieldId::McpCommandOrUrl,
                "Command / URL",
                server.command_or_url.clone(),
            ),
            self.editable(FieldId::McpArgs, "Args", server.args.clone()),
            self.editable(
                FieldId::McpEnvOrHeaders,
                "Env / headers",
                server.env_or_headers.clone(),
            ),
            self.editable(
                FieldId::McpTimeoutMs,
                "Timeout ms",
                server.timeout_ms.clone(),
            ),
            self.editable(
                FieldId::McpHelperOrName,
                "Helper / name",
                server.helper_or_name.clone(),
            ),
            self.editable(
                FieldId::McpOauthClientId,
                "OAuth client id",
                server.oauth_client_id.clone(),
            ),
            self.editable(
                FieldId::McpOauthCallbackPort,
                "OAuth callback port",
                server.oauth_callback_port.clone(),
            ),
            self.editable(
                FieldId::McpOauthMetadataUrl,
                "OAuth metadata URL",
                server.oauth_metadata_url.clone(),
            ),
            self.toggle(FieldId::McpOauthXaa, "OAuth XAA", server.oauth_xaa),
        ]
    }

    fn bridge_rows(&self) -> Vec<FieldRow> {
        let redact_secrets = self.settings.appearance.redact_secrets;
        vec![
            self.editable(
                FieldId::BridgeTelegramBotToken,
                "Telegram bot token",
                redact(&self.settings.bridge.telegram_bot_token, redact_secrets),
            ),
            self.editable(
                FieldId::BridgeWebhookUrl,
                "Webhook URL",
                self.settings.bridge.webhook_url.clone(),
            ),
            self.editable(
                FieldId::BridgeWebhookVerifyToken,
                "Webhook verify token",
                redact(&self.settings.bridge.webhook_verify_token, redact_secrets),
            ),
            self.editable(
                FieldId::BridgeWhatsappPhoneId,
                "WhatsApp phone id",
                self.settings.bridge.whatsapp_phone_id.clone(),
            ),
            self.editable(
                FieldId::BridgeWhatsappToken,
                "WhatsApp token",
                redact(&self.settings.bridge.whatsapp_token, redact_secrets),
            ),
            self.editable(
                FieldId::BridgeWhatsappAppSecret,
                "WhatsApp app secret",
                redact(&self.settings.bridge.whatsapp_app_secret, redact_secrets),
            ),
            self.editable(
                FieldId::BridgeFeishuAppId,
                "Feishu app id",
                self.settings.bridge.feishu_app_id.clone(),
            ),
            self.editable(
                FieldId::BridgeFeishuAppSecret,
                "Feishu app secret",
                redact(&self.settings.bridge.feishu_app_secret, redact_secrets),
            ),
        ]
    }

    fn appearance_rows(&self) -> Vec<FieldRow> {
        vec![
            self.editable(
                FieldId::AppearanceTheme,
                "Theme",
                self.settings.appearance.theme.label(),
            ),
            self.toggle(
                FieldId::AppearanceRedactSecrets,
                "Redact secrets",
                self.settings.appearance.redact_secrets,
            ),
            self.editable(
                FieldId::AppearanceKeybindingsHint,
                "Keybindings",
                self.settings.appearance.keybindings_hint.clone(),
            ),
        ]
    }

    fn review_rows(&self) -> Vec<FieldRow> {
        review_lines(&self.settings)
            .into_iter()
            .map(|line| self.readonly("Review", line))
            .collect()
    }
}
