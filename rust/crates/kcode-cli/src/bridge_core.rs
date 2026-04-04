//! BridgeCore: Manages multi-user sessions for the Kcode Bridge.
//! Runs in a dedicated background thread to handle !Send LiveCli instances safely.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use adapters::{
    BridgeInboundEvent, BridgeOutboundEvent, DeliveryMode, TelegramConfig, TelegramMode,
    TelegramTransport,
};
use runtime::PermissionMode;

use crate::LiveCli;

/// A message sent from the Webhook Server to the BridgeCore.
pub struct BridgeMessage {
    pub event: BridgeInboundEvent,
    pub reply_tx: Sender<BridgeOutboundEvent>,
}

/// Configuration for creating new sessions.
pub struct SessionConfig {
    pub model: String,
    pub model_explicit: bool,
    pub profile: Option<String>,
    pub permission_mode: PermissionMode,
}

/// Manages the lifecycle of individual LiveCli sessions.
pub struct SessionManager {
    sessions: HashMap<String, LiveCli>,
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
    /// Implements graceful fallback: if session file is corrupted, creates a new one.
    pub fn get_or_create_session(
        &mut self,
        chat_id: &str,
        channel: &str,
        default_config: &SessionConfig,
    ) -> Result<&mut LiveCli, String> {
        if !self.sessions.contains_key(chat_id) {
            println!("✨ Creating/Loading session for chat_id: {}", chat_id);

            let session_path = self.session_dir.join(format!("{}.jsonl", chat_id));

            // Ensure session directory exists
            if let Err(e) = std::fs::create_dir_all(&self.session_dir) {
                eprintln!("⚠ Failed to create session directory: {}", e);
            }

            let cli = if session_path.exists() {
                // Try to load existing session, fallback to new if corrupted
                match LiveCli::new(
                    default_config.model.clone(),
                    default_config.model_explicit,
                    default_config.profile.clone(),
                    true,
                    None,
                    default_config.permission_mode,
                    Some(session_path.clone()),
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("⚠ Session file corrupted for {}: {}. Creating new session.", chat_id, e);
                        // Backup corrupted file
                        let backup_path = session_path.with_extension("jsonl.bak");
                        let _ = std::fs::rename(&session_path, &backup_path);
                        
                        // Create fresh session
                        LiveCli::new(
                            default_config.model.clone(),
                            default_config.model_explicit,
                            default_config.profile.clone(),
                            true,
                            None,
                            default_config.permission_mode,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                    }
                }
            } else {
                LiveCli::new(
                    default_config.model.clone(),
                    default_config.model_explicit,
                    default_config.profile.clone(),
                    true,
                    None,
                    default_config.permission_mode,
                    None,
                )
                .map_err(|e| e.to_string())?
            };

            self.sessions.insert(chat_id.to_string(), cli);
        }
        Ok(self.sessions.get_mut(chat_id).unwrap())
    }
}

/// The core bridge engine running in a background thread.
pub struct BridgeCore {
    session_manager: SessionManager,
    _telegram_transport: TelegramTransport, // Kept for potential polling fallback
}

impl BridgeCore {
    pub fn new(session_dir: PathBuf, telegram_transport: TelegramTransport) -> Self {
        Self {
            session_manager: SessionManager::new(session_dir),
            _telegram_transport: telegram_transport,
        }
    }

    /// Run the bridge loop. Blocks the current thread.
    pub fn run(mut self, rx: Receiver<BridgeMessage>, config: SessionConfig) {
        println!("🤖 BridgeCore started.");

        while let Ok(msg) = rx.recv() {
            let chat_id = msg.event.channel_chat_id.clone();

            let result = self.handle_message(&msg, &config);

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

        let cli = match self.session_manager.get_or_create_session(&chat_id, &channel, config) {
            Ok(cli) => cli,
            Err(e) => {
                eprintln!("❌ Session creation failed: {}", e);
                return Some(self.create_error_response(&msg.event, e));
            }
        };

        match cli.run_turn_capture(&msg.event.text) {
            Ok(response) => Some(BridgeOutboundEvent {
                bridge_event_id: msg.event.bridge_event_id.clone(),
                session_id: chat_id.clone(),
                channel_capability_hint: channel.clone(),
                reply_target: Some(chat_id.clone()),
                render_items: vec![("text".to_string(), response)],
                delivery_mode: DeliveryMode::Reply { reply_to: chat_id },
            }),
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

/// Entry point for the Bridge service.
/// Reads environment variables, validates config, and starts the webhook server + bridge core.
pub fn run_bridge_service(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    use adapters::{
        FeishuConfig, SessionRouter, WhatsAppConfig,
        validate_bridge_config, print_config_summary,
    };
    use std::sync::Arc;
    use tokio::runtime::Builder;

    // Validate configuration before startup
    let errors = validate_bridge_config();
    if !errors.is_empty() {
        eprintln!("❌ Configuration errors found:");
        for err in &errors {
            eprintln!("  ⚠ {}: {}", err.var_name, err.message);
        }
        eprintln!("\nPlease fix these issues and restart.");
        return Ok(());
    }

    // Print configuration summary
    print_config_summary();

    // 1. Load Credentials
    let bot_token = std::env::var("KCODE_TELEGRAM_BOT_TOKEN").ok();
    let webhook_url = std::env::var("KCODE_WEBHOOK_URL").ok();
    let whatsapp_phone = std::env::var("KCODE_WHATSAPP_PHONE_ID").ok();
    let feishu_app_id = std::env::var("KCODE_FEISHU_APP_ID").ok();

    if bot_token.is_none() && whatsapp_phone.is_none() && feishu_app_id.is_none() {
        eprintln!("⚠ No channel credentials found.");
        return Ok(());
    }

    // 2. Build Configs
    let telegram_config = bot_token.map(|token| TelegramConfig {
        bot_token: token,
        mode: if let Some(url) = webhook_url {
            TelegramMode::Webhook { url, port: 3000 }
        } else {
            TelegramMode::Polling { timeout: 30 }
        },
    });

    let whatsapp_config = whatsapp_phone.map(|phone_id| WhatsAppConfig {
        access_token: std::env::var("KCODE_WHATSAPP_TOKEN").expect("Missing KCODE_WHATSAPP_TOKEN"),
        phone_number_id: phone_id,
        app_secret: std::env::var("KCODE_WHATSAPP_APP_SECRET").unwrap_or_default(),
        webhook_verify_token: std::env::var("KCODE_WEBHOOK_VERIFY_TOKEN").unwrap_or_default(),
    });

    let feishu_config = feishu_app_id.map(|app_id| FeishuConfig {
        app_id,
        app_secret: std::env::var("KCODE_FEISHU_APP_SECRET").expect("Missing KCODE_FEISHU_APP_SECRET"),
        webhook_verify_token: std::env::var("KCODE_WEBHOOK_VERIFY_TOKEN").unwrap_or_default(),
    });

    // 3. Initialize Transports
    let telegram_transport = telegram_config.clone().map(TelegramTransport::new);

    // 4. Setup Session Router
    let session_router = Arc::new(SessionRouter::new(PathBuf::from(".kcode/bridge-sessions")));

    // 5. Create Communication Channel
    let (core_tx, core_rx) = std::sync::mpsc::channel::<BridgeMessage>();

    // 6. Spawn BridgeCore
    if let Some(tg_transport) = telegram_transport {
        std::thread::spawn(move || {
            let core = BridgeCore::new(PathBuf::from(".kcode/bridge-sessions"), tg_transport);
            let config = SessionConfig {
                model,
                model_explicit,
                profile,
                permission_mode,
            };
            core.run(core_rx, config);
        });
    }

    // 7. Prepare Webhook Handler
    let webhook_tx = core_tx.clone();
    let handler = Box::new(move |event: BridgeInboundEvent| -> BridgeOutboundEvent {
        let (reply_tx, rx) = std::sync::mpsc::channel();
        if let Err(e) = webhook_tx.send(BridgeMessage { event, reply_tx }) {
            eprintln!("Failed to send event to BridgeCore: {}", e);
            return BridgeOutboundEvent {
                bridge_event_id: "error".to_string(),
                session_id: String::new(),
                channel_capability_hint: String::new(),
                reply_target: None,
                render_items: vec![("text".to_string(), "Error: Core unavailable".into())],
                delivery_mode: DeliveryMode::Single,
            };
        }

        match rx.recv() {
            Ok(res) => res,
            Err(_) => BridgeOutboundEvent {
                bridge_event_id: "error".to_string(),
                session_id: String::new(),
                channel_capability_hint: String::new(),
                reply_target: None,
                render_items: vec![("text".to_string(), "Error: Timeout".into())],
                delivery_mode: DeliveryMode::Single,
            },
        }
    });

    println!("🌐 Kcode Bridge started. Listening on 0.0.0.0:3000");

    // 8. Run Webhook Server (Async)
    let rt = Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async {
        adapters::start_webhook_server(
            "0.0.0.0:3000".parse().unwrap(),
            session_router,
            telegram_config,
            whatsapp_config,
            feishu_config,
            handler,
        )
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { e })
    })?;

    Ok(())
}
