//! Telegram Transport implementation using Long Polling.
//! Connects to the Telegram Bot API and converts updates to BridgeInboundEvents.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;

use async_trait::async_trait;
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info};

use super::transport::Transport;

/// Telegram Bot API configuration.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_updates: Vec<String>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            allowed_updates: vec!["message".to_string()],
        }
    }
}

/// Telegram Transport handling Long Polling.
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

    async fn send_message(&self, chat_id: &str, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = self.api_url("sendMessage");
        let mut body = HashMap::new();
        body.insert("chat_id", chat_id.to_string());
        body.insert("text", text.to_string());

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("Telegram send failed: {} - {}", status, body_text).into());
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl Transport for TelegramTransport {
    async fn run(
        &self,
        handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Starting Telegram Long Polling transport...");

        loop {
            let offset = self.offset.load(std::sync::atomic::Ordering::SeqCst);
            let url = self.api_url("getUpdates");

            // Build request params
            let mut params = HashMap::new();
            params.insert("offset", offset.to_string());
            params.insert("timeout", "30".to_string());

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
                            reply_to: message.reply_to_message.as_ref().map(|m| m.message_id.to_string()),
                            metadata: BTreeMap::new(),
                        };

                        // Call the handler to get the response event
                        let outbound_event = handler(event);

                        // Extract text from outbound event to send back
                        let text_to_send = outbound_event.render_items.iter()
                            .map(|(_, t)| t.as_str())
                            .collect::<Vec<&str>>()
                            .join("\n");

                        // Send the response back to Telegram using reply_target from outbound
                        if let Some(reply_target) = &outbound_event.reply_target {
                            if let Err(e) = self.send_message(reply_target, &text_to_send).await {
                                error!("Failed to send message to Telegram: {}", e);
                            }
                        } else {
                            error!("No reply_target in outbound event, cannot send message");
                        }
                    }
                }

                // Update offset
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
