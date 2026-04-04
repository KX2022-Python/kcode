//! Bridge command policy — controls which commands are exposed to channels.

use std::collections::BTreeSet;

/// Policy controlling which commands are bridge-safe.
#[derive(Debug, Clone)]
pub struct BridgeCommandPolicy {
    /// Prompt commands allowed over bridge.
    pub allowed_prompt_commands: BTreeSet<String>,
    /// Local commands explicitly allowed over bridge.
    pub allowed_local_commands: BTreeSet<String>,
    /// Local-UI commands explicitly allowed over bridge (default: empty = all blocked).
    pub allowed_local_ui_commands: BTreeSet<String>,
    /// Per-channel overrides.
    pub channel_safe_overrides: BTreeMap<String, ChannelOverride>,
}

/// Per-channel command overrides.
#[derive(Debug, Clone, Default)]
pub struct ChannelOverride {
    pub extra_allowed: BTreeSet<String>,
    pub extra_blocked: BTreeSet<String>,
}

impl BridgeCommandPolicy {
    /// Default v1.0 bridge-safe policy.
    pub fn v1_default() -> Self {
        Self {
            allowed_prompt_commands: BTreeSet::from_iter([
                // All normal text input is prompt-level.
            ]),
            allowed_local_commands: BTreeSet::from_iter([
                "help".into(),
                "compact".into(),
                "memory".into(),
                "model".into(),
                "permissions".into(),
                "mcp".into(),
                "status".into(),
                "config".into(),
            ]),
            allowed_local_ui_commands: BTreeSet::new(),
            channel_safe_overrides: BTreeMap::new(),
        }
    }

    /// Check if a command is bridge-safe for a given channel.
    pub fn is_command_allowed(&self, command_name: &str, channel: &str) -> bool {
        // Check channel-specific blocks first
        if let Some(override_) = self.channel_safe_overrides.get(channel) {
            if override_.extra_blocked.contains(command_name) {
                return false;
            }
            if override_.extra_allowed.contains(command_name) {
                return true;
            }
        }

        // Default allowlists
        self.allowed_local_commands.contains(command_name)
            || self.allowed_local_ui_commands.contains(command_name)
    }

    /// Check if a command type is bridge-safe.
    pub fn is_command_type_allowed(&self, command_kind: &str) -> bool {
        match command_kind {
            "prompt" => true,    // prompt commands are bridge-safe by default
            "local" => true,     // filtered by allowed_local_commands
            "local-ui" => false, // blocked by default
            _ => false,
        }
    }

    /// Get the list of commands visible on the bridge surface.
    pub fn bridge_visible_commands(&self) -> BTreeSet<String> {
        let mut commands = BTreeSet::new();
        commands.extend(self.allowed_local_commands.iter().cloned());
        commands.extend(self.allowed_local_ui_commands.iter().cloned());
        commands
    }
}

/// Named command policy profiles for different channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPolicyProfile {
    /// Standard — use v1.0 defaults.
    Standard,
    /// Restricted — only /help and text prompts.
    Restricted,
    /// Full — allow all local commands (for testing).
    Full,
}

impl CommandPolicyProfile {
    pub fn to_policy(self) -> BridgeCommandPolicy {
        match self {
            Self::Standard => BridgeCommandPolicy::v1_default(),
            Self::Restricted => {
                let mut policy = BridgeCommandPolicy::v1_default();
                policy.allowed_local_commands = BTreeSet::from_iter(["help".into()]);
                policy
            }
            Self::Full => {
                let mut policy = BridgeCommandPolicy::v1_default();
                policy.allowed_local_commands = BTreeSet::from_iter([
                    "help".into(),
                    "compact".into(),
                    "memory".into(),
                    "model".into(),
                    "permissions".into(),
                    "mcp".into(),
                    "status".into(),
                    "config".into(),
                    "clear".into(),
                    "cost".into(),
                    "diff".into(),
                ]);
                policy
            }
        }
    }
}

use std::collections::BTreeMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_default_allows_expected_commands() {
        let policy = BridgeCommandPolicy::v1_default();
        assert!(policy.is_command_allowed("help", "loopback"));
        assert!(policy.is_command_allowed("memory", "loopback"));
        assert!(policy.is_command_allowed("compact", "loopback"));
        assert!(policy.is_command_allowed("mcp", "loopback"));
    }

    #[test]
    fn v1_default_blocks_local_ui() {
        let policy = BridgeCommandPolicy::v1_default();
        assert!(!policy.is_command_type_allowed("local-ui"));
    }

    #[test]
    fn channel_override_blocks_specific_command() {
        let mut policy = BridgeCommandPolicy::v1_default();
        policy.channel_safe_overrides.insert(
            "telegram".into(),
            ChannelOverride {
                extra_blocked: BTreeSet::from_iter(["mcp".into()]),
                extra_allowed: BTreeSet::new(),
            },
        );

        assert!(policy.is_command_allowed("mcp", "loopback"));
        assert!(!policy.is_command_allowed("mcp", "telegram"));
    }

    #[test]
    fn restricted_profile_only_allows_help() {
        let policy = CommandPolicyProfile::Restricted.to_policy();
        assert!(policy.is_command_allowed("help", "loopback"));
        assert!(!policy.is_command_allowed("compact", "loopback"));
        assert!(!policy.is_command_allowed("memory", "loopback"));
    }

    #[test]
    fn bridge_visible_commands_excludes_ui() {
        let policy = BridgeCommandPolicy::v1_default();
        let commands = policy.bridge_visible_commands();
        assert!(commands.contains("help"));
        assert!(commands.contains("memory"));
        assert!(!commands.is_empty());
    }
}
