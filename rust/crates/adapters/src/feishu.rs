//! Feishu (Lark) channel adapter for kcode-bridge.
//!
//! This adapter normalizes inbound Feishu messages into `BridgeInboundEvent`
//! and renders `BridgeOutboundEvent` into Feishu-formatted messages (interactive cards
//! or plain text).

use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode};
use bridge::policy::BridgeCommandPolicy;
use bridge::session::{ChannelSessionKey, SessionMapping, SessionMappingMode};
use bridge::attachment::{AttachmentEnvelope, AttachmentKind};

/// Feishu-specific inbound message structure.
#[derive(Debug, Clone)]
pub struct FeishuMessage {
    pub message_id: String,
    pub open_chat_id: String,
    pub from_user_id: String,
    pub text: String,
    pub message_type: String,
    pub parent_id: Option<String>,
}

/// Feishu adapter configuration.
#[derive(Debug, Clone)]
pub struct FeishuAdapterConfig {
    pub app_id_env: String,
    pub app_secret_env: String,
    pub verification_token_env: String,
    pub session_mapping_mode: SessionMappingMode,
}

impl Default for FeishuAdapterConfig {
    fn default() -> Self {
        Self {
            app_id_env: "FEISHU_APP_ID".into(),
            app_secret_env: "FEISHU_APP_SECRET".into(),
            verification_token_env: "FEISHU_VERIFICATION_TOKEN".into(),
            session_mapping_mode: SessionMappingMode::OneToOne,
        }
    }
}

/// Feishu-specific outbound message.
#[derive(Debug, Clone)]
pub struct FeishuOutbound {
    pub receive_id: String,
    pub receive_id_type: String,
    pub msg_type: String,
    pub content: String,
    pub reply_in_chat: bool,
}

/// Feishu channel adapter.
pub struct FeishuAdapter {
    config: FeishuAdapterConfig,
    session_mapping: SessionMapping,
    command_policy: BridgeCommandPolicy,
}

impl FeishuAdapter {
    pub fn new(config: FeishuAdapterConfig, command_policy: BridgeCommandPolicy) -> Self {
        Self {
            config: config.clone(),
            session_mapping: SessionMapping::new(config.session_mapping_mode),
            command_policy,
        }
    }

    /// Normalize a Feishu message into a `BridgeInboundEvent`.
    pub fn normalize_inbound(&self, msg: &FeishuMessage, received_at_ms: u64) -> BridgeInboundEvent {
        let mut event = BridgeInboundEvent::new(
            format!("fs-{}-{}", msg.open_chat_id, msg.message_id),
            "feishu".into(),
            msg.from_user_id.clone(),
            msg.open_chat_id.clone(),
            msg.message_id.clone(),
            msg.text.clone(),
        );
        event.received_at = received_at_ms;
        if let Some(ref parent) = msg.parent_id {
            event.reply_to = Some(parent.clone());
        }
        event
    }

    /// Render a `BridgeOutboundEvent` into a Feishu message payload.
    pub fn render_outbound(&self, outbound: &BridgeOutboundEvent, original_msg: Option<&FeishuMessage>) -> FeishuOutbound {
        let receive_id = original_msg
            .map(|m| m.open_chat_id.clone())
            .or_else(|| outbound.reply_target.clone())
            .unwrap_or_default();

        // For short responses, use plain text; for longer/multi-part, use interactive card
        let flattened = outbound.flattened_text();
        if flattened.len() < 200 && outbound.render_items.len() <= 2 {
            FeishuOutbound {
                receive_id,
                receive_id_type: "chat_id".into(),
                msg_type: "text".into(),
                content: serde_json::json!({"text": flattened}).to_string(),
                reply_in_chat: true,
            }
        } else {
            let card_content = render_feishu_card(&outbound.render_items);
            FeishuOutbound {
                receive_id,
                receive_id_type: "chat_id".into(),
                msg_type: "interactive".into(),
                content: card_content,
                reply_in_chat: true,
            }
        }
    }

    pub fn session_key(&self, chat_id: &str, user_id: &str) -> ChannelSessionKey {
        ChannelSessionKey::new(
            "feishu".into(),
            "kcode-bot".into(),
            chat_id.into(),
            user_id.into(),
        )
    }

    pub fn resolve_session(&self, key: &ChannelSessionKey, creator: impl FnOnce() -> String) -> String {
        self.session_mapping.resolve_or_create(key, creator)
    }

    pub fn is_command_allowed(&self, command_name: &str) -> bool {
        self.command_policy.is_command_allowed(command_name, "feishu")
    }

    pub fn config(&self) -> &FeishuAdapterConfig {
        &self.config
    }
}

/// Render render items into a Feishu interactive card JSON string.
pub fn render_feishu_card(render_items: &[(String, String)]) -> String {
    let mut elements = Vec::new();
    for (role, text) in render_items {
        let (tag, content) = match role.as_str() {
            "error" => ("markdown", format!("**❌ Error**\n{}", escape_feishu_md(text))),
            "warning" => ("markdown", format!("**⚠️ Warning**\n{}", escape_feishu_md(text))),
            "success" => ("markdown", format!("**✅ Done**\n{}", escape_feishu_md(text))),
            "tool" => ("markdown", format!("`{}`", escape_feishu_inline(text))),
            "assistant" => ("markdown", escape_feishu_md(text)),
            "progress" => ("markdown", format!("**⟳ {}**", escape_feishu_md(text))),
            "compact" => ("markdown", format!("*{}*", escape_feishu_md(text))),
            "permission" => ("markdown", format!("**🔒 Permission**\n{}", escape_feishu_md(text))),
            "diff" => ("markdown", format!("```diff\n{}\n```", text)),
            "memory" => ("markdown", format!("*{}*", escape_feishu_md(text))),
            "system" => ("markdown", format!("*{}*", escape_feishu_md(text))),
            _ => ("markdown", escape_feishu_md(text)),
        };
        elements.push(serde_json::json!({
            "tag": tag,
            "content": content,
        }));
    }

    let card = serde_json::json!({
        "config": {
            "wide_screen_mode": true
        },
        "header": {
            "title": {
                "tag": "plain_text",
                "content": "Kcode"
            },
            "template": "blue"
        },
        "elements": elements
    });

    card.to_string()
}

fn escape_feishu_md(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn escape_feishu_inline(text: &str) -> String {
    text.replace('`', "\\`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge::policy::CommandPolicyProfile;

    #[test]
    fn normalizes_inbound_message() {
        let adapter = FeishuAdapter::new(
            FeishuAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let msg = FeishuMessage {
            message_id: "msg-001".into(),
            open_chat_id: "oc-abc".into(),
            from_user_id: "ou-123".into(),
            text: "/help".into(),
            message_type: "text".into(),
            parent_id: None,
        };
        let event = adapter.normalize_inbound(&msg, 2000);
        assert_eq!(event.channel, "feishu");
        assert_eq!(event.text, "/help");
        assert!(event.is_slash_command());
    }

    #[test]
    fn renders_outbound_as_text_for_short_response() {
        let adapter = FeishuAdapter::new(
            FeishuAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let outbound = BridgeOutboundEvent::new("e1".into(), "s1".into(), "plain".into())
            .with_render_item("assistant".into(), "OK".into());
        let fs_msg = adapter.render_outbound(&outbound, None);
        assert_eq!(fs_msg.msg_type, "text");
        assert!(fs_msg.content.contains("OK"));
    }

    #[test]
    fn renders_outbound_as_card_for_long_response() {
        let adapter = FeishuAdapter::new(
            FeishuAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        let long_text = "x".repeat(300);
        let outbound = BridgeOutboundEvent::new("e1".into(), "s1".into(), "plain".into())
            .with_render_item("assistant".into(), long_text)
            .with_render_item("tool".into(), "bash".into());
        let fs_msg = adapter.render_outbound(&outbound, None);
        assert_eq!(fs_msg.msg_type, "interactive");
        let parsed: serde_json::Value = serde_json::from_str(&fs_msg.content).unwrap();
        assert!(parsed["header"]["title"]["content"].as_str().unwrap() == "Kcode");
    }

    #[test]
    fn command_policy_applied_for_feishu() {
        let adapter = FeishuAdapter::new(
            FeishuAdapterConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );
        assert!(adapter.is_command_allowed("help"));
        assert!(adapter.is_command_allowed("compact"));
        assert!(!adapter.is_command_allowed("vim"));
    }
}
