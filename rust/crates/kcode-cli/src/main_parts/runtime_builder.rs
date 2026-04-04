#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime(
    session: Session,
    session_id: &str,
    model: String,
    model_override: Option<&str>,
    profile_override: Option<&str>,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let setup_context = load_setup_context(
        if emit_output {
            SetupMode::Interactive
        } else {
            SetupMode::Print
        },
        model_override,
        profile_override,
        permission_mode,
        Some(session_id),
    )?;
    ensure_setup_ready_for_runtime(&setup_context)?;
    let runtime_plugin_state =
        build_runtime_plugin_state(setup_context.active_profile.profile.supports_tools)?;
    build_runtime_with_plugin_state(
        session,
        session_id,
        model,
        system_prompt,
        enable_tools,
        emit_output,
        allowed_tools,
        permission_mode,
        progress_reporter,
        &setup_context,
        runtime_plugin_state,
    )
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime_with_plugin_state(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
    setup_context: &SetupContext,
    runtime_plugin_state: RuntimePluginState,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let RuntimePluginState {
        feature_config,
        tool_registry,
        plugin_registry,
    } = runtime_plugin_state;
    plugin_registry.initialize()?;
    let mut runtime = ConversationRuntime::new_with_features(
        session,
        ProviderRuntimeClient::new(
            session_id,
            model,
            enable_tools,
            emit_output,
            allowed_tools.clone(),
            tool_registry.clone(),
            progress_reporter,
            setup_context,
        )?,
        CliToolExecutor::new(allowed_tools.clone(), emit_output, tool_registry.clone()),
        permission_policy(
            permission_mode,
            &feature_config,
            &tool_registry,
            setup_context.active_profile.profile.supports_tools,
        )
        .map_err(std::io::Error::other)?,
        system_prompt,
        &feature_config,
    );
    if emit_output {
        runtime = runtime.with_hook_progress_reporter(Box::new(CliHookProgressReporter));
    }
    Ok(BuiltRuntime::new(
        runtime,
        plugin_registry,
        setup_context.active_profile.clone(),
    ))
}

struct CliHookProgressReporter;

impl runtime::HookProgressReporter for CliHookProgressReporter {
    fn on_event(&mut self, event: &runtime::HookProgressEvent) {
        match event {
            runtime::HookProgressEvent::Started {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
            runtime::HookProgressEvent::Completed {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook done {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
            runtime::HookProgressEvent::Cancelled {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook cancelled {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
        }
    }
}

struct CliPermissionPrompter {
    current_mode: PermissionMode,
}

impl CliPermissionPrompter {
    fn new(current_mode: PermissionMode) -> Self {
        Self { current_mode }
    }
}

impl runtime::PermissionPrompter for CliPermissionPrompter {
    fn decide(
        &mut self,
        request: &runtime::PermissionRequest,
    ) -> runtime::PermissionPromptDecision {
        println!();
        println!("Permission approval required");
        println!("  Tool             {}", request.tool_name);
        println!("  Current mode     {}", self.current_mode.as_str());
        println!("  Required mode    {}", request.required_mode.as_str());
        if let Some(reason) = &request.reason {
            println!("  Reason           {reason}");
        }
        println!("  Input            {}", request.input);
        print!("Approve this tool call? [y/N]: ");
        let _ = io::stdout().flush();

        let mut response = String::new();
        match io::stdin().read_line(&mut response) {
            Ok(_) => {
                let normalized = response.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "y" | "yes") {
                    runtime::PermissionPromptDecision::Allow
                } else {
                    runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied by user approval prompt",
                            request.tool_name
                        ),
                    }
                }
            }
            Err(error) => runtime::PermissionPromptDecision::Deny {
                reason: format!("permission approval failed: {error}"),
            },
        }
    }
}

struct ProviderRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: OpenAiCompatClient,
    model: String,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    progress_reporter: Option<InternalPromptProgressReporter>,
    supports_streaming: bool,
}

impl ProviderRuntimeClient {
    fn new(
        session_id: &str,
        model: String,
        enable_tools: bool,
        emit_output: bool,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        progress_reporter: Option<InternalPromptProgressReporter>,
        setup_context: &SetupContext,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let launch = ProviderLauncher::prepare(&setup_context.active_profile)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let tools_enabled = enable_tools && launch.supports_tools;
        Ok(Self {
            runtime: tokio::runtime::Runtime::new()?,
            client: OpenAiCompatClient::new(
                launch.api_key,
                OpenAiCompatConfig {
                    provider_name: "Kcode",
                    api_key_env: PRIMARY_API_KEY_ENV,
                    base_url_env: PRIMARY_BASE_URL_ENV,
                    default_base_url: "",
                },
            )
            .with_base_url(launch.base_url)
            .with_request_timeout(Duration::from_millis(launch.request_timeout_ms))
            .with_retry_policy(
                launch.max_retries,
                Duration::from_millis(200),
                Duration::from_secs(2),
            ),
            model,
            enable_tools: tools_enabled,
            emit_output,
            allowed_tools,
            tool_registry,
            progress_reporter,
            supports_streaming: launch.supports_streaming,
        })
    }
}
