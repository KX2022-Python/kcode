fn run_bridge(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Security: Load credentials from environment variables only.
    let bot_token = std::env::var("KCODE_TELEGRAM_BOT_TOKEN").ok();
    let whatsapp_phone = std::env::var("KCODE_WHATSAPP_PHONE_ID").ok();
    let feishu_app_id = std::env::var("KCODE_FEISHU_APP_ID").ok();

    if bot_token.is_none() && whatsapp_phone.is_none() && feishu_app_id.is_none() {
        eprintln!("⚠ No channel credentials found. Please set KCODE_TELEGRAM_BOT_TOKEN, KCODE_WHATSAPP_PHONE_ID, or KCODE_FEISHU_APP_ID.");
        return Ok(());
    }

    // Telegram Config
    let telegram_config = bot_token.map(|token| adapters::TelegramConfig {
        bot_token: token,
        mode: adapters::TelegramMode::Polling { timeout: 30 },
    });

    // WhatsApp Config
    let whatsapp_config = whatsapp_phone.map(|phone_id| adapters::WhatsAppConfig {
        access_token: std::env::var("KCODE_WHATSAPP_TOKEN").expect("KCODE_WHATSAPP_TOKEN required"),
        phone_number_id: phone_id,
        app_secret: std::env::var("KCODE_WHATSAPP_APP_SECRET").unwrap_or_default(),
        webhook_verify_token: std::env::var("KCODE_WEBHOOK_VERIFY_TOKEN").unwrap_or_default(),
    });

    // Feishu Config
    let feishu_config = feishu_app_id.map(|app_id| adapters::FeishuConfig {
        app_id,
        app_secret: std::env::var("KCODE_FEISHU_APP_SECRET").expect("KCODE_FEISHU_APP_SECRET required"),
        webhook_verify_token: std::env::var("KCODE_WEBHOOK_VERIFY_TOKEN").unwrap_or_default(),
    });

    // Setup Session Router for persistence
    let session_router = std::sync::Arc::new(adapters::SessionRouter::new(
        std::path::PathBuf::from(".kcode/bridge-sessions")
    ));

    // Create channel for BridgeCore
    let (core_tx, core_rx) = std::sync::mpsc::channel::<BridgeMessage>();

    // We create the telegram transport here for the BridgeCore thread
    let telegram_transport = telegram_config.clone().map(adapters::TelegramTransport::new);

    // If using Webhook mode for Telegram, configure it first
    if let Some(ref cfg) = telegram_config {
        if let adapters::TelegramMode::Webhook { url: _, port: _ } = cfg.mode {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
            let transport = adapters::TelegramTransport::new(cfg.clone());
            rt.block_on(async {
                transport.set_webhook().await
            }).map_err(|e| format!("Failed to set Telegram webhook: {}", e))?;
        }
    }

    // Spawn BridgeCore in a dedicated thread
    std::thread::spawn(move || {
        if let Some(transport) = telegram_transport {
            let core = BridgeCore::new(
                std::path::PathBuf::from(".kcode/bridge-sessions"),
                transport,
            );
            let config = SessionConfig {
                model,
                model_explicit,
                profile,
                permission_mode,
            };
            core.run(core_rx, config);
        }
    });

    // Webhook handler that forwards events to BridgeCore
    let webhook_tx = core_tx.clone();
    let handler = Box::new(move |event: adapters::BridgeInboundEvent| -> adapters::BridgeOutboundEvent {
        let (reply_tx, rx) = std::sync::mpsc::channel();
        if let Err(e) = webhook_tx.send(BridgeMessage { event, reply_tx }) {
            eprintln!("Failed to send event to BridgeCore: {}", e);
            return adapters::BridgeOutboundEvent {
                bridge_event_id: "error".to_string(),
                session_id: String::new(),
                channel_capability_hint: String::new(),
                reply_target: None,
                render_items: vec![("text".to_string(), "Error: Core unavailable".to_string())],
                delivery_mode: adapters::DeliveryMode::Single,
            };
        }

        match rx.recv() {
            Ok(outbound) => outbound,
            Err(_) => adapters::BridgeOutboundEvent {
                bridge_event_id: "error".to_string(),
                session_id: String::new(),
                channel_capability_hint: String::new(),
                reply_target: None,
                render_items: vec![("text".to_string(), "Error: Timeout".to_string())],
                delivery_mode: adapters::DeliveryMode::Single,
            },
        }
    });

    println!("🌐 Kcode Bridge started. Waiting for messages on all active channels...");

    // Run the webhook server
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async {
        adapters::start_webhook_server(
            "0.0.0.0:3000".parse().unwrap(),
            session_router,
            telegram_config,
            whatsapp_config,
            feishu_config,
            handler,
        ).await.map_err(|e| -> Box<dyn std::error::Error> { 
            eprintln!("\n❌ Bridge server failed to start: {}", e);
            eprintln!("💡 Run `kcode doctor --fix` to automatically repair configuration issues.");
            e 
        })
    })?;

    Ok(())
}

/// Lightweight pre-flight check to catch obvious issues before starting.
fn quick_preflight_check() -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let kcode_dir = format!("{}/.kcode", home);
    
    if !std::path::Path::new(&kcode_dir).exists() {
        return Err(format!("{} directory not found", kcode_dir));
    }
    
    // Check session directory is writeable
    let sessions_dir = format!("{}/sessions", kcode_dir);
    if !std::path::Path::new(&sessions_dir).exists() {
        return Err("sessions directory missing".to_string());
    }
    
    Ok(())
}
fn run_repl(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Quick pre-flight check before starting REPL
    if let Err(e) = quick_preflight_check() {
        eprintln!("⚠ Pre-flight warning: {}", e);
        eprintln!("💡 Run `kcode doctor` to diagnose or `kcode doctor --fix` to repair.\n");
    }

    let mut cli = match LiveCli::new(
        model,
        model_explicit,
        profile,
        true,
        allowed_tools,
        permission_mode,
        None,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to initialize Kcode runtime: {}", e);
            eprintln!("💡 Run `kcode doctor --fix` to automatically repair common issues.");
            return Err(e);
        }
    };
    let mut editor =
        input::LineEditor::new("> ", cli.repl_completion_candidates().unwrap_or_default());
    println!("{}", cli.startup_banner());

    loop {
        editor.set_completions(cli.repl_completion_candidates().unwrap_or_default());
        match editor.read_line()? {
            input::ReadOutcome::Submit(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed.as_str(), "/exit" | "/quit") {
                    cli.persist_session()?;
                    break;
                }
                match SlashCommand::parse(&trimmed) {
                    Ok(Some(command)) => {
                        if cli.handle_repl_command(command)? {
                            cli.persist_session()?;
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{error}");
                        continue;
                    }
                }
                editor.push_history(input);
                cli.run_turn(&trimmed)?;
            }
            input::ReadOutcome::Cancel => {}

            input::ReadOutcome::Exit => {
                cli.persist_session()?;
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct SessionHandle {
    pub(crate) id: String,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Clone)]
struct ManagedSessionSummary {
    id: String,
    path: PathBuf,
    modified_epoch_millis: u128,
    message_count: usize,
    parent_session_id: Option<String>,
    branch_name: Option<String>,
}

struct LiveCli {
    model: String,
    model_explicit: bool,
    profile_override: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    system_prompt: Vec<String>,
    runtime: BuiltRuntime,
    active_profile: ResolvedProviderProfile,
    session: SessionHandle,
}

struct RuntimePluginState {
    feature_config: runtime::RuntimeFeatureConfig,
    tool_registry: GlobalToolRegistry,
    plugin_registry: PluginRegistry,
}

struct BuiltRuntime {
    runtime: Option<ConversationRuntime<ProviderRuntimeClient, CliToolExecutor>>,
    plugin_registry: PluginRegistry,
    plugins_active: bool,
    active_profile: ResolvedProviderProfile,
}

impl BuiltRuntime {
    fn new(
        runtime: ConversationRuntime<ProviderRuntimeClient, CliToolExecutor>,
        plugin_registry: PluginRegistry,
        active_profile: ResolvedProviderProfile,
    ) -> Self {
        Self {
            runtime: Some(runtime),
            plugin_registry,
            plugins_active: true,
            active_profile,
        }
    }

    fn with_hook_abort_signal(mut self, hook_abort_signal: runtime::HookAbortSignal) -> Self {
        let runtime = self
            .runtime
            .take()
            .expect("runtime should exist before installing hook abort signal");
        self.runtime = Some(runtime.with_hook_abort_signal(hook_abort_signal));
        self
    }

    fn shutdown_plugins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.plugins_active {
            self.plugin_registry.shutdown()?;
            self.plugins_active = false;
        }
        Ok(())
    }
}

impl Deref for BuiltRuntime {
    type Target = ConversationRuntime<ProviderRuntimeClient, CliToolExecutor>;

    fn deref(&self) -> &Self::Target {
        self.runtime
            .as_ref()
            .expect("runtime should exist while built runtime is alive")
    }
}

impl DerefMut for BuiltRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.runtime
            .as_mut()
            .expect("runtime should exist while built runtime is alive")
    }
}

impl Drop for BuiltRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown_plugins();
    }
}
