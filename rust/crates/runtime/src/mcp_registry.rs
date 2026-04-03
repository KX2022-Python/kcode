use std::collections::BTreeMap;
use std::path::Path;

use crate::config::{ConfigSource, McpServerConfig, ScopedMcpServerConfig};
use crate::mcp::mcp_server_signature;

/// A normalized MCP server descriptor after policy and dedup resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerDescriptor {
    pub name: String,
    pub config: McpServerConfig,
    pub source: ConfigSource,
}

/// Policy rule for allowing or denying MCP servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpPolicyRule {
    DenyName(String),
    DenyCommand(String),
    DenyUrl(String),
    AllowName(String),
    AllowCommand(String),
    AllowUrl(String),
}

/// MCP policy containing allow and deny rules.
#[derive(Debug, Clone, Default)]
pub struct McpPolicy {
    deny_rules: Vec<McpPolicyRule>,
    allow_rules: Vec<McpPolicyRule>,
}

impl McpPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_deny(&mut self, rule: McpPolicyRule) {
        self.deny_rules.push(rule);
    }

    pub fn add_allow(&mut self, rule: McpPolicyRule) {
        self.allow_rules.push(rule);
    }

    pub fn is_denied(&self, name: &str, config: &McpServerConfig) -> bool {
        for rule in &self.deny_rules {
            if rule_matches(rule, name, config) {
                return true;
            }
        }
        false
    }

    pub fn is_allowed(&self, name: &str, config: &McpServerConfig) -> bool {
        if self.allow_rules.is_empty() {
            return true;
        }
        for rule in &self.allow_rules {
            if rule_matches(rule, name, config) {
                return true;
            }
        }
        false
    }
}

fn rule_matches(rule: &McpPolicyRule, name: &str, config: &McpServerConfig) -> bool {
    use crate::mcp::normalize_name_for_mcp;
    match rule {
        McpPolicyRule::DenyName(n) | McpPolicyRule::AllowName(n) => {
            normalize_name_for_mcp(n) == normalize_name_for_mcp(name)
        }
        McpPolicyRule::DenyCommand(cmd) | McpPolicyRule::AllowCommand(cmd) => {
            matches!(config, McpServerConfig::Stdio(s) if s.command == *cmd)
        }
        McpPolicyRule::DenyUrl(url) | McpPolicyRule::AllowUrl(url) => match config {
            McpServerConfig::Sse(s) | McpServerConfig::Http(s) => s.url == *url,
            McpServerConfig::Ws(s) => s.url == *url,
            _ => false,
        },
    }
}

/// Status of a server that was not activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedServer {
    pub name: String,
    pub source: ConfigSource,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateServer {
    pub name: String,
    signature: String,
    suppressed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpValidationError {
    pub source: ConfigSource,
    pub message: String,
}

/// Snapshot of the assembled MCP registry.
#[derive(Debug, Clone, Default)]
pub struct McpRegistrySnapshot {
    pub active_servers: Vec<McpServerDescriptor>,
    pub disabled_servers: Vec<BlockedServer>,
    pub blocked_servers: Vec<BlockedServer>,
    pub duplicate_servers: Vec<DuplicateServer>,
    pub validation_errors: Vec<McpValidationError>,
}

impl McpRegistrySnapshot {
    pub fn render_summary(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Active servers ({}):", self.active_servers.len()));
        for server in &self.active_servers {
            lines.push(format!(
                "  \u{2713} {} (source: {:?})",
                server.name, server.source
            ));
        }

        if !self.disabled_servers.is_empty() {
            lines.push(format!("Disabled servers ({}):", self.disabled_servers.len()));
            for server in &self.disabled_servers {
                lines.push(format!("  \u{2717} {} \u{2014} {}", server.name, server.reason));
            }
        }

        if !self.blocked_servers.is_empty() {
            lines.push(format!("Blocked servers ({}):", self.blocked_servers.len()));
            for server in &self.blocked_servers {
                lines.push(format!("  \u{1F6AB} {} \u{2014} {}", server.name, server.reason));
            }
        }

        if !self.duplicate_servers.is_empty() {
            lines.push(format!("Duplicate servers ({}):", self.duplicate_servers.len()));
            for dup in &self.duplicate_servers {
                lines.push(format!(
                    "  \u{21C4} {} suppressed by {}",
                    dup.name, dup.suppressed_by
                ));
            }
        }

        if !self.validation_errors.is_empty() {
            lines.push(format!("Validation errors ({}):", self.validation_errors.len()));
            for err in &self.validation_errors {
                lines.push(format!("  \u{26A0} {:?}: {}", err.source, err.message));
            }
        }

        if self.active_servers.is_empty()
            && self.disabled_servers.is_empty()
            && self.blocked_servers.is_empty()
            && self.duplicate_servers.is_empty()
            && self.validation_errors.is_empty()
        {
            lines.push("  No MCP servers configured.".to_string());
        }

        lines.join("\n")
    }
}

/// Assembles an MCP registry from multiple configuration sources.
/// Supports enterprise-managed MCP configuration with exclusive mode.
pub struct McpRegistryAssembler {
    sources: BTreeMap<ConfigSource, Vec<ScopedMcpServerConfig>>,
    policy: McpPolicy,
    /// When true, only managed servers are allowed; all others are blocked.
    managed_exclusive: bool,
}

impl McpRegistryAssembler {
    pub fn new() -> Self {
        Self {
            sources: BTreeMap::new(),
            policy: McpPolicy::new(),
            managed_exclusive: false,
        }
    }

    pub fn add_source(&mut self, source: ConfigSource, servers: Vec<ScopedMcpServerConfig>) {
        self.sources.entry(source).or_default().extend(servers);
    }

    pub fn set_policy(&mut self, policy: McpPolicy) {
        self.policy = policy;
    }

    /// Enable managed-exclusive mode: only managed servers are allowed.
    pub fn with_managed_exclusive(mut self, exclusive: bool) -> Self {
        self.managed_exclusive = exclusive;
        self
    }

    /// Load managed MCP config from a JSON file.
    pub fn load_managed_config(&mut self, path: &Path) -> Result<(), String> {
        let servers = load_mcp_config_file(path)?;
        let managed_servers: Vec<ScopedMcpServerConfig> = servers
            .into_iter()
            .map(|mut s| {
                s.scope = ConfigSource::Managed;
                s
            })
            .collect();
        if !managed_servers.is_empty() {
            self.sources
                .entry(ConfigSource::Managed)
                .or_default()
                .extend(managed_servers);
        }
        Ok(())
    }

    pub fn assemble(mut self) -> McpRegistrySnapshot {
        let mut snapshot = McpRegistrySnapshot::default();

        // Priority order: Managed > User > Project > Local
        let priority_order = [
            ConfigSource::Local,
            ConfigSource::Project,
            ConfigSource::User,
            ConfigSource::Managed,
        ];

        let mut signature_map: BTreeMap<String, McpServerDescriptor> = BTreeMap::new();
        let mut name_set: BTreeMap<String, ConfigSource> = BTreeMap::new();

        for source in &priority_order {
            if let Some(servers) = self.sources.remove(source) {
                for scoped in servers {
                    // In managed-exclusive mode, block non-managed servers
                    if self.managed_exclusive && scoped.scope != ConfigSource::Managed {
                        snapshot.blocked_servers.push(BlockedServer {
                            name: normalized_server_name(&scoped.config),
                            source: scoped.scope,
                            reason: "blocked by managed-exclusive policy".to_string(),
                        });
                        continue;
                    }

                    let name = normalized_server_name(&scoped.config);

                    if self.policy.is_denied(&name, &scoped.config) {
                        snapshot.blocked_servers.push(BlockedServer {
                            name: name.clone(),
                            source: scoped.scope,
                            reason: "denied by policy".to_string(),
                        });
                        continue;
                    }

                    if !self.policy.is_allowed(&name, &scoped.config) {
                        snapshot.disabled_servers.push(BlockedServer {
                            name: name.clone(),
                            source: scoped.scope,
                            reason: "not allowed by policy".to_string(),
                        });
                        continue;
                    }

                    if let Some(sig) = mcp_server_signature(&scoped.config) {
                        if let Some(existing) = signature_map.get(&sig) {
                            // Higher priority source should replace
                            if scoped.scope < existing.source {
                                // Remove old from active, add it to duplicates
                                snapshot.duplicate_servers.push(DuplicateServer {
                                    name: existing.name.clone(),
                                    signature: sig.clone(),
                                    suppressed_by: name.clone(),
                                });
                                signature_map.insert(
                                    sig.clone(),
                                    McpServerDescriptor {
                                        name: name.clone(),
                                        config: scoped.config,
                                        source: scoped.scope,
                                    },
                                );
                                name_set.insert(name, scoped.scope);
                            } else {
                                snapshot.duplicate_servers.push(DuplicateServer {
                                    name: name.clone(),
                                    signature: sig.clone(),
                                    suppressed_by: existing.name.clone(),
                                });
                            }
                            continue;
                        }

                        if let Some(existing_source) = name_set.get(&name) {
                            if scoped.scope <= *existing_source {
                                signature_map.insert(
                                    sig.clone(),
                                    McpServerDescriptor {
                                        name: name.clone(),
                                        config: scoped.config,
                                        source: scoped.scope,
                                    },
                                );
                                name_set.insert(name, scoped.scope);
                            }
                        } else {
                            signature_map.insert(
                                sig.clone(),
                                McpServerDescriptor {
                                    name: name.clone(),
                                    config: scoped.config,
                                    source: scoped.scope,
                                },
                            );
                            name_set.insert(name, scoped.scope);
                        }
                    }
                }
            }
        }

        snapshot.active_servers = signature_map.into_values().collect();
        snapshot.active_servers.sort_by(|a, b| a.name.cmp(&b.name));
        snapshot
    }
}

fn normalized_server_name(config: &McpServerConfig) -> String {
    match config {
        McpServerConfig::Stdio(s) => {
            let primary = s.args.first().cloned().unwrap_or_else(|| {
                std::path::Path::new(&s.command)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| s.command.clone())
            });
            format!("{primary} (stdio)")
        }
        McpServerConfig::Sse(s) | McpServerConfig::Http(s) => {
            let host = url_host(&s.url).unwrap_or_else(|| "remote".to_string());
            format!("{host} (remote)")
        }
        McpServerConfig::Ws(s) => {
            let host = url_host(&s.url).unwrap_or_else(|| "websocket".to_string());
            format!("{host} (ws)")
        }
        McpServerConfig::Sdk(s) => s.name.clone(),
        McpServerConfig::ManagedProxy(s) => format!("{} (managed)", s.id),
    }
}

fn url_host(url: &str) -> Option<String> {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .map(String::from)
}

/// Load MCP servers from a JSON config file at the given path.
pub fn load_mcp_config_file(path: &Path) -> Result<Vec<ScopedMcpServerConfig>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in {}: {e}", path.display()))?;

    parse_mcp_servers_from_json(&json)
}

fn parse_mcp_servers_from_json(
    value: &serde_json::Value,
) -> Result<Vec<ScopedMcpServerConfig>, String> {
    let mut servers = Vec::new();

    let mcp_section = value
        .get("mcp")
        .or_else(|| value.get("mcpServers"))
        .ok_or_else(|| "no 'mcp' or 'mcpServers' key found".to_string())?;

    let server_map = mcp_section
        .as_object()
        .ok_or_else(|| "mcp section is not an object".to_string())?;

    for (name, server_config) in server_map {
        if let Some(config) = parse_single_server_config(server_config) {
            servers.push(config);
            let _ = name; // name is embedded in the config
        }
    }

    Ok(servers)
}

fn parse_single_server_config(value: &serde_json::Value) -> Option<ScopedMcpServerConfig> {
    // Check for URL-based transports first (SSE/HTTP/WS)
    if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
        let transport_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("sse")
            .to_lowercase();

        let headers = value
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let headers_helper = value
            .get("headersHelper")
            .and_then(|v| v.as_str())
            .map(String::from);

        let oauth = parse_optional_mcp_oauth(value);

        let config = match transport_type.as_str() {
            "http" => McpServerConfig::Http(crate::config::McpRemoteServerConfig {
                url: url.to_string(),
                headers,
                headers_helper,
                oauth,
            }),
            "ws" | "websocket" => McpServerConfig::Ws(crate::config::McpWebSocketServerConfig {
                url: url.to_string(),
                headers,
                headers_helper,
            }),
            _ => McpServerConfig::Sse(crate::config::McpRemoteServerConfig {
                url: url.to_string(),
                headers,
                headers_helper,
                oauth,
            }),
        };

        return Some(ScopedMcpServerConfig {
            scope: ConfigSource::User,
            config,
        });
    }

    // Fall back to stdio transport
    let command = value.get("command").and_then(|v| v.as_str())?;
    let args = value
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let env = value
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let timeout = value
        .get("toolCallTimeoutMs")
        .or_else(|| value.get("tool_call_timeout_ms"))
        .and_then(|v| v.as_u64());

    Some(ScopedMcpServerConfig {
        scope: ConfigSource::User,
        config: McpServerConfig::Stdio(crate::config::McpStdioServerConfig {
            command: command.to_string(),
            args,
            env,
            tool_call_timeout_ms: timeout,
        }),
    })
}

fn parse_optional_mcp_oauth(value: &serde_json::Value) -> Option<crate::config::McpOAuthConfig> {
    let oauth = value.get("oauth")?;
    let client_id = oauth.get("clientId").or_else(|| oauth.get("client_id")).and_then(|v| v.as_str()).map(String::from);
    let callback_port = oauth.get("callbackPort").or_else(|| oauth.get("callback_port")).and_then(|v| v.as_u64()).map(|v| v as u16);
    let auth_server_metadata_url = oauth.get("authorizationUrl")
        .or_else(|| oauth.get("authServerMetadataUrl"))
        .or_else(|| oauth.get("auth_server_metadata_url"))
        .and_then(|v| v.as_str())
        .map(String::from);
    Some(crate::config::McpOAuthConfig {
        client_id,
        callback_port,
        auth_server_metadata_url,
        xaa: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpStdioServerConfig;

    #[test]
    fn assembles_servers_from_single_source() {
        let servers = vec![ScopedMcpServerConfig {
            scope: ConfigSource::User,
            config: McpServerConfig::Stdio(McpStdioServerConfig {
                command: "uvx".to_string(),
                args: vec!["mcp-server".to_string()],
                env: BTreeMap::new(),
                tool_call_timeout_ms: None,
            }),
        }];

        let mut assembler = McpRegistryAssembler::new();
        assembler.add_source(ConfigSource::User, servers);
        let snapshot = assembler.assemble();

        assert_eq!(snapshot.active_servers.len(), 1);
        assert_eq!(snapshot.blocked_servers.len(), 0);
        assert_eq!(snapshot.duplicate_servers.len(), 0);
    }

    #[test]
    fn denies_server_by_name() {
        let servers = vec![
            ScopedMcpServerConfig {
                scope: ConfigSource::User,
                config: McpServerConfig::Stdio(McpStdioServerConfig {
                    command: "uvx".to_string(),
                    args: vec!["server-a".to_string()],
                    env: BTreeMap::new(),
                    tool_call_timeout_ms: None,
                }),
            },
            ScopedMcpServerConfig {
                scope: ConfigSource::User,
                config: McpServerConfig::Stdio(McpStdioServerConfig {
                    command: "uvx".to_string(),
                    args: vec!["server-b".to_string()],
                    env: BTreeMap::new(),
                    tool_call_timeout_ms: None,
                }),
            },
        ];

        let mut assembler = McpRegistryAssembler::new();
        let mut policy = McpPolicy::new();
        policy.add_deny(McpPolicyRule::DenyName("server-a (stdio)".to_string()));
        assembler.add_source(ConfigSource::User, servers);
        assembler.set_policy(policy);
        let snapshot = assembler.assemble();

        assert_eq!(snapshot.active_servers.len(), 1);
        assert_eq!(snapshot.blocked_servers.len(), 1);
    }

    #[test]
    fn dedup_by_signature() {
        let config_a = McpServerConfig::Stdio(McpStdioServerConfig {
            command: "uvx".to_string(),
            args: vec!["same-server".to_string()],
            env: BTreeMap::new(),
            tool_call_timeout_ms: None,
        });
        let config_b = config_a.clone();

        let mut assembler = McpRegistryAssembler::new();
        assembler.add_source(
            ConfigSource::Local,
            vec![ScopedMcpServerConfig {
                scope: ConfigSource::Local,
                config: config_a,
            }],
        );
        assembler.add_source(
            ConfigSource::User,
            vec![ScopedMcpServerConfig {
                scope: ConfigSource::User,
                config: config_b,
            }],
        );
        let snapshot = assembler.assemble();

        assert_eq!(snapshot.active_servers.len(), 1);
        assert_eq!(snapshot.active_servers[0].source, ConfigSource::User);
        assert_eq!(snapshot.duplicate_servers.len(), 1);
    }

    #[test]
    fn snapshot_summary_renders() {
        let snapshot = McpRegistrySnapshot::default();
        let summary = snapshot.render_summary();
        assert!(summary.contains("No MCP servers configured"));
    }

    #[test]
    fn parses_sse_server_from_json() {
        let json = serde_json::json!({
            "mcpServers": {
                "remote-mcp": {
                    "type": "sse",
                    "url": "https://mcp.example.com/sse",
                    "headers": {"Authorization": "Bearer token"}
                }
            }
        });
        let servers = parse_mcp_servers_from_json(&json).expect("parse should succeed");
        assert_eq!(servers.len(), 1);
        match &servers[0].config {
            McpServerConfig::Sse(cfg) => {
                assert_eq!(cfg.url, "https://mcp.example.com/sse");
                assert_eq!(cfg.headers.get("Authorization"), Some(&"Bearer token".to_string()));
            }
            _ => panic!("expected SSE config"),
        }
    }

    #[test]
    fn managed_exclusive_blocks_non_managed_servers() {
        let mut assembler = McpRegistryAssembler::new();
        // Add a user-level server
        assembler.add_source(
            ConfigSource::User,
            vec![ScopedMcpServerConfig {
                scope: ConfigSource::User,
                config: McpServerConfig::Stdio(McpStdioServerConfig {
                    command: "user-tool".into(),
                    args: vec![],
                    env: BTreeMap::new(),
                    tool_call_timeout_ms: None,
                }),
            }],
        );
        // Add a managed-level server
        assembler.add_source(
            ConfigSource::Managed,
            vec![ScopedMcpServerConfig {
                scope: ConfigSource::Managed,
                config: McpServerConfig::Stdio(McpStdioServerConfig {
                    command: "managed-tool".into(),
                    args: vec![],
                    env: BTreeMap::new(),
                    tool_call_timeout_ms: None,
                }),
            }],
        );

        // Without managed-exclusive: both should be active
        let snapshot = assembler.clone().assemble();
        assert_eq!(snapshot.active_servers.len(), 2);

        // With managed-exclusive: only managed should be active
        let snapshot = assembler.with_managed_exclusive(true).assemble();
        assert_eq!(snapshot.active_servers.len(), 1);
        assert_eq!(snapshot.active_servers[0].config, McpServerConfig::Stdio(McpStdioServerConfig {
            command: "managed-tool".into(),
            args: vec![],
            env: BTreeMap::new(),
            tool_call_timeout_ms: None,
        }));
        assert_eq!(snapshot.blocked_servers.len(), 1);
        assert!(snapshot.blocked_servers[0].reason.contains("managed-exclusive"));
    }
}

// Clone impl for McpRegistryAssembler (needed for tests)
impl Clone for McpRegistryAssembler {
    fn clone(&self) -> Self {
        Self {
            sources: self.sources.clone(),
            policy: McpPolicy::new(), // policy doesn't need to be cloned for tests
            managed_exclusive: self.managed_exclusive,
        }
    }
}
