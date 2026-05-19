#[allow(clippy::too_many_arguments)]
fn run_ts_tui_engine(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut engine =
        TsTuiEngine::new(model, model_explicit, profile, allowed_tools, permission_mode)?;
    engine.run()
}

struct TsTuiEngine {
    cli: LiveCli,
    input_rx: Receiver<String>,
}

impl TsTuiEngine {
    fn new(
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cli = LiveCli::new(
            model,
            model_explicit,
            profile,
            true,
            allowed_tools,
            permission_mode,
            None,
        )?;
        Ok(Self {
            cli,
            input_rx: spawn_ts_tui_input_reader(),
        })
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.emit_bootstrap()?;
        while let Ok(line) = self.input_rx.recv() {
            if line.trim().is_empty() {
                continue;
            }
            match TsEngineInput::parse(&line) {
                Ok(TsEngineInput::Exit) => break,
                Ok(event) => self.handle_input(event)?,
                Err(message) => self.emit_error(None, message, true)?,
            }
        }
        self.cli.persist_session()?;
        self.emit(json!({
            "type": "done",
            "status": "complete",
            "sessionId": self.cli.session.id,
        }))
    }

    fn handle_input(&mut self, input: TsEngineInput) -> Result<(), Box<dyn std::error::Error>> {
        match input {
            TsEngineInput::Submit { id, text } => self.handle_submit(&id, &text),
            TsEngineInput::Slash { id, command } => self.handle_slash(&id, &command),
            TsEngineInput::CommandList { id, query } => {
                self.emit_command_list(&id, query.as_deref())
            }
            TsEngineInput::PermissionResponse { id, decision } => self.emit(json!({
                "type": "permission",
                "id": id,
                "status": "ignored",
                "decision": decision,
                "message": "No permission prompt is active.",
            })),
            TsEngineInput::Status { id } => self.emit_status(Some(&id)),
            TsEngineInput::Cancel { id } => self.emit(json!({
                "type": "cancel",
                "id": id,
                "status": "not-running",
                "message": "No backend turn is active.",
            })),
            TsEngineInput::Exit => Ok(()),
        }
    }

    fn emit_bootstrap(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.emit(json!({
            "type": "bootstrap",
            "message": "Kcode TS TUI connected to Rust engine.",
            "status": json_status(&self.cli),
        }))?;
        self.emit_command_list("bootstrap", None)?;
        self.emit_status(None)
    }

    fn handle_submit(
        &mut self,
        id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.emit(json!({ "type": "user_submit", "id": id, "text": text }))?;
        self.emit(json!({ "type": "progress", "id": id, "phase": "model" }))?;
        match run_ts_tui_turn(&mut self.cli, &self.input_rx, text) {
            Ok(result) => self.emit_backend_result(Some(id), result),
            Err(error) => self.emit_error(Some(id), error.to_string(), true),
        }
    }

    fn handle_slash(
        &mut self,
        id: &str,
        raw_command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.emit(json!({
            "type": "user_submit",
            "id": id,
            "text": raw_command,
            "slash": true,
        }))?;
        match raw_command.trim() {
            "/exit" | "/quit" => self.emit(json!({
                "type": "done",
                "id": id,
                "status": "exit-requested",
            })),
            "/commands" | "/commands show" => self.emit_command_list(id, None),
            _ => match self.cli.handle_tui_command(raw_command) {
                Ok(result) => self.emit_backend_result(Some(id), result),
                Err(error) => self.emit_error(Some(id), error.to_string(), true),
            },
        }
    }

    fn emit_backend_result(
        &mut self,
        id: Option<&str>,
        result: tui::repl::BackendResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for message in result.messages {
            self.emit_renderable(id, message)?;
        }
        if result.input_tokens.is_some() || result.output_tokens.is_some() {
            self.emit(json!({
                "type": "status",
                "id": id,
                "usage": {
                    "inputTokens": result.input_tokens,
                    "outputTokens": result.output_tokens,
                },
            }))?;
        }
        self.emit_status(id)?;
        self.emit(json!({ "type": "done", "id": id, "status": "complete" }))
    }

    fn emit_renderable(
        &mut self,
        id: Option<&str>,
        message: tui::repl::RenderableMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match message {
            tui::repl::RenderableMessage::User { text } => {
                self.emit(json!({ "type": "user_submit", "id": id, "text": text }))
            }
            tui::repl::RenderableMessage::AssistantText { text, streaming } => self.emit(json!({
                "type": "assistant_delta",
                "id": id,
                "text": text,
                "delta": streaming,
            })),
            tui::repl::RenderableMessage::AssistantThinking { text } => {
                self.emit(json!({ "type": "thinking", "id": id, "text": text }))
            }
            tui::repl::RenderableMessage::ToolCall { name, input, status } => {
                self.emit(json!({
                    "type": "tool",
                    "id": id,
                    "event": "call",
                    "name": name,
                    "input": input,
                    "status": tool_status_label(status),
                }))
            }
            tui::repl::RenderableMessage::ToolResult {
                name,
                output,
                is_error,
            } => self.emit(json!({
                "type": "tool",
                "id": id,
                "event": "result",
                "name": name,
                "output": output,
                "isError": is_error,
            })),
            tui::repl::RenderableMessage::System { message, level } => self.emit(json!({
                "type": "status",
                "id": id,
                "level": sys_level_label(level),
                "message": message,
            })),
            tui::repl::RenderableMessage::Error { message } => {
                self.emit_error(id, message, true)
            }
            tui::repl::RenderableMessage::Usage {
                input_tokens,
                output_tokens,
                cost,
            } => self.emit(json!({
                "type": "status",
                "id": id,
                "usage": {
                    "inputTokens": input_tokens,
                    "outputTokens": output_tokens,
                    "cost": cost,
                },
            })),
            tui::repl::RenderableMessage::CompactBoundary => self.emit(json!({
                "type": "status",
                "id": id,
                "message": "compact boundary",
            })),
        }
    }

    fn emit_command_list(
        &mut self,
        id: &str,
        query: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = CommandRegistryContext::for_surface(
            CommandSurface::CliLocal,
            self.cli.active_profile.profile.supports_tools,
        );
        let snapshot = build_command_registry_snapshot(&context, &[]);
        let query = query.map(str::trim).filter(|value| !value.is_empty());
        let commands = snapshot
            .session_commands
            .iter()
            .filter(|descriptor| descriptor.source == commands::CommandSource::Builtin)
            .filter(|descriptor| command_matches_query(descriptor, query))
            .map(json_command_descriptor)
            .collect::<Vec<_>>();
        self.emit(json!({
            "type": "commands",
            "id": id,
            "safetyProfile": snapshot.safety_profile,
            "commands": commands,
        }))
    }

    fn emit_status(&mut self, id: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        self.emit(json!({ "type": "status", "id": id, "status": json_status(&self.cli) }))
    }

    fn emit_error(
        &mut self,
        id: Option<&str>,
        message: String,
        recoverable: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.emit(json!({
            "type": "error",
            "id": id,
            "message": message,
            "recoverable": recoverable,
        }))
    }

    fn emit(&mut self, value: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        emit_engine_json(value)
    }
}

fn spawn_ts_tui_input_reader() -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    rx
}

fn emit_engine_json(value: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    serde_json::to_writer(&mut stdout, &value)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}
