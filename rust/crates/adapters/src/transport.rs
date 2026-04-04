//! Transport abstraction layer for channel adapters.
//! Defines the interface for receiving and sending messages to a channel.

use async_trait::async_trait;
use bridge::events::BridgeInboundEvent;
use bridge::events::BridgeOutboundEvent;
use std::error::Error;

/// Transport trait for channel communication.
/// Implementors handle the low-level network details (HTTP, WebSockets, etc.)
/// and convert them into bridge events.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start receiving messages from the channel.
    /// This method should run indefinitely or until a shutdown signal is received.
    async fn run(
        &self,
        on_message: Box<dyn Fn(BridgeInboundEvent) + Send + Sync + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send an outbound event back to the channel.
    async fn send(&self, event: &BridgeOutboundEvent) -> Result<(), Box<dyn Error + Send + Sync>>;
}
