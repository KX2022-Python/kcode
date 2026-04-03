//! Telegram channel adapter for kcode-bridge.
//!
//! This adapter normalizes inbound Telegram messages into `BridgeInboundEvent`
//! and renders `BridgeOutboundEvent` into Telegram-formatted messages (MarkdownV2).

use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode};
use bridge::policy::BridgeCommandPolicy;
use bridge::session::{ChannelSessionKey, SessionMapping, SessionMappingMode};
use bridge::attachment::{AttachmentEnvelope, AttachmentKind};

/// Telegram-specific inbound message structure.
#[derive(Debug, Clone)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub from_user_id: String,
    pub chat_id: String,
    pub text: String,
    pub reply_to_message_id: Option<i64>,
}

/// Telegram adapter configuration.
#[derive(Debug, Clone)]
pub struct TelegramAdapterConfig {
    pub bot_token_env: String,
    pub session_mapping_mode: SessionMappingMode,
}

impl Default for TelegramAdapterConfig {
    fn default() -> Self {
        Self {
            bot_token_env: "TELEGRAM_BOT_TOKEN".into(),
            session_mapping_mode: SessionMappingMode::OneToOne,
        }
    }
}

/// Telegram-specific outbound message.
#[derive(Debug, Clone)]
pub struct TelegramOutbound {
    pub chat_id: String,
    pub text: String,
    pub parse_mode: Option<String>,
    pub reply_to_message_id: Option<i64>,
}

/// Telegram channel adapter — normalizes inbound messages and renders outbound events.
pub struct TelegramAdapter {
    config: TelegramAdapterConfig,
    session_mapping: SessionMapping,
    command_policy: BridgeCommandPolicy,
}

impl TelegramAdapter {
    pub fn new(config: TelegramAdapterConfig, command_policy: BridgeCommandPolicy) -> Self {
        Self {
            config: config.clone(),
            session_mapping: SessionMapping::new(config.session_mapping_mode),
            command_policy,
        }
    }

    /// Normalize a Telegram message into a `BridgeInboundEvent`.
    pub fn normalize_inbound(&self, msg: &TelegramMessage, received_at_ms: u64) -> BridgeInboundEvent {
        let key = self.session_key(&msg.chat_id, &msg.from_user_id);
        let mut event = BridgeInboundEvent::new(
            format!("tg-{}-{}", msg.chat_id, msg.message_id),
            "telegram".into(),
            msg.from_user_id.clone(),
            msg.chat_id.clone(),
            msg.message_id.to_string(),
            msg.text.clone(),
        );
        event.received_at = received_at_ms;
        if let Some(reply_id) = msg.reply_to_message_id {
            event.reply_to = Some(reply_id.to_string());
        }
        event
    }

    /// Render a `BridgeOutboundEvent` into a Telegram-formatted message.
    pub fn render_outbound(&self, outbound: &BridgeOutboundEvent, original_msg: Option<&TelegramMessage>) -> TelegramOutbound {
        let text = render_telegram_message(&outbound.render_items);
        let chat_id = original_msg
            .map(|m| m.chat_id.clone())
            .or_else(|| outbound.reply_target.clone())
            .unwrap_or_default();
        let reply_to = original_msg.and_then(|m| m.reply_to_message_id);

        TelegramOutbound {
            chat_id,
            text,
            parse_mode: Some("MarkdownV2".into()),
            reply_to_message_id: reply_to,
        }
    }

    pub fn session_key(&self, chat_id: &str, user_id: &str) -> ChannelSessionKey {
        ChannelSessionKey::new(
            "telegram".into(),
            "kcode-bot".into(),
            chat_id.into(),
            user_id.into(),
        )
    }

    pub fn resolve_session(&self, key: &ChannelSessionKey, creator: impl FnOnce() -> String) -> String {
        self.session_mapping.resolve_or_create(key, creator)
    }

    pub fn is_command_allowed(&self, command_name: &str) -> bool {
        self.command_policy.is_command_allowed(command_name, "telegram")
    }

    pub fn config(&self) -> &TelegramAdapterConfig {
        &self.config
    }
}

/// Render render items into Telegram MarkdownV2 format.
pub fn render_telegram_message(render_items: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    for (role, text) in render_items {
        let formatted = match role.as_str() {
            "error" => format!("*Error:*\\n{}", escape_md(text)),
            "warning" => format!("*Warning:* {}", escape_md(text)),
            "success" => format!("*✓ Done:* {}", escape_md(text)),
            "tool" => format!("`{}`", escape_md_inline(text)),
            "assistant" | "user" => escape_md(text),
            "progress" => format!("*⟳ {}*", escape_md(text)),
            "compact" => format!("_{}_ ", escape_md(text)),
            "permission" => format!("*Permission:* {}", escape_md(text)),
            "diff" => format!("```\n{}\n```", escape_md_code(text)),
            "memory" => format!("_{}_ ", escape_md(text)),
            "system" => format!("_{}_ ", escape_md(text)),
            _ => escape_md(text),
        };
        parts.push(formatted);
    }

    let joined = parts.join("\n\n");

    // Split if exceeds Telegram's 4096 char limit
    if joined.len() > 4000 {
        let mut chunks = Vec::new();
        let mut current = String::new();
        for part in parts {
            if current.len() + part.len() + 2 > 4000 && !current.is_empty() {
                chunks.push(current);
                current = String::new();
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(&part);
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        chunks.join("\n\n---\n\n")
    } else {
        joined
    }
}

/// Escape text for Telegram MarkdownV2.
fn escape_md(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        match ch {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn escape_md_inline(text: &str) -> String {
    text.replace('`', "\\`")
}

fn escape_md_code(text: &str) -> String {
    text.replace("```", "\\`\\`\\`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge::policy::CommandPolicyProfile;

    #[test]
    fn normalizes_inbound_message() {
        let adapter = TelegramAdapter::new(
            TelegramAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let msg = TelegramMessage {
            message_id: 42,
            from_user_id: "12345".into(),
            chat_id: "67890".into(),
            text: "/help".into(),
            reply_to_message_id: None,
        };
        let event = adapter.normalize_inbound(&msg, 1000);
        assert_eq!(event.channel, "telegram");
        assert_eq!(event.text, "/help");
        assert!(event.is_slash_command());
    }

    #[test]
    fn renders_outbound_with_markdown() {
        let adapter = TelegramAdapter::new(
            TelegramAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let outbound = BridgeOutboundEvent::new("e1".into(), "s1".into(), "markdown".into())
            .with_render_item("assistant".into(), "Hello world".into())
            .with_render_item("tool".into(), "bash".into());
        let tg_msg = adapter.render_outbound(&outbound, None);
        // MarkdownV2 escapes backticks in tool names
        assert!(tg_msg.text.contains("Hello world"));
        assert!(tg_msg.text.contains("bash"));
        assert_eq!(tg_msg.parse_mode, Some("MarkdownV2".into()));
    }

    #[test]
    fn command_policy_applied_for_telegram() {
        let adapter = TelegramAdapter::new(
            TelegramAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        assert!(adapter.is_command_allowed("help"));
        assert!(adapter.is_command_allowed("memory"));
        assert!(!adapter.is_command_allowed("vim"));
    }

    #[test]
    fn long_messages_are_split() {
        let long_text = "x".repeat(3000);
        let items = vec![("assistant".into(), long_text.clone()), ("assistant".into(), long_text)];
        let rendered = render_telegram_message(&items);
        assert!(rendered.contains("---"));
    }
}
