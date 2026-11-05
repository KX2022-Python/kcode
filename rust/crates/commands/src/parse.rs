use crate::model::{SlashCommand, SlashCommandParseError};
use crate::parse_support::{
    command_error, parse_list_or_help_args, parse_skills_args, remainder_after_command, usage_error,
};
use crate::registry::{builtin_session_command_enabled, find_slash_command_spec};

impl SlashCommand {
    pub fn parse(input: &str) -> Result<Option<Self>, SlashCommandParseError> {
        validate_slash_command_input(input)
    }
}

#[allow(clippy::too_many_lines)]
pub fn validate_slash_command_input(
    input: &str,
) -> Result<Option<SlashCommand>, SlashCommandParseError> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return Ok(None);
    }

    let mut parts = trimmed.trim_start_matches('/').split_whitespace();
    let command = parts.next().unwrap_or_default();
    if command.is_empty() {
        return Err(SlashCommandParseError::new(
            "Slash command name is missing. Use /help to list available slash commands.",
        ));
    }

    let args = parts.collect::<Vec<_>>();
    let remainder = remainder_after_command(trimmed, command);

    if let Some(spec) = find_slash_command_spec(command) {
        if !builtin_session_command_enabled(spec.name) {
            return Ok(Some(SlashCommand::Unknown(command.to_string())));
        }
    }

    Ok(Some(match command {
        "help" => {
            validate_no_args(command, &args)?;
            SlashCommand::Help
        }
        "status" => {
            validate_no_args(command, &args)?;
            SlashCommand::Status
        }
        "sandbox" => {
            validate_no_args(command, &args)?;
            SlashCommand::Sandbox
        }
        "compact" => {
            validate_no_args(command, &args)?;
            SlashCommand::Compact
        }
        "bug" | "bughunter" => SlashCommand::Bughunter { scope: remainder },
        "commit" => {
            validate_no_args(command, &args)?;
            SlashCommand::Commit
        }
        "pr" => SlashCommand::Pr { context: remainder },
        "issue" => SlashCommand::Issue { context: remainder },
        "debug-tool-call" => {
            validate_no_args(command, &args)?;
            SlashCommand::DebugToolCall
        }
        "model" => SlashCommand::Model {
            model: optional_single_arg(command, &args, "[model]")?,
        },
        "permissions" => SlashCommand::Permissions {
            mode: parse_permissions_mode(&args)?,
        },
        "clear" => SlashCommand::Clear {
            confirm: parse_clear_args(&args)?,
        },
        "cost" => {
            validate_no_args(command, &args)?;
            SlashCommand::Cost
        }
        "resume" => SlashCommand::Resume {
            session_path: Some(required_remainder(command, remainder, "<session-path>")?),
        },
        "config" => SlashCommand::Config {
            section: parse_config_section(&args)?,
        },
        "mcp" => parse_mcp_command(&args)?,
        "memory" => {
            validate_no_args(command, &args)?;
            SlashCommand::Memory
        }
        "dream" => SlashCommand::Dream {
            mode: parse_dream_mode(&args)?,
        },
        "init" => {
            validate_no_args(command, &args)?;
            SlashCommand::Init
        }
        "diff" => {
            validate_no_args(command, &args)?;
            SlashCommand::Diff
        }
        "version" => {
            validate_no_args(command, &args)?;
            SlashCommand::Version
        }
        "export" => SlashCommand::Export { path: remainder },
        "session" => parse_session_command(&args)?,
        "plugin" | "plugins" | "marketplace" => parse_plugin_command(&args)?,
        "agents" => SlashCommand::Agents {
            args: parse_list_or_help_args(command, remainder)?,
        },
        "powerup" => {
            validate_no_args(command, &args)?;
            SlashCommand::Powerup
        }
        "btw" => SlashCommand::Btw {
            question: remainder,
        },
        "skills" => SlashCommand::Skills {
            args: parse_skills_args(remainder.as_deref())?,
        },
        "doctor" => {
            validate_no_args(command, &args)?;
            SlashCommand::Doctor
        }
        "login" => {
            validate_no_args(command, &args)?;
            SlashCommand::Login
        }
        "logout" => {
            validate_no_args(command, &args)?;
            SlashCommand::Logout
        }
        "vim" => {
            validate_no_args(command, &args)?;
            SlashCommand::Vim
        }
        "upgrade" => {
            validate_no_args(command, &args)?;
            SlashCommand::Upgrade
        }
        "stats" => {
            validate_no_args(command, &args)?;
            SlashCommand::Stats
        }
        "share" => {
            validate_no_args(command, &args)?;
            SlashCommand::Share
        }
        "feedback" => {
            validate_no_args(command, &args)?;
            SlashCommand::Feedback
        }
        "files" => {
            validate_no_args(command, &args)?;
            SlashCommand::Files
        }
        "fast" => {
            validate_no_args(command, &args)?;
            SlashCommand::Fast
        }
        "exit" => {
            validate_no_args(command, &args)?;
            SlashCommand::Exit
        }
        "summary" => {
            validate_no_args(command, &args)?;
            SlashCommand::Summary
        }
        "desktop" => {
            validate_no_args(command, &args)?;
            SlashCommand::Desktop
        }
        "schedule" => SlashCommand::Schedule { args: remainder },
        "loop" => SlashCommand::Loop { args: remainder },
        "brief" => {
            validate_no_args(command, &args)?;
            SlashCommand::Brief
        }
        "advisor" => {
            validate_no_args(command, &args)?;
            SlashCommand::Advisor
        }
        "stickers" => {
            validate_no_args(command, &args)?;
            SlashCommand::Stickers
        }
        "insights" => {
            validate_no_args(command, &args)?;
            SlashCommand::Insights
        }
        "thinkback" => {
            validate_no_args(command, &args)?;
            SlashCommand::Thinkback
        }
        "release-notes" => {
            validate_no_args(command, &args)?;
            SlashCommand::ReleaseNotes
        }
        "security-review" => {
            validate_no_args(command, &args)?;
            SlashCommand::SecurityReview
        }
        "keybindings" => {
            validate_no_args(command, &args)?;
            SlashCommand::Keybindings
        }
        "privacy-settings" => {
            validate_no_args(command, &args)?;
            SlashCommand::PrivacySettings
        }
        "plan" => SlashCommand::Plan {
            mode: parse_plan_mode(&args)?,
        },
        "review" => SlashCommand::Review { scope: remainder },
        "todos" | "tasks" => SlashCommand::Tasks { args: remainder },
        "theme" => SlashCommand::Theme { name: remainder },
        "voice" => SlashCommand::Voice { mode: remainder },
        "usage" => SlashCommand::Usage { scope: remainder },
        "rename" => SlashCommand::Rename { name: remainder },
        "copy" => SlashCommand::Copy { target: remainder },
        "hooks" => SlashCommand::Hooks { args: remainder },
        "context" => SlashCommand::Context { action: remainder },
        "color" => SlashCommand::Color { scheme: remainder },
        "effort" => SlashCommand::Effort { level: remainder },
        "branch" => SlashCommand::Branch { name: remainder },
        "rewind" => SlashCommand::Rewind { steps: remainder },
        "ide" => SlashCommand::Ide { target: remainder },
        "tag" => SlashCommand::Tag { label: remainder },
        "output-style" => SlashCommand::OutputStyle { style: remainder },
        "add-dir" => SlashCommand::AddDir { path: remainder },
        other => SlashCommand::Unknown(other.to_string()),
    }))
}

fn validate_no_args(command: &str, args: &[&str]) -> Result<(), SlashCommandParseError> {
    if args.is_empty() {
        return Ok(());
    }

    Err(command_error(
        &format!("Unexpected arguments for /{command}."),
        command,
        &format!("/{command}"),
    ))
}

fn optional_single_arg(
    command: &str,
    args: &[&str],
    argument_hint: &str,
) -> Result<Option<String>, SlashCommandParseError> {
    match args {
        [] => Ok(None),
        [value] => Ok(Some((*value).to_string())),
        _ => Err(usage_error(command, argument_hint)),
    }
}

fn required_remainder(
    command: &str,
    remainder: Option<String>,
    argument_hint: &str,
) -> Result<String, SlashCommandParseError> {
    remainder
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| usage_error(command, argument_hint))
}

fn parse_permissions_mode(args: &[&str]) -> Result<Option<String>, SlashCommandParseError> {
    let mode = optional_single_arg(
        "permissions",
        args,
        "[read-only|plan|workspace-write|danger-full-access]",
    )?;
    if let Some(mode) = mode {
        if matches!(
            mode.as_str(),
            "read-only" | "plan" | "workspace-write" | "danger-full-access"
        ) {
            return Ok(Some(mode));
        }
        return Err(command_error(
            &format!(
                "Unsupported /permissions mode '{mode}'. Use read-only, plan, workspace-write, or danger-full-access."
            ),
            "permissions",
            "/permissions [read-only|plan|workspace-write|danger-full-access]",
        ));
    }

    Ok(None)
}

fn parse_dream_mode(args: &[&str]) -> Result<Option<String>, SlashCommandParseError> {
    let mode = optional_single_arg("dream", args, "[on|off|status]")?;
    if let Some(mode) = mode {
        if matches!(mode.as_str(), "on" | "off" | "status") {
            return Ok(Some(mode));
        }

        return Err(command_error(
            &format!("Unsupported /dream mode '{mode}'. Use on, off, or status."),
            "dream",
            "/dream [on|off|status]",
        ));
    }

    Ok(None)
}

fn parse_plan_mode(args: &[&str]) -> Result<Option<String>, SlashCommandParseError> {
    let mode = optional_single_arg("plan", args, "[on|off|status]")?;
    if let Some(mode) = mode {
        if matches!(mode.as_str(), "on" | "off" | "status") {
            return Ok(Some(mode));
        }

        return Err(command_error(
            &format!("Unsupported /plan mode '{mode}'. Use on, off, or status."),
            "plan",
            "/plan [on|off|status]",
        ));
    }

    Ok(None)
}

fn parse_clear_args(args: &[&str]) -> Result<bool, SlashCommandParseError> {
    match args {
        [] => Ok(false),
        ["--confirm"] => Ok(true),
        [unexpected] => Err(command_error(
            &format!("Unsupported /clear argument '{unexpected}'. Use /clear or /clear --confirm."),
            "clear",
            "/clear [--confirm]",
        )),
        _ => Err(usage_error("clear", "[--confirm]")),
    }
}

fn parse_config_section(args: &[&str]) -> Result<Option<String>, SlashCommandParseError> {
    let section = optional_single_arg("config", args, "[env|hooks|model|plugins]")?;
    if let Some(section) = section {
        if matches!(section.as_str(), "env" | "hooks" | "model" | "plugins") {
            return Ok(Some(section));
        }
        return Err(command_error(
            &format!("Unsupported /config section '{section}'. Use env, hooks, model, or plugins."),
            "config",
            "/config [env|hooks|model|plugins]",
        ));
    }

    Ok(None)
}

fn parse_session_command(args: &[&str]) -> Result<SlashCommand, SlashCommandParseError> {
    match args {
        [] => Ok(SlashCommand::Session {
            action: None,
            target: None,
        }),
        ["list"] => Ok(SlashCommand::Session {
            action: Some("list".to_string()),
            target: None,
        }),
        ["list", ..] => Err(usage_error(
            "session",
            "[list|switch <session-id>|fork [branch-name]]",
        )),
        ["switch"] => Err(usage_error("session switch", "<session-id>")),
        ["switch", target] => Ok(SlashCommand::Session {
            action: Some("switch".to_string()),
            target: Some((*target).to_string()),
        }),
        ["switch", ..] => Err(command_error(
            "Unexpected arguments for /session switch.",
            "session",
            "/session switch <session-id>",
        )),
        ["fork"] => Ok(SlashCommand::Session {
            action: Some("fork".to_string()),
            target: None,
        }),
        ["fork", target] => Ok(SlashCommand::Session {
            action: Some("fork".to_string()),
            target: Some((*target).to_string()),
        }),
        ["fork", ..] => Err(command_error(
            "Unexpected arguments for /session fork.",
            "session",
            "/session fork [branch-name]",
        )),
        [action, ..] => Err(command_error(
            &format!(
                "Unknown /session action '{action}'. Use list, switch <session-id>, or fork [branch-name]."
            ),
            "session",
            "/session [list|switch <session-id>|fork [branch-name]]",
        )),
    }
}

fn parse_mcp_command(args: &[&str]) -> Result<SlashCommand, SlashCommandParseError> {
    match args {
        [] => Ok(SlashCommand::Mcp {
            action: None,
            target: None,
        }),
        ["list"] => Ok(SlashCommand::Mcp {
            action: Some("list".to_string()),
            target: None,
        }),
        ["list", ..] => Err(usage_error("mcp list", "")),
        ["show"] => Err(usage_error("mcp show", "<server>")),
        ["show", target] => Ok(SlashCommand::Mcp {
            action: Some("show".to_string()),
            target: Some((*target).to_string()),
        }),
        ["show", ..] => Err(command_error(
            "Unexpected arguments for /mcp show.",
            "mcp",
            "/mcp show <server>",
        )),
        ["help" | "-h" | "--help"] => Ok(SlashCommand::Mcp {
            action: Some("help".to_string()),
            target: None,
        }),
        [action, ..] => Err(command_error(
            &format!("Unknown /mcp action '{action}'. Use list, show <server>, or help."),
            "mcp",
            "/mcp [list|show <server>|help]",
        )),
    }
}

fn parse_plugin_command(args: &[&str]) -> Result<SlashCommand, SlashCommandParseError> {
    match args {
        [] => Ok(SlashCommand::Plugins {
            action: None,
            target: None,
        }),
        ["list"] => Ok(SlashCommand::Plugins {
            action: Some("list".to_string()),
            target: None,
        }),
        ["list", ..] => Err(usage_error("plugin list", "")),
        ["install"] => Err(usage_error("plugin install", "<path>")),
        ["install", target @ ..] => Ok(SlashCommand::Plugins {
            action: Some("install".to_string()),
            target: Some(target.join(" ")),
        }),
        ["enable"] => Err(usage_error("plugin enable", "<name>")),
        ["enable", target] => Ok(SlashCommand::Plugins {
            action: Some("enable".to_string()),
            target: Some((*target).to_string()),
        }),
        ["enable", ..] => Err(command_error(
            "Unexpected arguments for /plugin enable.",
            "plugin",
            "/plugin enable <name>",
        )),
        ["disable"] => Err(usage_error("plugin disable", "<name>")),
        ["disable", target] => Ok(SlashCommand::Plugins {
            action: Some("disable".to_string()),
            target: Some((*target).to_string()),
        }),
        ["disable", ..] => Err(command_error(
            "Unexpected arguments for /plugin disable.",
            "plugin",
            "/plugin disable <name>",
        )),
        ["uninstall"] => Err(usage_error("plugin uninstall", "<id>")),
        ["uninstall", target] => Ok(SlashCommand::Plugins {
            action: Some("uninstall".to_string()),
            target: Some((*target).to_string()),
        }),
        ["uninstall", ..] => Err(command_error(
            "Unexpected arguments for /plugin uninstall.",
            "plugin",
            "/plugin uninstall <id>",
        )),
        ["update"] => Err(usage_error("plugin update", "<id>")),
        ["update", target] => Ok(SlashCommand::Plugins {
            action: Some("update".to_string()),
            target: Some((*target).to_string()),
        }),
        ["update", ..] => Err(command_error(
            "Unexpected arguments for /plugin update.",
            "plugin",
            "/plugin update <id>",
        )),
        [action, ..] => Err(command_error(
            &format!(
                "Unknown /plugin action '{action}'. Use list, install <path>, enable <name>, disable <name>, uninstall <id>, or update <id>."
            ),
            "plugin",
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]",
        )),
    }
}
