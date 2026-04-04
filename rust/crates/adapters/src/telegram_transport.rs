//! Telegram Transport implementation using Long Polling.
//! Connects to the Telegram Bot API and converts updates to BridgeInboundEvents.

use std::collections::HashMap;
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
}

#[async_trait]
impl Transport for TelegramTransport {
    async fn run(
        &self,
        on_message: Box<dyn Fn(BridgeInboundEvent) + Send + Sync + 'static>,
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
                        
                        let event = BridgeInboundEvent::new(
                            format!("tg-{}-{}", chat_id, update.update_id),
                            "telegram".to_string(),
                            user_id,
                            chat_id.clone(),
                            update.update_id.to_string(),
                            text,
                        );

                        // Store chat_id in metadata for reply routing
                        // on_message(event); // This will be called by the loop below
                        
                        // We need to store the chat_id to reply later.
                        // The simplest way is to use a map in the transport or attach it to the event.
                        // For this v1.1 implementation, we'll attach it to metadata.
                        // However, BridgeInboundEvent has a metadata field.
                        
                        on_message(event);
                    }
                }
                
                // Update offset
                self.offset.store(update.update_id + 1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    async fn send(&self, event: &BridgeOutboundEvent) -> Result<(), Box<dyn Error + Send + Sync>> {
        let chat_id = event.reply_target.as_ref().ok_or("Missing chat_id in reply_target")?;
        let text = event.render_items.iter().map(|(_, t)| t.as_str()).collect::<Vec<&str>>().join("\n");

        let url = self.api_url("sendMessage");

        let mut body = HashMap::new();
        body.insert("chat_id", chat_id.clone());
        body.insert("text", text);
        
        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Telegram send failed: {}", resp.status()).into());
        }

        Ok(())
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
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}
