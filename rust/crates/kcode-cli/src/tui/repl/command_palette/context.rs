use std::path::Path;

use commands::{
    build_command_registry_snapshot_with_cwd, CommandDescriptor, CommandRegistryContext,
    CommandSource, CommandSurface,
};

use super::SlashCommandEntry;

const CC_COMMAND_ORDER: &[&str] = &[
    "help",
    "clear",
    "resume",
    "rename",
    "branch",
    "rewind",
    "compact",
    "config",
    "effort",
    "model",
    "permissions",
    "hooks",
    "init",
    "plugin",
    "agents",
    "powerup",
    "btw",
    "bug",
    "feedback",
    "login",
    "desktop",
    "schedule",
    "loop",
    "mcp",
    "review",
    "status",
    "cost",
    "todos",
    "commit",
];

pub(super) fn slash_command_entries(
    profile_supports_tools: bool,
    cwd: &Path,
) -> Vec<SlashCommandEntry> {
    let snapshot = build_command_registry_snapshot_with_cwd(
        &CommandRegistryContext::for_surface(CommandSurface::CliLocal, profile_supports_tools),
        &[],
        cwd,
    );
    let mut ordered = snapshot
        .session_commands
        .into_iter()
        .enumerate()
        .map(|(index, descriptor)| (command_rank(&descriptor, index), descriptor))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(rank, _)| *rank);

    ordered
        .into_iter()
        .map(|(_, descriptor)| SlashCommandEntry {
            usage: format_usage(&descriptor.name, descriptor.argument_hint.as_deref()),
            insert_text: default_insert_text(&descriptor.name, descriptor.argument_hint.as_deref()),
            name: descriptor.name,
            aliases: descriptor.aliases,
            description: descriptor.description,
            argument_hint: descriptor.argument_hint,
            source: descriptor.source,
        })
        .collect()
}

pub(super) fn palette_entries(
    root_commands: &[SlashCommandEntry],
    context_command: Option<&str>,
    available_models: &[String],
) -> Vec<SlashCommandEntry> {
    match context_command {
        Some(command) => context_entries(command, available_models),
        None => root_commands.to_vec(),
    }
}

pub(super) fn extract_palette_filter(
    input: &str,
    available_models: &[String],
) -> Option<(Option<String>, String)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('/') {
        return None;
    }

    let body = trimmed.trim_start_matches('/');
    if body.is_empty() {
        return Some((None, String::new()));
    }

    if !body.contains(' ') {
        let command = body.to_ascii_lowercase();
        if exact_context_palette_command(&command, available_models) {
            return Some((Some(command), String::new()));
        }
        return Some((None, command));
    }

    let mut parts = body.splitn(3, ' ');
    let command = parts.next().unwrap_or_default().to_ascii_lowercase();
    let second = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        return None;
    }

    if context_entries(&command, available_models).is_empty() {
        return None;
    }

    Some((Some(command), second.trim().to_ascii_lowercase()))
}

fn command_rank(descriptor: &CommandDescriptor, original_index: usize) -> (usize, usize, usize) {
    let cc_rank = CC_COMMAND_ORDER
        .iter()
        .position(|name| *name == descriptor.name)
        .unwrap_or(CC_COMMAND_ORDER.len() + original_index);
    let source_rank = match descriptor.source {
        CommandSource::Builtin => 0,
        CommandSource::Skills => 1,
        CommandSource::Plugins => 2,
        CommandSource::Workflow => 3,
        CommandSource::Mcp => 4,
    };
    (cc_rank, source_rank, original_index)
}

fn format_usage(name: &str, argument_hint: Option<&str>) -> String {
    match argument_hint {
        Some(argument_hint) => format!("/{name} {argument_hint}"),
        None => format!("/{name}"),
    }
}

fn default_insert_text(name: &str, argument_hint: Option<&str>) -> String {
    match argument_hint {
        Some(_) => format!("/{name} "),
        None => format!("/{name}"),
    }
}

fn exact_context_palette_command(command: &str, available_models: &[String]) -> bool {
    matches!(command, "model") && !context_entries(command, available_models).is_empty()
}

fn context_entries(command: &str, available_models: &[String]) -> Vec<SlashCommandEntry> {
    let builtin = CommandSource::Builtin;
    match command {
        "model" => {
            let mut entries = vec![submenu_entry(
                "current",
                "/model",
                "/model",
                "Show the active model and session totals",
                builtin,
            )];
            for model in available_models {
                if model.trim().is_empty() {
                    continue;
                }
                entries.push(submenu_entry(
                    model,
                    &format!("/model {model}"),
                    &format!("/model {model}"),
                    "Switch to this model",
                    builtin,
                ));
            }
            entries
        }
        "permissions" => vec![
            submenu_entry(
                "read-only",
                "/permissions read-only",
                "/permissions read-only",
                "Read/search tools only",
                builtin,
            ),
            submenu_entry(
                "plan",
                "/permissions plan",
                "/permissions plan",
                "Read-only planning mode",
                builtin,
            ),
            submenu_entry(
                "workspace-write",
                "/permissions workspace-write",
                "/permissions workspace-write",
                "Allow editing files inside the workspace",
                builtin,
            ),
            submenu_entry(
                "danger-full-access",
                "/permissions danger-full-access",
                "/permissions danger-full-access",
                "Allow unrestricted local tool access",
                builtin,
            ),
        ],
        "config" => vec![
            submenu_entry(
                "env",
                "/config env",
                "/config env",
                "Show resolved environment and config paths",
                builtin,
            ),
            submenu_entry(
                "hooks",
                "/config hooks",
                "/config hooks",
                "Inspect hook configuration",
                builtin,
            ),
            submenu_entry(
                "model",
                "/config model",
                "/config model",
                "Inspect model and profile configuration",
                builtin,
            ),
            submenu_entry(
                "plugins",
                "/config plugins",
                "/config plugins",
                "Inspect plugin configuration",
                builtin,
            ),
        ],
        "mcp" => vec![
            submenu_entry(
                "list",
                "/mcp list",
                "/mcp list",
                "List configured MCP servers",
                builtin,
            ),
            submenu_entry(
                "show",
                "/mcp show <server>",
                "/mcp show ",
                "Show one MCP server in detail",
                builtin,
            ),
            submenu_entry(
                "help",
                "/mcp help",
                "/mcp help",
                "Show MCP command help",
                builtin,
            ),
        ],
        "plugin" => vec![
            submenu_entry(
                "list",
                "/plugin list",
                "/plugin list",
                "List installed plugins",
                builtin,
            ),
            submenu_entry(
                "install",
                "/plugin install <path>",
                "/plugin install ",
                "Install a plugin from a local path",
                builtin,
            ),
            submenu_entry(
                "enable",
                "/plugin enable <name>",
                "/plugin enable ",
                "Enable an installed plugin",
                builtin,
            ),
            submenu_entry(
                "disable",
                "/plugin disable <name>",
                "/plugin disable ",
                "Disable an installed plugin",
                builtin,
            ),
            submenu_entry(
                "uninstall",
                "/plugin uninstall <id>",
                "/plugin uninstall ",
                "Remove an installed plugin",
                builtin,
            ),
            submenu_entry(
                "update",
                "/plugin update <id>",
                "/plugin update ",
                "Update an installed plugin",
                builtin,
            ),
        ],
        "session" => vec![
            submenu_entry(
                "list",
                "/session list",
                "/session list",
                "List saved sessions",
                builtin,
            ),
            submenu_entry(
                "switch",
                "/session switch <session-id>",
                "/session switch ",
                "Switch to another saved session",
                builtin,
            ),
            submenu_entry(
                "fork",
                "/session fork [branch-name]",
                "/session fork ",
                "Fork the current session into a new branch",
                builtin,
            ),
        ],
        "agents" => vec![
            submenu_entry(
                "list",
                "/agents list",
                "/agents list",
                "List configured agents",
                builtin,
            ),
            submenu_entry(
                "help",
                "/agents help",
                "/agents help",
                "Show agents command help",
                builtin,
            ),
        ],
        "skills" => vec![
            submenu_entry(
                "list",
                "/skills list",
                "/skills list",
                "List available skills",
                builtin,
            ),
            submenu_entry(
                "install",
                "/skills install <path>",
                "/skills install ",
                "Install a skill from a path or repo",
                builtin,
            ),
            submenu_entry(
                "help",
                "/skills help",
                "/skills help",
                "Show skills command help",
                builtin,
            ),
        ],
        "effort" => vec![
            submenu_entry(
                "low",
                "/effort low",
                "/effort low",
                "Favor faster responses",
                builtin,
            ),
            submenu_entry(
                "medium",
                "/effort medium",
                "/effort medium",
                "Balance speed and reasoning depth",
                builtin,
            ),
            submenu_entry(
                "high",
                "/effort high",
                "/effort high",
                "Favor deeper reasoning",
                builtin,
            ),
        ],
        "schedule" => vec![
            submenu_entry(
                "list",
                "/schedule list",
                "/schedule list",
                "List scheduled tasks",
                builtin,
            ),
            submenu_entry(
                "create",
                "/schedule create <cron> <prompt>",
                "/schedule create ",
                "Create a recurring scheduled task",
                builtin,
            ),
            submenu_entry(
                "delete",
                "/schedule delete <id>",
                "/schedule delete ",
                "Delete a scheduled task",
                builtin,
            ),
        ],
        _ => Vec::new(),
    }
}

fn submenu_entry(
    name: &str,
    usage: &str,
    insert_text: &str,
    description: &str,
    source: CommandSource,
) -> SlashCommandEntry {
    SlashCommandEntry {
        name: name.to_string(),
        usage: usage.to_string(),
        insert_text: insert_text.to_string(),
        aliases: Vec::new(),
        description: description.to_string(),
        argument_hint: None,
        source,
    }
}
