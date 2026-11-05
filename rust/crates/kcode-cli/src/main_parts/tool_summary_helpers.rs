fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn collect_tool_uses(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .assistant_messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(json!({
                "id": id,
                "name": name,
                "input": input,
            })),
            _ => None,
        })
        .collect()
}

fn collect_tool_results(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .tool_results
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => Some(json!({
                "tool_use_id": tool_use_id,
                "tool_name": tool_name,
                "output": output,
                "is_error": is_error,
            })),
            _ => None,
        })
        .collect()
}

fn collect_prompt_cache_events(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .prompt_cache_events
        .iter()
        .map(|event| {
            json!({
                "unexpected": event.unexpected,
                "reason": event.reason,
                "previous_cache_read_input_tokens": event.previous_cache_read_input_tokens,
                "current_cache_read_input_tokens": event.current_cache_read_input_tokens,
                "token_drop": event.token_drop,
            })
        })
        .collect()
}

fn slash_command_completion_candidates_with_sessions(
    model: &str,
    profile_supports_tools: bool,
    active_session_id: Option<&str>,
    recent_session_ids: Vec<String>,
) -> Vec<String> {
    let mut completions = BTreeSet::new();
    let snapshot = build_command_registry_snapshot(
        &CommandRegistryContext::for_surface(CommandSurface::CliLocal, profile_supports_tools),
        &[],
    );
    let mut visible_commands = BTreeSet::new();

    for descriptor in &snapshot.session_commands {
        completions.insert(format!("/{}", descriptor.name));
        visible_commands.insert(format!("/{}", descriptor.name));
        for alias in &descriptor.aliases {
            completions.insert(format!("/{alias}"));
            visible_commands.insert(format!("/{alias}"));
        }
    }

    for candidate in [
        "/clear --confirm",
        "/config ",
        "/config env",
        "/config hooks",
        "/config model",
        "/config plugins",
        "/config profile",
        "/config provider",
        "/mcp ",
        "/mcp list",
        "/mcp show ",
        "/export ",
        "/model ",
        "/model opus",
        "/model sonnet",
        "/model haiku",
        "/permissions ",
        "/permissions read-only",
        "/permissions plan",
        "/permissions workspace-write",
        "/permissions danger-full-access",
        "/plugin list",
        "/plugin install ",
        "/plugin enable ",
        "/plugin disable ",
        "/plugin uninstall ",
        "/plugin update ",
        "/plugins list",
        "/resume ",
        "/session list",
        "/session switch ",
        "/session fork",
        "/agents help",
        "/mcp help",
        "/skills help",
    ] {
        let base = candidate.split_whitespace().next().unwrap_or(candidate);
        if visible_commands.contains(base) {
            completions.insert(candidate.to_string());
        }
    }

    if visible_commands.contains("/model") && !model.trim().is_empty() {
        completions.insert(format!("/model {}", resolve_model_alias(model)));
        completions.insert(format!("/model {model}"));
    }

    if let Some(active_session_id) = active_session_id.filter(|value| !value.trim().is_empty()) {
        if visible_commands.contains("/resume") {
            completions.insert(format!("/resume {active_session_id}"));
        }
        if visible_commands.contains("/session") {
            completions.insert(format!("/session switch {active_session_id}"));
        }
    }

    for session_id in recent_session_ids
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .take(10)
    {
        if visible_commands.contains("/resume") {
            completions.insert(format!("/resume {session_id}"));
        }
        if visible_commands.contains("/session") {
            completions.insert(format!("/session switch {session_id}"));
        }
    }

    completions.into_iter().collect()
}
