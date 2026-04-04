//! Channel adapters for Kcode Bridge.
//! Supports Telegram, WhatsApp, and Feishu (Lark).

pub mod transport;
pub use transport::{Transport, TransportConfig};

pub mod telegram_transport;
pub use telegram_transport::{
    parse_telegram_webhook, TelegramConfig, TelegramMode, TelegramTransport, TelegramWebhookUpdate,
};

pub mod whatsapp_transport;
pub use whatsapp_transport::{
    parse_whatsapp_webhook, verify_whatsapp_signature, WhatsAppConfig, WhatsAppMessage,
    WhatsAppStatus, WhatsAppTransport, WhatsAppWebhookPayload,
};

pub mod feishu_transport;
pub use feishu_transport::{
    parse_feishu_webhook, FeishuChallengeResponse, FeishuConfig, FeishuTransport,
    FeishuWebhookPayload,
};

pub mod session_router;
pub use session_router::{ChannelSession, SessionRouter};

pub mod webhook_server;
pub use webhook_server::{start_webhook_server, WebhookState};

pub mod media_download;
pub use media_download::{
    download_feishu_file, download_file, download_telegram_file, download_whatsapp_file,
    media_storage_dir,
};

pub mod config_validator;
pub use config_validator::{print_config_summary, validate_bridge_config, EnvError};

pub mod bridge_env;
pub use bridge_env::{
    apply_bridge_env_defaults_to_process, bridge_env_path, known_bridge_keys,
    load_bridge_env_snapshot, write_bridge_env_file, BridgeEnvSnapshot,
};

// Re-exports for convenience
pub use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
pub use bridge::DeliveryMode;
