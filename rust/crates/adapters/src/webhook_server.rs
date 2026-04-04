//! Webhook HTTP Server implementation for Kcode Bridge.
//! Supports Telegram, WhatsApp, and Feishu.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use bridge::events::{BridgeInboundEvent, BridgeOutboundEvent};
use serde::Deserialize;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::feishu_transport::{parse_feishu_webhook, verify_feishu_signature, FeishuWebhookPayload};
use crate::session_router::SessionRouter;
use crate::telegram_transport::{parse_telegram_webhook, TelegramConfig, TelegramTransport};
use crate::transport::Transport;
use crate::whatsapp_transport::{
    parse_whatsapp_webhook, verify_whatsapp_signature, WhatsAppConfig, WhatsAppTransport,
    WhatsAppWebhookPayload,
};

/// Shared state for webhook handlers.
#[derive(Clone)]
pub struct WebhookState {
    pub session_router: Arc<SessionRouter>,
    pub telegram_transport: Option<Arc<TelegramTransport>>,
    pub whatsapp_config: Option<WhatsAppConfig>,
    pub whatsapp_transport: Option<Arc<WhatsAppTransport>>,
    pub feishu_config: Option<crate::feishu_transport::FeishuConfig>,
    pub feishu_transport: Option<Arc<crate::feishu_transport::FeishuTransport>>,
    pub handler: Arc<dyn Fn(BridgeInboundEvent) -> BridgeOutboundEvent + Send + Sync>,
}

/// Start the webhook server listening on the given address.
pub async fn start_webhook_server(
    addr: SocketAddr,
    session_router: Arc<SessionRouter>,
    telegram_config: Option<TelegramConfig>,
    whatsapp_config: Option<WhatsAppConfig>,
    feishu_config: Option<crate::feishu_transport::FeishuConfig>,
    handler: impl Fn(BridgeInboundEvent) -> BridgeOutboundEvent + Send + Sync + 'static,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let telegram_transport = telegram_config.map(|c| Arc::new(TelegramTransport::new(c)));
    let whatsapp_config_clone = whatsapp_config.clone();
    let whatsapp_transport = whatsapp_config.map(|c| Arc::new(WhatsAppTransport::new(c)));
    let feishu_config_clone = feishu_config.clone();
    let feishu_transport = feishu_config.map(|c| Arc::new(crate::feishu_transport::FeishuTransport::new(c)));

    let state = WebhookState {
        session_router,
        telegram_transport,
        whatsapp_config: whatsapp_config_clone,
        whatsapp_transport,
        feishu_config: feishu_config_clone,
        feishu_transport,
        handler: Arc::new(handler),
    };

    let app = Router::new()
        .route("/health", get(handle_health_check))
        .route("/webhook/telegram", post(handle_telegram_webhook))
        .route("/webhook/whatsapp", get(handle_whatsapp_verify).post(handle_whatsapp_webhook))
        .route("/webhook/feishu", get(handle_feishu_ping).post(handle_feishu_webhook))
        .with_state(state);

    info!("Starting Webhook server on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint for monitoring and load balancers.
async fn handle_health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "kcode-bridge",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    }))
}

// --- Telegram Handlers ---

async fn handle_telegram_webhook(
    State(state): State<WebhookState>,
    body: axum::body::Bytes,
) -> Result<StatusCode, StatusCode> {
    let events = match parse_telegram_webhook(&body) {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to parse Telegram webhook: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    for event in events {
        let outbound = (state.handler)(event);
        if let Some(transport) = &state.telegram_transport {
            if let Err(e) = transport.send_outbound(&outbound).await {
                error!("Telegram send failed: {}", e);
            }
        }
    }
    Ok(StatusCode::OK)
}

// --- WhatsApp Handlers ---

#[derive(Deserialize)]
struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

async fn handle_whatsapp_verify(
    Query(params): Query<WhatsAppVerifyQuery>,
    State(_state): State<WebhookState>,
) -> Result<String, StatusCode> {
    if params.mode.as_deref() == Some("subscribe") {
        // In production, compare verify_token with config
        if params.verify_token.is_some() {
            info!("WhatsApp webhook verified");
            return Ok(params.challenge.unwrap_or_default());
        }
    }
    Err(StatusCode::FORBIDDEN)
}

async fn handle_whatsapp_webhook(
    State(state): State<WebhookState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Verify signature if present
    if let Some(sig) = headers.get("X-Hub-Signature-256") {
        if let Some(config) = &state.whatsapp_config {
            let sig_str = sig.to_str().unwrap_or("");
            if !sig_str.is_empty() {
                if !verify_whatsapp_signature(&body, sig_str, &config.app_secret) {
                    error!("WhatsApp signature verification failed");
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    }

    let payload: WhatsAppWebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse WhatsApp webhook: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    for event in parse_whatsapp_webhook(&payload) {
        let outbound = (state.handler)(event);
        if let Some(transport) = &state.whatsapp_transport {
            if let Err(e) = transport.send_outbound(&outbound).await {
                error!("WhatsApp send failed: {}", e);
            }
        }
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

// --- Feishu Handlers ---

async fn handle_feishu_ping() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn handle_feishu_webhook(
    State(state): State<WebhookState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Verify Feishu signature if present
    if let (Some(ts), Some(nonce), Some(sig)) = (
        headers.get("X-Lark-Request-Timestamp"),
        headers.get("X-Lark-Request-Nonce"),
        headers.get("X-Lark-Signature"),
    ) {
        if let Some(config) = &state.feishu_config {
            let ts_str = ts.to_str().unwrap_or("");
            let nonce_str = nonce.to_str().unwrap_or("");
            let sig_str = sig.to_str().unwrap_or("");
            
            if !verify_feishu_signature(ts_str, nonce_str, sig_str, &body, &config.app_secret) {
                error!("Feishu signature verification failed");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    let payload: FeishuWebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse Feishu webhook: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Handle challenge verification
    if payload.r#type == "url_verification" {
        if let Some(challenge) = &payload.challenge {
            return Ok(Json(serde_json::json!({ "challenge": challenge })));
        }
    }

    if let Some(event) = parse_feishu_webhook(&payload) {
        let outbound = (state.handler)(event);
        if let Some(transport) = &state.feishu_transport {
            if let Err(e) = transport.send_outbound(&outbound).await {
                error!("Feishu send failed: {}", e);
            }
        }
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}
