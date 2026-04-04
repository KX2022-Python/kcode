fn render_repl_help() -> String {
    render_repl_help_for_profile(true)
}

fn render_repl_help_for_profile(profile_supports_tools: bool) -> String {
    [
        "REPL".to_string(),
        "  /exit                Quit the REPL".to_string(),
        "  /quit                Quit the REPL".to_string(),
        "  Up/Down              Navigate prompt history".to_string(),
        "  Tab                  Complete commands, modes, and recent sessions".to_string(),
        "  Ctrl-C               Clear input (or exit on empty prompt)".to_string(),
        "  Shift+Enter/Ctrl+J   Insert a newline".to_string(),
        "  Auto-save            .kcode/sessions/<session-id>.jsonl".to_string(),
        "  Resume latest        /resume latest".to_string(),
        "  Browse sessions      /session list".to_string(),
        String::new(),
        render_slash_command_help_for_context(&CommandRegistryContext::for_surface(
            CommandSurface::CliLocal,
            profile_supports_tools,
        )),
    ]
    .join("\n")
}

fn print_status_snapshot(
    model: &str,
    model_override: Option<&str>,
    profile_override: Option<&str>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = load_setup_context(
        SetupMode::Status,
        model_override,
        profile_override,
        permission_mode,
        None,
    )?;
    println!(
        "{}",
        format_status_report(
            model,
            Some(&setup.active_profile),
            StatusUsage {
                message_count: 0,
                turns: 0,
                latest: TokenUsage::default(),
                cumulative: TokenUsage::default(),
                estimated_tokens: 0,
            },
            permission_mode.as_str(),
            &status_context(None)?,
        )
    );
    Ok(())
}

fn status_context(
    session_path: Option<&Path>,
) -> Result<StatusContext, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let discovered_config_files = loader.discover().len();
    let runtime_config = loader.load()?;
    let project_context = ProjectContext::discover_with_git(&cwd, DEFAULT_DATE)?;
    let (project_root, git_branch) =
        parse_git_status_metadata(project_context.git_status.as_deref());
    let git_summary = parse_git_workspace_summary(project_context.git_status.as_deref());
    let sandbox_status = resolve_sandbox_status(runtime_config.sandbox(), &cwd);
    Ok(StatusContext {
        cwd,
        session_path: session_path.map(Path::to_path_buf),
        loaded_config_files: runtime_config.loaded_entries().len(),
        discovered_config_files,
        memory_file_count: project_context.instruction_files.len(),
        project_root,
        git_branch,
        git_summary,
        sandbox_status,
    })
}

fn format_status_report(
    model: &str,
    active_profile: Option<&ResolvedProviderProfile>,
    usage: StatusUsage,
    permission_mode: &str,
    context: &StatusContext,
) -> String {
    let provider_section = active_profile
        .map(format_provider_status_section)
        .unwrap_or_else(|| {
            "Provider
  Profile          <unknown>
  Endpoint         <unknown>"
                .to_string()
        });
    [
        format!(
            "Status
  Profile          {}
  Model            {model}
  Permission mode  {permission_mode}
  Messages         {}
  Turns            {}
  Estimated tokens {}",
            active_profile
                .map(|profile| profile.profile_name.as_str())
                .unwrap_or("unknown"),
            usage.message_count,
            usage.turns,
            usage.estimated_tokens,
        ),
        provider_section,
        format!(
            "Usage
  Latest total     {}
  Cumulative input {}
  Cumulative output {}
  Cumulative total {}",
            usage.latest.total_tokens(),
            usage.cumulative.input_tokens,
            usage.cumulative.output_tokens,
            usage.cumulative.total_tokens(),
        ),
        format!(
            "Workspace
  Cwd              {}
  Project root     {}
  Git branch       {}
  Git state        {}
  Changed files    {}
  Staged           {}
  Unstaged         {}
  Untracked        {}
  Session          {}
  Config files     loaded {}/{}
  Memory files     {}
  Suggested flow   /status → /diff → /commit",
            context.cwd.display(),
            context
                .project_root
                .as_ref()
                .map_or_else(|| "unknown".to_string(), |path| path.display().to_string()),
            context.git_branch.as_deref().unwrap_or("unknown"),
            context.git_summary.headline(),
            context.git_summary.changed_files,
            context.git_summary.staged_files,
            context.git_summary.unstaged_files,
            context.git_summary.untracked_files,
            context.session_path.as_ref().map_or_else(
                || "live-repl".to_string(),
                |path| path.display().to_string()
            ),
            context.loaded_config_files,
            context.discovered_config_files,
            context.memory_file_count,
        ),
        format_sandbox_report(&context.sandbox_status),
    ]
    .join(
        "

",
    )
}

fn format_provider_status_section(active_profile: &ResolvedProviderProfile) -> String {
    format!(
        "Provider
  Profile          {}
  Profile source   {}
  Endpoint         {}
  Endpoint source  {}
  Model source     {}
  Supports tools   {}
  Supports stream  {}
  Credential env   {}
  Credential source {}",
        active_profile.profile_name,
        active_profile.profile_source.label(),
        active_profile
            .base_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("<unset>"),
        active_profile.base_url_source.label(),
        active_profile.model_source.label(),
        active_profile.profile.supports_tools,
        active_profile.profile.supports_streaming,
        active_profile.credential.env_name,
        active_profile.credential.source.label(),
    )
}

fn format_sandbox_report(status: &runtime::SandboxStatus) -> String {
    format!(
        "Sandbox
  Enabled           {}
  Active            {}
  Supported         {}
  In container      {}
  Requested ns      {}
  Active ns         {}
  Requested net     {}
  Active net        {}
  Filesystem mode   {}
  Filesystem active {}
  Allowed mounts    {}
  Markers           {}
  Fallback reason   {}",
        status.enabled,
        status.active,
        status.supported,
        status.in_container,
        status.requested.namespace_restrictions,
        status.namespace_active,
        status.requested.network_isolation,
        status.network_active,
        status.filesystem_mode.as_str(),
        status.filesystem_active,
        if status.allowed_mounts.is_empty() {
            "<none>".to_string()
        } else {
            status.allowed_mounts.join(", ")
        },
        if status.container_markers.is_empty() {
            "<none>".to_string()
        } else {
            status.container_markers.join(", ")
        },
        status
            .fallback_reason
            .clone()
            .unwrap_or_else(|| "<none>".to_string()),
    )
}

fn format_commit_preflight_report(branch: Option<&str>, summary: GitWorkspaceSummary) -> String {
    format!(
        "Commit
  Result           ready
  Branch           {}
  Workspace        {}
  Changed files    {}
  Action           create a git commit from the current workspace changes",
        branch.unwrap_or("unknown"),
        summary.headline(),
        summary.changed_files,
    )
}

fn format_commit_skipped_report() -> String {
    "Commit
  Result           skipped
  Reason           no workspace changes
  Action           create a git commit from the current workspace changes
  Next             /status to inspect context · /diff to inspect repo changes"
        .to_string()
}

fn print_sandbox_status_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader
        .load()
        .unwrap_or_else(|_| runtime::RuntimeConfig::empty());
    println!(
        "{}",
        format_sandbox_report(&resolve_sandbox_status(runtime_config.sandbox(), &cwd))
    );
    Ok(())
}

fn print_doctor(
    model_override: Option<&str>,
    profile_override: Option<&str>,
    fix: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if fix {
        run_doctor_fix(model_override, profile_override)?;
    } else {
        println!(
            "{}",
            render_doctor_report(model_override, profile_override)?
        );
    }
    Ok(())
}

