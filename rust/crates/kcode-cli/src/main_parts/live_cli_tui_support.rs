struct TuiPermissionPrompter {
    current_mode: PermissionMode,
    allow_for_turn: BTreeSet<String>,
    deny_for_turn: BTreeSet<String>,
}

impl TuiPermissionPrompter {
    fn new(current_mode: PermissionMode) -> Self {
        Self {
            current_mode,
            allow_for_turn: BTreeSet::new(),
            deny_for_turn: BTreeSet::new(),
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
}

impl runtime::PermissionPrompter for TuiPermissionPrompter {
    fn decide(&mut self, request: &runtime::PermissionRequest) -> runtime::PermissionPromptDecision {
        if let Some(decision) = self.cached_decision(request) {
            return decision;
        }

        match prompt_tui_permission(request, self.current_mode) {
            Ok(TuiPermissionChoice::AllowOnce) => runtime::PermissionPromptDecision::Allow,
            Ok(TuiPermissionChoice::AllowForTurn) => {
                self.allow_for_turn.insert(permission_memory_key(request));
                runtime::PermissionPromptDecision::Allow
            }
            Ok(TuiPermissionChoice::DenyOnce) => runtime::PermissionPromptDecision::Deny {
                reason: format!("tool '{}' denied from the TUI permission dialog", request.tool_name),
            },
            Ok(TuiPermissionChoice::DenyForTurn) => {
                self.deny_for_turn.insert(permission_memory_key(request));
                runtime::PermissionPromptDecision::Deny {
                    reason: format!(
                        "tool '{}' denied for the rest of the current turn",
                        request.tool_name
                    ),
                }
            }
            Err(error) => runtime::PermissionPromptDecision::Deny {
                reason: format!("TUI permission prompt failed: {error}"),
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_tui_runtime(
    session: Session,
    session_id: &str,
    model: String,
    model_override: Option<&str>,
    profile_override: Option<&str>,
    system_prompt: Vec<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let setup_context = load_setup_context(
        SetupMode::Interactive,
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
        setup_context.active_profile.model.clone(),
        system_prompt,
        true,
        false,
        allowed_tools,
        permission_mode,
        None,
        &setup_context,
        runtime_plugin_state,
    )
}

impl LiveCli {
    fn tui_model_candidates(&self) -> Vec<String> {
        let mut candidates = merge_model_candidates(
            &self.model,
            &self.active_profile.profile.default_model,
            fetch_provider_models(&self.active_profile).unwrap_or_default(),
        );
        if candidates.is_empty() {
            candidates.push(self.model.clone());
        }
        candidates
    }

    fn tui_session_command_result(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => Ok(tui_text_result(render_session_list(&self.session.id)?)),
            Some("switch") => {
                let Some(target) = target else {
                    return Ok(tui_text_result("Usage: /session switch <session-id>".to_string()));
                };
                let handle = resolve_session_reference(target)?;
                let session = Session::load_from_path(&handle.path)?;
                let message_count = session.messages.len();
                self.session = SessionHandle {
                    id: session.session_id.clone(),
                    path: handle.path,
                };
                self.replace_tui_runtime(
                    session,
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                Ok(tui_text_result(format!(
                    "Session switched\n  Active session   {}\n  File             {}\n  Messages         {}",
                    self.session.id,
                    self.session.path.display(),
                    message_count,
                )))
            }
            Some("fork") => {
                let forked = self.runtime.fork_session(target.map(ToOwned::to_owned));
                let parent_session_id = self.session.id.clone();
                let handle = create_managed_session_handle(&forked.session_id)?;
                let branch_name = forked.fork.as_ref().and_then(|fork| fork.branch_name.clone());
                let forked = forked.with_persistence_path(handle.path.clone());
                let message_count = forked.messages.len();
                forked.save_to_path(&handle.path)?;
                self.session = handle;
                self.replace_tui_runtime(
                    forked,
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                Ok(tui_text_result(format!(
                    "Session forked\n  Parent session   {}\n  Active session   {}\n  Branch           {}\n  File             {}\n  Messages         {}",
                    parent_session_id,
                    self.session.id,
                    branch_name.as_deref().unwrap_or("(unnamed)"),
                    self.session.path.display(),
                    message_count,
                )))
            }
            Some(other) => Ok(tui_text_result(format!(
                "Unknown /session action '{other}'. Use /session list, /session switch <session-id>, or /session fork [branch-name]."
            ))),
        }
    }

    fn tui_plugins_command_result(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader.load()?;
        let mut manager = build_plugin_manager(&cwd, &loader, &runtime_config);
        let result = handle_plugins_slash_command(action, target, &mut manager)?;
        if result.reload_runtime {
            self.reload_runtime_features()?;
        }
        Ok(tui_text_result(result.message))
    }

    fn tui_btw_result(
        &self,
        question: Option<&str>,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        let Some(question) = question.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(tui_text_result(render_btw_usage()));
        };
        Ok(tui_text_result(self.run_internal_prompt_text(question, false)?))
    }
}

fn merge_model_candidates(
    current_model: &str,
    default_model: &str,
    fetched_models: Vec<String>,
) -> Vec<String> {
    let mut merged = Vec::new();
    push_unique_model(&mut merged, current_model);
    push_unique_model(&mut merged, default_model);
    for model in fetched_models {
        push_unique_model(&mut merged, &model);
    }
    merged
}

fn push_unique_model(target: &mut Vec<String>, candidate: &str) {
    let candidate = candidate.trim();
    if candidate.is_empty() || target.iter().any(|existing| existing == candidate) {
        return;
    }
    target.push(candidate.to_string());
}

fn fetch_provider_models(
    active_profile: &ResolvedProviderProfile,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let launch = ProviderLauncher::prepare(active_profile)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let endpoint = format!("{}/models", launch.base_url.trim_end_matches('/'));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let response = runtime.block_on(async {
        reqwest::Client::new()
            .get(endpoint)
            .bearer_auth(&launch.api_key)
            .timeout(Duration::from_secs(2))
            .send()
            .await?
            .error_for_status()
    })?;
    let payload = runtime.block_on(response.json::<serde_json::Value>())?;
    let models = payload
        .get("data")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(|value| value.as_str()))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    Ok(models)
}

fn tui_text_result(text: String) -> tui::repl::BackendResult {
    tui::repl::BackendResult {
        messages: vec![tui::repl::RenderableMessage::AssistantText {
            text,
            streaming: false,
        }],
        ..tui::repl::BackendResult::default()
    }
}

fn backend_result_from_session_slice(
    messages: &[ConversationMessage],
    previous_message_count: usize,
) -> tui::repl::BackendResult {
    let renderable = messages
        .iter()
        .skip(previous_message_count)
        .flat_map(session_message_to_renderable)
        .collect();

    tui::repl::BackendResult {
        messages: renderable,
        ..tui::repl::BackendResult::default()
    }
}

fn session_message_to_renderable(
    message: &ConversationMessage,
) -> Vec<tui::repl::RenderableMessage> {
    let mut renderable = Vec::new();

    match message.role {
        MessageRole::System | MessageRole::User => {}
        MessageRole::Assistant => {
            for block in &message.blocks {
                match block {
                    ContentBlock::Text { text } => {
                        if !text.trim().is_empty() {
                            renderable.push(tui::repl::RenderableMessage::AssistantText {
                                text: text.clone(),
                                streaming: false,
                            });
                        }
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        renderable.push(tui::repl::RenderableMessage::ToolCall {
                            name: name.clone(),
                            input: input.clone(),
                            status: tui::repl::ToolStatus::Completed,
                        });
                    }
                    ContentBlock::ToolResult {
                        tool_name,
                        output,
                        is_error,
                        ..
                    } => {
                        renderable.push(tui::repl::RenderableMessage::ToolResult {
                            name: tool_name.clone(),
                            output: output.clone(),
                            is_error: *is_error,
                        });
                    }
                }
            }
        }
        MessageRole::Tool => {
            for block in &message.blocks {
                if let ContentBlock::ToolResult {
                    tool_name,
                    output,
                    is_error,
                    ..
                } = block
                {
                    renderable.push(tui::repl::RenderableMessage::ToolResult {
                        name: tool_name.clone(),
                        output: output.clone(),
                        is_error: *is_error,
                    });
                }
            }
        }
    }

    renderable
}

#[cfg(test)]
mod live_cli_tui_support_tests {
    use super::merge_model_candidates;

    #[test]
    fn merge_model_candidates_keeps_current_default_and_deduplicates() {
        let merged = merge_model_candidates(
            "gpt-5.4",
            "gpt-5.4-mini",
            vec![
                "gpt-5.4-mini".to_string(),
                "gpt-5.4".to_string(),
                "gpt-5.2".to_string(),
            ],
        );

        assert_eq!(merged, vec!["gpt-5.4", "gpt-5.4-mini", "gpt-5.2"]);
    }
}
