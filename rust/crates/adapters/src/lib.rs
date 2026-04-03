//! Channel adapter implementations for kcode-bridge.
//!
//! Each adapter normalizes inbound channel messages into `BridgeInboundEvent`
//! and renders `BridgeOutboundEvent` into channel-specific formats.
//! Adapters do NOT own session state — they only consume the bridge abstraction.

pub mod telegram;
pub mod feishu;
pub mod whatsapp;

pub use telegram::{TelegramAdapter, TelegramAdapterConfig, TelegramMessage, TelegramOutbound};
pub use feishu::{FeishuAdapter, FeishuAdapterConfig, FeishuMessage, FeishuOutbound};
pub use whatsapp::{WhatsAppAdapter, WhatsAppAdapterConfig, WhatsAppMessage, WhatsAppOutbound};
