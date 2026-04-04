//! WhatsApp Cloud API Transport implementation.
//! Handles the 24-hour session window, template messages, and media management.

use std::error::Error;

use super::transport::{Transport, TransportConfig};
use async_trait::async_trait;
use bridge::attachment::{AttachmentEnvelope, AttachmentKind};
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// WhatsApp Cloud API configuration.
#[derive(Debug, Clone)]
pub struct WhatsAppConfig {
    /// System user access token or temporary access token.
    pub access_token: String,
    /// WhatsApp Business Account Phone Number ID.
    pub phone_number_id: String,
    /// App secret for webhook signature verification.
    pub app_secret: String,
    /// Webhook verify token (custom string you define).
    pub webhook_verify_token: String,
}

impl TransportConfig for WhatsAppConfig {
    fn channel_id(&self) -> &str {
        "whatsapp"
    }
}

/// WhatsApp Transport.
pub struct WhatsAppTransport {
    config: WhatsAppConfig,
    client: Client,
}

impl WhatsAppTransport {
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "https://graph.facebook.com/v18.0/{}{}",
            self.config.phone_number_id, path
        )
    }

    /// Send a text message to WhatsApp.
    async fn send_text(&self, to: &str, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        // WhatsApp has a ~4096 character limit for text messages
        const MAX_LEN: usize = 4096;

        let chunks: Vec<&str> = if text.len() <= MAX_LEN {
            vec![text]
        } else {
            text.as_bytes()
                .chunks(MAX_LEN)
                .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                .collect()
        };

        for chunk in chunks {
            let url = self.api_url("/messages");
            let body = WhatsAppSendBody {
                messaging_product: "whatsapp".to_string(),
                recipient_type: "individual".to_string(),
                to: to.to_string(),
                r#type: "text".to_string(),
                text: Some(WhatsAppText {
                    body: chunk.to_string(),
                    preview_url: true,
                }),
                template: None,
            };

            let resp = self
                .client
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.access_token),
                )
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            let status = resp.status();
            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!("WhatsApp send failed ({}): {}", status, body_text).into());
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Transport for WhatsAppTransport {
    async fn run(
        &self,
        _handler: Box<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + Send + Sync + 'static>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // WhatsApp requires Webhook mode (no polling support)
        // This would require an HTTP server like axum
        Err("WhatsApp requires Webhook mode. Use a separate HTTP server to receive events.".into())
    }

    async fn send_outbound(
        &self,
        event: &BridgeOutboundEvent,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let to = event
            .reply_target
            .as_ref()
            .ok_or("Missing reply_target (phone number)")?;

        let text = event
            .render_items
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        self.send_text(to, &text).await
    }
}

/// Parse an incoming WhatsApp webhook payload into a BridgeInboundEvent.
/// This should be called by the external HTTP server handling webhooks.
pub fn parse_whatsapp_webhook(payload: &WhatsAppWebhookPayload) -> Vec<BridgeInboundEvent> {
    let mut events = Vec::new();
    for entry in &payload.entry {
        for change in &entry.changes {
            if let Some(messages) = &change.value.messages {
                for msg in messages {
                    let mut text = String::new();
                    let mut attachments = Vec::new();

                    if let Some(text_obj) = &msg.text {
                        text = text_obj.body.clone();
                    }

                    if let Some(image) = &msg.image {
                        attachments.push(AttachmentEnvelope {
                            kind: AttachmentKind::Image,
                            name: "image.jpg".to_string(),
                            mime_type: image.mime_type.clone().or(Some("image/jpeg".to_string())),
                            url_or_path: Some(format!(
                                "https://graph.facebook.com/v18.0/{}",
                                image.id
                            )),
                            text_content: image.caption.clone(),
                            size_bytes: None,
                        });
                        if text.is_empty() {
                            text = "[Received an image]".to_string();
                        }
                    }

                    if let Some(audio) = &msg.audio {
                        attachments.push(AttachmentEnvelope {
                            kind: AttachmentKind::PlatformMetadata,
                            name: "audio.ogg".to_string(),
                            mime_type: audio.mime_type.clone().or(Some("audio/ogg".to_string())),
                            url_or_path: Some(format!(
                                "https://graph.facebook.com/v18.0/{}",
                                audio.id
                            )),
                            text_content: None,
                            size_bytes: None,
                        });
                        if text.is_empty() {
                            text = "[Received an audio message]".to_string();
                        }
                    }

                    if let Some(doc) = &msg.document {
                        attachments.push(AttachmentEnvelope {
                            kind: AttachmentKind::Document,
                            name: doc
                                .filename
                                .clone()
                                .unwrap_or_else(|| "document".to_string()),
                            mime_type: doc.mime_type.clone(),
                            url_or_path: Some(format!(
                                "https://graph.facebook.com/v18.0/{}",
                                doc.id
                            )),
                            text_content: doc.caption.clone(),
                            size_bytes: None,
                        });
                        if text.is_empty() {
                            text =
                                format!("[Received a file: {}]", attachments.last().unwrap().name);
                        }
                    }

                    events.push(BridgeInboundEvent {
                        bridge_event_id: msg.id.clone(),
                        channel: "whatsapp".to_string(),
                        channel_user_id: msg.from.clone(),
                        channel_chat_id: msg.from.clone(),
                        channel_message_id: msg.id.clone(),
                        text,
                        attachments,
                        received_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        reply_to: msg.context.as_ref().map(|c| c.id.clone()),
                        metadata: std::collections::BTreeMap::new(),
                    });
                }
            }
        }
    }
    events
}

/// Verify the WhatsApp webhook signature (X-Hub-Signature-256).
pub fn verify_whatsapp_signature(payload: &[u8], signature: &str, app_secret: &str) -> bool {
    use hmac::Mac;
    use sha2::Sha256;

    // Signature format: sha256=<hex_hash>
    if !signature.starts_with("sha256=") {
        return false;
    }
    let expected_hash = &signature[7..];

    let mut mac = hmac::Hmac::<Sha256>::new_from_slice(app_secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload);
    let result = mac.finalize();
    let actual_hash = hex::encode(result.into_bytes());

    actual_hash == expected_hash
}

// WhatsApp API Types
#[derive(Debug, Serialize)]
struct WhatsAppSendBody {
    messaging_product: String,
    recipient_type: String,
    to: String,
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<WhatsAppText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template: Option<WhatsAppTemplate>,
}

#[derive(Debug, Serialize)]
struct WhatsAppText {
    body: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    preview_url: bool,
}

#[derive(Debug, Serialize)]
struct WhatsAppTemplate {
    name: String,
    language: WhatsAppTemplateLanguage,
    components: Vec<WhatsAppTemplateComponent>,
}

#[derive(Debug, Serialize)]
struct WhatsAppTemplateLanguage {
    code: String,
}

#[derive(Debug, Serialize)]
struct WhatsAppTemplateComponent {
    r#type: String,
    parameters: Vec<WhatsAppTemplateParameter>,
}

#[derive(Debug, Serialize)]
struct WhatsAppTemplateParameter {
    r#type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppWebhookPayload {
    pub object: String,
    pub entry: Vec<WhatsAppEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppEntry {
    pub id: String,
    pub changes: Vec<WhatsAppChange>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppChange {
    pub field: String,
    pub value: WhatsAppChangeValue,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppChangeValue {
    pub messaging_product: Option<String>,
    pub metadata: Option<WhatsAppMetadata>,
    pub messages: Option<Vec<WhatsAppMessage>>,
    pub statuses: Option<Vec<WhatsAppStatus>>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMetadata {
    pub phone_number_id: String,
    pub display_phone_number: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMessage {
    pub id: String,
    pub from: String,
    pub timestamp: String,
    pub r#type: Option<String>,
    pub text: Option<WhatsAppTextMessage>,
    pub context: Option<WhatsAppMessageContext>,
    // Media fields
    pub image: Option<WhatsAppMedia>,
    pub audio: Option<WhatsAppMedia>,
    pub document: Option<WhatsAppDocument>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMedia {
    pub id: String,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppDocument {
    pub id: String,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppTextMessage {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMessageContext {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppStatus {
    pub id: String,
    pub status: String,
    pub timestamp: String,
    pub recipient_id: String,
}
