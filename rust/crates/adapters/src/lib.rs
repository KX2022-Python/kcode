//! Channel adapters for Kcode Bridge.
//! Supports Telegram, WhatsApp, and Feishu (Lark).

pub mod transport;
pub use transport::{Transport, TransportConfig};

pub mod telegram_transport;
pub use telegram_transport::{TelegramConfig, TelegramMode, TelegramTransport};

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
pub use webhook_server::{WebhookServerConfig, WebhookHandler, start_webhook_server};
