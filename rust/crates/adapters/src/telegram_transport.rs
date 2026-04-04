//! Telegram Transport implementation.
//! Supports Long Polling and Webhook modes.
//! Handles MarkdownV2 formatting and 4096 char message splitting.

use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode};
use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info};

use super::transport::{Transport, TransportConfig};

/// Telegram Bot API configuration.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub mode: TelegramMode,
}

#[derive(Debug, Clone)]
pub enum TelegramMode {
    /// Long Polling mode (default, no server required).
    Polling { timeout: u32 },
    /// Webhook mode (requires public HTTPS endpoint).
    Webhook { url: String, port: u16 },
}

impl TransportConfig for TelegramConfig {
    fn channel_id(&self) -> &str { "telegram" }
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            mode: TelegramMode::Polling { timeout: 30 },
        }
    }
}

/// Characters that must be escaped in MarkdownV2.
const MARKDOWN_V2_ESCAPE_CHARS: &[char] = &[
    '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
];

/// Escape text for Telegram MarkdownV2.
fn escape_markdown_v2(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if MARKDOWN_V2_ESCAPE_CHARS.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

/// Telegram Transport handling Long Polling or Webhook.
pub struct TelegramTransport {
    config: TelegramConfig,
    client: Client,
    offset: std::sync::atomic::AtomicI64,
}

impl TelegramTransport {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            offset: std::sync::atomic::AtomicI64::new(0),
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.config.bot_token, method)
    }

    /// Send a text message to Telegram, auto-splitting if > 4096 chars.
    async fn send_text(&self, chat_id: &str, text: &str, reply_to: Option<&str>) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Telegram max message length is 4096 UTF-8 characters
        const MAX_LEN: usize = 4096;
        let escaped = escape_markdown_v2(text);
        
        // Split into chunks if needed
        let chunks: Vec<String> = if escaped.len() <= MAX_LEN {
            vec![escaped]
        } else {
            escaped
                .as_bytes()
                .chunks(MAX_LEN)
                .map(|chunk| String::from_utf8_lossy(chunk).to_string())
                .collect()
        };

        for (i, chunk) in chunks.iter().enumerate() {
            let url = self.api_url("sendMessage");
            let mut body = HashMap::new();
            body.insert("chat_id", chat_id.to_string());
            body.insert("text", chunk.clone());
            body.insert("parse_mode", "MarkdownV2".to_string());
            if i == 0 {
                if let Some(reply_to_id) = reply_to {
                    body.insert("reply_to_message_id", reply_to_id.to_string());
                }
            }

            let resp = self.client.post(&url).json(&body).send().await?;
            let status = resp.status();
            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!("Telegram send failed ({}): {}", status, body_text).into());
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Transport for TelegramTransport {
    async fn run(
        &self,
        handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + Send + Sync + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match &self.config.mode {
            TelegramMode::Polling { timeout } => {
                self.run_polling(handler, *timeout).await
            }
            TelegramMode::Webhook { url: _, port } => {
                // Webhook mode requires a separate HTTP server (not implemented here)
                Err(format!("Webhook mode requires external HTTP server on port {}", port).into())
            }
        }
    }

    async fn send_outbound(
        &self,
        event: &BridgeOutboundEvent,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let chat_id = event.reply_target.as_ref().ok_or("Missing reply_target (chat_id)")?;
        
        // Extract text from render items
        let text = event.render_items.iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        // Handle reply mode
        let reply_to = match &event.delivery_mode {
            DeliveryMode::Reply { reply_to } => Some(reply_to.as_str()),
            _ => None,
        };

        self.send_text(chat_id, &text, reply_to).await
    }
}

impl TelegramTransport {
    async fn run_polling(
        &self,
        handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + Send + Sync + 'static>,
        timeout: u32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Starting Telegram Long Polling transport (timeout={}s)...", timeout);

        loop {
            let offset = self.offset.load(std::sync::atomic::Ordering::SeqCst);
            let url = self.api_url("getUpdates");

            let mut params = HashMap::new();
            params.insert("offset", offset.to_string());
            params.insert("timeout", timeout.to_string());
            // Serialize allowed_updates as a JSON array string
            params.insert("allowed_updates", serde_json::json!(["message"]).to_string());

            let resp = match self.client.post(&url).json(&params).send().await {
                Ok(r) => r,
                Err(e) => {
                    error!("Telegram getUpdates failed: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            if !resp.status().is_success() {
                error!("Telegram API error: {}", resp.status());
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }

            let result: TelegramResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to parse Telegram response: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            if !result.ok {
                error!("Telegram returned ok=false: {:?}", result.description);
                continue;
            }

            for update in result.result {
                if let Some(message) = update.message {
                    if let Some(text) = message.text {
                        let chat_id = message.chat.id.to_string();
                        let user_id = message.from.map(|u| u.id.to_string()).unwrap_or_else(|| "unknown".to_string());
                        let reply_to = message.reply_to_message.as_ref().map(|m| m.message_id.to_string());

                        let event = BridgeInboundEvent {
                            bridge_event_id: format!("tg-{}-{}", chat_id, update.update_id),
                            channel: "telegram".to_string(),
                            channel_user_id: user_id,
                            channel_chat_id: chat_id.clone(),
                            channel_message_id: update.update_id.to_string(),
                            text,
                            attachments: vec![],
                            received_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0),
                            reply_to,
                            metadata: std::collections::BTreeMap::new(),
                        };

                        let outbound = handler(event);
                        
                        // Send response back to Telegram
                        if let Err(e) = self.send_outbound(&outbound).await {
                            error!("Failed to send outbound to Telegram: {}", e);
                        }
                    }
                }

                // Update offset for next poll
                self.offset.store(update.update_id + 1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

// Telegram API Types
#[derive(Debug, Deserialize)]
struct TelegramResponse {
    ok: bool,
    description: Option<String>,
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    text: Option<String>,
    reply_to_message: Option<Box<TelegramMessage>>,
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

/// Telegram Webhook Payload (single update object).
#[derive(Debug, Deserialize)]
pub struct TelegramWebhookUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

/// Parse a Telegram webhook payload into a list of BridgeInboundEvents.
pub fn parse_telegram_webhook(body: &[u8]) -> Result<Vec<BridgeInboundEvent>, Box<dyn Error>> {
    let update: TelegramWebhookUpdate = serde_json::from_slice(body)?;
    let mut events = Vec::new();

    if let Some(message) = update.message {
        if let Some(text) = message.text {
            let chat_id = message.chat.id.to_string();
            let user_id = message.from.map(|u| u.id.to_string()).unwrap_or_else(|| "unknown".to_string());
            let reply_to = message.reply_to_message.as_ref().map(|m| m.message_id.to_string());

            events.push(BridgeInboundEvent {
                bridge_event_id: format!("tg-{}-{}", chat_id, update.update_id),
                channel: "telegram".to_string(),
                channel_user_id: user_id,
                channel_chat_id: chat_id.clone(),
                channel_message_id: update.update_id.to_string(),
                text,
                attachments: vec![],
                received_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                reply_to,
                metadata: std::collections::BTreeMap::new(),
            });
        }
    }
    Ok(events)
}
