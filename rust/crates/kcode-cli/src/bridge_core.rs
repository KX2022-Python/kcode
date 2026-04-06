//! BridgeCore: Manages multi-user sessions for the Kcode Bridge.
//! Runs in a dedicated background thread to handle !Send LiveCli instances safely.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use adapters::{
    BridgeInboundEvent, BridgeOutboundEvent, ChannelSession, DeliveryMode, SessionRouter,
    TelegramConfig, TelegramMode,
};
use runtime::{ConfigLoader, PermissionMode};

use crate::LiveCli;

const BRIDGE_SESSION_DIR_NAME: &str = "bridge-sessions";

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
    session_router: SessionRouter,
}

impl SessionManager {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            sessions: HashMap::new(),
            session_router: SessionRouter::new(session_dir),
        }
    }

    /// Get an existing session or create a new one for the given channel/chat pair.
    /// Implements graceful fallback: if session file is corrupted, creates a new one.
    pub fn get_or_create_session(
        &mut self,
        chat_id: &str,
        channel: &str,
        default_config: &SessionConfig,
    ) -> Result<(&mut LiveCli, ChannelSession), String> {
        let session_route = self.session_router.get_or_create_session(channel, chat_id);
        let session_key = session_route.session_id.clone();
        let session_path = self.session_router.session_path(channel, chat_id);
        let session_dir = session_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        if !self.sessions.contains_key(&session_key) {
            println!(
                "✨ Creating/Loading bridge session for {}:{}",
                channel, chat_id
            );

            if let Err(error) = std::fs::create_dir_all(&session_dir) {
                eprintln!("⚠ Failed to create session directory: {}", error);
            }

            let cli = match LiveCli::new(
                default_config.model.clone(),
                default_config.model_explicit,
                default_config.profile.clone(),
                true,
                None,
                default_config.permission_mode,
                Some(session_path.clone()),
            ) {
                Ok(cli) => cli,
                Err(error) if session_path.exists() => {
                    eprintln!(
                        "⚠ Session file corrupted for {}:{}: {}. Creating new session.",
                        channel, chat_id, error
                    );
                    let backup_path = session_path.with_extension("jsonl.bak");
                    let _ = std::fs::rename(&session_path, &backup_path);

                    LiveCli::new(
                        default_config.model.clone(),
                        default_config.model_explicit,
                        default_config.profile.clone(),
                        true,
                        None,
                        default_config.permission_mode,
                        Some(session_path.clone()),
                    )
                    .map_err(|retry_error| retry_error.to_string())?
                }
                Err(error) => return Err(error.to_string()),
            };

            self.sessions.insert(session_key.clone(), cli);
        }

        Ok((self.sessions.get_mut(&session_key).unwrap(), session_route))
    }
}

fn bridge_session_dir_from_config_home(config_home: &Path) -> PathBuf {
    config_home.join(BRIDGE_SESSION_DIR_NAME)
}

fn bridge_session_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    Ok(bridge_session_dir_from_config_home(
        ConfigLoader::default_for(&cwd).config_home(),
    ))
}

fn bridge_response_session_id(channel: &str, chat_id: &str) -> String {
    SessionRouter::session_id_for(channel, chat_id)
}

/// The core bridge engine running in a background thread.
pub struct BridgeCore {
    session_manager: SessionManager,
}

impl BridgeCore {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            session_manager: SessionManager::new(session_dir),
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
    fn handle_message(
        &mut self,
        msg: &BridgeMessage,
        config: &SessionConfig,
    ) -> Option<BridgeOutboundEvent> {
        let chat_id = msg.event.channel_chat_id.clone();
        let channel = msg.event.channel.clone();

        let (cli, session_route) = match self
            .session_manager
            .get_or_create_session(&chat_id, &channel, config)
        {
            Ok(result) => result,
            Err(error) => {
                eprintln!("❌ Session creation failed: {}", error);
                return Some(self.create_error_response(&msg.event, error));
            }
        };

        match cli.run_turn_capture(&msg.event.text) {
            Ok(response) => Some(BridgeOutboundEvent {
                bridge_event_id: msg.event.bridge_event_id.clone(),
                session_id: session_route.session_id,
                channel_capability_hint: channel.clone(),
                reply_target: Some(chat_id.clone()),
                render_items: vec![("text".to_string(), response)],
                delivery_mode: DeliveryMode::Reply { reply_to: chat_id },
            }),
            Err(error) => {
                eprintln!("❌ Processing failed for {}: {}", chat_id, error);
                Some(self.create_error_response(&msg.event, error.to_string()))
            }
        }
    }

    fn create_error_response(
        &self,
        event: &BridgeInboundEvent,
        error: String,
    ) -> BridgeOutboundEvent {
        BridgeOutboundEvent {
            bridge_event_id: event.bridge_event_id.clone(),
            session_id: bridge_response_session_id(&event.channel, &event.channel_chat_id),
            channel_capability_hint: event.channel.clone(),
            reply_target: Some(event.channel_chat_id.clone()),
            render_items: vec![("text".to_string(), format!("⚠ Error: {}", error))],
            delivery_mode: DeliveryMode::Reply {
                reply_to: event.channel_chat_id.clone(),
            },
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
        apply_bridge_env_defaults_to_process, print_config_summary, validate_bridge_config,
        FeishuConfig, TelegramTransport, WhatsAppConfig,
    };
    use tokio::runtime::Builder;

    let _ = apply_bridge_env_defaults_to_process();

    let errors = validate_bridge_config();
    if !errors.is_empty() {
        eprintln!("❌ Configuration errors found:");
        for error in &errors {
            eprintln!("  ⚠ {}: {}", error.var_name, error.message);
        }
        eprintln!("\nPlease fix these issues and restart.");
        return Ok(());
    }

    print_config_summary();

    let bot_token = std::env::var("KCODE_TELEGRAM_BOT_TOKEN").ok();
    let webhook_url = std::env::var("KCODE_WEBHOOK_URL").ok();
    let whatsapp_phone = std::env::var("KCODE_WHATSAPP_PHONE_ID").ok();
    let feishu_app_id = std::env::var("KCODE_FEISHU_APP_ID").ok();

    if bot_token.is_none() && whatsapp_phone.is_none() && feishu_app_id.is_none() {
        eprintln!("⚠ No channel credentials found.");
        return Ok(());
    }

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
        app_secret: std::env::var("KCODE_FEISHU_APP_SECRET")
            .expect("Missing KCODE_FEISHU_APP_SECRET"),
        webhook_verify_token: std::env::var("KCODE_WEBHOOK_VERIFY_TOKEN").unwrap_or_default(),
    });

    let session_dir = bridge_session_dir()?;
    std::fs::create_dir_all(&session_dir)?;

    if let Some(config) = telegram_config.as_ref() {
        match &config.mode {
            TelegramMode::Webhook { url, .. } => {
                println!("🔌 Telegram delivery mode: Webhook");
                println!("  Public URL       {}", url);
                println!("  Local receiver   http://0.0.0.0:3000/webhook/telegram");
                println!(
                    "  Requirement      Expose port 3000 through a public HTTPS reverse proxy or tunnel"
                );
                println!("  Fallback         Unset KCODE_WEBHOOK_URL to use Telegram long polling");

                let transport = TelegramTransport::new(config.clone());
                let rt = Builder::new_current_thread().enable_all().build()?;
                rt.block_on(async { transport.set_webhook().await })
                    .map_err(|error| {
                        std::io::Error::other(format!("Failed to set Telegram webhook: {error}"))
                    })?;
            }
            TelegramMode::Polling { .. } => {
                println!("🔌 Telegram delivery mode: Long Polling");
            }
        }
    }

    let (core_tx, core_rx) = std::sync::mpsc::channel::<BridgeMessage>();

    std::thread::spawn(move || {
        let core = BridgeCore::new(session_dir);
        let config = SessionConfig {
            model,
            model_explicit,
            profile,
            permission_mode,
        };
        core.run(core_rx, config);
    });

    let webhook_tx = core_tx.clone();
    let handler = Box::new(move |event: BridgeInboundEvent| -> BridgeOutboundEvent {
        let (reply_tx, rx) = std::sync::mpsc::channel();
        if let Err(error) = webhook_tx.send(BridgeMessage { event, reply_tx }) {
            eprintln!("Failed to send event to BridgeCore: {}", error);
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
            Ok(response) => response,
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

    let rt = Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async {
        adapters::start_webhook_server(
            "0.0.0.0:3000".parse().unwrap(),
            telegram_config,
            whatsapp_config,
            feishu_config,
            handler,
        )
        .await
        .map_err(|error| -> Box<dyn std::error::Error> { error })
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bridge_session_dir_from_config_home;
    use std::path::{Path, PathBuf};

    #[test]
    fn bridge_session_dir_lives_under_config_home() {
        assert_eq!(
            bridge_session_dir_from_config_home(Path::new("/tmp/kcode-home")),
            PathBuf::from("/tmp/kcode-home/bridge-sessions")
        );
    }
}
