struct TuiPermissionPrompter {
    current_mode: PermissionMode,
}

impl TuiPermissionPrompter {
    fn new(current_mode: PermissionMode) -> Self {
        Self { current_mode }
    }
}

impl runtime::PermissionPrompter for TuiPermissionPrompter {
    fn decide(&mut self, request: &runtime::PermissionRequest) -> runtime::PermissionPromptDecision {
        runtime::PermissionPromptDecision::Deny {
            reason: format!(
                "TUI 当前未实现交互式权限审批。工具 `{}` 需要 `{}`，当前模式是 `{}`。请先用 `/permissions workspace-write` 或 `/permissions danger-full-access` 后重试。",
                request.tool_name,
                request.required_mode.as_str(),
                self.current_mode.as_str(),
            ),
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
