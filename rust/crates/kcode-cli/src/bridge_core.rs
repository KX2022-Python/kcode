//! BridgeCore: Manages multi-user sessions for the Kcode Bridge.
//! Runs in a dedicated background thread to handle !Send LiveCli instances safely.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use adapters::{
    BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode, TelegramTransport,
};
use runtime::PermissionMode;

use crate::LiveCli;

/// A message sent from the Webhook Server to the BridgeCore.
pub struct BridgeMessage {
    pub event: BridgeInboundEvent,
    pub reply_tx: Sender<BridgeOutboundEvent>,
}

/// Manages the lifecycle of individual LiveCli sessions.
pub struct SessionManager {
    sessions: HashMap<String, LiveCli>,
    // In a future step, this will handle persistence to disk.
    session_dir: PathBuf,
}

impl SessionManager {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            sessions: HashMap::new(),
            session_dir,
        }
    }

    /// Get an existing session or create a new one for the given chat_id.
    pub fn get_or_create_session(
        &mut self,
        chat_id: &str,
        channel: &str,
        default_config: &SessionConfig,
    ) -> Result<&mut LiveCli, String> {
        if !self.sessions.contains_key(chat_id) {
            println!("✨ Creating/Loading session for chat_id: {}", chat_id);
            
            // Calculate session path
            let session_path = self.session_dir.join(format!("{}.jsonl", chat_id));
            
            // Check if we should load existing session
            let cli = if session_path.exists() {
                LiveCli::new(
                    default_config.model.clone(),
                    default_config.model_explicit,
                    default_config.profile.clone(),
                    true,
                    None,
                    default_config.permission_mode,
                    Some(session_path),
                ).map_err(|e| e.to_string())?
            } else {
                LiveCli::new(
                    default_config.model.clone(),
                    default_config.model_explicit,
                    default_config.profile.clone(),
                    true,
                    None,
                    default_config.permission_mode,
                    None,
                ).map_err(|e| e.to_string())?
            };

            self.sessions.insert(chat_id.to_string(), cli);
        }
        Ok(self.sessions.get_mut(chat_id).unwrap())
    }
}

/// Configuration for creating new sessions.
pub struct SessionConfig {
    pub model: String,
    pub model_explicit: bool,
    pub profile: Option<String>,
    pub permission_mode: crate::PermissionMode,
}

/// The core bridge engine running in a background thread.
pub struct BridgeCore {
    session_manager: SessionManager,
    telegram_transport: TelegramTransport,
    // Future: WhatsApp and Feishu transports will be added here.
}

impl BridgeCore {
    pub fn new(session_dir: PathBuf, telegram_transport: TelegramTransport) -> Self {
        Self {
            session_manager: SessionManager::new(session_dir),
            telegram_transport,
        }
    }

    /// Run the bridge loop. Blocks the current thread.
    pub fn run(mut self, rx: Receiver<BridgeMessage>, config: SessionConfig) {
        println!("🤖 BridgeCore started.");

        while let Ok(msg) = rx.recv() {
            let chat_id = msg.event.channel_chat_id.clone();

            let result = self.handle_message(&msg, &config);

            // Send response back to the webhook handler
            if let Some(outbound) = result {
                if msg.reply_tx.send(outbound).is_err() {
                    eprintln!("⚠ Failed to send response for chat_id: {}", chat_id);
                }
            }
        }
        println!("👋 BridgeCore shutting down.");
    }

    /// Route an incoming event to the correct session and process it.
    fn handle_message(&mut self, msg: &BridgeMessage, config: &SessionConfig) -> Option<BridgeOutboundEvent> {
        let chat_id = msg.event.channel_chat_id.clone();
        let channel = msg.event.channel.clone();

        // 1. Resolve Session
        let cli = match self.session_manager.get_or_create_session(&chat_id, &channel, config) {
            Ok(cli) => cli,
            Err(e) => {
                eprintln!("❌ Session creation failed: {}", e);
                return Some(self.create_error_response(&msg.event, e));
            }
        };

        // 2. Process Turn
        match cli.run_turn_capture(&msg.event.text) {
            Ok(response) => {
                // 3. Construct Outbound Event
                Some(BridgeOutboundEvent {
                    bridge_event_id: msg.event.bridge_event_id.clone(),
                    session_id: chat_id.clone(),
                    channel_capability_hint: channel.clone(),
                    reply_target: Some(chat_id.clone()),
                    render_items: vec![("text".to_string(), response)],
                    delivery_mode: DeliveryMode::Reply { reply_to: chat_id },
                })
            }
            Err(e) => {
                eprintln!("❌ Processing failed for {}: {}", chat_id, e);
                Some(self.create_error_response(&msg.event, e.to_string()))
            }
        }
    }

    fn create_error_response(&self, event: &BridgeInboundEvent, error: String) -> BridgeOutboundEvent {
        BridgeOutboundEvent {
            bridge_event_id: event.bridge_event_id.clone(),
            session_id: event.channel_chat_id.clone(),
            channel_capability_hint: event.channel.clone(),
            reply_target: Some(event.channel_chat_id.clone()),
            render_items: vec![("text".to_string(), format!("⚠ Error: {}", error))],
            delivery_mode: DeliveryMode::Reply { reply_to: event.channel_chat_id.clone() },
        }
    }
}
