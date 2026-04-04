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
        model,
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
