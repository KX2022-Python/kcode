//! Lightweight Webhook HTTP Server for receiving channel events.
//! Supports WhatsApp and Feishu Webhook modes.
//! This module provides the server skeleton; actual routing is handled by the CLI.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
use tracing::info;

use crate::feishu_transport::FeishuConfig;
use crate::session_router::SessionRouter;
use crate::whatsapp_transport::WhatsAppConfig;

/// Webhook server configuration.
pub struct WebhookServerConfig {
    pub addr: SocketAddr,
    pub whatsapp: Option<WhatsAppConfig>,
    pub feishu: Option<FeishuConfig>,
}

/// Webhook handler type for processing inbound events.
pub type WebhookHandler = dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + Send + Sync;

/// Start the webhook server.
/// This is a placeholder for the actual axum-based implementation.
/// The CLI will use this to start the HTTP server for WhatsApp/Feishu webhooks.
pub async fn start_webhook_server(
    _config: WebhookServerConfig,
    _session_router: Arc<SessionRouter>,
    _handler: Box<WebhookHandler>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Webhook server placeholder - requires axum integration");
    // In production, this would start an axum server with routes:
    // - POST /webhook/whatsapp - WhatsApp Cloud API webhook
    // - GET  /webhook/whatsapp - WhatsApp verification
    // - POST /webhook/feishu   - Feishu event subscription
    // - GET  /webhook/feishu   - Feishu challenge verification
    Ok(())
}
