struct TsTuiPermissionPrompter<'a> {
    input_rx: &'a Receiver<String>,
    allow_for_turn: BTreeSet<String>,
    deny_for_turn: BTreeSet<String>,
    next_id: u64,
}

impl<'a> TsTuiPermissionPrompter<'a> {
    fn new(input_rx: &'a Receiver<String>) -> Self {
        Self {
            input_rx,
            allow_for_turn: BTreeSet::new(),
            deny_for_turn: BTreeSet::new(),
            next_id: 0,
        }
    }

    fn cached_decision(
        &self,
        request: &runtime::PermissionRequest,
    ) -> Option<runtime::PermissionPromptDecision> {
        let key = permission_memory_key(request);
        if self.allow_for_turn.contains(&key) {
            return Some(runtime::PermissionPromptDecision::Allow);
        }
        if self.deny_for_turn.contains(&key) {
            return Some(runtime::PermissionPromptDecision::Deny {
                reason: format!(
                    "tool '{}' denied for the rest of the current turn",
                    request.tool_name
                ),
            });
        }
        None
    }

    fn next_permission_id(&mut self, tool_name: &str) -> String {
        self.next_id += 1;
        format!("perm-{}-{}", self.next_id, tool_name)
    }

    fn wait_for_decision(
        &mut self,
        permission_id: &str,
        request: &runtime::PermissionRequest,
    ) -> runtime::PermissionPromptDecision {
        loop {
            let line = match self.input_rx.recv() {
                Ok(line) => line,
                Err(_) => {
                    return runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied because TS TUI input closed",
                            request.tool_name
                        ),
                    };
                }
            };
            let parsed = match TsEngineInput::parse(&line) {
                Ok(parsed) => parsed,
                Err(message) => {
                    let _ = emit_engine_json(json!({
                        "type": "error",
                        "id": permission_id,
                        "message": message,
                        "recoverable": true,
                    }));
                    continue;
                }
            };
            match parsed {
                TsEngineInput::PermissionResponse { id, decision } if id == permission_id => {
                    return self.apply_decision(request, &decision);
                }
                TsEngineInput::PermissionResponse { id, .. } => {
                    let _ = emit_engine_json(json!({
                        "type": "permission",
                        "id": id,
                        "status": "ignored",
                        "message": "permission response did not match active request",
                    }));
                }
                TsEngineInput::Cancel { .. } | TsEngineInput::Exit => {
                    return runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied because the TS TUI cancelled the turn",
                            request.tool_name
                        ),
                    };
                }
                TsEngineInput::Status { id } => {
                    let _ = emit_engine_json(json!({
                        "type": "status",
                        "id": id,
                        "message": "turn is waiting for permission",
                    }));
                }
                TsEngineInput::CommandList { id, .. } => {
                    let _ = emit_engine_json(json!({
                        "type": "status",
                        "id": id,
                        "message": "command list is unavailable while permission is pending",
                    }));
                }
                TsEngineInput::Submit { id, .. } | TsEngineInput::Slash { id, .. } => {
                    let _ = emit_engine_json(json!({
                        "type": "status",
                        "id": id,
                        "message": "turn is waiting for permission",
                    }));
                }
            }
        }
    }

    fn apply_decision(
        &mut self,
        request: &runtime::PermissionRequest,
        decision: &str,
    ) -> runtime::PermissionPromptDecision {
        match decision {
            "allow_once" => runtime::PermissionPromptDecision::Allow,
            "allow_turn" => {
                self.allow_for_turn.insert(permission_memory_key(request));
                runtime::PermissionPromptDecision::Allow
            }
            "deny_turn" => {
                self.deny_for_turn.insert(permission_memory_key(request));
                runtime::PermissionPromptDecision::Deny {
                    reason: format!(
                        "tool '{}' denied for the rest of the current turn",
                        request.tool_name
                    ),
                }
            }
            _ => runtime::PermissionPromptDecision::Deny {
                reason: format!("tool '{}' denied from the TS TUI", request.tool_name),
            },
        }
    }
}

impl runtime::PermissionPrompter for TsTuiPermissionPrompter<'_> {
    fn decide(&mut self, request: &runtime::PermissionRequest) -> runtime::PermissionPromptDecision {
        if let Some(decision) = self.cached_decision(request) {
            return decision;
        }

        let permission_id = self.next_permission_id(&request.tool_name);
        let _ = emit_engine_json(json!({
            "type": "permission_request",
            "id": permission_id,
            "toolName": request.tool_name,
            "input": request.input,
            "currentMode": request.current_mode.as_str(),
            "requiredMode": request.required_mode.as_str(),
            "reason": request.reason,
            "status": "active",
        }));
        let decision = self.wait_for_decision(&permission_id, request);
        let status = match decision {
            runtime::PermissionPromptDecision::Allow => "allowed",
            runtime::PermissionPromptDecision::Deny { .. } => "denied",
        };
        let _ = emit_engine_json(json!({
            "type": "permission",
            "id": permission_id,
            "status": status,
        }));
        decision
    }
}

fn run_ts_tui_turn(
    cli: &mut LiveCli,
    input_rx: &Receiver<String>,
    input: &str,
) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
    let previous_message_count = cli.runtime.session().messages.len();
    let (mut runtime, hook_abort_monitor) = cli.prepare_tui_turn_runtime()?;
    let mut permission_prompter = TsTuiPermissionPrompter::new(input_rx);
    let result = runtime.run_turn(input, Some(&mut permission_prompter));
    hook_abort_monitor.stop();

    match result {
        Ok(summary) => {
            cli.replace_runtime(runtime)?;
            cli.persist_session()?;

            let mut backend = backend_result_from_session_slice(
                &cli.runtime.session().messages,
                previous_message_count,
            );
            if let Some(event) = summary.auto_compaction {
                backend.messages.push(tui::repl::RenderableMessage::System {
                    message: format_auto_compaction_notice(event.removed_message_count),
                    level: tui::repl::SysLevel::Info,
                });
            }
            if summary.compaction_circuit_tripped {
                backend.messages.push(tui::repl::RenderableMessage::System {
                    message: format!(
                        "自动压缩保护已触发：连续失败 {} 次，请运行 /compact 检查。",
                        MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES
                    ),
                    level: tui::repl::SysLevel::Warning,
                });
            }

            let usage = cli.runtime.usage().current_turn_usage();
            backend.input_tokens = Some(u64::from(usage.input_tokens));
            backend.output_tokens = Some(u64::from(usage.output_tokens));
            Ok(backend)
        }
        Err(error) => {
            runtime.shutdown_plugins()?;
            Err(Box::new(error))
        }
    }
}
