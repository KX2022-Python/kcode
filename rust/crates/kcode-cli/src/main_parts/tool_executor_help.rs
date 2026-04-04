struct CliToolExecutor {
    renderer: TerminalRenderer,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
}

impl CliToolExecutor {
    fn new(
        allowed_tools: Option<AllowedToolSet>,
        emit_output: bool,
        tool_registry: GlobalToolRegistry,
    ) -> Self {
        Self {
            renderer: TerminalRenderer::new(),
            emit_output,
            allowed_tools,
            tool_registry,
        }
    }
}

impl ToolExecutor for CliToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current --allowedTools setting"
            )));
        }
        let value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        match self.tool_registry.execute(tool_name, &value) {
            Ok(output) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &output, false);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|error| ToolError::new(error.to_string()))?;
                }
                Ok(output)
            }
            Err(error) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &error, true);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|stream_error| ToolError::new(stream_error.to_string()))?;
                }
                Err(ToolError::new(error))
            }
        }
    }
}

fn permission_policy(
    mode: PermissionMode,
    feature_config: &runtime::RuntimeFeatureConfig,
    tool_registry: &GlobalToolRegistry,
    profile_supports_tools: bool,
) -> Result<PermissionPolicy, String> {
    let policy =
        PermissionPolicy::new(mode).with_permission_rules(feature_config.permission_rules());
    if !profile_supports_tools {
        return Ok(policy.with_tool_use_disabled(
            "tool use is unavailable because the active profile disables tools",
        ));
    }

    Ok(tool_registry.permission_specs(None)?.into_iter().fold(
        policy,
        |policy, (name, required_permission)| {
            policy.with_tool_requirement(name, required_permission)
        },
    ))
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

fn help_profile_supports_tools(profile_override: Option<&str>) -> bool {
    load_setup_context(
        SetupMode::Config,
        None,
        profile_override,
        default_permission_mode(),
        None,
    )
    .map(|setup| setup.active_profile.profile.supports_tools)
    .unwrap_or(true)
}

#[allow(clippy::too_many_lines)]
fn print_help_to_for_profile(out: &mut impl Write, profile_supports_tools: bool) -> io::Result<()> {
    let context =
        CommandRegistryContext::for_surface(CommandSurface::CliLocal, profile_supports_tools);
    let snapshot = build_command_registry_snapshot(&context, &[]);
    let mcp_available = snapshot
        .process_commands
        .iter()
        .any(|descriptor| descriptor.name == "mcp");
    let resume_commands = snapshot
        .session_commands
        .iter()
        .filter(|descriptor| descriptor.resume_supported)
        .map(command_descriptor_usage)
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(out, "{CLI_NAME} v{VERSION}")?;
    writeln!(out)?;
    writeln!(out, "Usage:")?;
    if profile_supports_tools {
        writeln!(
            out,
            "  {CLI_NAME} [--model MODEL] [--profile PROFILE] [--allowedTools TOOL[,TOOL...]]"
        )?;
    } else {
        writeln!(out, "  {CLI_NAME} [--model MODEL] [--profile PROFILE]")?;
    }
    writeln!(out, "      Start the interactive REPL")?;
    writeln!(
        out,
        "  {CLI_NAME} [--model MODEL] [--profile PROFILE] [--output-format text|json] prompt TEXT"
    )?;
    writeln!(out, "      Send one prompt and exit")?;
    writeln!(
        out,
        "  {CLI_NAME} [--model MODEL] [--profile PROFILE] [--output-format text|json] TEXT"
    )?;
    writeln!(out, "      Shorthand non-interactive prompt mode")?;
    writeln!(
        out,
        "  {CLI_NAME} --resume [SESSION.jsonl|session-id|latest] [/status] [/compact] [...]"
    )?;
    writeln!(
        out,
        "      Inspect or maintain a saved session without entering the REPL"
    )?;
    writeln!(out, "  {CLI_NAME} help")?;
    writeln!(out, "      Alias for --help")?;
    writeln!(out, "  {CLI_NAME} version")?;
    writeln!(out, "      Alias for --version")?;
    writeln!(out, "  {CLI_NAME} status")?;
    writeln!(
        out,
        "      Show the current local workspace status snapshot"
    )?;
    writeln!(out, "  {CLI_NAME} sandbox")?;
    writeln!(out, "      Show the current sandbox isolation snapshot")?;
    writeln!(out, "  {CLI_NAME} agents")?;
    if mcp_available {
        writeln!(out, "  {CLI_NAME} mcp")?;
    }
    writeln!(out, "  {CLI_NAME} skills")?;
    writeln!(out, "  {CLI_NAME} commands [show [local|bridge]]")?;
    writeln!(out, "  {CLI_NAME} profile [list|show [name]]")?;
    writeln!(
        out,
        "  {CLI_NAME} system-prompt [--cwd PATH] [--date YYYY-MM-DD]"
    )?;
    writeln!(out, "  {CLI_NAME} login")?;
    writeln!(out, "  {CLI_NAME} logout")?;
    writeln!(out, "  {CLI_NAME} init")?;
    writeln!(out)?;
    writeln!(out, "Flags:")?;
    writeln!(
        out,
        "  --model MODEL              Override the active model"
    )?;
    writeln!(
        out,
        "  --profile PROFILE          Override the active provider profile"
    )?;
    writeln!(
        out,
        "  --output-format FORMAT     Non-interactive output format: text or json"
    )?;
    writeln!(
        out,
        "  --permission-mode MODE     Set read-only, workspace-write, or danger-full-access"
    )?;
    writeln!(
        out,
        "  --dangerously-skip-permissions  Skip all permission checks"
    )?;
    if profile_supports_tools {
        writeln!(
            out,
            "  --allowedTools TOOLS       Restrict enabled tools (repeatable; comma-separated aliases supported)"
        )?;
    }
    writeln!(
        out,
        "  --version, -V              Print version and build information locally"
    )?;
    writeln!(out)?;
    writeln!(out, "Interactive slash commands:")?;
    writeln!(out, "{}", render_slash_command_help_for_context(&context))?;
    writeln!(out)?;
    writeln!(out, "Resume-safe commands: {resume_commands}")?;
    writeln!(out)?;
    writeln!(out, "Session shortcuts:")?;
    writeln!(
        out,
        "  REPL turns auto-save to .kcode/sessions/<session-id>.{PRIMARY_SESSION_EXTENSION}"
    )?;
    writeln!(
        out,
        "  Use `{LATEST_SESSION_REFERENCE}` with --resume, /resume, or /session switch to target the newest saved session"
    )?;
    writeln!(
        out,
        "  Use /session list in the REPL to browse managed sessions"
    )?;
    writeln!(out, "Examples:")?;
    writeln!(
        out,
        "  {CLI_NAME} --model gpt-4.1-mini --profile custom \"summarize this repo\""
    )?;
    writeln!(
        out,
        "  {CLI_NAME} --output-format json prompt \"explain src/main.rs\""
    )?;
    if profile_supports_tools {
        writeln!(
            out,
            "  {CLI_NAME} --allowedTools read,glob \"summarize Cargo.toml\""
        )?;
    }
    writeln!(out, "  {CLI_NAME} --resume {LATEST_SESSION_REFERENCE}")?;
    writeln!(
        out,
        "  {CLI_NAME} --resume {LATEST_SESSION_REFERENCE} /status /diff /export notes.txt"
    )?;
    writeln!(out, "  {CLI_NAME} agents")?;
    if mcp_available {
        writeln!(out, "  {CLI_NAME} mcp show my-server")?;
    }
    writeln!(out, "  {CLI_NAME} commands show bridge")?;
    writeln!(out, "  {CLI_NAME} profile show nvidia")?;
    writeln!(out, "  {CLI_NAME} profile list")?;
    writeln!(out, "  {CLI_NAME} /skills")?;
    writeln!(out, "  {CLI_NAME} init")?;
    writeln!(out, "  {CLI_NAME} doctor")?;
    writeln!(out, "  {CLI_NAME} config show")?;
    Ok(())
}

fn print_help_to(out: &mut impl Write) -> io::Result<()> {
    print_help_to_for_profile(out, true)
}

fn print_help_to_with_profile_override(
    out: &mut impl Write,
    profile_override: Option<&str>,
) -> io::Result<()> {
    print_help_to_for_profile(out, help_profile_supports_tools(profile_override))
}

fn print_help(profile_override: Option<&str>) {
    let _ = print_help_to_with_profile_override(&mut io::stdout(), profile_override);
}
