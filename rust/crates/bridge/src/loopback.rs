//! Loopback adapter — simulates a channel for local verification.
//!
//! This adapter runs entirely in-process, sending messages through the bridge
//! and receiving responses. It verifies that:
//! - Inbound messages are normalized correctly
//! - Session mapping works
//! - Command filtering applies bridge-safe policy
//! - Outbound events are produced with correct semantics

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::events::{BridgeInboundEvent, BridgeOutboundEvent};
#[cfg(test)]
use crate::events::DeliveryMode;
use crate::policy::BridgeCommandPolicy;
use crate::session::{ChannelSessionKey, SessionMapping, SessionMappingMode};

/// Configuration for the loopback adapter.
#[derive(Debug, Clone)]
pub struct LoopbackConfig {
    /// Simulated user ID.
    pub user_id: String,
    /// Simulated chat ID.
    pub chat_id: String,
    /// Session mapping mode.
    pub session_mapping_mode: SessionMappingMode,
}

impl Default for LoopbackConfig {
    fn default() -> Self {
        Self {
            user_id: "loopback-user".into(),
            chat_id: "loopback-chat".into(),
            session_mapping_mode: SessionMappingMode::OneToOne,
        }
    }
}

/// A message sent through the loopback adapter.
#[derive(Debug, Clone)]
pub struct LoopbackMessage {
    pub text: String,
    pub event_counter: u64,
}

/// Loopback adapter — bridges simulated channel messages through the bridge layer.
pub struct LoopbackAdapter {
    config: LoopbackConfig,
    session_mapping: SessionMapping,
    policy: BridgeCommandPolicy,
    event_counter: Mutex<u64>,
    /// Queue of inbound events ready for processing.
    inbound_queue: Mutex<VecDeque<BridgeInboundEvent>>,
    /// Queue of outbound events ready for delivery.
    outbound_queue: Mutex<VecDeque<BridgeOutboundEvent>>,
}

impl LoopbackAdapter {
    pub fn new(config: LoopbackConfig, policy: BridgeCommandPolicy) -> Self {
        Self {
            config: config.clone(),
            session_mapping: SessionMapping::new(config.session_mapping_mode),
            policy,
            event_counter: Mutex::new(0),
            inbound_queue: Mutex::new(VecDeque::new()),
            outbound_queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Send a message into the bridge (simulates user sending a message).
    pub fn send_inbound(&self, text: String) -> Result<String, String> {
        let mut counter = self.event_counter.lock().expect("counter poisoned");
        *counter += 1;
        let event_id = format!("loopback-{}", *counter);
        drop(counter);

        let key = ChannelSessionKey::new(
            "loopback".into(),
            "kcode-bot".into(),
            self.config.chat_id.clone(),
            self.config.user_id.clone(),
        );

        let inbound = BridgeInboundEvent::new(
            event_id.clone(),
            "loopback".into(),
            self.config.user_id.clone(),
            self.config.chat_id.clone(),
            event_id.clone(),
            text,
        );

        // Apply command policy filter
        if inbound.is_slash_command() {
            if let Some(cmd) = inbound.command_name() {
                if !self.policy.is_command_allowed(&cmd, "loopback") {
                    // Generate a blocked response
                    let outbound = BridgeOutboundEvent::new(
                        event_id.clone(),
                        key.as_session_id(),
                        "plain".into(),
                    )
                    .with_render_item(
                        "error".into(),
                        format!("Command /{cmd} is not available on this channel."),
                    );
                    self.outbound_queue
                        .lock()
                        .expect("outbound queue poisoned")
                        .push_back(outbound);
                    return Ok(event_id);
                }
            }
        }

        self.inbound_queue
            .lock()
            .expect("inbound queue poisoned")
            .push_back(inbound);

        Ok(event_id)
    }

    /// Take the next inbound event for processing.
    pub fn take_inbound(&self) -> Option<BridgeInboundEvent> {
        self.inbound_queue
            .lock()
            .expect("inbound queue poisoned")
            .pop_front()
    }

    /// Submit a response to an inbound event (simulates engine reply).
    pub fn submit_outbound(&self, outbound: BridgeOutboundEvent) {
        self.outbound_queue
            .lock()
            .expect("outbound queue poisoned")
            .push_back(outbound);
    }

    /// Take the next outbound event (simulates delivery to user).
    pub fn take_outbound(&self) -> Option<BridgeOutboundEvent> {
        self.outbound_queue
            .lock()
            .expect("outbound queue poisoned")
            .pop_front()
    }

    /// Get the session ID for the current loopback user.
    pub fn session_id(&self) -> String {
        let key = ChannelSessionKey::new(
            "loopback".into(),
            "kcode-bot".into(),
            self.config.chat_id.clone(),
            self.config.user_id.clone(),
        );
        self.session_mapping
            .resolve_or_create(&key, || format!("session-{}", key.as_session_id()))
    }

    /// Count pending outbound messages.
    pub fn outbound_pending(&self) -> usize {
        self.outbound_queue.lock().expect("outbound queue poisoned").len()
    }

    /// Count pending inbound messages.
    pub fn inbound_pending(&self) -> usize {
        self.inbound_queue.lock().expect("inbound queue poisoned").len()
    }

    /// Reset all queues and counters (for testing).
    pub fn reset(&self) {
        *self.event_counter.lock().expect("counter poisoned") = 0;
        self.inbound_queue.lock().expect("inbound queue poisoned").clear();
        self.outbound_queue.lock().expect("outbound queue poisoned").clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::CommandPolicyProfile;

    #[test]
    fn loopback_sends_and_receives_messages() {
        let adapter = LoopbackAdapter::new(
            LoopbackConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );

        let event_id = adapter.send_inbound("hello".into()).unwrap();
        assert!(event_id.starts_with("loopback-"));

        let inbound = adapter.take_inbound().expect("should have inbound");
        assert_eq!(inbound.text, "hello");
        assert_eq!(inbound.channel, "loopback");
    }

    #[test]
    fn loopback_blocks_unsafe_commands() {
        let adapter = LoopbackAdapter::new(
            LoopbackConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );

        // /help is allowed
        adapter.send_inbound("/help".into()).unwrap();
        // /help should NOT generate an error outbound
        assert_eq!(adapter.outbound_pending(), 0);

        // /vim is not in the allowlist
        adapter.send_inbound("/vim".into()).unwrap();
        // Should generate an error outbound
        let outbound = adapter.take_outbound().expect("should have error");
        let text = outbound.flattened_text();
        assert!(text.contains("/vim"));
        assert!(text.contains("not available"));
    }

    #[test]
    fn loopback_session_id_is_stable() {
        let adapter = LoopbackAdapter::new(
            LoopbackConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );

        let s1 = adapter.session_id();
        let s2 = adapter.session_id();
        assert_eq!(s1, s2);
    }

    #[test]
    fn loopback_outbound_render_items() {
        let adapter = LoopbackAdapter::new(
            LoopbackConfig::default(),
            CommandPolicyProfile::Standard.to_policy(),
        );

        let inbound = adapter.send_inbound("test".into()).unwrap();
        let _ = adapter.take_inbound();

        let outbound = BridgeOutboundEvent::new(
            inbound,
            adapter.session_id(),
            "markdown".into(),
        )
        .with_render_item("assistant".into(), "Hello!".into())
        .with_delivery_mode(DeliveryMode::Reply {
            reply_to: "msg-1".into(),
        });

        adapter.submit_outbound(outbound);
        let delivered = adapter.take_outbound().expect("should deliver");
        assert_eq!(delivered.flattened_text(), "Hello!");
        assert!(matches!(delivered.delivery_mode, DeliveryMode::Reply { .. }));
    }
}
