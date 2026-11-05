fn run_bridge(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    run_bridge_service(model, model_explicit, profile, permission_mode)
}

/// Lightweight REPL pre-flight check to catch obvious issues before starting.
fn quick_repl_preflight_check() -> Result<(), String> {
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
    if let Err(e) = quick_repl_preflight_check() {
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
