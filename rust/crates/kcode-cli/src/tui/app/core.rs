use std::error::Error;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::config_store::{load_settings, save_settings};
use crate::tui::state::{ConfigScope, McpServerDraft, Section, TuiSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldId {
    ReadOnly,
    ProviderActiveProfile,
    ProviderDefaultModel,
    ProviderBaseUrl,
    ProviderApiKeyEnv,
    ProviderRequestTimeoutMs,
    ProviderMaxRetries,
    ProviderSupportsTools,
    ProviderSupportsStreaming,
    RuntimePermissionMode,
    RuntimeSessionDir,
    RuntimePermissionAllow,
    RuntimePermissionDeny,
    RuntimePermissionAsk,
    SandboxEnabled,
    SandboxNamespaceRestrictions,
    SandboxNetworkIsolation,
    SandboxFilesystemMode,
    SandboxAllowedMounts,
    ExtensionsPreToolUse,
    ExtensionsPostToolUse,
    ExtensionsPostToolUseFailure,
    ExtensionsEnabledPlugins,
    ExtensionsDisabledPlugins,
    ExtensionsExternalDirectories,
    ExtensionsInstallRoot,
    ExtensionsRegistryPath,
    ExtensionsBundledRoot,
    McpServerName,
    McpTransport,
    McpCommandOrUrl,
    McpArgs,
    McpEnvOrHeaders,
    McpTimeoutMs,
    McpHelperOrName,
    McpOauthClientId,
    McpOauthCallbackPort,
    McpOauthMetadataUrl,
    McpOauthXaa,
    BridgeTelegramBotToken,
    BridgeWebhookUrl,
    BridgeWebhookVerifyToken,
    BridgeWhatsappPhoneId,
    BridgeWhatsappToken,
    BridgeWhatsappAppSecret,
    BridgeFeishuAppId,
    BridgeFeishuAppSecret,
    AppearanceTheme,
    AppearanceRedactSecrets,
    AppearanceKeybindingsHint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldRow {
    pub(crate) id: FieldId,
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) editable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorState {
    pub(crate) target: FieldId,
    pub(crate) title: String,
    pub(crate) value: String,
    pub(crate) cursor: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TuiApp {
    pub(super) settings: TuiSettings,
    pub(super) saved_settings: TuiSettings,
    pub(super) section: Section,
    pub(super) field_index: usize,
    pub(super) status: String,
    pub(super) quitting: bool,
    pub(super) discard_armed: bool,
    pub(super) editor: Option<EditorState>,
}

impl TuiApp {
    pub(crate) fn load(cwd: PathBuf, section: Section) -> Result<Self, Box<dyn Error>> {
        let settings = load_settings(&cwd, ConfigScope::User)?;
        Ok(Self {
            saved_settings: settings.clone(),
            settings,
            section,
            field_index: 0,
            status: "左右切换页面，回车编辑，s 保存，g 切换作用域，q 退出。".to_string(),
            quitting: false,
            discard_armed: false,
            editor: None,
        })
    }

    pub(crate) fn section(&self) -> Section {
        self.section
    }

    pub(crate) fn settings(&self) -> &TuiSettings {
        &self.settings
    }

    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.settings != self.saved_settings
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.quitting
    }

    pub(crate) fn editor(&self) -> Option<&EditorState> {
        self.editor.as_ref()
    }

    pub(crate) fn field_index(&self) -> usize {
        self.field_index
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        if self.editor.is_some() {
            self.handle_editor_key(key);
            return;
        }

        match key.code {
            KeyCode::Left | KeyCode::BackTab => {
                self.discard_armed = false;
                self.shift_section(-1);
            }
            KeyCode::Right | KeyCode::Tab => {
                self.discard_armed = false;
                self.shift_section(1);
            }
            KeyCode::Up => {
                self.discard_armed = false;
                self.move_field(-1);
            }
            KeyCode::Down => {
                self.discard_armed = false;
                self.move_field(1);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.discard_armed = false;
                self.activate_current_field();
            }
            KeyCode::Char('s') if key.modifiers.is_empty() => {
                self.discard_armed = false;
                self.try_save();
            }
            KeyCode::Char('r' | 'd') if key.modifiers.is_empty() => {
                self.discard_armed = false;
                self.try_reload(self.settings.scope);
            }
            KeyCode::Char('g') if key.modifiers.is_empty() => {
                self.discard_armed = false;
                self.try_toggle_scope();
            }
            KeyCode::Char('n') if key.modifiers.is_empty() && self.section == Section::Mcp => {
                self.discard_armed = false;
                self.add_mcp_server();
            }
            KeyCode::Char('x') if key.modifiers.is_empty() && self.section == Section::Mcp => {
                self.discard_armed = false;
                self.remove_mcp_server();
            }
            KeyCode::Char('[') if key.modifiers.is_empty() && self.section == Section::Mcp => {
                self.discard_armed = false;
                self.shift_mcp(-1);
            }
            KeyCode::Char(']') if key.modifiers.is_empty() && self.section == Section::Mcp => {
                self.discard_armed = false;
                self.shift_mcp(1);
            }
            KeyCode::Esc | KeyCode::Char('q') if key.modifiers.is_empty() => self.try_quit(),
            _ => {}
        }
    }

    pub(super) fn try_save(&mut self) {
        match save_settings(&mut self.settings) {
            Ok(outcome) => {
                self.saved_settings = self.settings.clone();
                self.field_index = 0;
                self.status = format!(
                    "已保存到 {}，bridge env: {}。",
                    outcome.config_path.display(),
                    outcome.bridge_env_path.display()
                );
            }
            Err(error) => self.status = format!("保存失败: {error}"),
        }
    }

    fn try_reload(&mut self, scope: ConfigScope) {
        if self.is_dirty() {
            self.status = "存在未保存改动，先保存或按 q 放弃后再重载。".to_string();
            return;
        }
        match load_settings(&self.settings.overview.cwd, scope) {
            Ok(settings) => {
                self.settings = settings.clone();
                self.saved_settings = settings;
                self.field_index = 0;
                self.status = format!("已从 {} 作用域重载。", scope.label());
            }
            Err(error) => self.status = format!("重载失败: {error}"),
        }
    }

    fn try_toggle_scope(&mut self) {
        if self.is_dirty() {
            self.status = "存在未保存改动，保存后再切换 user/project。".to_string();
            return;
        }
        self.try_reload(self.settings.scope.toggle());
    }

    fn try_quit(&mut self) {
        if !self.is_dirty() || self.discard_armed {
            self.quitting = true;
            return;
        }
        self.discard_armed = true;
        self.status = "存在未保存改动，再按 q 放弃退出，或按 s 保存。".to_string();
    }

    fn shift_section(&mut self, delta: isize) {
        self.section = shift_section(self.section, delta);
        self.field_index = 0;
    }

    fn move_field(&mut self, delta: isize) {
        let len = self.rows().len();
        if len == 0 {
            self.field_index = 0;
        } else {
            self.field_index = shift_index(self.field_index, len, delta);
        }
    }

    fn add_mcp_server(&mut self) {
        let mut server = McpServerDraft::default();
        server.name = self.unique_mcp_name();
        self.settings.mcp.servers.push(server);
        self.settings.mcp.selected = self.settings.mcp.servers.len().saturating_sub(1);
        self.field_index = 1;
        self.status = "已新增 MCP server，按 Enter 继续编辑。".to_string();
    }

    fn remove_mcp_server(&mut self) {
        if self.settings.mcp.servers.is_empty() {
            self.status = "当前没有 MCP server。".to_string();
            return;
        }
        self.settings.mcp.servers.remove(self.settings.mcp.selected);
        if self.settings.mcp.selected >= self.settings.mcp.servers.len() {
            self.settings.mcp.selected = self.settings.mcp.servers.len().saturating_sub(1);
        }
        self.field_index = 0;
        self.status = "已删除当前 MCP server。".to_string();
    }

    fn shift_mcp(&mut self, delta: isize) {
        let len = self.settings.mcp.servers.len();
        if len == 0 {
            self.status = "当前没有 MCP server，可按 n 新增。".to_string();
            return;
        }
        self.settings.mcp.selected = shift_index(self.settings.mcp.selected, len, delta);
        self.field_index = self.field_index.min(self.rows().len().saturating_sub(1));
    }

    pub(super) fn unique_mcp_name(&self) -> String {
        if !self
            .settings
            .mcp
            .servers
            .iter()
            .any(|server| server.name == "new-server")
        {
            return "new-server".to_string();
        }
        for index in 2..1000 {
            let candidate = format!("new-server-{index}");
            if !self
                .settings
                .mcp
                .servers
                .iter()
                .any(|server| server.name == candidate)
            {
                return candidate;
            }
        }
        "new-server-overflow".to_string()
    }

    pub(super) fn clamp_field_index(&mut self) {
        self.field_index = self.field_index.min(self.rows().len().saturating_sub(1));
    }

    pub(super) fn editable(
        &self,
        id: FieldId,
        label: impl Into<String>,
        value: impl Into<String>,
    ) -> FieldRow {
        FieldRow {
            id,
            label: label.into(),
            value: value.into(),
            editable: true,
        }
    }

    pub(super) fn readonly(&self, label: impl Into<String>, value: impl Into<String>) -> FieldRow {
        FieldRow {
            id: FieldId::ReadOnly,
            label: label.into(),
            value: value.into(),
            editable: false,
        }
    }

    pub(super) fn toggle(&self, id: FieldId, label: impl Into<String>, value: bool) -> FieldRow {
        self.editable(id, label, yes_no(value))
    }
}

pub(super) fn shift_index(index: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        0
    } else {
        ((index as isize + delta).rem_euclid(len as isize)) as usize
    }
}

fn shift_section(section: Section, delta: isize) -> Section {
    let index = Section::ALL
        .iter()
        .position(|candidate| *candidate == section)
        .unwrap_or(0);
    Section::ALL[shift_index(index, Section::ALL.len(), delta)]
}

pub(super) fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
