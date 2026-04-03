//! WhatsApp channel adapter for kcode-bridge.
//!
//! This adapter normalizes inbound WhatsApp messages into `BridgeInboundEvent`
//! and renders `BridgeOutboundEvent` into WhatsApp-formatted messages.
//!
//! Note: WhatsApp Business API requires Meta Cloud API access. This adapter
//! provides the normalization and rendering layer; actual HTTP transport
//! (webhook receiving, message sending via Meta API) is handled by the
//! bridge transport layer.

use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode};
use bridge::policy::BridgeCommandPolicy;
use bridge::session::{ChannelSessionKey, SessionMapping, SessionMappingMode};
use bridge::attachment::{AttachmentEnvelope, AttachmentKind};

/// WhatsApp-specific inbound message structure.
#[derive(Debug, Clone)]
pub struct WhatsAppMessage {
    pub wa_message_id: String,
    pub from_user_id: String,
    pub chat_id: String,
    pub text: String,
    pub reply_to_message_id: Option<String>,
}

/// WhatsApp adapter configuration.
#[derive(Debug, Clone)]
pub struct WhatsAppAdapterConfig {
    pub phone_number_id_env: String,
    pub access_token_env: String,
    pub webhook_verify_token_env: String,
    pub session_mapping_mode: SessionMappingMode,
}

impl Default for WhatsAppAdapterConfig {
    fn default() -> Self {
        Self {
            phone_number_id_env: "WA_PHONE_NUMBER_ID".into(),
            access_token_env: "WA_ACCESS_TOKEN".into(),
            webhook_verify_token_env: "WA_WEBHOOK_VERIFY_TOKEN".into(),
            session_mapping_mode: SessionMappingMode::OneToOne,
        }
    }
}

/// WhatsApp-specific outbound message.
#[derive(Debug, Clone)]
pub struct WhatsAppOutbound {
    pub to_user_id: String,
    pub text: String,
    pub context_msg_id: Option<String>,
}

/// WhatsApp channel adapter.
pub struct WhatsAppAdapter {
    config: WhatsAppAdapterConfig,
    session_mapping: SessionMapping,
    command_policy: BridgeCommandPolicy,
}

impl WhatsAppAdapter {
    pub fn new(config: WhatsAppAdapterConfig, command_policy: BridgeCommandPolicy) -> Self {
        Self {
            config: config.clone(),
            session_mapping: SessionMapping::new(config.session_mapping_mode),
            command_policy,
        }
    }

    /// Normalize a WhatsApp message into a `BridgeInboundEvent`.
    pub fn normalize_inbound(&self, msg: &WhatsAppMessage, received_at_ms: u64) -> BridgeInboundEvent {
        let mut event = BridgeInboundEvent::new(
            format!("wa-{}-{}", msg.chat_id, msg.wa_message_id),
            "whatsapp".into(),
            msg.from_user_id.clone(),
            msg.chat_id.clone(),
            msg.wa_message_id.clone(),
            msg.text.clone(),
        );
        event.received_at = received_at_ms;
        if let Some(ref reply) = msg.reply_to_message_id {
            event.reply_to = Some(reply.clone());
        }
        event
    }

    /// Render a `BridgeOutboundEvent` into a WhatsApp-formatted message.
    pub fn render_outbound(&self, outbound: &BridgeOutboundEvent, original_msg: Option<&WhatsAppMessage>) -> WhatsAppOutbound {
        let text = render_whatsapp_message(&outbound.render_items);
        let to_user_id = original_msg
            .map(|m| m.from_user_id.clone())
            .or_else(|| outbound.reply_target.clone())
            .unwrap_or_default();
        let context = original_msg.map(|m| m.wa_message_id.clone());

        WhatsAppOutbound {
            to_user_id,
            text,
            context_msg_id: context,
        }
    }

    pub fn session_key(&self, chat_id: &str, user_id: &str) -> ChannelSessionKey {
        ChannelSessionKey::new(
            "whatsapp".into(),
            "kcode-bot".into(),
            chat_id.into(),
            user_id.into(),
        )
    }

    pub fn resolve_session(&self, key: &ChannelSessionKey, creator: impl FnOnce() -> String) -> String {
        self.session_mapping.resolve_or_create(key, creator)
    }

    pub fn is_command_allowed(&self, command_name: &str) -> bool {
        self.command_policy.is_command_allowed(command_name, "whatsapp")
    }

    pub fn config(&self) -> &WhatsAppAdapterConfig {
        &self.config
    }
}

/// Render render items into a WhatsApp-formatted message.
/// WhatsApp supports *bold*, _italic_, ~strikethrough~, and ```code```.
pub fn render_whatsapp_message(render_items: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    let mut total_len = 0;
    const WHATSAPP_MAX_LEN: usize = 4000;

    for (role, text) in render_items {
        let formatted = match role.as_str() {
            "error" => format!("*Error:*\n{}", text),
            "warning" => format!("*Warning:* {}", text),
            "success" => format!("*Done:* {}", text),
            "tool" => format!("```{}```", text),
            "assistant" | "user" => text.clone(),
            "progress" => format!("*⟳ {}*", text),
            "compact" => format!("_{}_", text),
            "permission" => format!("*Permission:* {}", text),
            "diff" => format!("```diff\n{}\n```", text),
            "memory" => format!("_{}_", text),
            "system" => format!("_{}_", text),
            _ => text.clone(),
        };

        if total_len + formatted.len() + 2 > WHATSAPP_MAX_LEN && !parts.is_empty() {
            // This part would exceed the limit — it'll be a separate message
            total_len = formatted.len();
            parts.push(formatted);
        } else {
            total_len += formatted.len() + 2;
            parts.push(formatted);
        }
    }

    let joined = parts.join("\n\n");

    // If still too long, truncate with notice
    if joined.len() > WHATSAPP_MAX_LEN {
        format!("{}...\n\n_Message truncated._", &joined[..WHATSAPP_MAX_LEN - 30])
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge::policy::CommandPolicyProfile;

    #[test]
    fn normalizes_inbound_message() {
        let adapter = WhatsAppAdapter::new(
            WhatsAppAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let msg = WhatsAppMessage {
            wa_message_id: "wamid-001".into(),
            from_user_id: "1234567890".into(),
            chat_id: "1234567890".into(),
            text: "/help".into(),
            reply_to_message_id: None,
        };
        let event = adapter.normalize_inbound(&msg, 3000);
        assert_eq!(event.channel, "whatsapp");
        assert_eq!(event.text, "/help");
        assert!(event.is_slash_command());
    }

    #[test]
    fn renders_outbound_with_formatting() {
        let adapter = WhatsAppAdapter::new(
            WhatsAppAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let outbound = BridgeOutboundEvent::new("e1".into(), "s1".into(), "plain".into())
            .with_render_item("tool".into(), "bash".into())
            .with_render_item("assistant".into(), "Hello!".into());
        let wa_msg = adapter.render_outbound(&outbound, None);
        assert!(wa_msg.text.contains("```bash```"));
        assert!(wa_msg.text.contains("Hello!"));
    }

    #[test]
    fn command_policy_applied_for_whatsapp() {
        let adapter = WhatsAppAdapter::new(
            WhatsAppAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        assert!(adapter.is_command_allowed("help"));
        assert!(adapter.is_command_allowed("mcp"));
        assert!(!adapter.is_command_allowed("vim"));
    }

    #[test]
    fn long_messages_are_truncated() {
        let long_text = "x".repeat(5000);
        let items = vec![("assistant".into(), long_text)];
        let rendered = render_whatsapp_message(&items);
        assert!(rendered.len() <= 4000);
        assert!(rendered.contains("truncated"));
    }
}
