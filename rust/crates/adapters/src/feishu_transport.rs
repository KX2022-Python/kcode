//! Feishu (Lark) Transport implementation.
//! Handles Challenge verification, Interactive Cards, and Rich Text messages.

use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::transport::{Transport, TransportConfig};

/// Feishu/Lark Bot API configuration.
#[derive(Debug, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub webhook_verify_token: String,
}

impl TransportConfig for FeishuConfig {
    fn channel_id(&self) -> &str { "feishu" }
}

/// Feishu Transport.
pub struct FeishuTransport {
    config: FeishuConfig,
    client: Client,
    tenant_token: std::sync::RwLock<Option<String>>,
}

impl FeishuTransport {
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            tenant_token: std::sync::RwLock::new(None),
        }
    }

    /// Get or refresh the tenant_access_token (valid for 2 hours).
    async fn get_tenant_token(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Check cache
        {
            let guard = self.tenant_token.read().unwrap();
            if let Some(token) = guard.as_ref() {
                return Ok(token.clone());
            }
        }

        // Request new token
        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
        let body = serde_json::json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret,
        });

        let resp = self.client.post(url).json(&body).send().await?;
        let json: FeishuTokenResponse = resp.json().await?;
        
        if json.code != 0 {
            return Err(format!("Feishu token request failed: code={}, msg={}", json.code, json.msg).into());
        }

        let token = json.tenant_access_token;
        {
            let mut guard = self.tenant_token.write().unwrap();
            *guard = Some(token.clone());
        }

        Ok(token)
    }

    fn api_url(&self, path: &str) -> String {
        format!("https://open.feishu.cn/open-apis{}", path)
    }

    /// Handle Feishu webhook challenge verification.
    /// Returns Some(challenge_response) if this is a verification request, None otherwise.
    pub fn handle_challenge(&self, payload: &FeishuWebhookPayload) -> Option<FeishuChallengeResponse> {
        if payload.r#type == "url_verification" {
            Some(FeishuChallengeResponse {
                challenge: payload.challenge.clone().unwrap_or_default(),
            })
        } else {
            None
        }
    }

    /// Send a text message to Feishu.
    async fn send_text(&self, receive_id: &str, text: &str, receive_id_type: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let token = self.get_tenant_token().await?;
        let url = format!("{}?receive_id_type={}", self.api_url("/im/v1/messages"), receive_id_type);
        
        let content = serde_json::json!({"text": text}).to_string();
        let body = FeishuSendBody {
            receive_id: receive_id.to_string(),
            msg_type: "text".to_string(),
            content,
        };

        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Feishu send failed ({}): {}", status, body_text).into());
        }
        
        let json: FeishuSendResponse = serde_json::from_str(&body_text)
            .map_err(|e| format!("Failed to parse Feishu response: {}", e))?;
        
        if json.code != 0 {
            return Err(format!("Feishu API error {}: {}", json.code, json.msg).into());
        }

        Ok(())
    }

    /// Send an interactive card message to Feishu.
    async fn send_card(&self, receive_id: &str, card_json: &str, receive_id_type: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let token = self.get_tenant_token().await?;
        let url = format!("{}?receive_id_type={}", self.api_url("/im/v1/messages"), receive_id_type);
        
        let body = FeishuSendBody {
            receive_id: receive_id.to_string(),
            msg_type: "interactive".to_string(),
            content: card_json.to_string(),
        };

        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("Feishu card send failed ({}): {}", status, body_text).into());
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl Transport for FeishuTransport {
    async fn run(
        &self,
        _handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Feishu requires Webhook mode (no polling support)
        Err("Feishu requires Webhook mode. Use a separate HTTP server to receive events.".into())
    }

    async fn send_outbound(
        &self,
        event: &BridgeOutboundEvent,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let chat_id = event.reply_target.as_ref().ok_or("Missing reply_target (chat_id/open_id)")?;
        
        let text = event.render_items.iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        // Determine receive_id_type based on chat_id format
        let receive_id_type = if event.channel_capability_hint == "feishu-p2p" {
            "open_id"
        } else {
            "chat_id"
        };

        self.send_text(chat_id, &text, receive_id_type).await
    }
}

/// Parse an incoming Feishu webhook payload into a BridgeInboundEvent.
pub fn parse_feishu_webhook(payload: &FeishuWebhookPayload) -> Option<BridgeInboundEvent> {
    if payload.r#type == "url_verification" {
        return None;
    }

    let event_data = &payload.event;
    let message = &event_data.message;
    
    // Parse the content field (it's a stringified JSON)
    let content: serde_json::Value = match serde_json::from_str(&message.content) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let text = if message.message_type == "text" {
        content.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string()
    } else {
        format!("[{}] Message received", message.message_type)
    };

    Some(BridgeInboundEvent {
        bridge_event_id: message.message_id.clone(),
        channel: "feishu".to_string(),
        channel_user_id: event_data.sender.sender_id.open_id.clone(),
        channel_chat_id: event_data.message.chat_id.clone(),
        channel_message_id: message.message_id.clone(),
        text,
        attachments: vec![],
        received_at: event_data.message.create_time,
        reply_to: message.parent_id.clone().filter(|id| id != "0" && !id.is_empty()),
        metadata: std::collections::BTreeMap::new(),
    })
}

// Feishu API Types
#[derive(Debug, Deserialize)]
pub struct FeishuTokenResponse {
    pub code: i64,
    pub msg: String,
    #[serde(rename = "tenant_access_token")]
    pub tenant_access_token: String,
    pub expire: u64,
}

#[derive(Debug, Deserialize)]
pub struct FeishuWebhookPayload {
    pub schema: String,
    pub header: Option<FeishuHeader>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub challenge: Option<String>,
    pub event: FeishuEvent,
}

#[derive(Debug, Deserialize)]
pub struct FeishuHeader {
    #[serde(rename = "event_id")]
    pub event_id: String,
    #[serde(rename = "event_type")]
    pub event_type: String,
}

#[derive(Debug, Deserialize)]
pub struct FeishuEvent {
    pub sender: FeishuSender,
    pub message: FeishuMessage,
}

#[derive(Debug, Deserialize)]
pub struct FeishuSender {
    #[serde(rename = "sender_id")]
    pub sender_id: FeishuSenderId,
}

#[derive(Debug, Deserialize)]
pub struct FeishuSenderId {
    #[serde(rename = "open_id")]
    pub open_id: String,
}

#[derive(Debug, Deserialize)]
pub struct FeishuMessage {
    #[serde(rename = "message_id")]
    pub message_id: String,
    #[serde(rename = "message_type")]
    pub message_type: String,
    pub chat_id: String,
    pub chat_type: String,
    pub content: String,
    pub create_time: u64,
    pub parent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeishuSendBody {
    #[serde(rename = "receive_id")]
    pub receive_id: String,
    #[serde(rename = "msg_type")]
    pub msg_type: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct FeishuSendResponse {
    pub code: i64,
    pub msg: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct FeishuChallengeResponse {
    pub challenge: String,
}
