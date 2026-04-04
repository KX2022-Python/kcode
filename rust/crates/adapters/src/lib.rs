//! Channel adapters for Kcode Bridge.
//! Supports Telegram, WhatsApp, and Feishu (Lark).

pub mod transport;
pub use transport::{Transport, TransportConfig};

pub mod telegram_transport;
pub use telegram_transport::{
    TelegramConfig, TelegramMode, TelegramTransport,
    parse_telegram_webhook, TelegramWebhookUpdate,
};

pub mod whatsapp_transport;
pub use whatsapp_transport::{
    WhatsAppConfig, WhatsAppTransport,
    parse_whatsapp_webhook, verify_whatsapp_signature,
    WhatsAppWebhookPayload, WhatsAppMessage, WhatsAppStatus,
};

pub mod feishu_transport;
pub use feishu_transport::{
    FeishuConfig, FeishuTransport,
    parse_feishu_webhook,
    FeishuWebhookPayload, FeishuChallengeResponse,
};

pub mod session_router;
pub use session_router::{ChannelSession, SessionRouter};

pub mod webhook_server;
pub use webhook_server::{start_webhook_server, WebhookState};

pub mod media_download;
pub use media_download::{
    download_file, download_feishu_file, download_telegram_file, download_whatsapp_file,
    media_storage_dir,
};

pub mod config_validator;
pub use config_validator::{print_config_summary, validate_bridge_config, EnvError};

// Re-exports for convenience
pub use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
pub use bridge::DeliveryMode;
