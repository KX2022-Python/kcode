//! Transport abstraction layer for channel adapters.
//! Defines the interface for receiving and sending messages to a channel.

use async_trait::async_trait;
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
use std::error::Error;

/// Transport trait for channel communication.
/// Implementors handle the low-level network details (HTTP, WebSockets, etc.)
/// and convert them into bridge events.
#[async_trait(?Send)]
pub trait Transport: Send + Sync {
    /// Start receiving messages from the channel.
    /// This method runs the message loop indefinitely.
    /// The `handler` callback is called for each inbound message and should return
    /// an outbound event to be sent back to the channel.
    async fn run(
        &self,
        handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
