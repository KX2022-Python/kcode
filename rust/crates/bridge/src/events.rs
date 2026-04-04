//! Bridge inbound/outbound event models.

use std::collections::BTreeMap;

use crate::attachment::AttachmentEnvelope;

/// Unified inbound event from any channel.
#[derive(Debug, Clone)]
pub struct BridgeInboundEvent {
    /// Unique event ID for dedup and tracing.
    pub bridge_event_id: String,
    /// Channel identifier: "telegram", "feishu", "whatsapp", "loopback".
    pub channel: String,
    /// User identifier on the channel.
    pub channel_user_id: String,
    /// Chat/group identifier on the channel.
    pub channel_chat_id: String,
    /// Platform-specific message ID.
    pub channel_message_id: String,
    /// Normalized text content.
    pub text: String,
    /// Normalized attachments (may be empty).
    pub attachments: Vec<AttachmentEnvelope>,
    /// When the message was received (unix timestamp ms).
    pub received_at: u64,
    /// Original message ID this replies to, if any.
    pub reply_to: Option<String>,
    /// Optional capability metadata from the channel.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl BridgeInboundEvent {
    pub fn new(
        bridge_event_id: String,
        channel: String,
        channel_user_id: String,
        channel_chat_id: String,
        channel_message_id: String,
        text: String,
    ) -> Self {
        Self {
            bridge_event_id,
            channel,
            channel_user_id,
            channel_chat_id,
            channel_message_id,
            text,
            attachments: Vec::new(),
            received_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            reply_to: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_attachments(mut self, attachments: Vec<AttachmentEnvelope>) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn with_reply_to(mut self, reply_to: String) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn is_slash_command(&self) -> bool {
        self.text.trim().starts_with('/')
    }

    pub fn command_name(&self) -> Option<String> {
        let trimmed = self.text.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        trimmed[1..].split_whitespace().next().map(String::from)
    }
}

/// Delivery mode for outbound events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryMode {
    /// Send as a single message.
    Single,
    /// Send as a streaming sequence of chunks.
    Stream,
    /// Send as a reply to a specific message.
    Reply { reply_to: String },
}

/// Unified outbound event to any channel.
#[derive(Debug, Clone)]
pub struct BridgeOutboundEvent {
    /// Correlated bridge event ID.
    pub bridge_event_id: String,
    /// Session this event belongs to.
    pub session_id: String,
    /// Semantic render items (role + text pairs).
    pub render_items: Vec<(String, String)>,
    /// How to deliver to the channel.
    pub delivery_mode: DeliveryMode,
    /// Channel capability hint for rendering decisions.
    pub channel_capability_hint: String,
    /// Target for reply, if applicable.
    pub reply_target: Option<String>,
}

impl BridgeOutboundEvent {
    pub fn new(
        bridge_event_id: String,
        session_id: String,
        channel_capability_hint: String,
    ) -> Self {
        Self {
            bridge_event_id,
            session_id,
            render_items: Vec::new(),
            delivery_mode: DeliveryMode::Single,
            channel_capability_hint,
            reply_target: None,
        }
    }

    pub fn with_render_item(mut self, role: String, text: String) -> Self {
        self.render_items.push((role, text));
        self
    }

    pub fn with_delivery_mode(mut self, mode: DeliveryMode) -> Self {
        self.delivery_mode = mode;
        self
    }

    pub fn with_reply_target(mut self, target: String) -> Self {
        self.reply_target = Some(target);
        self
    }

    /// Flatten render items into a single text block for simple channels.
    pub fn flattened_text(&self) -> String {
        self.render_items
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_slash_commands() {
        let event = BridgeInboundEvent::new(
            "e1".into(),
            "loopback".into(),
            "u1".into(),
            "c1".into(),
            "m1".into(),
            "/help".into(),
        );
        assert!(event.is_slash_command());
        assert_eq!(event.command_name(), Some("help".to_string()));
    }

    #[test]
    fn non_command_text_returns_none() {
        let event = BridgeInboundEvent::new(
            "e2".into(),
            "loopback".into(),
            "u1".into(),
            "c1".into(),
            "m2".into(),
            "hello".into(),
        );
        assert!(!event.is_slash_command());
        assert_eq!(event.command_name(), None);
    }

    #[test]
    fn outbound_event_flattens_render_items() {
        let outbound = BridgeOutboundEvent::new("e1".into(), "s1".into(), "plain".into())
            .with_render_item("assistant".into(), "Hello".into())
            .with_render_item("tool".into(), "Done".into());
        assert_eq!(outbound.flattened_text(), "Hello\nDone");
    }
}
