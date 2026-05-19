fn json_command_descriptor(descriptor: &CommandDescriptor) -> serde_json::Value {
    json!({
        "id": descriptor.id,
        "name": descriptor.name,
        "usage": command_descriptor_usage(descriptor),
        "kind": command_kind_label(descriptor.kind),
        "source": command_source_label(descriptor.source),
        "scope": command_scope_label(descriptor.scope),
        "category": slash_command_category(&descriptor.name),
        "description": descriptor.description,
        "argumentHint": descriptor.argument_hint,
        "aliases": descriptor.aliases,
        "resumeSupported": descriptor.resume_supported,
        "remoteSafe": descriptor.remote_safe,
        "channelSafe": descriptor.channel_safe,
        "enabled": descriptor.enabled,
        "wiring": command_wiring_label(descriptor.wiring),
        "experimental": !descriptor.visibility_tags.iter().any(|tag| tag == "v1-core"),
        "tags": descriptor.visibility_tags,
    })
}

fn json_status(cli: &LiveCli) -> serde_json::Value {
    let status = status_context(Some(&cli.session.path)).ok();
    json!({
        "model": cli.model,
        "profile": cli.active_profile.profile_name,
        "sessionId": cli.session.id,
        "permissionMode": cli.permission_mode.as_str(),
        "supportsTools": cli.active_profile.profile.supports_tools,
        "supportsStreaming": cli.active_profile.profile.supports_streaming,
        "messageCount": cli.runtime.session().messages.len(),
        "turns": cli.runtime.usage().turns(),
        "estimatedTokens": cli.runtime.estimated_tokens(),
        "sessionPath": cli.session.path.display().to_string(),
        "cwd": status.as_ref().map(|item| item.cwd.display().to_string()),
        "projectRoot": status.as_ref().and_then(|item| {
            item.project_root.as_ref().map(|path| path.display().to_string())
        }),
        "gitBranch": status.as_ref().and_then(|item| item.git_branch.clone()),
        "gitState": status.as_ref().map(|item| item.git_summary.headline()),
        "controls": json_control_status(cli, status.as_ref()),
    })
}

fn json_control_status(cli: &LiveCli, status: Option<&StatusContext>) -> serde_json::Value {
    let tool_names = GlobalToolRegistry::builtin()
        .definitions(cli.allowed_tools.as_ref())
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();
    let tools_enabled = cli.active_profile.profile.supports_tools;
    json!({
        "model": {
            "current": cli.model,
            "profile": cli.active_profile.profile_name,
        },
        "permission": {
            "mode": cli.permission_mode.as_str(),
        },
        "mcp": {
            "available": tools_enabled,
            "command": if tools_enabled { Some("/mcp list") } else { None },
        },
        "memory": {
            "projectFiles": status.map(|item| item.memory_file_count).unwrap_or_default(),
            "estimatedTokens": cli.runtime.estimated_tokens(),
            "command": "/memory",
        },
        "agent": {
            "available": tools_enabled && tool_names.iter().any(|name| name == "Agent"),
            "tool": "Agent",
        },
        "goal": {
            "available": false,
            "status": "not-reported",
        },
    })
}

fn command_kind_label(kind: commands::CommandKind) -> &'static str {
    match kind {
        commands::CommandKind::Prompt => "prompt",
        commands::CommandKind::Local => "local",
        commands::CommandKind::LocalUi => "local-ui",
    }
}

fn command_source_label(source: commands::CommandSource) -> &'static str {
    match source {
        commands::CommandSource::Builtin => "builtin",
        commands::CommandSource::Skills => "skills",
        commands::CommandSource::Plugins => "plugin",
        commands::CommandSource::Workflow => "workflow",
        commands::CommandSource::Mcp => "mcp",
    }
}

fn command_scope_label(scope: CommandScope) -> &'static str {
    match scope {
        CommandScope::Process => "process",
        CommandScope::Session => "session",
    }
}

fn command_wiring_label(wiring: commands::CommandWiring) -> &'static str {
    match wiring {
        commands::CommandWiring::Executable => "executable",
        commands::CommandWiring::Informational => "informational",
        commands::CommandWiring::Hidden => "hidden",
    }
}
