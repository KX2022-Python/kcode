//! Transport abstraction layer for channel adapters.
//! Defines the interface for receiving and sending messages to a channel.
//! Supports both Long Polling and Webhook modes.

use async_trait::async_trait;
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
use std::error::Error;

/// Configuration for a channel transport.
pub trait TransportConfig: Send + Sync + 'static {
    /// Unique channel identifier (e.g., "telegram", "whatsapp", "feishu").
    fn channel_id(&self) -> &str;
}

/// Unified transport trait for all channels.
/// Implementors handle low-level network details and convert them to bridge events.
#[async_trait(?Send)]
pub trait Transport: Send + Sync {
    /// Start the transport message loop.
    /// In Long Polling mode, this runs indefinitely.
    /// In Webhook mode, this starts an HTTP server and blocks until shutdown.
    ///
    /// The `handler` callback processes each inbound event and returns an outbound event.
    async fn run(
        &self,
        handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send an outbound event to the channel.
    /// This is called by the handler's return value to deliver responses.
    async fn send_outbound(
        &self,
        event: &BridgeOutboundEvent,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
