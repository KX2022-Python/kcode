use crate::model::{
    CommandDescriptor, CommandKind, CommandRegistryContext, CommandScope, SlashCommandSpec,
};
use crate::registry::{
    build_command_registry_snapshot, find_slash_command_spec, session_command_descriptor,
};

fn slash_command_usage(spec: &SlashCommandSpec) -> String {
    match spec.argument_hint {
        Some(argument_hint) => format!("/{} {argument_hint}", spec.name),
        None => format!("/{}", spec.name),
    }
}

fn slash_command_category(name: &str) -> &'static str {
    match name {
        "help" | "status" | "sandbox" | "model" | "permissions" | "cost" | "resume" | "session"
        | "version" | "login" | "logout" | "usage" | "stats" | "rename" | "privacy-settings" => {
            "Session & visibility"
        }
        "compact" | "clear" | "config" | "memory" | "dream" | "init" | "diff" | "commit" | "pr"
        | "issue" | "export" | "plugin" | "branch" | "add-dir" | "files" | "hooks" | "plan"
        | "release-notes" => "Workspace & git",
        "agents" | "skills" | "debug-tool-call" | "mcp" | "context" | "todos" | "doctor"
        | "ide" | "desktop" | "powerup" | "schedule" | "loop" => "Discovery & debugging",
        "review" | "security-review" | "advisor" | "insights" => "Analysis & automation",
        "theme" | "vim" | "voice" | "color" | "effort" | "fast" | "brief" | "output-style"
        | "keybindings" | "stickers" => "Appearance & input",
        "copy" | "share" | "feedback" | "summary" | "tag" | "thinkback" | "exit" | "upgrade"
        | "rewind" | "btw" | "bug" => "Communication & control",
        _ => "Other",
    }
}

fn format_command_help_line(descriptor: &CommandDescriptor) -> String {
    let usage = match &descriptor.argument_hint {
        Some(argument_hint) => format!("/{} {argument_hint}", descriptor.name),
        None => format!("/{}", descriptor.name),
    };
    let alias_suffix = if descriptor.aliases.is_empty() {
        String::new()
    } else {
        format!(
            " (aliases: {})",
            descriptor
                .aliases
                .iter()
                .map(|alias| format!("/{alias}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let resume = if descriptor.resume_supported {
        " [resume]"
    } else {
        ""
    };
    format!(
        "  {usage:<66} {}{alias_suffix}{resume}",
        descriptor.description
    )
}

fn slash_command_detail_lines(spec: &SlashCommandSpec) -> Vec<String> {
    let descriptor = session_command_descriptor(spec);
    let mut lines = vec![format!("/{}", spec.name)];
    lines.push(format!("  Summary          {}", spec.summary));
    lines.push(format!("  Usage            {}", slash_command_usage(spec)));
    lines.push(format!(
        "  Kind             {}",
        match descriptor.kind {
            CommandKind::Prompt => "prompt",
            CommandKind::Local => "local",
            CommandKind::LocalUi => "local-ui",
        }
    ));
    lines.push(format!(
        "  Scope            {}",
        match descriptor.scope {
            CommandScope::Process => "process",
            CommandScope::Session => "session",
        }
    ));
    lines.push(format!(
        "  Category         {}",
        slash_command_category(spec.name)
    ));
    lines.push(format!(
        "  Surface          local={} bridge={}",
        descriptor.availability.cli_visible, descriptor.availability.bridge_visible
    ));
    lines.push(format!(
        "  Enabled          {}",
        if descriptor.enabled {
            "yes"
        } else {
            "hidden from default manifest"
        }
    ));
    if !spec.aliases.is_empty() {
        lines.push(format!(
            "  Aliases          {}",
            spec.aliases
                .iter()
                .map(|alias| format!("/{alias}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if spec.resume_supported {
        lines.push("  Resume           Supported with --resume SESSION.jsonl".to_string());
    }
    if !descriptor.visibility_tags.is_empty() {
        lines.push(format!(
            "  Tags             {}",
            descriptor.visibility_tags.join(", ")
        ));
    }
    lines
}

#[must_use]
pub fn render_slash_command_help_detail(name: &str) -> Option<String> {
    find_slash_command_spec(name).map(|spec| slash_command_detail_lines(spec).join("\n"))
}

#[must_use]
pub fn resume_supported_slash_commands() -> Vec<&'static SlashCommandSpec> {
    let snapshot = build_command_registry_snapshot(&CommandRegistryContext::cli_local(), &[]);
    snapshot
        .session_commands
        .iter()
        .filter(|descriptor| descriptor.resume_supported)
        .filter_map(|descriptor| find_slash_command_spec(&descriptor.name))
        .collect()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }

    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != *right_char);
            current[right_index + 1] = (current[right_index] + 1)
                .min(previous[right_index + 1] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

#[must_use]
pub fn suggest_slash_commands(input: &str, limit: usize) -> Vec<String> {
    let query = input.trim().trim_start_matches('/').to_ascii_lowercase();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let snapshot = build_command_registry_snapshot(&CommandRegistryContext::cli_local(), &[]);
    let mut suggestions = snapshot
        .session_commands
        .iter()
        .filter_map(|descriptor| {
            let best = std::iter::once(descriptor.name.as_str())
                .chain(descriptor.aliases.iter().map(String::as_str))
                .map(str::to_ascii_lowercase)
                .map(|candidate| {
                    let prefix_rank =
                        if candidate.starts_with(&query) || query.starts_with(&candidate) {
                            0
                        } else if candidate.contains(&query) || query.contains(&candidate) {
                            1
                        } else {
                            2
                        };
                    let distance = levenshtein_distance(&candidate, &query);
                    (prefix_rank, distance)
                })
                .min();

            best.and_then(|(prefix_rank, distance)| {
                if prefix_rank <= 1 || distance <= 2 {
                    Some((
                        prefix_rank,
                        distance,
                        descriptor.name.len(),
                        descriptor.name.as_str(),
                    ))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    suggestions.sort_unstable();
    suggestions
        .into_iter()
        .map(|(_, _, _, name)| format!("/{name}"))
        .take(limit)
        .collect()
}

#[must_use]
pub fn render_slash_command_help_for_context(context: &CommandRegistryContext) -> String {
    let snapshot = build_command_registry_snapshot(context, &[]);
    let start_here = ["doctor", "config", "status", "mcp", "memory"]
        .into_iter()
        .filter(|name| {
            snapshot
                .session_commands
                .iter()
                .any(|descriptor| descriptor.name == *name)
        })
        .map(|name| format!("/{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut lines = vec![
        "Slash commands".to_string(),
        format!(
            "  Active surface    {} ({} commands)",
            snapshot.safety_profile,
            snapshot.session_commands.len()
        ),
        format!("  Start here        {start_here}"),
        "  [resume]          also works with --resume SESSION.jsonl".to_string(),
        String::new(),
    ];

    lines.push("Core v1.0 manifest".to_string());
    for descriptor in snapshot.session_commands.iter().filter(|descriptor| {
        descriptor
            .visibility_tags
            .iter()
            .any(|tag| tag == "v1-core")
    }) {
        lines.push(format_command_help_line(descriptor));
    }
    lines.push(String::new());

    lines.push("Extended local surface".to_string());
    let categories = [
        "Session & visibility",
        "Workspace & git",
        "Discovery & debugging",
    ];
    for category in categories {
        lines.push(category.to_string());
        for descriptor in snapshot.session_commands.iter().filter(|descriptor| {
            !descriptor
                .visibility_tags
                .iter()
                .any(|tag| tag == "v1-core")
                && slash_command_category(&descriptor.name) == category
        }) {
            lines.push(format_command_help_line(descriptor));
        }
        lines.push(String::new());
    }

    lines
        .into_iter()
        .rev()
        .skip_while(String::is_empty)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn render_slash_command_help() -> String {
    render_slash_command_help_for_context(&CommandRegistryContext::cli_local())
}
