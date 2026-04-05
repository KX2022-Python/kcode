struct ResumeCommandOutcome {
    session: Session,
    message: Option<String>,
}

#[derive(Debug, Clone)]
struct StatusContext {
    cwd: PathBuf,
    session_path: Option<PathBuf>,
    loaded_config_files: usize,
    discovered_config_files: usize,
    memory_file_count: usize,
    project_root: Option<PathBuf>,
    git_branch: Option<String>,
    git_summary: GitWorkspaceSummary,
    sandbox_status: runtime::SandboxStatus,
}

#[derive(Debug, Clone, Copy)]
struct StatusUsage {
    message_count: usize,
    turns: u32,
    latest: TokenUsage,
    cumulative: TokenUsage,
    estimated_tokens: usize,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GitWorkspaceSummary {
    changed_files: usize,
    staged_files: usize,
    unstaged_files: usize,
    untracked_files: usize,
    conflicted_files: usize,
}

impl GitWorkspaceSummary {
    fn is_clean(self) -> bool {
        self.changed_files == 0
    }

    fn headline(self) -> String {
        if self.is_clean() {
            "clean".to_string()
        } else {
            let mut details = Vec::new();
            if self.staged_files > 0 {
                details.push(format!("{} staged", self.staged_files));
            }
            if self.unstaged_files > 0 {
                details.push(format!("{} unstaged", self.unstaged_files));
            }
            if self.untracked_files > 0 {
                details.push(format!("{} untracked", self.untracked_files));
            }
            if self.conflicted_files > 0 {
                details.push(format!("{} conflicted", self.conflicted_files));
            }
            format!(
                "dirty · {} files · {}",
                self.changed_files,
                details.join(", ")
            )
        }
    }
}

#[cfg(test)]
fn format_unknown_slash_command_message(name: &str) -> String {
    let suggestions = suggest_slash_commands(name);
    if suggestions.is_empty() {
        format!("unknown slash command: /{name}. Use /help to list available commands.")
    } else {
        format!(
            "unknown slash command: /{name}. Did you mean {}? Use /help to list available commands.",
            suggestions.join(", ")
        )
    }
}

fn format_model_report(model: &str, profile: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Model
  Active profile   {profile}
  Current model    {model}
  Session messages {message_count}
  Session turns    {turns}

Usage
  Inspect current model with /model
  Switch models with /model <name>"
    )
}

fn format_model_switch_report(
    previous: &str,
    next: &str,
    profile: &str,
    message_count: usize,
) -> String {
    format!(
        "Model updated
  Active profile   {profile}
  Previous         {previous}
  Current          {next}
  Preserved msgs   {message_count}"
    )
}

fn format_permissions_report(mode: &str) -> String {
    let modes = [
        ("read-only", "Read/search tools only", mode == "read-only"),
        (
            "workspace-write",
            "Edit files inside the workspace",
            mode == "workspace-write",
        ),
        (
            "danger-full-access",
            "Unrestricted tool access",
            mode == "danger-full-access",
        ),
    ]
    .into_iter()
    .map(|(name, description, is_current)| {
        let marker = if is_current {
            "● current"
        } else {
            "○ available"
        };
        format!("  {name:<18} {marker:<11} {description}")
    })
    .collect::<Vec<_>>()
    .join(
        "
",
    );

    format!(
        "Permissions
  Active mode      {mode}
  Mode status      live session default

Modes
{modes}

Usage
  Inspect current mode with /permissions
  Switch modes with /permissions <mode>"
    )
}

fn format_permissions_switch_report(previous: &str, next: &str) -> String {
    format!(
        "Permissions updated
  Result           mode switched
  Previous mode    {previous}
  Active mode      {next}
  Applies to       subsequent tool calls
  Usage            /permissions to inspect current mode"
    )
}

fn format_cost_report(usage: TokenUsage) -> String {
    format!(
        "Cost
  Input tokens     {}
  Output tokens    {}
  Cache create     {}
  Cache read       {}
  Total tokens     {}",
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_creation_input_tokens,
        usage.cache_read_input_tokens,
        usage.total_tokens(),
    )
}

fn format_resume_report(session_path: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Session resumed
  Session file     {session_path}
  Messages         {message_count}
  Turns            {turns}"
    )
}

fn render_resume_usage() -> String {
    format!(
        "Resume
  Usage            /resume <session-path|session-id|{LATEST_SESSION_REFERENCE}>
  Auto-save        .kcode/sessions/<session-id>.{PRIMARY_SESSION_EXTENSION}
  Tip              use /session list to inspect saved sessions"
    )
}

fn format_powerup_report() -> String {
    "Powerup
  Guided lessons   /model, /permissions, /mcp, /todos, /branch
  Workflow         start with /status, then /model and /permissions
  Tips             use `/` to open the command palette and Shift+Tab to cycle mode"
        .to_string()
}

fn render_btw_usage() -> String {
    "BTW
  Usage            /btw <question>
  Behavior         answer a side question without writing it into the active session history
  Example          /btw what changed between async fn and spawn_blocking?"
        .to_string()
}

fn format_bug_report(description: Option<&str>, session_id: &str, session_path: &std::path::Path) -> String {
    format!(
        "Bug report
  Description      {}
  Session          {session_id}
  Transcript       {}
  Scope            local-only diagnostic export
  Next step        use /export if you want a shareable transcript copy",
        description.unwrap_or("not provided"),
        session_path.display(),
    )
}

fn format_feedback_report(description: Option<&str>) -> String {
    format!(
        "Feedback
  Summary          {}
  Scope            local Kcode feedback note
  Next step        include concrete steps, expected behavior, and actual behavior for triage",
        description.unwrap_or("not provided"),
    )
}

fn format_login_report(profile: &str, model: &str) -> String {
    format!(
        "Profiles
  Active profile   {profile}
  Active model     {model}
  Auth model       Kcode uses local profile and environment configuration instead of Claude account login
  Next step        run /config model or /doctor to inspect endpoint and credential wiring",
    )
}

fn format_desktop_report() -> String {
    "Desktop
  Status           no dedicated desktop handoff is configured in this build
  Alternative      keep using the fullscreen TUI, /diff, and /export for review workflows"
        .to_string()
}

fn format_schedule_report(args: Option<&str>) -> String {
    match args.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("list") => "Schedule
  Usage            /schedule list
  Usage            /schedule create <cron> <prompt>
  Usage            /schedule delete <id>
  Status           scheduled task management is exposed through CronCreate/CronList/CronDelete tools"
            .to_string(),
        Some(other) => format!(
            "Schedule
  Requested        {other}
  Usage            /schedule list
  Usage            /schedule create <cron> <prompt>
  Usage            /schedule delete <id>"
        ),
    }
}

fn format_loop_report(args: Option<&str>) -> String {
    match args.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => format!(
            "Loop
  Requested        {value}
  Status           loop execution is not yet automated in the TUI shell
  Tip              use /schedule for recurring jobs or ask Kcode to create a watcher script"
        ),
        None => "Loop
  Usage            /loop <interval> <prompt>
  Example          /loop 5m run tests
  Example          /loop 30m check build status"
            .to_string(),
    }
}

fn render_todos_report(cwd: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let store_path = cwd.join(".clawd-todos.json");
    if !store_path.exists() {
        return Ok(format!(
            "Todos
  Store            {}
  Status           no todo list recorded yet
  Source           TodoWrite tool populates this file during longer tasks",
            store_path.display()
        ));
    }

    let raw = std::fs::read_to_string(&store_path)?;
    let todos = serde_json::from_str::<Vec<serde_json::Value>>(&raw)?;
    let mut lines = vec![
        "Todos".to_string(),
        format!("  Store            {}", store_path.display()),
    ];

    for (index, todo) in todos.iter().enumerate() {
        let status = todo
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let marker = match status {
            "completed" => "[x]",
            "in_progress" => "[~]",
            _ => "[ ]",
        };
        let content = todo
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("(missing content)");
        let active_form = todo
            .get("activeForm")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-");
        lines.push(format!(
            "  {index:>2}. {marker} {content}  status={status}  active={active_form}",
            index = index + 1
        ));
    }

    Ok(lines.join("\n"))
}

fn format_command_not_ready(command: &str, detail: &str) -> String {
    format!(
        "{command}
  Status           command shape is available in Kcode, but this flow is not fully implemented yet
  Detail           {detail}"
    )
}
