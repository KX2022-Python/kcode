use crate::specs::ProcessCommandSpec;

pub(crate) const PROCESS_COMMAND_SPECS: &[ProcessCommandSpec] = &[
    ProcessCommandSpec {
        name: "init",
        aliases: &[],
        summary: "Bootstrap the user config home",
        argument_hint: None,
    },
    ProcessCommandSpec {
        name: "doctor",
        aliases: &[],
        summary: "Diagnose setup, profile, and environment health",
        argument_hint: None,
    },
    ProcessCommandSpec {
        name: "config",
        aliases: &[],
        summary: "Inspect discovered config files or merged sections",
        argument_hint: Some("[show [env|hooks|model|plugins|profile|provider]]"),
    },
    ProcessCommandSpec {
        name: "profile",
        aliases: &[],
        summary: "Inspect built-in provider profiles and the active selection",
        argument_hint: Some("[list|show [name]]"),
    },
    ProcessCommandSpec {
        name: "commands",
        aliases: &[],
        summary: "Inspect command registry surfaces for the active profile",
        argument_hint: Some("[show [local|bridge]]"),
    },
    ProcessCommandSpec {
        name: "resume",
        aliases: &[],
        summary: "Resume a saved session and optionally dispatch slash commands",
        argument_hint: Some("<session-path|latest>"),
    },
    ProcessCommandSpec {
        name: "mcp",
        aliases: &[],
        summary: "Inspect configured MCP servers",
        argument_hint: Some("[list|show <server>|help]"),
    },
    ProcessCommandSpec {
        name: "agents",
        aliases: &[],
        summary: "Inspect configured agents",
        argument_hint: Some("[list|help]"),
    },
    ProcessCommandSpec {
        name: "skills",
        aliases: &[],
        summary: "Inspect or install skills",
        argument_hint: Some("[list|install <path>|help]"),
    },
    ProcessCommandSpec {
        name: "status",
        aliases: &[],
        summary: "Show current model, profile, and workspace status",
        argument_hint: None,
    },
    ProcessCommandSpec {
        name: "sandbox",
        aliases: &[],
        summary: "Show current sandbox isolation state",
        argument_hint: None,
    },
    ProcessCommandSpec {
        name: "prompt",
        aliases: &[],
        summary: "Run a non-interactive one-shot prompt",
        argument_hint: Some("<prompt>"),
    },
];
