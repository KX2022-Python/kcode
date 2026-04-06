use std::path::Path;

use crate::catalog_discovery::{discover_skill_roots, load_skills_from_roots, SkillSummary};
use crate::model::{
    slash_command_specs, CommandAvailability, CommandDescriptor, CommandKind, CommandManifestEntry,
    CommandRegistryContext, CommandRegistrySnapshot, CommandScope, CommandSource, CommandSurface,
    FilteredCommand, ProcessCommandSpec, SlashCommandSpec, PROCESS_COMMAND_SPECS,
};

pub(crate) fn find_slash_command_spec(name: &str) -> Option<&'static SlashCommandSpec> {
    slash_command_specs().iter().find(|spec| {
        spec.name.eq_ignore_ascii_case(name)
            || spec
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(name))
    })
}

fn session_command_kind(name: &str) -> CommandKind {
    match name {
        "help" | "context" | "btw" => CommandKind::Prompt,
        "plan" | "todos" | "vim" | "theme" | "voice" | "ide" | "desktop" | "powerup" => {
            CommandKind::LocalUi
        }
        _ => CommandKind::Local,
    }
}

fn process_command_kind(name: &str) -> CommandKind {
    match name {
        "prompt" => CommandKind::Prompt,
        _ => CommandKind::Local,
    }
}

const HIDDEN_SESSION_COMMANDS: &[&str] = &[
    "logout",
    "vim",
    "upgrade",
    "stats",
    "share",
    "files",
    "fast",
    "exit",
    "summary",
    "brief",
    "advisor",
    "stickers",
    "insights",
    "thinkback",
    "release-notes",
    "security-review",
    "review",
    "usage",
    "rename",
    "copy",
    "context",
    "effort",
    "rewind",
    "ide",
    "tag",
    "add-dir",
];

pub(crate) fn builtin_session_command_enabled(name: &str) -> bool {
    !HIDDEN_SESSION_COMMANDS.contains(&name)
}

fn process_command_enabled(name: &str) -> bool {
    matches!(
        name,
        "init"
            | "doctor"
            | "config"
            | "profile"
            | "commands"
            | "resume"
            | "mcp"
            | "agents"
            | "skills"
            | "status"
            | "sandbox"
            | "prompt"
    )
}

pub(crate) fn is_v1_core_session_command(name: &str) -> bool {
    matches!(
        name,
        "help"
            | "clear"
            | "compact"
            | "config"
            | "doctor"
            | "init"
            | "model"
            | "memory"
            | "resume"
            | "mcp"
            | "permissions"
    )
}

fn session_command_safety(name: &str, kind: CommandKind) -> (bool, bool) {
    match name {
        "help" | "compact" | "model" | "permissions" | "memory" | "mcp" => (true, true),
        "status" | "sandbox" => (true, false),
        _ => default_safety_for_kind(kind),
    }
}

fn default_safety_for_kind(kind: CommandKind) -> (bool, bool) {
    match kind {
        CommandKind::Prompt => (true, true),
        CommandKind::Local => (false, false),
        CommandKind::LocalUi => (false, false),
    }
}

pub(crate) fn session_command_descriptor(spec: &SlashCommandSpec) -> CommandDescriptor {
    let kind = session_command_kind(spec.name);
    let (remote_safe, channel_safe) = session_command_safety(spec.name, kind);
    let mut visibility_tags = vec!["session".to_string()];
    if is_v1_core_session_command(spec.name) {
        visibility_tags.push("v1-core".to_string());
    }
    if spec.resume_supported {
        visibility_tags.push("resume".to_string());
    }
    if !builtin_session_command_enabled(spec.name) {
        visibility_tags.push("compat-hidden".to_string());
    }
    CommandDescriptor {
        id: format!("builtin.session.{}", spec.name),
        name: spec.name.to_string(),
        kind,
        source: CommandSource::Builtin,
        scope: CommandScope::Session,
        availability: CommandAvailability {
            cli_visible: true,
            bridge_visible: remote_safe && channel_safe,
        },
        enabled: builtin_session_command_enabled(spec.name),
        remote_safe,
        channel_safe,
        aliases: spec
            .aliases
            .iter()
            .map(|alias| (*alias).to_string())
            .collect(),
        description: spec.summary.to_string(),
        argument_hint: spec.argument_hint.map(ToOwned::to_owned),
        resume_supported: spec.resume_supported,
        visibility_tags,
    }
}

fn process_command_descriptor(spec: &ProcessCommandSpec) -> CommandDescriptor {
    let kind = process_command_kind(spec.name);
    let (remote_safe, channel_safe) = default_safety_for_kind(kind);
    let mut visibility_tags = vec!["process".to_string()];
    if spec.name == "profile" {
        visibility_tags.push("provider-profile".to_string());
    }
    CommandDescriptor {
        id: format!("builtin.process.{}", spec.name),
        name: spec.name.to_string(),
        kind,
        source: CommandSource::Builtin,
        scope: CommandScope::Process,
        availability: CommandAvailability {
            cli_visible: true,
            bridge_visible: remote_safe && channel_safe,
        },
        enabled: process_command_enabled(spec.name),
        remote_safe,
        channel_safe,
        aliases: spec
            .aliases
            .iter()
            .map(|alias| (*alias).to_string())
            .collect(),
        description: spec.summary.to_string(),
        argument_hint: spec.argument_hint.map(ToOwned::to_owned),
        resume_supported: false,
        visibility_tags,
    }
}

#[must_use]
pub fn build_command_registry_snapshot(
    context: &CommandRegistryContext,
    extra_descriptors: &[CommandDescriptor],
) -> CommandRegistrySnapshot {
    build_command_registry_snapshot_with_cwd(
        context,
        extra_descriptors,
        &std::env::current_dir().unwrap_or_default(),
    )
}

#[must_use]
pub fn build_command_registry_snapshot_with_cwd(
    context: &CommandRegistryContext,
    extra_descriptors: &[CommandDescriptor],
    cwd: &Path,
) -> CommandRegistrySnapshot {
    let skill_summaries = load_skills_from_roots(&discover_skill_roots(cwd)).unwrap_or_default();
    let skill_descriptors = skill_summaries
        .into_iter()
        .map(|summary: SkillSummary| CommandDescriptor {
            id: format!("skill.{}.{}", summary.origin_label(), summary.name),
            name: summary.name.clone(),
            kind: CommandKind::Local,
            source: CommandSource::Skills,
            scope: CommandScope::Session,
            availability: CommandAvailability {
                cli_visible: true,
                bridge_visible: false,
            },
            enabled: true,
            remote_safe: false,
            channel_safe: false,
            aliases: vec![],
            description: summary.description.clone().unwrap_or_default(),
            argument_hint: None,
            resume_supported: false,
            visibility_tags: vec!["skill".to_string(), summary.origin_label()],
        })
        .collect::<Vec<_>>();

    let all_extras = skill_descriptors
        .into_iter()
        .chain(extra_descriptors.iter().cloned())
        .collect::<Vec<_>>();

    build_snapshot_inner(context, &all_extras)
}

fn build_snapshot_inner(
    context: &CommandRegistryContext,
    extra_descriptors: &[CommandDescriptor],
) -> CommandRegistrySnapshot {
    let mut snapshot = CommandRegistrySnapshot {
        safety_profile: match context.surface {
            CommandSurface::CliLocal => "cli-local".to_string(),
            CommandSurface::Bridge => "bridge-safe".to_string(),
        },
        ..CommandRegistrySnapshot::default()
    };

    let descriptors = slash_command_specs()
        .iter()
        .map(session_command_descriptor)
        .chain(PROCESS_COMMAND_SPECS.iter().map(process_command_descriptor))
        .chain(extra_descriptors.iter().cloned())
        .collect::<Vec<_>>();

    for descriptor in descriptors {
        let allowed_by_surface = match context.surface {
            CommandSurface::CliLocal => descriptor.availability.cli_visible,
            CommandSurface::Bridge => {
                descriptor.availability.bridge_visible
                    && descriptor.remote_safe
                    && descriptor.channel_safe
            }
        };

        let local_ui_allowed = context.include_local_ui || descriptor.kind != CommandKind::LocalUi;
        let tools_allowed = context.profile_supports_tools
            || !matches!(descriptor.name.as_str(), "mcp" | "plugin" | "plugins");
        let denied_by_rule = context
            .denied_commands
            .iter()
            .any(|pattern| descriptor.name.contains(pattern));

        let filtered_reason = if !descriptor.enabled {
            Some("disabled".to_string())
        } else if denied_by_rule {
            Some("denied by command policy".to_string())
        } else if !allowed_by_surface {
            Some(match context.surface {
                CommandSurface::CliLocal => "hidden on local CLI surface".to_string(),
                CommandSurface::Bridge => "not bridge-safe".to_string(),
            })
        } else if !local_ui_allowed {
            Some("requires local UI".to_string())
        } else if !tools_allowed {
            Some("active profile does not expose tool-capable commands".to_string())
        } else {
            None
        };

        if let Some(reason) = filtered_reason {
            snapshot.filtered_out_commands.push(FilteredCommand {
                id: descriptor.id,
                scope: descriptor.scope,
                reason,
            });
            continue;
        }

        *snapshot
            .source_breakdown
            .entry(descriptor.source)
            .or_insert(0) += 1;

        match descriptor.scope {
            CommandScope::Process => snapshot.process_commands.push(descriptor),
            CommandScope::Session => snapshot.session_commands.push(descriptor),
        }
    }

    snapshot
}

#[must_use]
pub fn v1_command_manifest() -> Vec<CommandManifestEntry> {
    slash_command_specs()
        .iter()
        .filter(|spec| is_v1_core_session_command(spec.name))
        .map(|spec| CommandManifestEntry {
            name: spec.name.to_string(),
            source: CommandSource::Builtin,
            scope: CommandScope::Session,
            kind: session_command_kind(spec.name),
        })
        .collect()
}
