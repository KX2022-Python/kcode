#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::unneeded_struct_pattern,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
mod init;
mod input;
mod render;
mod render_semantic;
mod render_theme;

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::net::TcpListener;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, UNIX_EPOCH};

use api::{
    AuthSource, ContentBlockDelta, InputContentBlock, InputMessage,
    MessageRequest, MessageResponse, OpenAiCompatClient, OpenAiCompatConfig, OutputContentBlock,
    StreamEvent as ApiStreamEvent, ToolChoice, ToolDefinition, ToolResultContentBlock,
};

use commands::{
    build_command_registry_snapshot, handle_agents_slash_command, handle_mcp_slash_command,
    handle_plugins_slash_command, handle_skills_slash_command, render_slash_command_help,
    render_slash_command_help_for_context, resume_supported_slash_commands, slash_command_specs,
    validate_slash_command_input, CommandDescriptor, CommandRegistryContext, CommandScope,
    CommandSurface, FilteredCommand, SlashCommand,
};
use init::{initialize_repo, initialize_user_config};
use plugins::{PluginHooks, PluginManager, PluginManagerConfig, PluginRegistry};
use render::{MarkdownStreamState, Spinner, TerminalRenderer};
use render_semantic::{RenderIntent, RenderPolicy, SemanticRole};
use render_theme::{render_intents, render_with_palette, ThemePalette};
use runtime::{
    builtin_profiles, clear_oauth_credentials, default_memory_dir, ensure_memory_dir,
    ensure_memory_index, generate_pkce_pair, generate_state, list_memories,
    load_system_prompt, MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES, parse_oauth_callback_request_target,
    render_memory_summary, resolve_sandbox_status, save_oauth_credentials, ApiClient, ApiRequest,
    AssistantEvent, BootstrapInputs, CompactionConfig, ConfigLoader, ConfigSource, ContentBlock,
    ConversationMessage, ConversationRuntime, DiagnosticCheck, DiagnosticStatus, MemoryType,
    MessageRole, OAuthAuthorizationRequest, OAuthConfig, OAuthTokenExchangeRequest, PermissionMode,
    PermissionPolicy, ProfileResolver, ProjectContext, PromptCacheEvent, ProviderLaunchConfig,
    ProviderLauncher, ProviderProfile, ProviderProfileError, ResolutionSource, ResolvedConfig,
    ResolvedPermissionMode, ResolvedProviderProfile, RuntimeError, Session, SetupContext,
    SetupMode, StdioMode, TokenUsage, ToolError, ToolExecutor, TrustPolicyContext, UsageTracker,
};
use serde_json::json;
use tools::GlobalToolRegistry;

// v1.1 Bridge Modules
mod bridge_core;
use bridge_core::{BridgeCore, BridgeMessage, SessionConfig};

use adapters::{TelegramConfig, TelegramMode, TelegramTransport};

const DEFAULT_MODEL: &str = "claude-opus-4-6";
const CLI_NAME: &str = "kcode";
const PRIMARY_CONFIG_DIR_NAME: &str = ".kcode";
const LEGACY_CONFIG_DIR_NAME: &str = ".claw";
const PRIMARY_SESSION_DIR_ENV: &str = "KCODE_SESSION_DIR";
const LEGACY_SESSION_DIR_ENV: &str = "CLAW_SESSION_DIR";
const PRIMARY_PERMISSION_MODE_ENV: &str = "KCODE_PERMISSION_MODE";
const LEGACY_PERMISSION_MODE_ENV: &str = "RUSTY_CLAUDE_PERMISSION_MODE";
const PRIMARY_MODEL_ENV: &str = "KCODE_MODEL";
const PRIMARY_BASE_URL_ENV: &str = "KCODE_BASE_URL";
const PRIMARY_API_KEY_ENV: &str = "KCODE_API_KEY";
const PRIMARY_PROFILE_ENV: &str = "KCODE_PROFILE";
const PRIMARY_CONFIG_HOME_ENV: &str = "KCODE_CONFIG_HOME";
const LEGACY_CONFIG_HOME_ENV: &str = "CLAW_CONFIG_HOME";
fn max_tokens_for_model(model: &str) -> u32 {
    if model.contains("opus") {
        32_000
    } else {
        64_000
    }
}
const DEFAULT_DATE: &str = "2026-03-31";
const DEFAULT_OAUTH_CALLBACK_PORT: u16 = 4545;
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_TARGET: Option<&str> = option_env!("TARGET");
const GIT_SHA: Option<&str> = option_env!("GIT_SHA");
const INTERNAL_PROGRESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
const PRIMARY_SESSION_EXTENSION: &str = "jsonl";
const LEGACY_SESSION_EXTENSION: &str = "json";
const LATEST_SESSION_REFERENCE: &str = "latest";
const SESSION_REFERENCE_ALIASES: &[&str] = &[LATEST_SESSION_REFERENCE, "last", "recent"];
const CLI_OPTION_SUGGESTIONS: &[&str] = &[
    "--help",
    "-h",
    "--version",
    "-V",
    "--model",
    "--profile",
    "--output-format",
    "--permission-mode",
    "--dangerously-skip-permissions",
    "--allowedTools",
    "--allowed-tools",
    "--resume",
    "--print",
    "-p",
];

type AllowedToolSet = BTreeSet<String>;

fn main() {
    if let Err(error) = run() {
        let message = error.to_string();
        if message.contains(&format!("`{CLI_NAME} --help`")) {
            eprintln!("error: {message}");
        } else {
            eprintln!(
                "error: {message}

Run `{CLI_NAME} --help` for usage."
            );
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        CliAction::Agents { args } => LiveCli::print_agents(args.as_deref())?,
        CliAction::Mcp { args, profile } => {
            ensure_process_command_available("mcp", None, profile.as_deref())?;
            LiveCli::print_mcp(args.as_deref())?
        }
        CliAction::Skills { args } => LiveCli::print_skills(args.as_deref())?,
        CliAction::PrintSystemPrompt { cwd, date } => print_system_prompt(cwd, date),
        CliAction::Version => print_version(),
        CliAction::ResumeSession {
            session_path,
            commands,
        } => resume_session(&session_path, &commands),
        CliAction::Doctor {
            model,
            model_explicit,
            profile,
        } => print_doctor(model_explicit.then_some(model.as_str()), profile.as_deref())?,
        CliAction::ConfigShow {
            section,
            model,
            model_explicit,
            profile,
        } => print_config_show(
            section.as_deref(),
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
        )?,
        CliAction::Commands {
            surface,
            model,
            model_explicit,
            profile,
        } => print_commands_report(
            surface,
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
        )?,
        CliAction::Profile {
            selection,
            model,
            model_explicit,
            profile,
        } => print_profile_report(
            &selection,
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
        )?,
        CliAction::Status {
            model,
            model_explicit,
            profile,
            permission_mode,
        } => print_status_snapshot(
            &model,
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
            permission_mode,
        )?,
        CliAction::Sandbox => print_sandbox_status_snapshot()?,
        CliAction::Prompt {
            prompt,
            model,
            model_explicit,
            profile,
            output_format,
            allowed_tools,
            permission_mode,
        } => LiveCli::new(
            model,
            model_explicit,
            profile,
            true,
            allowed_tools,
            permission_mode,
        )?
        .run_turn_with_output(&prompt, output_format)?,
        CliAction::Login => run_login()?,
        CliAction::Logout => run_logout()?,
        CliAction::Init => run_init()?,
        CliAction::Repl {
            model,
            model_explicit,
            profile,
            allowed_tools,
            permission_mode,
        } => run_repl(
            model,
            model_explicit,
            profile,
            allowed_tools,
            permission_mode,
        )?,
        CliAction::Bridge {
            model,
            model_explicit,
            profile,
            permission_mode,
        } => run_bridge(
            model,
            model_explicit,
            profile,
            permission_mode,
        )?,
        CliAction::Help { profile } => print_help(profile.as_deref()),
    }
    Ok(())
}

// ... (other existing code)

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    Agents {
        args: Option<String>,
    },
    Mcp {
        args: Option<String>,
        profile: Option<String>,
    },
    Skills {
        args: Option<String>,
    },
    PrintSystemPrompt {
        cwd: PathBuf,
        date: String,
    },
    Version,
    ResumeSession {
        session_path: PathBuf,
        commands: Vec<String>,
    },
    Doctor {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    ConfigShow {
        section: Option<String>,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    Commands {
        surface: CommandReportSurfaceSelection,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    Profile {
        selection: ProfileCommandSelection,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    Status {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        permission_mode: PermissionMode,
    },
    Sandbox,
    Prompt {
        prompt: String,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        output_format: CliOutputFormat,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    Login,
    Logout,
    Init,
    Repl {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    Bridge {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        permission_mode: PermissionMode,
    },
    // prompt-mode formatting is only supported for non-interactive runs
    Help {
        profile: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfileCommandSelection {
    List,
    Show { profile_name: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandReportSurfaceSelection {
    Local,
    Bridge,
}

impl CommandReportSurfaceSelection {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "bridge" => Ok(Self::Bridge),
            other => Err(format!(
                "unsupported commands surface: {other} (expected local or bridge)"
            )),
        }
    }

    const fn command_surface(self) -> CommandSurface {
        match self {
            Self::Local => CommandSurface::CliLocal,
            Self::Bridge => CommandSurface::Bridge,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Bridge => "bridge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliOutputFormat {
    Text,
    Json,
}

impl CliOutputFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unsupported value for --output-format: {other} (expected text or json)"
            )),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn parse_args(args: &[String]) -> Result<CliAction, String> {
    let mut model = DEFAULT_MODEL.to_string();
    let mut model_explicit = false;
    let mut profile = None;
    let mut output_format = CliOutputFormat::Text;
    let mut permission_mode = default_permission_mode();
    let mut wants_help = false;
    let mut wants_version = false;
    let mut allowed_tool_values = Vec::new();
    let mut rest = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" if rest.is_empty() => {
                wants_help = true;
                index += 1;
            }
            "--version" | "-V" => {
                wants_version = true;
                index += 1;
            }
            "--model" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --model".to_string())?;
                model = resolve_model_alias(value).to_string();
                model_explicit = true;
                index += 2;
            }
            "--profile" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --profile".to_string())?;
                profile = Some(value.trim().to_string());
                index += 2;
            }
            flag if flag.starts_with("--model=") => {
                model = resolve_model_alias(&flag[8..]).to_string();
                model_explicit = true;
                index += 1;
            }
            flag if flag.starts_with("--profile=") => {
                profile = Some(flag[10..].trim().to_string());
                index += 1;
            }
            "--output-format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --output-format".to_string())?;
                output_format = CliOutputFormat::parse(value)?;
                index += 2;
            }
            "--permission-mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --permission-mode".to_string())?;
                permission_mode = parse_permission_mode_arg(value)?;
                index += 2;
            }
            flag if flag.starts_with("--output-format=") => {
                output_format = CliOutputFormat::parse(&flag[16..])?;
                index += 1;
            }
            flag if flag.starts_with("--permission-mode=") => {
                permission_mode = parse_permission_mode_arg(&flag[18..])?;
                index += 1;
            }
            "--dangerously-skip-permissions" => {
                permission_mode = PermissionMode::DangerFullAccess;
                index += 1;
            }
            "-p" => {
                // Claw Code compat: -p "prompt" = one-shot prompt
                let prompt = args[index + 1..].join(" ");
                if prompt.trim().is_empty() {
                    return Err("-p requires a prompt string".to_string());
                }
                return Ok(CliAction::Prompt {
                    prompt,
                    model: resolve_model_alias(&model).to_string(),
                    model_explicit,
                    profile: profile.clone(),
                    output_format,
                    allowed_tools: normalize_allowed_tools(
                        &allowed_tool_values,
                        profile.as_deref(),
                    )?,
                    permission_mode,
                });
            }
            "--print" => {
                // Claw Code compat: --print makes output non-interactive
                output_format = CliOutputFormat::Text;
                index += 1;
            }
            "--resume" if rest.is_empty() => {
                rest.push("--resume".to_string());
                index += 1;
            }
            flag if rest.is_empty() && flag.starts_with("--resume=") => {
                rest.push("--resume".to_string());
                rest.push(flag[9..].to_string());
                index += 1;
            }
            "--allowedTools" | "--allowed-tools" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --allowedTools".to_string())?;
                allowed_tool_values.push(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--allowedTools=") => {
                allowed_tool_values.push(flag[15..].to_string());
                index += 1;
            }
            flag if flag.starts_with("--allowed-tools=") => {
                allowed_tool_values.push(flag[16..].to_string());
                index += 1;
            }
            other if rest.is_empty() && other.starts_with('-') => {
                return Err(format_unknown_option(other))
            }
            other => {
                rest.push(other.to_string());
                index += 1;
            }
        }
    }

    if wants_help {
        return Ok(CliAction::Help {
            profile: profile.clone(),
        });
    }

    if wants_version {
        return Ok(CliAction::Version);
    }

    let allowed_tools = normalize_allowed_tools(&allowed_tool_values, profile.as_deref())?;

    if rest.is_empty() {
        return Ok(CliAction::Repl {
            model,
            model_explicit,
            profile,
            allowed_tools,
            permission_mode,
        });
    }
    if rest.first().map(String::as_str) == Some("--resume") {
        return parse_resume_args(&rest[1..]);
    }
    if let Some(action) = parse_single_word_command_alias(
        &rest,
        &model,
        model_explicit,
        profile.as_deref(),
        permission_mode,
    ) {
        return action;
    }

    match rest[0].as_str() {
        "agents" => Ok(CliAction::Agents {
            args: join_optional_args(&rest[1..]),
        }),
        "mcp" => Ok(CliAction::Mcp {
            args: join_optional_args(&rest[1..]),
            profile,
        }),
        "skills" => Ok(CliAction::Skills {
            args: join_optional_args(&rest[1..]),
        }),
        "system-prompt" => parse_system_prompt_args(&rest[1..]),
        "doctor" => Ok(CliAction::Doctor {
            model,
            model_explicit,
            profile,
        }),
        "config" => parse_config_args(&rest[1..], &model, model_explicit, profile.clone()),
        "commands" => parse_commands_args(&rest[1..], &model, model_explicit, profile.clone()),
        "profile" => parse_profile_args(&rest[1..], &model, model_explicit, profile.clone()),
        "login" => Ok(CliAction::Login),
        "logout" => Ok(CliAction::Logout),
        "init" => Ok(CliAction::Init),
        "bridge" => Ok(CliAction::Bridge {
            model,
            model_explicit,
            profile,
            permission_mode,
        }),
        "prompt" => {
            let prompt = rest[1..].join(" ");
            if prompt.trim().is_empty() {
                return Err("prompt subcommand requires a prompt string".to_string());
            }
            Ok(CliAction::Prompt {
                prompt,
                model,
                model_explicit,
                profile,
                output_format,
                allowed_tools,
                permission_mode,
            })
        }
        other if other.starts_with('/') => parse_direct_slash_cli_action(&rest, profile.clone()),
        _other => Ok(CliAction::Prompt {
            prompt: rest.join(" "),
            model,
            model_explicit,
            profile,
            output_format,
            allowed_tools,
            permission_mode,
        }),
    }
}

fn parse_single_word_command_alias(
    rest: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<&str>,
    permission_mode: PermissionMode,
) -> Option<Result<CliAction, String>> {
    if rest.len() != 1 {
        return None;
    }

    match rest[0].as_str() {
        "help" => Some(Ok(CliAction::Help {
            profile: profile.map(ToOwned::to_owned),
        })),
        "version" => Some(Ok(CliAction::Version)),
        "doctor" => Some(Ok(CliAction::Doctor {
            model: model.to_string(),
            model_explicit,
            profile: profile.map(ToOwned::to_owned),
        })),
        "profile" => Some(Ok(CliAction::Profile {
            selection: ProfileCommandSelection::Show { profile_name: None },
            model: model.to_string(),
            model_explicit,
            profile: profile.map(ToOwned::to_owned),
        })),
        "status" => Some(Ok(CliAction::Status {
            model: model.to_string(),
            model_explicit,
            profile: profile.map(ToOwned::to_owned),
            permission_mode,
        })),
        "sandbox" => Some(Ok(CliAction::Sandbox)),
        other => bare_slash_command_guidance(other).map(Err),
    }
}

fn bare_slash_command_guidance(command_name: &str) -> Option<String> {
    if matches!(
        command_name,
        "agents"
            | "mcp"
            | "skills"
            | "profile"
            | "system-prompt"
            | "doctor"
            | "config"
            | "login"
            | "logout"
            | "init"
            | "prompt"
    ) {
        return None;
    }
    let slash_command = slash_command_specs()
        .iter()
        .find(|spec| spec.name == command_name)?;
    let guidance = if slash_command.resume_supported {
        format!(
            "`{CLI_NAME} {command_name}` is a slash command. Use `{CLI_NAME} --resume SESSION.jsonl /{command_name}` or start `{CLI_NAME}` and run `/{command_name}`."
        )
    } else {
        format!(
            "`{CLI_NAME} {command_name}` is a slash command. Start `{CLI_NAME}` and run `/{command_name}` inside the REPL."
        )
    };
    Some(guidance)
}

fn parse_config_args(
    args: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<String>,
) -> Result<CliAction, String> {
    match args {
        [] => Ok(CliAction::ConfigShow {
            section: None,
            model: model.to_string(),
            model_explicit,
            profile,
        }),
        [subcommand] if subcommand == "show" => Ok(CliAction::ConfigShow {
            section: None,
            model: model.to_string(),
            model_explicit,
            profile,
        }),
        [subcommand, section] if subcommand == "show" => Ok(CliAction::ConfigShow {
            section: Some(section.clone()),
            model: model.to_string(),
            model_explicit,
            profile,
        }),
        _ => Err("usage: kcode config show [env|hooks|model|plugins|profile|provider]".to_string()),
    }
}

fn parse_profile_args(
    args: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<String>,
) -> Result<CliAction, String> {
    let selection = match args {
        [] => ProfileCommandSelection::Show { profile_name: None },
        [subcommand] if subcommand == "list" => ProfileCommandSelection::List,
        [subcommand] if subcommand == "show" => {
            ProfileCommandSelection::Show { profile_name: None }
        }
        [subcommand, name] if subcommand == "show" => ProfileCommandSelection::Show {
            profile_name: Some(name.clone()),
        },
        _ => return Err("usage: kcode profile [list|show [name]]".to_string()),
    };

    Ok(CliAction::Profile {
        selection,
        model: model.to_string(),
        model_explicit,
        profile,
    })
}

fn parse_commands_args(
    args: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<String>,
) -> Result<CliAction, String> {
    let surface = match args {
        [] => CommandReportSurfaceSelection::Local,
        [subcommand] if subcommand == "show" => CommandReportSurfaceSelection::Local,
        [surface] => CommandReportSurfaceSelection::parse(surface)?,
        [subcommand, surface] if subcommand == "show" => {
            CommandReportSurfaceSelection::parse(surface)?
        }
        _ => return Err("usage: kcode commands [show [local|bridge]]".to_string()),
    };

    Ok(CliAction::Commands {
        surface,
        model: model.to_string(),
        model_explicit,
        profile,
    })
}

fn join_optional_args(args: &[String]) -> Option<String> {
    let joined = args.join(" ");
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn parse_direct_slash_cli_action(
    rest: &[String],
    profile: Option<String>,
) -> Result<CliAction, String> {
    let raw = rest.join(" ");
    match SlashCommand::parse(&raw) {
        Ok(Some(SlashCommand::Help)) => Ok(CliAction::Help { profile }),
        Ok(Some(SlashCommand::Agents { args })) => Ok(CliAction::Agents { args }),
        Ok(Some(SlashCommand::Mcp { action, target })) => Ok(CliAction::Mcp {
            args: match (action, target) {
                (None, None) => None,
                (Some(action), None) => Some(action),
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (None, Some(target)) => Some(target),
            },
            profile,
        }),
        Ok(Some(SlashCommand::Skills { args })) => Ok(CliAction::Skills { args }),
        Ok(Some(SlashCommand::Unknown(name))) => Err(format_unknown_direct_slash_command(&name)),
        Ok(Some(command)) => Err({
            let _ = command;
            format!(
                "slash command {command_name} is interactive-only. Start `{CLI_NAME}` and run it there, or use `{CLI_NAME} --resume SESSION.jsonl {command_name}` / `{CLI_NAME} --resume {latest} {command_name}` when the command is marked [resume] in /help.",
                command_name = rest[0],
                latest = LATEST_SESSION_REFERENCE,
            )
        }),
        Ok(None) => Err(format!("unknown subcommand: {}", rest[0])),
        Err(error) => Err(error.to_string()),
    }
}

fn format_unknown_option(option: &str) -> String {
    let mut message = format!("unknown option: {option}");
    if let Some(suggestion) = suggest_closest_term(option, CLI_OPTION_SUGGESTIONS) {
        message.push_str("\nDid you mean ");
        message.push_str(suggestion);
        message.push('?');
    }
    message.push_str(&format!("\nRun `{CLI_NAME} --help` for usage."));
    message
}

fn format_unknown_direct_slash_command(name: &str) -> String {
    let mut message = format!("unknown slash command outside the REPL: /{name}");
    if let Some(suggestions) = render_suggestion_line("Did you mean", &suggest_slash_commands(name))
    {
        message.push('\n');
        message.push_str(&suggestions);
    }
    message.push_str(&format!(
        "\nRun `{CLI_NAME} --help` for CLI usage, or start `{CLI_NAME}` and use /help."
    ));
    message
}

fn format_unknown_slash_command(name: &str) -> String {
    let mut message = format!("Unknown slash command: /{name}");
    if let Some(suggestions) = render_suggestion_line("Did you mean", &suggest_slash_commands(name))
    {
        message.push('\n');
        message.push_str(&suggestions);
    }
    message.push_str("\n  Help             /help lists available slash commands");
    message
}

fn render_suggestion_line(label: &str, suggestions: &[String]) -> Option<String> {
    (!suggestions.is_empty()).then(|| format!("  {label:<16} {}", suggestions.join(", "),))
}

fn suggest_slash_commands(input: &str) -> Vec<String> {
    let mut candidates = slash_command_specs()
        .iter()
        .flat_map(|spec| {
            std::iter::once(spec.name)
                .chain(spec.aliases.iter().copied())
                .map(|name| format!("/{name}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    let candidate_refs = candidates.iter().map(String::as_str).collect::<Vec<_>>();
    ranked_suggestions(input.trim_start_matches('/'), &candidate_refs)
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn suggest_closest_term<'a>(input: &str, candidates: &'a [&'a str]) -> Option<&'a str> {
    ranked_suggestions(input, candidates).into_iter().next()
}

fn ranked_suggestions<'a>(input: &str, candidates: &'a [&'a str]) -> Vec<&'a str> {
    let normalized_input = input.trim_start_matches('/').to_ascii_lowercase();
    let mut ranked = candidates
        .iter()
        .filter_map(|candidate| {
            let normalized_candidate = candidate.trim_start_matches('/').to_ascii_lowercase();
            let distance = levenshtein_distance(&normalized_input, &normalized_candidate);
            let prefix_bonus = usize::from(
                !(normalized_candidate.starts_with(&normalized_input)
                    || normalized_input.starts_with(&normalized_candidate)),
            );
            let score = distance + prefix_bonus;
            (score <= 4).then_some((score, *candidate))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.cmp(right).then_with(|| left.1.cmp(right.1)));
    ranked
        .into_iter()
        .map(|(_, candidate)| candidate)
        .take(3)
        .collect()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
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
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

fn resolve_model_alias(model: &str) -> &str {
    match model {
        "opus" => "claude-opus-4-6",
        "sonnet" => "claude-sonnet-4-6",
        "haiku" => "claude-haiku-4-5-20251213",
        _ => model,
    }
}

fn normalize_allowed_tools(
    values: &[String],
    profile_override: Option<&str>,
) -> Result<Option<AllowedToolSet>, String> {
    if values.is_empty() {
        return Ok(None);
    }

    let (active_profile, tool_registry) = current_tool_registry(profile_override)?;
    if !active_profile.profile.supports_tools {
        return Err(format!(
            "`--allowedTools` is unavailable because active profile `{}` disables tools",
            active_profile.profile_name
        ));
    }

    tool_registry.normalize_allowed_tools(values)
}

fn current_tool_registry(
    profile_override: Option<&str>,
) -> Result<(ResolvedProviderProfile, GlobalToolRegistry), String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load().map_err(|error| error.to_string())?;
    let active_profile = ProfileResolver::resolve(&runtime_config, profile_override, None)
        .map_err(|error| error.to_string())?;
    let plugin_manager = build_plugin_manager(&cwd, &loader, &runtime_config);
    let plugin_tools = plugin_manager
        .aggregated_tools()
        .map_err(|error| error.to_string())?;
    let tool_registry =
        GlobalToolRegistry::with_plugin_tools(plugin_tools).map_err(|error| error.to_string())?;
    Ok((active_profile, tool_registry))
}

fn parse_permission_mode_arg(value: &str) -> Result<PermissionMode, String> {
    normalize_permission_mode(value)
        .ok_or_else(|| {
            format!(
                "unsupported permission mode '{value}'. Use read-only, workspace-write, or danger-full-access."
            )
        })
        .map(permission_mode_from_label)
}

fn permission_mode_from_label(mode: &str) -> PermissionMode {
    match mode {
        "read-only" => PermissionMode::ReadOnly,
        "workspace-write" => PermissionMode::WorkspaceWrite,
        "danger-full-access" => PermissionMode::DangerFullAccess,
        other => panic!("unsupported permission mode label: {other}"),
    }
}

fn permission_mode_from_resolved(mode: ResolvedPermissionMode) -> PermissionMode {
    match mode {
        ResolvedPermissionMode::ReadOnly => PermissionMode::ReadOnly,
        ResolvedPermissionMode::WorkspaceWrite => PermissionMode::WorkspaceWrite,
        ResolvedPermissionMode::DangerFullAccess => PermissionMode::DangerFullAccess,
    }
}

fn default_permission_mode() -> PermissionMode {
    env::var(PRIMARY_PERMISSION_MODE_ENV)
        .ok()
        .or_else(|| env::var(LEGACY_PERMISSION_MODE_ENV).ok())
        .as_deref()
        .and_then(normalize_permission_mode)
        .map(permission_mode_from_label)
        .or_else(config_permission_mode_for_current_dir)
        .unwrap_or(PermissionMode::DangerFullAccess)
}

fn config_permission_mode_for_current_dir() -> Option<PermissionMode> {
    let cwd = env::current_dir().ok()?;
    let loader = ConfigLoader::default_for(&cwd);
    loader
        .load()
        .ok()?
        .permission_mode()
        .map(permission_mode_from_resolved)
}

fn filter_tool_specs(
    tool_registry: &GlobalToolRegistry,
    allowed_tools: Option<&AllowedToolSet>,
) -> Vec<ToolDefinition> {
    tool_registry.definitions(allowed_tools)
}

/// Filter tool specs by permission mode before sending to the model.
/// Tools that require a higher permission level than the active mode
/// are excluded from the model-visible tool list.
fn filter_tools_by_permission_mode(
    tool_registry: &GlobalToolRegistry,
    allowed_tools: Option<&AllowedToolSet>,
    active_mode: PermissionMode,
) -> Vec<ToolDefinition> {
    tool_registry
        .definitions(allowed_tools)
        .into_iter()
        .filter(|def| {
            // Check if this tool's required permission is compatible with active mode
            // ReadOnly tools are always visible; higher tools require matching mode
            match def.name.as_str() {
                "bash" => matches!(active_mode, PermissionMode::DangerFullAccess),
                "write_file" | "edit_file" | "notebook_edit" => {
                    matches!(
                        active_mode,
                        PermissionMode::WorkspaceWrite | PermissionMode::DangerFullAccess
                    )
                }
                _ => true, // ReadOnly tools always visible
            }
        })
        .collect()
}

fn parse_system_prompt_args(args: &[String]) -> Result<CliAction, String> {
    let mut cwd = env::current_dir().map_err(|error| error.to_string())?;
    let mut date = DEFAULT_DATE.to_string();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--cwd" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --cwd".to_string())?;
                cwd = PathBuf::from(value);
                index += 2;
            }
            "--date" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --date".to_string())?;
                date.clone_from(value);
                index += 2;
            }
            other => return Err(format!("unknown system-prompt option: {other}")),
        }
    }

    Ok(CliAction::PrintSystemPrompt { cwd, date })
}

fn parse_resume_args(args: &[String]) -> Result<CliAction, String> {
    let (session_path, command_tokens): (PathBuf, &[String]) = match args.first() {
        None => (PathBuf::from(LATEST_SESSION_REFERENCE), &[]),
        Some(first) if looks_like_slash_command_token(first) => {
            (PathBuf::from(LATEST_SESSION_REFERENCE), args)
        }
        Some(first) => (PathBuf::from(first), &args[1..]),
    };
    let mut commands = Vec::new();
    let mut current_command = String::new();

    for token in command_tokens {
        if token.trim_start().starts_with('/') {
            if resume_command_can_absorb_token(&current_command, token) {
                current_command.push(' ');
                current_command.push_str(token);
                continue;
            }
            if !current_command.is_empty() {
                commands.push(current_command);
            }
            current_command = String::from(token.as_str());
            continue;
        }

        if current_command.is_empty() {
            return Err("--resume trailing arguments must be slash commands".to_string());
        }

        current_command.push(' ');
        current_command.push_str(token);
    }

    if !current_command.is_empty() {
        commands.push(current_command);
    }

    Ok(CliAction::ResumeSession {
        session_path,
        commands,
    })
}

fn resume_command_can_absorb_token(current_command: &str, token: &str) -> bool {
    matches!(
        SlashCommand::parse(current_command),
        Ok(Some(SlashCommand::Export { path: None }))
    ) && !looks_like_slash_command_token(token)
}

fn looks_like_slash_command_token(token: &str) -> bool {
    let trimmed = token.trim_start();
    let Some(name) = trimmed.strip_prefix('/').and_then(|value| {
        value
            .split_whitespace()
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }) else {
        return false;
    };

    slash_command_specs()
        .iter()
        .any(|spec| spec.name == name || spec.aliases.contains(&name))
}

fn default_oauth_config() -> OAuthConfig {
    OAuthConfig {
        client_id: String::from("9d1c250a-e61b-44d9-88ed-5944d1962f5e"),
        authorize_url: String::from("https://platform.claude.com/oauth/authorize"),
        token_url: String::from("https://platform.claude.com/v1/oauth/token"),
        callback_port: None,
        manual_redirect_url: None,
        scopes: vec![
            String::from("user:profile"),
            String::from("user:inference"),
            String::from("user:sessions:claude_code"),
        ],
    }
}

fn run_login() -> Result<(), Box<dyn std::error::Error>> {
    println!("Login is retired. Kcode uses configuration-driven authentication.");
    println!("Set your credentials via KCODE_API_KEY, KCODE_BASE_URL, KCODE_MODEL env vars.");
    println!("Or edit ~/.kcode/config.toml directly.");
    println!("Run `kcode doctor` to verify your setup.");
    Ok(())
}

fn run_logout() -> Result<(), Box<dyn std::error::Error>> {
    println!("Logout is retired. Kcode does not use OAuth authentication.");
    println!("To change your API key, update KCODE_API_KEY or edit ~/.kcode/config.toml.");
    Ok(())
}

fn open_browser(url: &str) -> io::Result<()> {
    let commands = if cfg!(target_os = "macos") {
        vec![("open", vec![url])]
    } else if cfg!(target_os = "windows") {
        vec![("cmd", vec!["/C", "start", "", url])]
    } else {
        vec![("xdg-open", vec![url])]
    };
    for (program, args) in commands {
        match Command::new(program).args(args).spawn() {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no supported browser opener command found",
    ))
}

fn wait_for_oauth_callback(
    port: u16,
) -> Result<runtime::OAuthCallbackParams, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let (mut stream, _) = listener.accept()?;
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing callback request line")
    })?;
    let target = request_line.split_whitespace().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "missing callback request target",
        )
    })?;
    let callback = parse_oauth_callback_request_target(target)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let body = if callback.error.is_some() {
        "Claude OAuth login failed. You can close this window."
    } else {
        "Claude OAuth login succeeded. You can close this window."
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(callback)
}

fn print_system_prompt(cwd: PathBuf, date: String) {
    match load_system_prompt(cwd, date, env::consts::OS, "unknown") {
        Ok(sections) => println!("{}", sections.join("\n\n")),
        Err(error) => {
            eprintln!("failed to build system prompt: {error}");
            std::process::exit(1);
        }
    }
}

fn print_version() {
    println!("{}", render_version_report());
}

fn resume_session(session_path: &Path, commands: &[String]) {
    let resolved_path = if session_path.exists() {
        session_path.to_path_buf()
    } else {
        match resolve_session_reference(&session_path.display().to_string()) {
            Ok(handle) => handle.path,
            Err(error) => {
                eprintln!("failed to restore session: {error}");
                std::process::exit(1);
            }
        }
    };

    let session = match Session::load_from_path(&resolved_path) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("failed to restore session: {error}");
            std::process::exit(1);
        }
    };

    if commands.is_empty() {
        println!(
            "Restored session from {} ({} messages).",
            resolved_path.display(),
            session.messages.len()
        );
        return;
    }

    let mut session = session;
    for raw_command in commands {
        let command = match SlashCommand::parse(raw_command) {
            Ok(Some(command)) => command,
            Ok(None) => {
                eprintln!("unsupported resumed command: {raw_command}");
                std::process::exit(2);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        };
        match run_resume_command(&resolved_path, &session, &command) {
            Ok(ResumeCommandOutcome {
                session: next_session,
                message,
            }) => {
                session = next_session;
                if let Some(message) = message {
                    println!("{message}");
                }
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    }
}

#[derive(Debug, Clone)]
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

fn load_setup_context(
    mode: SetupMode,
    model_override: Option<&str>,
    profile_override: Option<&str>,
    permission_mode: PermissionMode,
    session_id: Option<&str>,
) -> Result<SetupContext, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let discovered_entries = loader.discover();
    let runtime_config = loader.load()?;
    let active_profile =
        ProfileResolver::resolve(&runtime_config, profile_override, model_override)?;
    let project_context = ProjectContext::discover_with_git(&cwd, DEFAULT_DATE)?;
    let git_root = find_git_root_in(&cwd).ok();
    let project_root = git_root.clone().unwrap_or_else(|| cwd.clone());
    let config_home = loader.config_home().to_path_buf();
    let session_dir = resolve_setup_session_dir(&cwd, &runtime_config);
    let oauth_credentials_present = runtime::load_oauth_credentials()?.is_some();
    let legacy_paths =
        collect_legacy_paths(&discovered_entries, &project_context.instruction_files);
    let resolved_config = ResolvedConfig {
        config_home: config_home.clone(),
        session_dir,
        discovered_entries,
        loaded_entries: runtime_config.loaded_entries().to_vec(),
        config_file_present: runtime_config.loaded_entries().iter().any(|entry| {
            entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "config.toml")
        }),
        model: active_profile.model.clone(),
        base_url: active_profile.base_url.clone(),
        api_key_env: active_profile.credential.env_name.clone(),
        api_key_present: active_profile.credential.api_key.is_some(),
        oauth_credentials_present,
        profile: Some(active_profile.profile_name.clone()),
        legacy_paths,
    };
    let trust_policy = TrustPolicyContext {
        permission_mode: permission_mode.as_str().to_string(),
        workspace_writeable: path_or_parent_writeable(&cwd),
        config_home_writeable: path_or_parent_writeable(&config_home),
        trusted_workspace: path_or_parent_writeable(&cwd),
    };

    Ok(SetupContext {
        inputs: BootstrapInputs {
            argv: env::args().collect(),
            cwd: cwd.clone(),
            platform: env::consts::OS.to_string(),
            stdio_mode: current_stdio_mode(),
            invocation_kind: mode,
        },
        session_id: session_id.map(ToOwned::to_owned),
        cwd,
        project_root,
        git_root,
        resolved_config,
        active_profile,
        trust_policy,
        mode,
    })
}

fn current_stdio_mode() -> StdioMode {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        StdioMode::Interactive
    } else {
        StdioMode::NonInteractive
    }
}

fn read_non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn config_string_value(runtime_config: &runtime::RuntimeConfig, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| runtime_config.get(key))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn resolve_setup_session_dir(cwd: &Path, runtime_config: &runtime::RuntimeConfig) -> PathBuf {
    env::var_os(PRIMARY_SESSION_DIR_ENV)
        .map(PathBuf::from)
        .or_else(|| env::var_os(LEGACY_SESSION_DIR_ENV).map(PathBuf::from))
        .or_else(|| {
            config_string_value(runtime_config, &["session_dir", "sessionDir"]).map(|value| {
                let path = PathBuf::from(value);
                if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                }
            })
        })
        .unwrap_or_else(|| cwd.join(PRIMARY_CONFIG_DIR_NAME).join("sessions"))
}

fn collect_legacy_paths(
    discovered_entries: &[runtime::ConfigEntry],
    instruction_files: &[runtime::ContextFile],
) -> Vec<PathBuf> {
    let mut legacy_paths = discovered_entries
        .iter()
        .map(|entry| entry.path.clone())
        .filter(|path| {
            let rendered = path.display().to_string();
            rendered.contains(".claw") || rendered.contains(".claude")
        })
        .collect::<Vec<_>>();

    for file in instruction_files {
        let rendered = file.path.display().to_string();
        if (rendered.contains(".claw")
            || rendered.contains(".claude")
            || rendered.ends_with("CLAUDE.md"))
            && !legacy_paths.iter().any(|path| path == &file.path)
        {
            legacy_paths.push(file.path.clone());
        }
    }

    legacy_paths
}

fn path_or_parent_writeable(path: &Path) -> bool {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return runtime::is_path_effectively_writeable(candidate);
        }
        current = candidate.parent();
    }
    false
}

fn has_explicit_bootstrap_inputs(setup: &SetupContext) -> bool {
    setup.resolved_config.config_file_present
        || setup.resolved_config.base_url.is_some()
        || setup.resolved_config.api_key_present
        || !matches!(
            setup.active_profile.profile_source,
            ResolutionSource::ProfileDefault
        )
}

fn ensure_setup_ready_for_runtime(setup: &SetupContext) -> Result<(), Box<dyn std::error::Error>> {
    if !has_explicit_bootstrap_inputs(setup) {
        return Err(format!(
            "Kcode is not initialized yet.\nRun `{CLI_NAME} init` to create `~/.kcode/config.toml`, then run `{CLI_NAME} doctor`."
        )
        .into());
    }
    if setup
        .resolved_config
        .base_url
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err(format!(
            "missing base URL.\nSet `{PRIMARY_BASE_URL_ENV}` or `base_url` in `~/.kcode/config.toml`, then rerun `{CLI_NAME} doctor`."
        )
        .into());
    }
    if !setup.resolved_config.api_key_present {
        return Err(format!(
            "missing API credentials.\nSet `{PRIMARY_API_KEY_ENV}` or the env named by `api_key_env`, then rerun `{CLI_NAME} doctor`."
        )
        .into());
    }
    if !path_or_parent_writeable(&setup.resolved_config.session_dir) {
        return Err(format!(
            "session directory is not writeable: {}\nAdjust `session_dir` or `{PRIMARY_SESSION_DIR_ENV}` before continuing.",
            setup.resolved_config.session_dir.display()
        )
        .into());
    }
    Ok(())
}

fn format_compact_report(removed: usize, resulting_messages: usize, skipped: bool) -> String {
    if skipped {
        format!(
            "Compact
  Result           skipped
  Reason           session below compaction threshold
  Messages kept    {resulting_messages}"
        )
    } else {
        format!(
            "Compact
  Result           compacted
  Messages removed {removed}
  Messages kept    {resulting_messages}"
        )
    }
}

fn format_auto_compaction_notice(removed: usize) -> String {
    format!("[auto-compacted: removed {removed} messages]")
}

fn parse_git_status_metadata(status: Option<&str>) -> (Option<PathBuf>, Option<String>) {
    parse_git_status_metadata_for(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        status,
    )
}

fn parse_git_status_branch(status: Option<&str>) -> Option<String> {
    let status = status?;
    let first_line = status.lines().next()?;
    let line = first_line.strip_prefix("## ")?;
    if line.starts_with("HEAD") {
        return Some("detached HEAD".to_string());
    }
    let branch = line.split(['.', ' ']).next().unwrap_or_default().trim();
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

fn parse_git_workspace_summary(status: Option<&str>) -> GitWorkspaceSummary {
    let mut summary = GitWorkspaceSummary::default();
    let Some(status) = status else {
        return summary;
    };

    for line in status.lines() {
        if line.starts_with("## ") || line.trim().is_empty() {
            continue;
        }

        summary.changed_files += 1;
        let mut chars = line.chars();
        let index_status = chars.next().unwrap_or(' ');
        let worktree_status = chars.next().unwrap_or(' ');

        if index_status == '?' && worktree_status == '?' {
            summary.untracked_files += 1;
            continue;
        }

        if index_status != ' ' {
            summary.staged_files += 1;
        }
        if worktree_status != ' ' {
            summary.unstaged_files += 1;
        }
        if (matches!(index_status, 'U' | 'A') && matches!(worktree_status, 'U' | 'A'))
            || index_status == 'U'
            || worktree_status == 'U'
        {
            summary.conflicted_files += 1;
        }
    }

    summary
}

fn resolve_git_branch_for(cwd: &Path) -> Option<String> {
    let branch = run_git_capture_in(cwd, &["branch", "--show-current"])?;
    let branch = branch.trim();
    if !branch.is_empty() {
        return Some(branch.to_string());
    }

    let fallback = run_git_capture_in(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let fallback = fallback.trim();
    if fallback.is_empty() {
        None
    } else if fallback == "HEAD" {
        Some("detached HEAD".to_string())
    } else {
        Some(fallback.to_string())
    }
}

fn run_git_capture_in(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn find_git_root_in(cwd: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        return Err("not a git repository".into());
    }
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    if path.is_empty() {
        return Err("empty git root".into());
    }
    Ok(PathBuf::from(path))
}

fn parse_git_status_metadata_for(
    cwd: &Path,
    status: Option<&str>,
) -> (Option<PathBuf>, Option<String>) {
    let branch = resolve_git_branch_for(cwd).or_else(|| parse_git_status_branch(status));
    let project_root = find_git_root_in(cwd).ok();
    (project_root, branch)
}

#[allow(clippy::too_many_lines)]
fn run_resume_command(
    session_path: &Path,
    session: &Session,
    command: &SlashCommand,
) -> Result<ResumeCommandOutcome, Box<dyn std::error::Error>> {
    let resume_setup = load_setup_context(
        SetupMode::Resume,
        None,
        None,
        default_permission_mode(),
        Some(&session.session_id),
    )
    .ok();
    match command {
        SlashCommand::Help => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(match &resume_setup {
                Some(setup) => {
                    render_repl_help_for_profile(setup.active_profile.profile.supports_tools)
                }
                None => render_repl_help(),
            }),
        }),
        SlashCommand::Compact => {
            let result = runtime::compact_session(
                session,
                CompactionConfig {
                    max_estimated_tokens: 0,
                    ..CompactionConfig::default()
                },
            );
            let removed = result.removed_message_count;
            let kept = result.compacted_session.messages.len();
            let skipped = removed == 0;
            result.compacted_session.save_to_path(session_path)?;
            Ok(ResumeCommandOutcome {
                session: result.compacted_session,
                message: Some(format_compact_report(removed, kept, skipped)),
            })
        }
        SlashCommand::Clear { confirm } => {
            if !confirm {
                return Ok(ResumeCommandOutcome {
                    session: session.clone(),
                    message: Some(
                        "clear: confirmation required; rerun with /clear --confirm".to_string(),
                    ),
                });
            }
            let backup_path = write_session_clear_backup(session, session_path)?;
            let previous_session_id = session.session_id.clone();
            let cleared = Session::new();
            let new_session_id = cleared.session_id.clone();
            cleared.save_to_path(session_path)?;
            Ok(ResumeCommandOutcome {
                session: cleared,
                message: Some(format!(
                    "Session cleared\n  Mode             resumed session reset\n  Previous session {previous_session_id}\n  Backup           {}\n  Resume previous  {CLI_NAME} --resume {}\n  New session      {new_session_id}\n  Session file     {}",
                    backup_path.display(),
                    backup_path.display(),
                    session_path.display()
                )),
            })
        }
        SlashCommand::Status => {
            let tracker = UsageTracker::from_session(session);
            let usage = tracker.cumulative_usage();
            let setup = resume_setup.ok_or("missing resume setup context")?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_status_report(
                    "restored-session",
                    Some(&setup.active_profile),
                    StatusUsage {
                        message_count: session.messages.len(),
                        turns: tracker.turns(),
                        latest: tracker.current_turn_usage(),
                        cumulative: usage,
                        estimated_tokens: 0,
                    },
                    default_permission_mode().as_str(),
                    &status_context(Some(session_path))?,
                )),
            })
        }
        SlashCommand::Sandbox => {
            let cwd = env::current_dir()?;
            let loader = ConfigLoader::default_for(&cwd);
            let runtime_config = loader.load()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_sandbox_report(&resolve_sandbox_status(
                    runtime_config.sandbox(),
                    &cwd,
                ))),
            })
        }
        SlashCommand::Cost => {
            let usage = UsageTracker::from_session(session).cumulative_usage();
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_cost_report(usage)),
            })
        }
        SlashCommand::Doctor => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_doctor_report(None, None)?),
        }),
        SlashCommand::Config { section } => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_config_report(section.as_deref(), None, None)?),
        }),
        SlashCommand::Mcp { action, target } => {
            if let Some(setup) = &resume_setup {
                if let Err(message) =
                    ensure_session_command_available_for_profile("mcp", &setup.active_profile)
                {
                    return Ok(ResumeCommandOutcome {
                        session: session.clone(),
                        message: Some(message),
                    });
                }
            }
            let cwd = env::current_dir()?;
            let args = match (action.as_deref(), target.as_deref()) {
                (None, None) => None,
                (Some(action), None) => Some(action.to_string()),
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (None, Some(target)) => Some(target.to_string()),
            };
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_mcp_slash_command(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Memory => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_memory_report()?),
        }),
        SlashCommand::Tasks { args } => {
            let summary = match args.as_deref() {
                None | Some("list") => {
                    "Tasks\n  Background tasks are managed through the Agent tool.\n  Use /help Agent to see how to create and manage tasks.".to_string()
                }
                Some("help") => {
                    "Tasks\n  Background tasks allow running multiple agent sessions in parallel.\n\n  Commands:\n    /tasks              List active tasks\n    /tasks help         Show this help\n\n  Task management is done through the Agent tool:\n    Agent(action=create, description='...')   Create a new task\n    Agent(action=list)                        List all tasks\n    Agent(action=stop, task_id='...')         Stop a running task\n    Agent(action=output, task_id='...')       Get task output".to_string()
                }
                other => format!("Unknown tasks argument: {}. Use /tasks help for usage.", other.unwrap_or("")),
            };
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(summary),
            })
        }
        SlashCommand::Init => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(init_repo_kcode_md()?),
        }),
        SlashCommand::Diff => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_diff_report_for(
                session_path.parent().unwrap_or_else(|| Path::new(".")),
            )?),
        }),
        SlashCommand::Version => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_version_report()),
        }),
        SlashCommand::Export { path } => {
            let export_path = resolve_export_path(path.as_deref(), session)?;
            fs::write(&export_path, render_export_text(session))?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format!(
                    "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
                    export_path.display(),
                    session.messages.len(),
                )),
            })
        }
        SlashCommand::Agents { args } => {
            let cwd = env::current_dir()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_agents_slash_command(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Skills { args } => {
            let cwd = env::current_dir()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_skills_slash_command(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Unknown(name) => Err(format_unknown_slash_command(name).into()),
        SlashCommand::Bughunter { .. }
        | SlashCommand::Commit { .. }
        | SlashCommand::Pr { .. }
        | SlashCommand::Issue { .. }
        | SlashCommand::DebugToolCall { .. }
        | SlashCommand::Resume { .. }
        | SlashCommand::Model { .. }
        | SlashCommand::Permissions { .. }
        | SlashCommand::Session { .. }
        | SlashCommand::Plugins { .. }
        | SlashCommand::Login
        | SlashCommand::Logout
        | SlashCommand::Vim
        | SlashCommand::Upgrade
        | SlashCommand::Stats
        | SlashCommand::Share
        | SlashCommand::Feedback
        | SlashCommand::Files
        | SlashCommand::Fast
        | SlashCommand::Exit
        | SlashCommand::Summary
        | SlashCommand::Desktop
        | SlashCommand::Brief
        | SlashCommand::Advisor
        | SlashCommand::Stickers
        | SlashCommand::Insights
        | SlashCommand::Thinkback
        | SlashCommand::ReleaseNotes
        | SlashCommand::SecurityReview
        | SlashCommand::Keybindings
        | SlashCommand::PrivacySettings
        | SlashCommand::Plan { .. }
        | SlashCommand::Review { .. }
        | SlashCommand::Theme { .. }
        | SlashCommand::Voice { .. }
        | SlashCommand::Usage { .. }
        | SlashCommand::Rename { .. }
        | SlashCommand::Copy { .. }
        | SlashCommand::Hooks { .. }
        | SlashCommand::Context { .. }
        | SlashCommand::Color { .. }
        | SlashCommand::Effort { .. }
        | SlashCommand::Branch { .. }
        | SlashCommand::Rewind { .. }
        | SlashCommand::Ide { .. }
        | SlashCommand::Tag { .. }
        | SlashCommand::OutputStyle { .. }
        | SlashCommand::AddDir { .. } => Err("unsupported resumed slash command".into()),
    }
}

fn run_bridge(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Security: Load credentials from environment variables only.
    let bot_token = std::env::var("KCODE_TELEGRAM_BOT_TOKEN")
        .map_err(|_| "Environment variable KCODE_TELEGRAM_BOT_TOKEN is not set.")?;

    let telegram_config = TelegramConfig {
        bot_token,
        mode: TelegramMode::Polling { timeout: 30 },
    };
    let telegram_transport = TelegramTransport::new(telegram_config.clone());

    // Setup Session Router for persistence
    let session_router = std::sync::Arc::new(adapters::SessionRouter::new(
        std::path::PathBuf::from(".kcode/bridge-sessions")
    ));

    // Create channel for BridgeCore
    let (core_tx, core_rx) = std::sync::mpsc::channel::<BridgeMessage>();

    // Spawn BridgeCore in a dedicated thread to handle !Send LiveCli
    std::thread::spawn(move || {
        let core = BridgeCore::new(
            std::path::PathBuf::from(".kcode/bridge-sessions"),
            telegram_transport,
        );
        let config = SessionConfig {
            model: model, // Moved from main thread
            model_explicit: model_explicit,
            profile: profile, // Moved from main thread
            permission_mode: permission_mode, // Moved from main thread
        };
        core.run(core_rx, config);
    });

    // Webhook handler that forwards events to BridgeCore
    let handler = Box::new(move |event: adapters::BridgeInboundEvent| -> adapters::BridgeOutboundEvent {
        let (reply_tx, rx) = std::sync::mpsc::channel();
        if let Err(e) = core_tx.send(BridgeMessage { event, reply_tx }) {
            eprintln!("Failed to send event to BridgeCore: {}", e);
            return adapters::BridgeOutboundEvent {
                bridge_event_id: "error".to_string(),
                session_id: String::new(),
                channel_capability_hint: String::new(),
                reply_target: None,
                render_items: vec![("text".to_string(), "Error: Core unavailable".to_string())],
                delivery_mode: adapters::DeliveryMode::Single,
            };
        }

        // Block until BridgeCore processes the turn (sync reply for webhook response)
        match rx.recv() {
            Ok(outbound) => outbound,
            Err(_) => adapters::BridgeOutboundEvent {
                bridge_event_id: "error".to_string(),
                session_id: String::new(),
                channel_capability_hint: String::new(),
                reply_target: None,
                render_items: vec![("text".to_string(), "Error: Timeout".to_string())],
                delivery_mode: adapters::DeliveryMode::Single,
            },
        }
    });

    println!("Kcode Bridge started (Telegram). Waiting for messages...");

    // Run the webhook server
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async {
        adapters::start_webhook_server(
            "0.0.0.0:3000".parse().unwrap(),
            session_router,
            Some(telegram_config),
            None, // whatsapp
            None, // feishu
            handler,
        ).await.map_err(|e| -> Box<dyn std::error::Error> { e })
    })?;

    Ok(())
}
fn run_repl(
    model: String,
    model_explicit: bool,
    profile: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = LiveCli::new(
        model,
        model_explicit,
        profile,
        true,
        allowed_tools,
        permission_mode,
    )?;
    let mut editor =
        input::LineEditor::new("> ", cli.repl_completion_candidates().unwrap_or_default());
    println!("{}", cli.startup_banner());

    loop {
        editor.set_completions(cli.repl_completion_candidates().unwrap_or_default());
        match editor.read_line()? {
            input::ReadOutcome::Submit(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed.as_str(), "/exit" | "/quit") {
                    cli.persist_session()?;
                    break;
                }
                match SlashCommand::parse(&trimmed) {
                    Ok(Some(command)) => {
                        if cli.handle_repl_command(command)? {
                            cli.persist_session()?;
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{error}");
                        continue;
                    }
                }
                editor.push_history(input);
                cli.run_turn(&trimmed)?;
            }
            input::ReadOutcome::Cancel => {}

            input::ReadOutcome::Exit => {
                cli.persist_session()?;
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct SessionHandle {
    id: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct ManagedSessionSummary {
    id: String,
    path: PathBuf,
    modified_epoch_millis: u128,
    message_count: usize,
    parent_session_id: Option<String>,
    branch_name: Option<String>,
}

struct LiveCli {
    model: String,
    model_explicit: bool,
    profile_override: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    system_prompt: Vec<String>,
    runtime: BuiltRuntime,
    active_profile: ResolvedProviderProfile,
    session: SessionHandle,
}

struct RuntimePluginState {
    feature_config: runtime::RuntimeFeatureConfig,
    tool_registry: GlobalToolRegistry,
    plugin_registry: PluginRegistry,
}

struct BuiltRuntime {
    runtime: Option<ConversationRuntime<ProviderRuntimeClient, CliToolExecutor>>,
    plugin_registry: PluginRegistry,
    plugins_active: bool,
    active_profile: ResolvedProviderProfile,
}

impl BuiltRuntime {
    fn new(
        runtime: ConversationRuntime<ProviderRuntimeClient, CliToolExecutor>,
        plugin_registry: PluginRegistry,
        active_profile: ResolvedProviderProfile,
    ) -> Self {
        Self {
            runtime: Some(runtime),
            plugin_registry,
            plugins_active: true,
            active_profile,
        }
    }

    fn with_hook_abort_signal(mut self, hook_abort_signal: runtime::HookAbortSignal) -> Self {
        let runtime = self
            .runtime
            .take()
            .expect("runtime should exist before installing hook abort signal");
        self.runtime = Some(runtime.with_hook_abort_signal(hook_abort_signal));
        self
    }

    fn shutdown_plugins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.plugins_active {
            self.plugin_registry.shutdown()?;
            self.plugins_active = false;
        }
        Ok(())
    }
}

impl Deref for BuiltRuntime {
    type Target = ConversationRuntime<ProviderRuntimeClient, CliToolExecutor>;

    fn deref(&self) -> &Self::Target {
        self.runtime
            .as_ref()
            .expect("runtime should exist while built runtime is alive")
    }
}

impl DerefMut for BuiltRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.runtime
            .as_mut()
            .expect("runtime should exist while built runtime is alive")
    }
}

impl Drop for BuiltRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown_plugins();
    }
}

struct HookAbortMonitor {
    stop_tx: Option<Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl HookAbortMonitor {
    fn spawn(abort_signal: runtime::HookAbortSignal) -> Self {
        Self::spawn_with_waiter(abort_signal, move |stop_rx, abort_signal| {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };

            runtime.block_on(async move {
                let wait_for_stop = tokio::task::spawn_blocking(move || {
                    let _ = stop_rx.recv();
                });

                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if result.is_ok() {
                            abort_signal.abort();
                        }
                    }
                    _ = wait_for_stop => {}
                }
            });
        })
    }

    fn spawn_with_waiter<F>(abort_signal: runtime::HookAbortSignal, wait_for_interrupt: F) -> Self
    where
        F: FnOnce(Receiver<()>, runtime::HookAbortSignal) + Send + 'static,
    {
        let (stop_tx, stop_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || wait_for_interrupt(stop_rx, abort_signal));

        Self {
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }
    }

    fn stop(mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl LiveCli {
    fn new(
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        enable_tools: bool,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let system_prompt = build_system_prompt()?;
        let session_state = Session::new();
        let session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime(
            session_state.with_persistence_path(session.path.clone()),
            &session.id,
            model.clone(),
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
            system_prompt.clone(),
            enable_tools,
            true,
            allowed_tools.clone(),
            permission_mode,
            None,
        )?;
        let active_profile = runtime.active_profile.clone();
        let cli = Self {
            model,
            model_explicit,
            profile_override: profile,
            allowed_tools,
            permission_mode,
            system_prompt,
            runtime,
            active_profile,
            session,
        };
        cli.persist_session()?;
        Ok(cli)
    }

    fn startup_banner(&self) -> String {
        let cwd = env::current_dir().map_or_else(
            |_| "<unknown>".to_string(),
            |path| path.display().to_string(),
        );
        let status = status_context(None).ok();
        let git_branch = status
            .as_ref()
            .and_then(|context| context.git_branch.as_deref())
            .unwrap_or("unknown");
        let workspace = status.as_ref().map_or_else(
            || "unknown".to_string(),
            |context| context.git_summary.headline(),
        );
        let session_path = self.session.path.strip_prefix(Path::new(&cwd)).map_or_else(
            |_| self.session.path.display().to_string(),
            |path| path.display().to_string(),
        );
        format!(
            "\x1b[38;5;208m\
██╗  ██╗ ██████╗ ██████╗ ██████╗ ███████╗\n\
██║ ██╔╝██╔════╝██╔═══██╗██╔══██╗██╔════╝\n\
█████╔╝ ██║     ██║   ██║██║  ██║█████╗\n\
██╔═██╗ ██║     ██║   ██║██║  ██║██╔══╝\n\
██║  ██╗╚██████╗╚██████╔╝██████╔╝███████╗\n\
╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝\x1b[0m\n\n\
  \x1b[2mModel\x1b[0m            {}\n\
  \x1b[2mProfile\x1b[0m          {}\n\
  \x1b[2mPermissions\x1b[0m      {}\n\
  \x1b[2mBranch\x1b[0m           {}\n\
  \x1b[2mWorkspace\x1b[0m        {}\n\
  \x1b[2mDirectory\x1b[0m        {}\n\
  \x1b[2mSession\x1b[0m          {}\n\
  \x1b[2mAuto-save\x1b[0m        {}\n\n\
  Type \x1b[1m/help\x1b[0m for commands · \x1b[1m/status\x1b[0m for live context · \x1b[2m/resume latest\x1b[0m jumps back to the newest session · \x1b[1m/diff\x1b[0m then \x1b[1m/commit\x1b[0m to ship · \x1b[2mTab\x1b[0m for workflow completions · \x1b[2mShift+Enter\x1b[0m for newline",
            self.model,
            self.active_profile.profile_name,
            self.permission_mode.as_str(),
            git_branch,
            workspace,
            cwd,
            self.session.id,
            session_path,
        )
    }

    fn repl_completion_candidates(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(slash_command_completion_candidates_with_sessions(
            &self.model,
            self.active_profile.profile.supports_tools,
            Some(&self.session.id),
            list_managed_sessions()?
                .into_iter()
                .map(|session| session.id)
                .collect(),
        ))
    }

    fn prepare_turn_runtime(
        &self,
        emit_output: bool,
    ) -> Result<(BuiltRuntime, HookAbortMonitor), Box<dyn std::error::Error>> {
        let hook_abort_signal = runtime::HookAbortSignal::new();
        let runtime = build_runtime(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            emit_output,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?
        .with_hook_abort_signal(hook_abort_signal.clone());
        let hook_abort_monitor = HookAbortMonitor::spawn(hook_abort_signal);

        Ok((runtime, hook_abort_monitor))
    }

    fn replace_runtime(&mut self, runtime: BuiltRuntime) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.shutdown_plugins()?;
        self.active_profile = runtime.active_profile.clone();
        self.runtime = runtime;
        Ok(())
    }

    fn run_turn_capture(&mut self, input: &str) -> Result<String, Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(true)?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        match result {
            Ok(_) => {
                self.replace_runtime(runtime)?;
                
                // Extract the last assistant text from the current session
                let session = self.runtime.runtime.as_ref().unwrap().session();
                let mut response_text = String::new();
                for msg in session.messages.iter().rev() {
                    if msg.role == MessageRole::Assistant {
                        for block in &msg.blocks {
                            if let ContentBlock::Text { text } = block {
                                response_text = text.clone();
                                break;
                            }
                        }
                        if !response_text.is_empty() { break; }
                    }
                }
                Ok(response_text)
            }
            Err(error) => {
                Err(Box::new(error))
            }
        }
    }

    fn run_turn(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(true)?;
        let mut spinner = Spinner::new();
        let mut stdout = io::stdout();
        spinner.tick(
            "🦀 Thinking...",
            TerminalRenderer::new().color_theme(),
            &mut stdout,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        match result {
            Ok(summary) => {
                self.replace_runtime(runtime)?;
                spinner.finish(
                    "✨ Done",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;
                println!();
                if let Some(event) = summary.auto_compaction {
                    println!(
                        "{}",
                        format_auto_compaction_notice(event.removed_message_count)
                    );
                }
                if summary.compaction_circuit_tripped {
                    eprintln!(
                        "⚠ Auto-compaction circuit tripped: compact failed {} times. Use /compact to diagnose.",
                        MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES
                    );
                }
                self.persist_session()?;
                Ok(())
            }
            Err(error) => {
                runtime.shutdown_plugins()?;
                spinner.fail(
                    "❌ Request failed",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;
                Err(Box::new(error))
            }
        }
    }

    fn run_turn_with_output(
        &mut self,
        input: &str,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match output_format {
            CliOutputFormat::Text => self.run_turn(input),
            CliOutputFormat::Json => self.run_prompt_json(input),
        }
    }

    fn run_prompt_json(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(false)?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        let summary = result?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        println!(
            "{}",
            json!({
                "message": final_assistant_text(&summary),
                "model": self.model,
                "iterations": summary.iterations,
                "auto_compaction": summary.auto_compaction.map(|event| json!({
                    "removed_messages": event.removed_message_count,
                    "notice": format_auto_compaction_notice(event.removed_message_count),
                })),
                "tool_uses": collect_tool_uses(&summary),
                "tool_results": collect_tool_results(&summary),
                "prompt_cache_events": collect_prompt_cache_events(&summary),
                "usage": {
                    "input_tokens": summary.usage.input_tokens,
                    "output_tokens": summary.usage.output_tokens,
                    "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                    "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                }
            })
        );
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn handle_repl_command(
        &mut self,
        command: SlashCommand,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(match command {
            SlashCommand::Help => {
                println!(
                    "{}",
                    render_repl_help_for_profile(self.active_profile.profile.supports_tools)
                );
                false
            }
            SlashCommand::Status => {
                self.print_status();
                false
            }
            SlashCommand::Bughunter { scope } => {
                self.run_bughunter(scope.as_deref())?;
                false
            }
            SlashCommand::Commit => {
                self.run_commit(None)?;
                false
            }
            SlashCommand::Pr { context } => {
                self.run_pr(context.as_deref())?;
                false
            }
            SlashCommand::Issue { context } => {
                self.run_issue(context.as_deref())?;
                false
            }
            SlashCommand::DebugToolCall => {
                self.run_debug_tool_call(None)?;
                false
            }
            SlashCommand::Sandbox => {
                Self::print_sandbox_status();
                false
            }
            SlashCommand::Compact => {
                self.compact()?;
                false
            }
            SlashCommand::Tasks { args } => {
                self.print_tasks(args.as_deref())?;
                false
            }
            SlashCommand::Model { model } => self.set_model(model)?,
            SlashCommand::Permissions { mode } => self.set_permissions(mode)?,
            SlashCommand::Clear { confirm } => self.clear_session(confirm)?,
            SlashCommand::Cost => {
                self.print_cost();
                false
            }
            SlashCommand::Resume { session_path } => self.resume_session(session_path)?,
            SlashCommand::Config { section } => {
                self.print_config(section.as_deref())?;
                false
            }
            SlashCommand::Mcp { action, target } => {
                if let Err(message) =
                    ensure_session_command_available_for_profile("mcp", &self.active_profile)
                {
                    eprintln!("{message}");
                    return Ok(false);
                }
                let args = match (action.as_deref(), target.as_deref()) {
                    (None, None) => None,
                    (Some(action), None) => Some(action.to_string()),
                    (Some(action), Some(target)) => Some(format!("{action} {target}")),
                    (None, Some(target)) => Some(target.to_string()),
                };
                Self::print_mcp(args.as_deref())?;
                false
            }
            SlashCommand::Memory => {
                Self::print_memory()?;
                false
            }
            SlashCommand::Doctor => {
                self.print_doctor()?;
                false
            }
            SlashCommand::Init => {
                println!("{}", init_repo_kcode_md()?);
                false
            }
            SlashCommand::Diff => {
                Self::print_diff()?;
                false
            }
            SlashCommand::Version => {
                Self::print_version();
                false
            }
            SlashCommand::Export { path } => {
                self.export_session(path.as_deref())?;
                false
            }
            SlashCommand::Session { action, target } => {
                self.handle_session_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Plugins { action, target } => {
                if let Err(message) =
                    ensure_session_command_available_for_profile("plugin", &self.active_profile)
                {
                    eprintln!("{message}");
                    return Ok(false);
                }
                self.handle_plugins_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Agents { args } => {
                Self::print_agents(args.as_deref())?;
                false
            }
            SlashCommand::Skills { args } => {
                Self::print_skills(args.as_deref())?;
                false
            }
            SlashCommand::Login
            | SlashCommand::Logout
            | SlashCommand::Vim
            | SlashCommand::Upgrade
            | SlashCommand::Stats
            | SlashCommand::Share
            | SlashCommand::Feedback
            | SlashCommand::Files
            | SlashCommand::Fast
            | SlashCommand::Exit
            | SlashCommand::Summary
            | SlashCommand::Desktop
            | SlashCommand::Brief
            | SlashCommand::Advisor
            | SlashCommand::Stickers
            | SlashCommand::Insights
            | SlashCommand::Thinkback
            | SlashCommand::ReleaseNotes
            | SlashCommand::SecurityReview
            | SlashCommand::Keybindings
            | SlashCommand::PrivacySettings
            | SlashCommand::Plan { .. }
            | SlashCommand::Review { .. }
            | SlashCommand::Theme { .. }
            | SlashCommand::Voice { .. }
            | SlashCommand::Usage { .. }
            | SlashCommand::Rename { .. }
            | SlashCommand::Copy { .. }
            | SlashCommand::Hooks { .. }
            | SlashCommand::Context { .. }
            | SlashCommand::Color { .. }
            | SlashCommand::Effort { .. }
            | SlashCommand::Branch { .. }
            | SlashCommand::Rewind { .. }
            | SlashCommand::Ide { .. }
            | SlashCommand::Tag { .. }
            | SlashCommand::OutputStyle { .. }
            | SlashCommand::AddDir { .. } => {
                eprintln!("Command registered but not yet implemented.");
                false
            }
            SlashCommand::Unknown(name) => {
                eprintln!("{}", format_unknown_slash_command(&name));
                false
            }
        })
    }

    fn persist_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.session().save_to_path(&self.session.path)?;
        Ok(())
    }

    fn print_status(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        let latest = self.runtime.usage().current_turn_usage();
        println!(
            "{}",
            format_status_report(
                &self.model,
                Some(&self.active_profile),
                StatusUsage {
                    message_count: self.runtime.session().messages.len(),
                    turns: self.runtime.usage().turns(),
                    latest,
                    cumulative,
                    estimated_tokens: self.runtime.estimated_tokens(),
                },
                self.permission_mode.as_str(),
                &status_context(Some(&self.session.path)).expect("status context should load"),
            )
        );
    }

    fn print_sandbox_status() {
        let cwd = env::current_dir().expect("current dir");
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader
            .load()
            .unwrap_or_else(|_| runtime::RuntimeConfig::empty());
        println!(
            "{}",
            format_sandbox_report(&resolve_sandbox_status(runtime_config.sandbox(), &cwd))
        );
    }

    fn set_model(&mut self, model: Option<String>) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(model) = model else {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    &self.active_profile.profile_name,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        };

        let model = resolve_model_alias(&model).to_string();

        if model == self.model {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    &self.active_profile.profile_name,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        }

        let previous = self.model.clone();
        let session = self.runtime.session().clone();
        let message_count = session.messages.len();
        let runtime = build_runtime(
            session,
            &self.session.id,
            model.clone(),
            Some(model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.model.clone_from(&model);
        self.model_explicit = true;
        println!(
            "{}",
            format_model_switch_report(
                &previous,
                &model,
                &self.active_profile.profile_name,
                message_count,
            )
        );
        Ok(true)
    }

    fn set_permissions(
        &mut self,
        mode: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(mode) = mode else {
            println!(
                "{}",
                format_permissions_report(self.permission_mode.as_str())
            );
            return Ok(false);
        };

        let normalized = normalize_permission_mode(&mode).ok_or_else(|| {
            format!(
                "unsupported permission mode '{mode}'. Use read-only, workspace-write, or danger-full-access."
            )
        })?;

        if normalized == self.permission_mode.as_str() {
            println!("{}", format_permissions_report(normalized));
            return Ok(false);
        }

        let previous = self.permission_mode.as_str().to_string();
        let session = self.runtime.session().clone();
        self.permission_mode = permission_mode_from_label(normalized);
        let runtime = build_runtime(
            session,
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        println!(
            "{}",
            format_permissions_switch_report(&previous, normalized)
        );
        Ok(true)
    }

    fn clear_session(&mut self, confirm: bool) -> Result<bool, Box<dyn std::error::Error>> {
        if !confirm {
            println!(
                "clear: confirmation required; run /clear --confirm to start a fresh session."
            );
            return Ok(false);
        }

        let previous_session = self.session.clone();
        let session_state = Session::new();
        self.session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime(
            session_state.with_persistence_path(self.session.path.clone()),
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        println!(
            "Session cleared\n  Mode             fresh session\n  Previous session {}\n  Resume previous  /resume {}\n  Preserved model  {}\n  Permission mode  {}\n  New session      {}\n  Session file     {}",
            previous_session.id,
            previous_session.id,
            self.model,
            self.permission_mode.as_str(),
            self.session.id,
            self.session.path.display(),
        );
        Ok(true)
    }

    fn print_cost(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        println!("{}", format_cost_report(cumulative));
    }

    fn resume_session(
        &mut self,
        session_path: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(session_ref) = session_path else {
            println!("{}", render_resume_usage());
            return Ok(false);
        };

        let handle = resolve_session_reference(&session_ref)?;
        let session = Session::load_from_path(&handle.path)?;
        let message_count = session.messages.len();
        let session_id = session.session_id.clone();
        let runtime = build_runtime(
            session,
            &handle.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.session = SessionHandle {
            id: session_id,
            path: handle.path,
        };
        println!(
            "{}",
            format_resume_report(
                &self.session.path.display().to_string(),
                message_count,
                self.runtime.usage().turns(),
            )
        );
        Ok(true)
    }

    fn print_config(&self, section: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{}",
            render_config_report(
                section,
                self.model_explicit.then_some(self.model.as_str()),
                self.profile_override.as_deref(),
            )?
        );
        Ok(())
    }

    fn print_memory() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_memory_report()?);
        Ok(())
    }

    fn print_tasks(&self, args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        match args {
            None | Some("list") => {
                println!(
                    "Tasks
  Background tasks are managed through the Agent tool.
  Use /help Agent to see how to create and manage tasks.

  To create a task: Use the Agent tool with action=create
  To list tasks:   Use the Agent tool with action=list
  To stop a task:  Use the Agent tool with action=stop"
                );
            }
            Some("help") => {
                println!(
                    "Tasks
  Background tasks allow running multiple agent sessions in parallel.

  Commands:
    /tasks              List active tasks
    /tasks help         Show this help

  Task management is done through the Agent tool:
    Agent(action=create, description='...')   Create a new task
    Agent(action=list)                        List all tasks
    Agent(action=stop, task_id='...')         Stop a running task
    Agent(action=output, task_id='...')       Get task output"
                );
            }
            other => {
                println!("Unknown tasks argument: {}. Use /tasks help for usage.", other.unwrap_or(""));
            }
        }
        Ok(())
    }

    fn print_doctor(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{}",
            render_doctor_report(
                self.model_explicit.then_some(self.model.as_str()),
                self.profile_override.as_deref(),
            )?
        );
        Ok(())
    }

    fn print_agents(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        println!("{}", handle_agents_slash_command(args, &cwd)?);
        Ok(())
    }

    fn print_mcp(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        println!("{}", handle_mcp_slash_command(args, &cwd)?);
        Ok(())
    }

    fn print_skills(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        println!("{}", handle_skills_slash_command(args, &cwd)?);
        Ok(())
    }

    fn print_diff() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_diff_report()?);
        Ok(())
    }

    fn print_version() {
        println!("{}", render_version_report());
    }

    fn export_session(
        &self,
        requested_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let export_path = resolve_export_path(requested_path, self.runtime.session())?;
        fs::write(&export_path, render_export_text(self.runtime.session()))?;
        println!(
            "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
            export_path.display(),
            self.runtime.session().messages.len(),
        );
        Ok(())
    }

    fn handle_session_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => {
                println!("{}", render_session_list(&self.session.id)?);
                Ok(false)
            }
            Some("switch") => {
                let Some(target) = target else {
                    println!("Usage: /session switch <session-id>");
                    return Ok(false);
                };
                let handle = resolve_session_reference(target)?;
                let session = Session::load_from_path(&handle.path)?;
                let message_count = session.messages.len();
                let session_id = session.session_id.clone();
                let runtime = build_runtime(
                    session,
                    &handle.id,
                    self.model.clone(),
                    self.model_explicit.then_some(self.model.as_str()),
                    self.profile_override.as_deref(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    None,
                )?;
                self.replace_runtime(runtime)?;
                self.session = SessionHandle {
                    id: session_id,
                    path: handle.path,
                };
                println!(
                    "Session switched\n  Active session   {}\n  File             {}\n  Messages         {}",
                    self.session.id,
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some("fork") => {
                let forked = self.runtime.fork_session(target.map(ToOwned::to_owned));
                let parent_session_id = self.session.id.clone();
                let handle = create_managed_session_handle(&forked.session_id)?;
                let branch_name = forked
                    .fork
                    .as_ref()
                    .and_then(|fork| fork.branch_name.clone());
                let forked = forked.with_persistence_path(handle.path.clone());
                let message_count = forked.messages.len();
                forked.save_to_path(&handle.path)?;
                let runtime = build_runtime(
                    forked,
                    &handle.id,
                    self.model.clone(),
                    self.model_explicit.then_some(self.model.as_str()),
                    self.profile_override.as_deref(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    None,
                )?;
                self.replace_runtime(runtime)?;
                self.session = handle;
                println!(
                    "Session forked\n  Parent session   {}\n  Active session   {}\n  Branch           {}\n  File             {}\n  Messages         {}",
                    parent_session_id,
                    self.session.id,
                    branch_name.as_deref().unwrap_or("(unnamed)"),
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some(other) => {
                println!(
                    "Unknown /session action '{other}'. Use /session list, /session switch <session-id>, or /session fork [branch-name]."
                );
                Ok(false)
            }
        }
    }

    fn handle_plugins_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader.load()?;
        let mut manager = build_plugin_manager(&cwd, &loader, &runtime_config);
        let result = handle_plugins_slash_command(action, target, &mut manager)?;
        println!("{}", result.message);
        if result.reload_runtime {
            self.reload_runtime_features()?;
        }
        Ok(false)
    }

    fn reload_runtime_features(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = build_runtime(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.persist_session()
    }

    fn compact(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.runtime.compact(CompactionConfig::default());
        let removed = result.removed_message_count;
        let kept = result.compacted_session.messages.len();
        let skipped = removed == 0;
        let runtime = build_runtime(
            result.compacted_session,
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        println!("{}", format_compact_report(removed, kept, skipped));
        Ok(())
    }

    fn run_internal_prompt_text_with_progress(
        &self,
        prompt: &str,
        enable_tools: bool,
        progress: Option<InternalPromptProgressReporter>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session = self.runtime.session().clone();
        let mut runtime = build_runtime(
            session,
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            enable_tools,
            false,
            self.allowed_tools.clone(),
            self.permission_mode,
            progress,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let summary = runtime.run_turn(prompt, Some(&mut permission_prompter))?;
        let text = final_assistant_text(&summary).trim().to_string();
        runtime.shutdown_plugins()?;
        Ok(text)
    }

    fn run_internal_prompt_text(
        &self,
        prompt: &str,
        enable_tools: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.run_internal_prompt_text_with_progress(prompt, enable_tools, None)
    }

    fn run_bughunter(&self, scope: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_bughunter_report(scope));
        Ok(())
    }

    fn run_ultraplan(&self, task: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_ultraplan_report(task));
        Ok(())
    }

    #[allow(clippy::unused_self)]
    fn run_teleport(&self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(target) = target.map(str::trim).filter(|value| !value.is_empty()) else {
            println!("Usage: /teleport <symbol-or-path>");
            return Ok(());
        };

        println!("{}", render_teleport_report(target)?);
        Ok(())
    }

    fn run_debug_tool_call(&self, args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        validate_no_args("/debug-tool-call", args)?;
        println!("{}", render_last_tool_debug_report(self.runtime.session())?);
        Ok(())
    }

    fn run_commit(&mut self, args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        validate_no_args("/commit", args)?;
        let status = git_output(&["status", "--short", "--branch"])?;
        let summary = parse_git_workspace_summary(Some(&status));
        let branch = parse_git_status_branch(Some(&status));
        if summary.is_clean() {
            println!("{}", format_commit_skipped_report());
            return Ok(());
        }

        println!(
            "{}",
            format_commit_preflight_report(branch.as_deref(), summary)
        );
        Ok(())
    }

    fn run_pr(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let branch =
            resolve_git_branch_for(&env::current_dir()?).unwrap_or_else(|| "unknown".to_string());
        println!("{}", format_pr_report(&branch, context));
        Ok(())
    }

    fn run_issue(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_issue_report(context));
        Ok(())
    }
}

fn sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = primary_sessions_dir()?;
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn primary_sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(env::var_os(PRIMARY_SESSION_DIR_ENV)
        .map(PathBuf::from)
        .or_else(|| env::var_os(LEGACY_SESSION_DIR_ENV).map(PathBuf::from))
        .unwrap_or_else(|| cwd.join(PRIMARY_CONFIG_DIR_NAME).join("sessions")))
}

fn legacy_sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(cwd.join(LEGACY_CONFIG_DIR_NAME).join("sessions"))
}

fn session_search_dirs() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut dirs = Vec::new();
    for path in [primary_sessions_dir()?, legacy_sessions_dir()?] {
        if !dirs.iter().any(|candidate| candidate == &path) {
            dirs.push(path);
        }
    }
    Ok(dirs)
}

fn create_managed_session_handle(
    session_id: &str,
) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    let id = session_id.to_string();
    let path = sessions_dir()?.join(format!("{id}.{PRIMARY_SESSION_EXTENSION}"));
    Ok(SessionHandle { id, path })
}

fn resolve_session_reference(reference: &str) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    if SESSION_REFERENCE_ALIASES
        .iter()
        .any(|alias| reference.eq_ignore_ascii_case(alias))
    {
        let latest = latest_managed_session()?;
        return Ok(SessionHandle {
            id: latest.id,
            path: latest.path,
        });
    }

    let direct = PathBuf::from(reference);
    let looks_like_path = direct.extension().is_some() || direct.components().count() > 1;
    let path = if direct.exists() {
        direct
    } else if looks_like_path {
        return Err(format_missing_session_reference(reference).into());
    } else {
        resolve_managed_session_path(reference)?
    };
    let id = path
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|name| {
            name.strip_suffix(&format!(".{PRIMARY_SESSION_EXTENSION}"))
                .or_else(|| name.strip_suffix(&format!(".{LEGACY_SESSION_EXTENSION}")))
        })
        .unwrap_or(reference)
        .to_string();
    Ok(SessionHandle { id, path })
}

fn resolve_managed_session_path(session_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    for directory in session_search_dirs()? {
        for extension in [PRIMARY_SESSION_EXTENSION, LEGACY_SESSION_EXTENSION] {
            let path = directory.join(format!("{session_id}.{extension}"));
            if path.exists() {
                return Ok(path);
            }
        }
    }
    Err(format_missing_session_reference(session_id).into())
}

fn is_managed_session_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|extension| {
            extension == PRIMARY_SESSION_EXTENSION || extension == LEGACY_SESSION_EXTENSION
        })
}

fn list_managed_sessions() -> Result<Vec<ManagedSessionSummary>, Box<dyn std::error::Error>> {
    let mut sessions = Vec::new();
    for directory in session_search_dirs()? {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !is_managed_session_file(&path) {
                continue;
            }
            let metadata = entry.metadata()?;
            let modified_epoch_millis = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis())
                .unwrap_or_default();
            let (id, message_count, parent_session_id, branch_name) =
                match Session::load_from_path(&path) {
                    Ok(session) => {
                        let parent_session_id = session
                            .fork
                            .as_ref()
                            .map(|fork| fork.parent_session_id.clone());
                        let branch_name = session
                            .fork
                            .as_ref()
                            .and_then(|fork| fork.branch_name.clone());
                        (
                            session.session_id,
                            session.messages.len(),
                            parent_session_id,
                            branch_name,
                        )
                    }
                    Err(_) => (
                        path.file_stem()
                            .and_then(|value| value.to_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        0,
                        None,
                        None,
                    ),
                };
            sessions.push(ManagedSessionSummary {
                id,
                path,
                modified_epoch_millis,
                message_count,
                parent_session_id,
                branch_name,
            });
        }
    }
    sessions.sort_by(|left, right| {
        right
            .modified_epoch_millis
            .cmp(&left.modified_epoch_millis)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(sessions)
}

fn latest_managed_session() -> Result<ManagedSessionSummary, Box<dyn std::error::Error>> {
    list_managed_sessions()?
        .into_iter()
        .next()
        .ok_or_else(|| format_no_managed_sessions().into())
}

fn format_missing_session_reference(reference: &str) -> String {
    format!(
        "session not found: {reference}\nHint: managed sessions live in .kcode/sessions/. Try `{LATEST_SESSION_REFERENCE}` for the most recent session or `/session list` in the REPL."
    )
}

fn format_no_managed_sessions() -> String {
    format!(
        "no managed sessions found in .kcode/sessions/\nStart `{CLI_NAME}` to create a session, then rerun with `--resume {LATEST_SESSION_REFERENCE}`."
    )
}

fn render_session_list(active_session_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let sessions = list_managed_sessions()?;
    let mut lines = vec![
        "Sessions".to_string(),
        format!("  Directory         {}", sessions_dir()?.display()),
    ];
    if sessions.is_empty() {
        lines.push("  No managed sessions saved yet.".to_string());
        return Ok(lines.join("\n"));
    }
    for session in sessions {
        let marker = if session.id == active_session_id {
            "● current"
        } else {
            "○ saved"
        };
        let lineage = match (
            session.branch_name.as_deref(),
            session.parent_session_id.as_deref(),
        ) {
            (Some(branch_name), Some(parent_session_id)) => {
                format!(" branch={branch_name} from={parent_session_id}")
            }
            (None, Some(parent_session_id)) => format!(" from={parent_session_id}"),
            (Some(branch_name), None) => format!(" branch={branch_name}"),
            (None, None) => String::new(),
        };
        lines.push(format!(
            "  {id:<20} {marker:<10} msgs={msgs:<4} modified={modified}{lineage} path={path}",
            id = session.id,
            msgs = session.message_count,
            modified = format_session_modified_age(session.modified_epoch_millis),
            lineage = lineage,
            path = session.path.display(),
        ));
    }
    Ok(lines.join("\n"))
}

fn format_session_modified_age(modified_epoch_millis: u128) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(modified_epoch_millis, |duration| duration.as_millis());
    let delta_seconds = now
        .saturating_sub(modified_epoch_millis)
        .checked_div(1_000)
        .unwrap_or_default();
    match delta_seconds {
        0..=4 => "just-now".to_string(),
        5..=59 => format!("{delta_seconds}s-ago"),
        60..=3_599 => format!("{}m-ago", delta_seconds / 60),
        3_600..=86_399 => format!("{}h-ago", delta_seconds / 3_600),
        _ => format!("{}d-ago", delta_seconds / 86_400),
    }
}

fn write_session_clear_backup(
    session: &Session,
    session_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let backup_path = session_clear_backup_path(session_path);
    session.save_to_path(&backup_path)?;
    Ok(backup_path)
}

fn session_clear_backup_path(session_path: &Path) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(0, |duration| duration.as_millis());
    let file_name = session_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("session.jsonl");
    session_path.with_file_name(format!("{file_name}.before-clear-{timestamp}.bak"))
}

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
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_doctor_report(model_override, profile_override)?
    );
    Ok(())
}

fn print_config_show(
    section: Option<&str>,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_config_report(section, model_override, profile_override)?
    );
    Ok(())
}

fn print_commands_report(
    surface: CommandReportSurfaceSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_commands_report(surface, model_override, profile_override)?
    );
    Ok(())
}

fn print_profile_report(
    selection: &ProfileCommandSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_profile_report(selection, model_override, profile_override)?
    );
    Ok(())
}

fn command_registry_context(
    setup: &SetupContext,
    surface: CommandReportSurfaceSelection,
) -> CommandRegistryContext {
    CommandRegistryContext::for_surface(
        surface.command_surface(),
        setup.active_profile.profile.supports_tools,
    )
}

fn command_descriptor_usage(descriptor: &CommandDescriptor) -> String {
    match (&descriptor.scope, &descriptor.argument_hint) {
        (CommandScope::Session, Some(argument_hint)) => {
            format!("/{} {}", descriptor.name, argument_hint)
        }
        (CommandScope::Session, None) => format!("/{}", descriptor.name),
        (CommandScope::Process, Some(argument_hint)) => {
            format!("{} {}", descriptor.name, argument_hint)
        }
        (CommandScope::Process, None) => descriptor.name.clone(),
    }
}

fn filtered_command_usage(filtered: &FilteredCommand) -> String {
    let name = filtered
        .id
        .rsplit('.')
        .next()
        .unwrap_or(filtered.id.as_str());
    match filtered.scope {
        CommandScope::Process => name.to_string(),
        CommandScope::Session => format!("/{name}"),
    }
}

fn filtered_command_name(filtered: &FilteredCommand) -> &str {
    filtered
        .id
        .rsplit('.')
        .next()
        .unwrap_or(filtered.id.as_str())
}

fn local_command_block_reason(
    profile: &ResolvedProviderProfile,
    scope: CommandScope,
    name: &str,
) -> Option<String> {
    let snapshot = build_command_registry_snapshot(
        &CommandRegistryContext::for_surface(
            CommandSurface::CliLocal,
            profile.profile.supports_tools,
        ),
        &[],
    );
    snapshot
        .filtered_out_commands
        .iter()
        .find(|command| {
            command.scope == scope
                && filtered_command_name(command) == name
                && command.reason == "active profile does not expose tool-capable commands"
        })
        .map(|command| command.reason.clone())
}

fn command_blocked_message(
    scope: CommandScope,
    name: &str,
    profile_name: &str,
    reason: &str,
) -> String {
    let rendered_name = match scope {
        CommandScope::Process => name.to_string(),
        CommandScope::Session => format!("/{name}"),
    };
    format!(
        "command `{rendered_name}` is unavailable for active profile `{profile_name}`: {reason}\nRun `{CLI_NAME} commands show local` to inspect the current command surface."
    )
}

fn ensure_process_command_available(
    name: &str,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(setup) = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    ) else {
        return Ok(());
    };
    if let Some(reason) =
        local_command_block_reason(&setup.active_profile, CommandScope::Process, name)
    {
        return Err(command_blocked_message(
            CommandScope::Process,
            name,
            &setup.active_profile.profile_name,
            &reason,
        )
        .into());
    }
    Ok(())
}

fn ensure_session_command_available_for_profile(
    command_name: &str,
    profile: &ResolvedProviderProfile,
) -> Result<(), String> {
    if let Some(reason) = local_command_block_reason(profile, CommandScope::Session, command_name) {
        return Err(command_blocked_message(
            CommandScope::Session,
            command_name,
            &profile.profile_name,
            &reason,
        ));
    }
    Ok(())
}

fn render_commands_report(
    surface: CommandReportSurfaceSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let setup = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;
    let snapshot = build_command_registry_snapshot(&command_registry_context(&setup, surface), &[]);
    let actionable_filtered = snapshot
        .filtered_out_commands
        .iter()
        .filter(|command| command.reason != "disabled")
        .collect::<Vec<_>>();

    let mut lines = vec![
        "Commands".to_string(),
        format!("  Active profile    {}", setup.active_profile.profile_name),
        format!(
            "  Selected via      {}",
            setup.active_profile.profile_source.label()
        ),
        format!("  Surface           {}", surface.label()),
        format!("  Safety profile    {}", snapshot.safety_profile),
        format!(
            "  Supports tools    {}",
            setup.active_profile.profile.supports_tools
        ),
        format!(
            "  Supports stream   {}",
            setup.active_profile.profile.supports_streaming
        ),
        format!("  Process commands  {}", snapshot.process_commands.len()),
        format!("  Session commands  {}", snapshot.session_commands.len()),
        format!("  Filtered commands {}", actionable_filtered.len()),
    ];

    lines.push("Process commands".to_string());
    for descriptor in &snapshot.process_commands {
        lines.push(format!(
            "  {:<34} {}",
            command_descriptor_usage(descriptor),
            descriptor.description
        ));
    }

    lines.push(String::new());
    lines.push("Session commands".to_string());
    for descriptor in &snapshot.session_commands {
        lines.push(format!(
            "  {:<34} {}",
            command_descriptor_usage(descriptor),
            descriptor.description
        ));
    }

    if !actionable_filtered.is_empty() {
        lines.push(String::new());
        lines.push("Filtered".to_string());
        for filtered in actionable_filtered {
            lines.push(format!(
                "  {:<34} {}",
                filtered_command_usage(filtered),
                filtered.reason
            ));
        }
    }

    Ok(lines.join("\n"))
}

fn render_doctor_report(
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let setup = load_setup_context(
        SetupMode::Doctor,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;
    Ok(render_doctor_report_from_setup(&setup))
}

fn render_doctor_report_from_setup(setup: &SetupContext) -> String {
    let checks = doctor_checks(setup);
    let runtime_ready = !checks
        .iter()
        .any(|check| check.status == DiagnosticStatus::Fail);
    let mut lines = vec![format!(
        "Doctor
  Working directory {}
  Config home      {}
  Session dir      {}
  Active profile   {}
  Runtime ready    {}",
        setup.cwd.display(),
        setup.resolved_config.config_home.display(),
        setup.resolved_config.session_dir.display(),
        setup.active_profile.profile_name,
        if runtime_ready { "yes" } else { "no" }
    )];

    lines.push("Checks".to_string());
    for check in checks {
        lines.push(format!(
            "  [{:<4}] {:<16} {}",
            check.status.label(),
            check.name,
            check.detail
        ));
    }

    lines.push(format!(
        "Next step        {}",
        doctor_next_step(setup, runtime_ready)
    ));
    lines.join("\n")
}

fn doctor_checks(setup: &SetupContext) -> Vec<DiagnosticCheck> {
    let config_file_path = setup.resolved_config.config_home.join("config.toml");
    let loaded_config_path = setup
        .resolved_config
        .loaded_entries
        .iter()
        .find(|entry| {
            entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "config.toml")
        })
        .map(|entry| entry.path.clone())
        .unwrap_or(config_file_path);

    let credentials_path = runtime::credentials_path().ok();
    let credential_detail = if setup.resolved_config.api_key_present {
        DiagnosticCheck {
            name: "api credentials".to_string(),
            status: DiagnosticStatus::Ok,
            detail: format!(
                "env `{}` is available ({})",
                setup.active_profile.credential.env_name,
                setup.active_profile.credential.source.label()
            ),
        }
    } else if setup.resolved_config.oauth_credentials_present {
        DiagnosticCheck {
            name: "api credentials".to_string(),
            status: DiagnosticStatus::Warn,
            detail: format!(
                "legacy OAuth credentials detected{}; provider profiles ignore OAuth",
                credentials_path
                    .as_ref()
                    .map(|path| format!(" at {}", path.display()))
                    .unwrap_or_default()
            ),
        }
    } else {
        DiagnosticCheck {
            name: "api credentials".to_string(),
            status: DiagnosticStatus::Fail,
            detail: format!(
                "unset; export `{}` or `{}`",
                PRIMARY_API_KEY_ENV, setup.active_profile.credential.env_name
            ),
        }
    };

    let legacy_detail = if setup.resolved_config.legacy_paths.is_empty() {
        "none detected".to_string()
    } else {
        setup
            .resolved_config
            .legacy_paths
            .iter()
            .take(3)
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    vec![
        DiagnosticCheck {
            name: "config file".to_string(),
            status: if setup.resolved_config.config_file_present {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Fail
            },
            detail: if setup.resolved_config.config_file_present {
                format!("loaded {}", loaded_config_path.display())
            } else {
                format!(
                    "missing {}; run `{CLI_NAME} init` first",
                    loaded_config_path.display()
                )
            },
        },
        DiagnosticCheck {
            name: "profile".to_string(),
            status: DiagnosticStatus::Ok,
            detail: format!(
                "{} ({})",
                setup.active_profile.profile_name,
                setup.active_profile.profile_source.label()
            ),
        },
        DiagnosticCheck {
            name: "model".to_string(),
            status: DiagnosticStatus::Ok,
            detail: format!(
                "{} ({})",
                setup.resolved_config.model,
                setup.active_profile.model_source.label()
            ),
        },
        DiagnosticCheck {
            name: "tool capability".to_string(),
            status: DiagnosticStatus::Ok,
            detail: if setup.active_profile.profile.supports_tools {
                "enabled by active profile".to_string()
            } else {
                format!(
                    "disabled by active profile `{}`; tool-capable commands stay hidden",
                    setup.active_profile.profile_name
                )
            },
        },
        DiagnosticCheck {
            name: "base url".to_string(),
            status: if setup
                .resolved_config
                .base_url
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Fail
            },
            detail: setup
                .resolved_config
                .base_url
                .clone()
                .filter(|value| !value.trim().is_empty())
                .map(|value| format!("{value} ({})", setup.active_profile.base_url_source.label()))
                .unwrap_or_else(|| {
                    format!(
                        "unset; set `{PRIMARY_BASE_URL_ENV}` or `base_url` in `~/.kcode/config.toml`"
                    )
                }),
        },
        credential_detail,
        DiagnosticCheck {
            name: "session dir".to_string(),
            status: if path_or_parent_writeable(&setup.resolved_config.session_dir) {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Fail
            },
            detail: if path_or_parent_writeable(&setup.resolved_config.session_dir) {
                format!("writeable {}", setup.resolved_config.session_dir.display())
            } else {
                format!(
                    "not writeable {}; adjust `session_dir` or `{PRIMARY_SESSION_DIR_ENV}`",
                    setup.resolved_config.session_dir.display()
                )
            },
        },
        DiagnosticCheck {
            name: "permission mode".to_string(),
            status: DiagnosticStatus::Ok,
            detail: setup.trust_policy.permission_mode.clone(),
        },
        DiagnosticCheck {
            name: "legacy residue".to_string(),
            status: if setup.resolved_config.legacy_paths.is_empty() {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Warn
            },
            detail: legacy_detail,
        },
    ]
}

fn doctor_next_step(setup: &SetupContext, runtime_ready: bool) -> String {
    if !setup.resolved_config.config_file_present {
        return format!(
            "run `{CLI_NAME} init`, fill `config.toml`, then rerun `{CLI_NAME} doctor`"
        );
    }
    if setup
        .resolved_config
        .base_url
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return format!(
            "set `{PRIMARY_BASE_URL_ENV}` or `base_url` in `~/.kcode/config.toml`, then rerun `{CLI_NAME} doctor`"
        );
    }
    if !setup.resolved_config.api_key_present {
        return format!(
            "export `{PRIMARY_API_KEY_ENV}` or the env named by `api_key_env`, then rerun `{CLI_NAME} doctor`"
        );
    }
    if !path_or_parent_writeable(&setup.resolved_config.session_dir) {
        return format!(
            "fix `session_dir` or `{PRIMARY_SESSION_DIR_ENV}` so sessions can be written"
        );
    }
    if runtime_ready {
        return format!("start `{CLI_NAME}` or run `{CLI_NAME} -p \"hello\"`");
    }
    "review warnings above before starting interactive sessions".to_string()
}

fn render_resolved_profile_report(profile: &ResolvedProviderProfile) -> String {
    let launch = ProviderLauncher::prepare(profile);
    let credential_detail = if profile.credential.api_key.is_some() {
        format!(
            "present via {} ({})",
            profile.credential.env_name,
            profile.credential.source.label()
        )
    } else {
        format!("missing {}", profile.credential.env_name)
    };

    let mut lines = vec![
        "Profile".to_string(),
        format!("  Name              {}", profile.profile_name),
        format!("  Selected via      {}", profile.profile_source.label()),
        format!("  Model             {}", profile.model),
        format!("  Model source      {}", profile.model_source.label()),
        format!(
            "  Base URL          {}",
            profile
                .base_url
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<unset>")
        ),
        format!("  Base URL source   {}", profile.base_url_source.label()),
        format!("  Base URL env      {}", profile.profile.base_url_env),
        format!("  API key env       {}", profile.credential.env_name),
        format!("  Credential        {credential_detail}"),
        format!("  Default model     {}", profile.profile.default_model),
        format!("  Supports tools    {}", profile.profile.supports_tools),
        format!("  Supports stream   {}", profile.profile.supports_streaming),
        format!("  Timeout ms        {}", profile.profile.request_timeout_ms),
        format!("  Max retries       {}", profile.profile.max_retries),
        format!(
            "  Launch ready      {}",
            if launch.is_ok() { "yes" } else { "no" }
        ),
    ];
    if let Err(error) = launch {
        lines.push(format!("  Launch detail     {error}"));
    }
    lines.join("\n")
}

fn render_active_profile_report(setup: &SetupContext) -> String {
    render_resolved_profile_report(&setup.active_profile)
}

fn render_profile_report(
    selection: &ProfileCommandSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    let setup = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;

    match selection {
        ProfileCommandSelection::List => {
            let names = ProfileResolver::available_profile_names(&runtime_config);
            let mut lines = vec![
                "Profile".to_string(),
                format!("  Active profile    {}", setup.active_profile.profile_name),
                format!(
                    "  Selected via      {}",
                    setup.active_profile.profile_source.label()
                ),
                format!(
                    "  Launch ready      {}",
                    if ProviderLauncher::prepare(&setup.active_profile).is_ok() {
                        "yes"
                    } else {
                        "no"
                    }
                ),
                format!("  Known profiles    {}", names.len()),
                String::new(),
                "Profiles".to_string(),
            ];
            for name in names {
                match ProfileResolver::resolve_named(&runtime_config, &name, None) {
                    Ok(profile) => {
                        let marker = if profile.profile_name == setup.active_profile.profile_name {
                            "*"
                        } else {
                            " "
                        };
                        lines.push(format!(
                            "  {marker} {name:<12} key={key:<18} model={model:<24} tools={tools} stream={stream}",
                            name = profile.profile_name,
                            key = profile.credential.env_name,
                            model = profile.model,
                            tools = profile.profile.supports_tools,
                            stream = profile.profile.supports_streaming,
                        ));
                    }
                    Err(error) => lines.push(format!("    {name:<12} error={error}")),
                }
            }
            Ok(lines.join("\n"))
        }
        ProfileCommandSelection::Show { profile_name: None } => {
            Ok(render_active_profile_report(&setup))
        }
        ProfileCommandSelection::Show {
            profile_name: Some(name),
        } if name.eq_ignore_ascii_case(&setup.active_profile.profile_name) => {
            Ok(render_active_profile_report(&setup))
        }
        ProfileCommandSelection::Show {
            profile_name: Some(name),
        } => {
            let resolved = ProfileResolver::resolve_named(&runtime_config, name, model_override)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            Ok(render_resolved_profile_report(&resolved))
        }
    }
}

fn render_config_report(
    section: Option<&str>,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let setup = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;
    let loader = ConfigLoader::default_for(&setup.cwd);
    let discovered = loader.discover();
    let runtime_config = loader.load()?;

    let mut lines = vec![
        format!(
            "Config
  Working directory {}
  Config home      {}
  Session dir      {}
  Effective profile {}
  Effective model  {}
  Loaded files     {}
  Merged keys      {}",
            setup.cwd.display(),
            setup.resolved_config.config_home.display(),
            setup.resolved_config.session_dir.display(),
            setup.active_profile.profile_name,
            setup.resolved_config.model,
            runtime_config.loaded_entries().len(),
            runtime_config.merged().len()
        ),
        "Discovered files".to_string(),
    ];
    for entry in discovered {
        let source = match entry.source {
            ConfigSource::User => "user",
            ConfigSource::Project => "project",
            ConfigSource::Local => "local",
            ConfigSource::Managed => "managed",
        };
        let status = if runtime_config
            .loaded_entries()
            .iter()
            .any(|loaded_entry| loaded_entry.path == entry.path)
        {
            "loaded"
        } else {
            "missing"
        };
        lines.push(format!(
            "  {source:<7} {status:<7} {}",
            entry.path.display()
        ));
    }

    if let Some(section) = section {
        lines.push(format!("Merged section: {section}"));
        match section {
            "env" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("env")
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "hooks" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("hooks")
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "model" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("model")
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "plugins" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("plugins")
                    .or_else(|| runtime_config.get("enabledPlugins"))
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "profile" => {
                lines.extend(
                    render_active_profile_report(&setup)
                        .lines()
                        .skip(1)
                        .map(|line| format!("  {line}")),
                );
            }
            "provider" => match ProviderLauncher::prepare(&setup.active_profile) {
                Ok(launch) => {
                    lines.push(format!("  Profile          {}", launch.profile_name));
                    lines.push(format!("  Provider         {}", launch.provider_label));
                    lines.push(format!("  Base URL         {}", launch.base_url));
                    lines.push(format!("  Model            {}", launch.model));
                    lines.push(format!("  Timeout ms       {}", launch.request_timeout_ms));
                    lines.push(format!("  Max retries      {}", launch.max_retries));
                    lines.push(format!("  Supports tools   {}", launch.supports_tools));
                    lines.push(format!("  Supports stream  {}", launch.supports_streaming));
                }
                Err(error) => lines.push(format!("  Launch error     {error}")),
            },
            other => {
                lines.push(format!(
                    "  Unsupported config section '{other}'. Use env, hooks, model, plugins, profile, or provider."
                ));
                return Ok(lines.join(
                    "
",
                ));
            }
        }
        return Ok(lines.join(
            "
",
        ));
    }

    lines.push("Merged JSON".to_string());
    lines.push(format!("  {}", runtime_config.as_json().render()));
    Ok(lines.join(
        "
",
    ))
}

fn render_memory_report() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let dir = default_memory_dir();
    ensure_memory_dir(&dir)?;
    ensure_memory_index(&dir.join("MEMORY.md"))?;

    let entries = list_memories(&dir)?;
    let summary = render_memory_summary(&entries);

    let project_context = ProjectContext::discover(&cwd, DEFAULT_DATE)?;
    let mut lines = vec![summary];

    if !project_context.instruction_files.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Project instruction files ({}):",
            project_context.instruction_files.len()
        ));
        for (index, file) in project_context.instruction_files.iter().enumerate() {
            let preview = file.content.lines().next().unwrap_or("").trim();
            let preview = if preview.is_empty() {
                "<empty>"
            } else {
                preview
            };
            lines.push(format!(
                "  {}. {} (lines={}, preview={})",
                index + 1,
                file.path.display(),
                file.content.lines().count(),
                preview
            ));
        }
    }

    Ok(lines.join("\n"))
}

fn init_repo_kcode_md() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(initialize_repo(&cwd)?.render())
}

fn run_init() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let config_home = ConfigLoader::default_for(&cwd).config_home().to_path_buf();
    println!("{}", initialize_user_config(&config_home)?.render());
    Ok(())
}

fn normalize_permission_mode(mode: &str) -> Option<&'static str> {
    match mode.trim() {
        "read-only" => Some("read-only"),
        "workspace-write" => Some("workspace-write"),
        "danger-full-access" => Some("danger-full-access"),
        _ => None,
    }
}

fn render_diff_report() -> Result<String, Box<dyn std::error::Error>> {
    render_diff_report_for(&env::current_dir()?)
}

fn render_diff_report_for(cwd: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let staged = run_git_diff_command_in(cwd, &["diff", "--cached"])?;
    let unstaged = run_git_diff_command_in(cwd, &["diff"])?;
    if staged.trim().is_empty() && unstaged.trim().is_empty() {
        return Ok(
            "Diff\n  Result           clean working tree\n  Detail           no current changes"
                .to_string(),
        );
    }

    let mut sections = Vec::new();
    if !staged.trim().is_empty() {
        sections.push(format!("Staged changes:\n{}", staged.trim_end()));
    }
    if !unstaged.trim().is_empty() {
        sections.push(format!("Unstaged changes:\n{}", unstaged.trim_end()));
    }

    Ok(format!("Diff\n\n{}", sections.join("\n\n")))
}

fn run_git_diff_command_in(
    cwd: &Path,
    args: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn render_teleport_report(target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;

    let file_list = Command::new("rg")
        .args(["--files"])
        .current_dir(&cwd)
        .output()?;
    let file_matches = if file_list.status.success() {
        String::from_utf8(file_list.stdout)?
            .lines()
            .filter(|line| line.contains(target))
            .take(10)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let content_output = Command::new("rg")
        .args(["-n", "-S", "--color", "never", target, "."])
        .current_dir(&cwd)
        .output()?;

    let mut lines = vec![
        "Teleport".to_string(),
        format!("  Target           {target}"),
        "  Action           search workspace files and content for the target".to_string(),
    ];
    if !file_matches.is_empty() {
        lines.push(String::new());
        lines.push("File matches".to_string());
        lines.extend(file_matches.into_iter().map(|path| format!("  {path}")));
    }

    if content_output.status.success() {
        let matches = String::from_utf8(content_output.stdout)?;
        if !matches.trim().is_empty() {
            lines.push(String::new());
            lines.push("Content matches".to_string());
            lines.push(truncate_for_prompt(&matches, 4_000));
        }
    }

    if lines.len() == 1 {
        lines.push("  Result           no matches found".to_string());
    }

    Ok(lines.join("\n"))
}

fn render_last_tool_debug_report(session: &Session) -> Result<String, Box<dyn std::error::Error>> {
    let last_tool_use = session
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            message.blocks.iter().rev().find_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
        })
        .ok_or_else(|| "no prior tool call found in session".to_string())?;

    let tool_result = session.messages.iter().rev().find_map(|message| {
        message.blocks.iter().rev().find_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } if tool_use_id == &last_tool_use.0 => {
                Some((tool_name.clone(), output.clone(), *is_error))
            }
            _ => None,
        })
    });

    let mut lines = vec![
        "Debug tool call".to_string(),
        "  Action           inspect the last recorded tool call and its result".to_string(),
        format!("  Tool id          {}", last_tool_use.0),
        format!("  Tool name        {}", last_tool_use.1),
        "  Input".to_string(),
        indent_block(&last_tool_use.2, 4),
    ];

    match tool_result {
        Some((tool_name, output, is_error)) => {
            lines.push("  Result".to_string());
            lines.push(format!("    name           {tool_name}"));
            lines.push(format!(
                "    status         {}",
                if is_error { "error" } else { "ok" }
            ));
            lines.push(indent_block(&output, 4));
        }
        None => lines.push("  Result           missing tool result".to_string()),
    }

    Ok(lines.join("\n"))
}

fn indent_block(value: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_no_args(
    command_name: &str,
    args: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(args) = args.map(str::trim).filter(|value| !value.is_empty()) {
        return Err(format!(
            "{command_name} does not accept arguments. Received: {args}\nUsage: {command_name}"
        )
        .into());
    }
    Ok(())
}

fn format_bughunter_report(scope: Option<&str>) -> String {
    format!(
        "Bughunter
  Scope            {}
  Action           inspect the selected code for likely bugs and correctness issues
  Output           findings should include file paths, severity, and suggested fixes",
        scope.unwrap_or("the current repository")
    )
}

fn format_ultraplan_report(task: Option<&str>) -> String {
    format!(
        "Ultraplan
  Task             {}
  Action           break work into a multi-step execution plan
  Output           plan should cover goals, risks, sequencing, verification, and rollback",
        task.unwrap_or("the current repo work")
    )
}

fn format_pr_report(branch: &str, context: Option<&str>) -> String {
    format!(
        "PR
  Branch           {branch}
  Context          {}
  Action           draft or create a pull request for the current branch
  Output           title and markdown body suitable for GitHub",
        context.unwrap_or("none")
    )
}

fn format_issue_report(context: Option<&str>) -> String {
    format!(
        "Issue
  Context          {}
  Action           draft or create a GitHub issue from the current context
  Output           title and markdown body suitable for GitHub",
        context.unwrap_or("none")
    )
}

fn git_output(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn git_status_ok(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(())
}

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn write_temp_text_file(
    filename: &str,
    contents: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = env::temp_dir().join(filename);
    fs::write(&path, contents)?;
    Ok(path)
}

fn recent_user_context(session: &Session, limit: usize) -> String {
    let requests = session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .filter_map(|message| {
            message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.trim().to_string()),
                _ => None,
            })
        })
        .rev()
        .take(limit)
        .collect::<Vec<_>>();

    if requests.is_empty() {
        "<no prior user messages>".to_string()
    } else {
        requests
            .into_iter()
            .rev()
            .enumerate()
            .map(|(index, text)| format!("{}. {}", index + 1, text))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn truncate_for_prompt(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.trim().to_string()
    } else {
        let truncated = value.chars().take(limit).collect::<String>();
        format!("{}\n…[truncated]", truncated.trim_end())
    }
}

fn sanitize_generated_message(value: &str) -> String {
    value.trim().trim_matches('`').trim().replace("\r\n", "\n")
}

fn parse_titled_body(value: &str) -> Option<(String, String)> {
    let normalized = sanitize_generated_message(value);
    let title = normalized
        .lines()
        .find_map(|line| line.strip_prefix("TITLE:").map(str::trim))?;
    let body_start = normalized.find("BODY:")?;
    let body = normalized[body_start + "BODY:".len()..].trim();
    Some((title.to_string(), body.to_string()))
}

fn render_version_report() -> String {
    let git_sha = GIT_SHA.unwrap_or("unknown");
    let target = BUILD_TARGET.unwrap_or("unknown");
    format!(
        "Kcode\n  Version          {VERSION}\n  Git SHA          {git_sha}\n  Target           {target}\n  Build date       {DEFAULT_DATE}"
    )
}

fn render_export_text(session: &Session) -> String {
    let mut lines = vec!["# Conversation Export".to_string(), String::new()];
    for (index, message) in session.messages.iter().enumerate() {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        lines.push(format!("## {}. {role}", index + 1));
        for block in &message.blocks {
            match block {
                ContentBlock::Text { text } => lines.push(text.clone()),
                ContentBlock::ToolUse { id, name, input } => {
                    lines.push(format!("[tool_use id={id} name={name}] {input}"));
                }
                ContentBlock::ToolResult {
                    tool_use_id,
                    tool_name,
                    output,
                    is_error,
                } => {
                    lines.push(format!(
                        "[tool_result id={tool_use_id} name={tool_name} error={is_error}] {output}"
                    ));
                }
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn default_export_filename(session: &Session) -> String {
    let stem = session
        .messages
        .iter()
        .find_map(|message| match message.role {
            MessageRole::User => message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        })
        .map_or("conversation", |text| {
            text.lines().next().unwrap_or("conversation")
        })
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    let fallback = if stem.is_empty() {
        "conversation"
    } else {
        &stem
    };
    format!("{fallback}.txt")
}

fn resolve_export_path(
    requested_path: Option<&str>,
    session: &Session,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let file_name =
        requested_path.map_or_else(|| default_export_filename(session), ToOwned::to_owned);
    let final_name = if Path::new(&file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
    {
        file_name
    } else {
        format!("{file_name}.txt")
    };
    Ok(cwd.join(final_name))
}

fn build_system_prompt() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(load_system_prompt(
        env::current_dir()?,
        DEFAULT_DATE,
        env::consts::OS,
        "unknown",
    )?)
}

fn build_runtime_plugin_state(
    profile_supports_tools: bool,
) -> Result<RuntimePluginState, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    build_runtime_plugin_state_with_loader(&cwd, &loader, &runtime_config, profile_supports_tools)
}

fn build_runtime_plugin_state_with_loader(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &runtime::RuntimeConfig,
    profile_supports_tools: bool,
) -> Result<RuntimePluginState, Box<dyn std::error::Error>> {
    if !profile_supports_tools {
        let feature_config = runtime_config
            .feature_config()
            .clone()
            .with_hooks(runtime::RuntimeHookConfig::default())
            .with_plugins(runtime::RuntimePluginConfig::default());
        return Ok(RuntimePluginState {
            feature_config,
            tool_registry: GlobalToolRegistry::empty(),
            plugin_registry: PluginRegistry::new(Vec::new()),
        });
    }

    let plugin_manager = build_plugin_manager(cwd, loader, runtime_config);
    let plugin_registry = plugin_manager.plugin_registry()?;
    let plugin_hook_config =
        runtime_hook_config_from_plugin_hooks(plugin_registry.aggregated_hooks()?);
    let feature_config = runtime_config
        .feature_config()
        .clone()
        .with_hooks(runtime_config.hooks().merged(&plugin_hook_config));
    let tool_registry = GlobalToolRegistry::with_plugin_tools(plugin_registry.aggregated_tools()?)?;
    Ok(RuntimePluginState {
        feature_config,
        tool_registry,
        plugin_registry,
    })
}

fn build_plugin_manager(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &runtime::RuntimeConfig,
) -> PluginManager {
    let plugin_settings = runtime_config.plugins();
    let mut plugin_config = PluginManagerConfig::new(loader.config_home().to_path_buf());
    plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
    plugin_config.external_dirs = plugin_settings
        .external_directories()
        .iter()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path))
        .collect();
    plugin_config.install_root = plugin_settings
        .install_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.registry_path = plugin_settings
        .registry_path()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.bundled_root = plugin_settings
        .bundled_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    PluginManager::new(plugin_config)
}

fn resolve_plugin_path(cwd: &Path, config_home: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else if value.starts_with('.') {
        cwd.join(path)
    } else {
        config_home.join(path)
    }
}

fn runtime_hook_config_from_plugin_hooks(hooks: PluginHooks) -> runtime::RuntimeHookConfig {
    runtime::RuntimeHookConfig::new(
        hooks.pre_tool_use,
        hooks.post_tool_use,
        hooks.post_tool_use_failure,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalPromptProgressState {
    command_label: &'static str,
    task_label: String,
    step: usize,
    phase: String,
    detail: Option<String>,
    saw_final_text: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalPromptProgressEvent {
    Started,
    Update,
    Heartbeat,
    Complete,
    Failed,
}

#[derive(Debug)]
struct InternalPromptProgressShared {
    state: Mutex<InternalPromptProgressState>,
    output_lock: Mutex<()>,
    started_at: Instant,
}

#[derive(Debug, Clone)]
struct InternalPromptProgressReporter {
    shared: Arc<InternalPromptProgressShared>,
}

#[derive(Debug)]
struct InternalPromptProgressRun {
    reporter: InternalPromptProgressReporter,
    heartbeat_stop: Option<mpsc::Sender<()>>,
    heartbeat_handle: Option<thread::JoinHandle<()>>,
}

impl InternalPromptProgressReporter {
    fn ultraplan(task: &str) -> Self {
        Self {
            shared: Arc::new(InternalPromptProgressShared {
                state: Mutex::new(InternalPromptProgressState {
                    command_label: "Ultraplan",
                    task_label: task.to_string(),
                    step: 0,
                    phase: "planning started".to_string(),
                    detail: Some(format!("task: {task}")),
                    saw_final_text: false,
                }),
                output_lock: Mutex::new(()),
                started_at: Instant::now(),
            }),
        }
    }

    fn emit(&self, event: InternalPromptProgressEvent, error: Option<&str>) {
        let snapshot = self.snapshot();
        let line = format_internal_prompt_progress_line(event, &snapshot, self.elapsed(), error);
        self.write_line(&line);
    }

    fn mark_model_phase(&self) {
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            state.step += 1;
            state.phase = if state.step == 1 {
                "analyzing request".to_string()
            } else {
                "reviewing findings".to_string()
            };
            state.detail = Some(format!("task: {}", state.task_label));
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn mark_tool_phase(&self, name: &str, input: &str) {
        let detail = describe_tool_progress(name, input);
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            state.step += 1;
            state.phase = format!("running {name}");
            state.detail = Some(detail);
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn mark_text_phase(&self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let detail = truncate_for_summary(first_visible_line(trimmed), 120);
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            if state.saw_final_text {
                return;
            }
            state.saw_final_text = true;
            state.step += 1;
            state.phase = "drafting final plan".to_string();
            state.detail = (!detail.is_empty()).then_some(detail);
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn emit_heartbeat(&self) {
        let snapshot = self.snapshot();
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn snapshot(&self) -> InternalPromptProgressState {
        self.shared
            .state
            .lock()
            .expect("internal prompt progress state poisoned")
            .clone()
    }

    fn elapsed(&self) -> Duration {
        self.shared.started_at.elapsed()
    }

    fn write_line(&self, line: &str) {
        let _guard = self
            .shared
            .output_lock
            .lock()
            .expect("internal prompt progress output lock poisoned");
        let mut stdout = io::stdout();
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    }
}

impl InternalPromptProgressRun {
    fn start_ultraplan(task: &str) -> Self {
        let reporter = InternalPromptProgressReporter::ultraplan(task);
        reporter.emit(InternalPromptProgressEvent::Started, None);

        let (heartbeat_stop, heartbeat_rx) = mpsc::channel();
        let heartbeat_reporter = reporter.clone();
        let heartbeat_handle = thread::spawn(move || loop {
            match heartbeat_rx.recv_timeout(INTERNAL_PROGRESS_HEARTBEAT_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => heartbeat_reporter.emit_heartbeat(),
            }
        });

        Self {
            reporter,
            heartbeat_stop: Some(heartbeat_stop),
            heartbeat_handle: Some(heartbeat_handle),
        }
    }

    fn reporter(&self) -> InternalPromptProgressReporter {
        self.reporter.clone()
    }

    fn finish_success(&mut self) {
        self.stop_heartbeat();
        self.reporter
            .emit(InternalPromptProgressEvent::Complete, None);
    }

    fn finish_failure(&mut self, error: &str) {
        self.stop_heartbeat();
        self.reporter
            .emit(InternalPromptProgressEvent::Failed, Some(error));
    }

    fn stop_heartbeat(&mut self) {
        if let Some(sender) = self.heartbeat_stop.take() {
            let _ = sender.send(());
        }
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for InternalPromptProgressRun {
    fn drop(&mut self) {
        self.stop_heartbeat();
    }
}

fn format_internal_prompt_progress_line(
    event: InternalPromptProgressEvent,
    snapshot: &InternalPromptProgressState,
    elapsed: Duration,
    error: Option<&str>,
) -> String {
    let elapsed_seconds = elapsed.as_secs();
    let step_label = if snapshot.step == 0 {
        "current step pending".to_string()
    } else {
        format!("current step {}", snapshot.step)
    };
    let mut status_bits = vec![step_label, format!("phase {}", snapshot.phase)];
    if let Some(detail) = snapshot
        .detail
        .as_deref()
        .filter(|detail| !detail.is_empty())
    {
        status_bits.push(detail.to_string());
    }
    let status = status_bits.join(" · ");
    match event {
        InternalPromptProgressEvent::Started => {
            format!(
                "🧭 {} status · planning started · {status}",
                snapshot.command_label
            )
        }
        InternalPromptProgressEvent::Update => {
            format!("… {} status · {status}", snapshot.command_label)
        }
        InternalPromptProgressEvent::Heartbeat => format!(
            "… {} heartbeat · {elapsed_seconds}s elapsed · {status}",
            snapshot.command_label
        ),
        InternalPromptProgressEvent::Complete => format!(
            "✔ {} status · completed · {elapsed_seconds}s elapsed · {} steps total",
            snapshot.command_label, snapshot.step
        ),
        InternalPromptProgressEvent::Failed => format!(
            "✘ {} status · failed · {elapsed_seconds}s elapsed · {}",
            snapshot.command_label,
            error.unwrap_or("unknown error")
        ),
    }
}

fn describe_tool_progress(name: &str, input: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));
    match name {
        "bash" | "Bash" => {
            let command = parsed
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if command.is_empty() {
                "running shell command".to_string()
            } else {
                format!("command {}", truncate_for_summary(command.trim(), 100))
            }
        }
        "read_file" | "Read" => format!("reading {}", extract_tool_path(&parsed)),
        "write_file" | "Write" => format!("writing {}", extract_tool_path(&parsed)),
        "edit_file" | "Edit" => format!("editing {}", extract_tool_path(&parsed)),
        "glob_search" | "Glob" => {
            let pattern = parsed
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let scope = parsed
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            format!("glob `{pattern}` in {scope}")
        }
        "grep_search" | "Grep" => {
            let pattern = parsed
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let scope = parsed
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            format!("grep `{pattern}` in {scope}")
        }
        "web_search" | "WebSearch" => parsed
            .get("query")
            .and_then(|value| value.as_str())
            .map_or_else(
                || "running web search".to_string(),
                |query| format!("query {}", truncate_for_summary(query, 100)),
            ),
        _ => {
            let summary = summarize_tool_payload(input);
            if summary.is_empty() {
                format!("running {name}")
            } else {
                format!("{name}: {summary}")
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime(
    session: Session,
    session_id: &str,
    model: String,
    model_override: Option<&str>,
    profile_override: Option<&str>,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let setup_context = load_setup_context(
        if emit_output {
            SetupMode::Interactive
        } else {
            SetupMode::Print
        },
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
        enable_tools,
        emit_output,
        allowed_tools,
        permission_mode,
        progress_reporter,
        &setup_context,
        runtime_plugin_state,
    )
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime_with_plugin_state(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
    setup_context: &SetupContext,
    runtime_plugin_state: RuntimePluginState,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let RuntimePluginState {
        feature_config,
        tool_registry,
        plugin_registry,
    } = runtime_plugin_state;
    plugin_registry.initialize()?;
    let mut runtime = ConversationRuntime::new_with_features(
        session,
        ProviderRuntimeClient::new(
            session_id,
            model,
            enable_tools,
            emit_output,
            allowed_tools.clone(),
            tool_registry.clone(),
            progress_reporter,
            setup_context,
        )?,
        CliToolExecutor::new(allowed_tools.clone(), emit_output, tool_registry.clone()),
        permission_policy(
            permission_mode,
            &feature_config,
            &tool_registry,
            setup_context.active_profile.profile.supports_tools,
        )
        .map_err(std::io::Error::other)?,
        system_prompt,
        &feature_config,
    );
    if emit_output {
        runtime = runtime.with_hook_progress_reporter(Box::new(CliHookProgressReporter));
    }
    Ok(BuiltRuntime::new(
        runtime,
        plugin_registry,
        setup_context.active_profile.clone(),
    ))
}

struct CliHookProgressReporter;

impl runtime::HookProgressReporter for CliHookProgressReporter {
    fn on_event(&mut self, event: &runtime::HookProgressEvent) {
        match event {
            runtime::HookProgressEvent::Started {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
            runtime::HookProgressEvent::Completed {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook done {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
            runtime::HookProgressEvent::Cancelled {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook cancelled {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
        }
    }
}

struct CliPermissionPrompter {
    current_mode: PermissionMode,
}

impl CliPermissionPrompter {
    fn new(current_mode: PermissionMode) -> Self {
        Self { current_mode }
    }
}

impl runtime::PermissionPrompter for CliPermissionPrompter {
    fn decide(
        &mut self,
        request: &runtime::PermissionRequest,
    ) -> runtime::PermissionPromptDecision {
        println!();
        println!("Permission approval required");
        println!("  Tool             {}", request.tool_name);
        println!("  Current mode     {}", self.current_mode.as_str());
        println!("  Required mode    {}", request.required_mode.as_str());
        if let Some(reason) = &request.reason {
            println!("  Reason           {reason}");
        }
        println!("  Input            {}", request.input);
        print!("Approve this tool call? [y/N]: ");
        let _ = io::stdout().flush();

        let mut response = String::new();
        match io::stdin().read_line(&mut response) {
            Ok(_) => {
                let normalized = response.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "y" | "yes") {
                    runtime::PermissionPromptDecision::Allow
                } else {
                    runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied by user approval prompt",
                            request.tool_name
                        ),
                    }
                }
            }
            Err(error) => runtime::PermissionPromptDecision::Deny {
                reason: format!("permission approval failed: {error}"),
            },
        }
    }
}

struct ProviderRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: OpenAiCompatClient,
    model: String,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    progress_reporter: Option<InternalPromptProgressReporter>,
    supports_streaming: bool,
}

impl ProviderRuntimeClient {
    fn new(
        session_id: &str,
        model: String,
        enable_tools: bool,
        emit_output: bool,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        progress_reporter: Option<InternalPromptProgressReporter>,
        setup_context: &SetupContext,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let launch = ProviderLauncher::prepare(&setup_context.active_profile)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let tools_enabled = enable_tools && launch.supports_tools;
        Ok(Self {
            runtime: tokio::runtime::Runtime::new()?,
            client: OpenAiCompatClient::new(
                launch.api_key,
                OpenAiCompatConfig {
                    provider_name: "Kcode",
                    api_key_env: PRIMARY_API_KEY_ENV,
                    base_url_env: PRIMARY_BASE_URL_ENV,
                    default_base_url: "",
                },
            )
            .with_base_url(launch.base_url)
            .with_request_timeout(Duration::from_millis(launch.request_timeout_ms))
            .with_retry_policy(
                launch.max_retries,
                Duration::from_millis(200),
                Duration::from_secs(2),
            ),
            model,
            enable_tools: tools_enabled,
            emit_output,
            allowed_tools,
            tool_registry,
            progress_reporter,
            supports_streaming: launch.supports_streaming,
        })
    }
}

impl ApiClient for ProviderRuntimeClient {
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        if let Some(progress_reporter) = &self.progress_reporter {
            progress_reporter.mark_model_phase();
        }
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
            tools: self
                .enable_tools
                .then(|| filter_tool_specs(&self.tool_registry, self.allowed_tools.as_ref())),
            tool_choice: self.enable_tools.then_some(ToolChoice::Auto),
            stream: self.supports_streaming,
        };

        self.runtime.block_on(async {
            if !self.supports_streaming {
                let mut stdout = io::stdout();
                let mut sink = io::sink();
                let out: &mut dyn Write = if self.emit_output {
                    &mut stdout
                } else {
                    &mut sink
                };
                let response = self
                    .client
                    .send_message(&MessageRequest {
                        stream: false,
                        ..message_request.clone()
                    })
                    .await
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                return response_to_events(response, out);
            }

            let mut stream = self
                .client
                .stream_message(&message_request)
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut stdout = io::stdout();
            let mut sink = io::sink();
            let out: &mut dyn Write = if self.emit_output {
                &mut stdout
            } else {
                &mut sink
            };
            let renderer = TerminalRenderer::new();
            let mut markdown_stream = MarkdownStreamState::default();
            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;
            let mut saw_stop = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?
            {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(block, out, &mut events, &mut pending_tool, true)?;
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            out,
                            &mut events,
                            &mut pending_tool,
                            true,
                        )?;
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                if let Some(progress_reporter) = &self.progress_reporter {
                                    progress_reporter.mark_text_phase(&text);
                                }
                                if let Some(rendered) = markdown_stream.push(&renderer, &text) {
                                    write!(out, "{rendered}")
                                        .and_then(|()| out.flush())
                                        .map_err(|error| RuntimeError::new(error.to_string()))?;
                                }
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = &mut pending_tool {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. }
                        | ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(_) => {
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        if let Some((id, name, input)) = pending_tool.take() {
                            if let Some(progress_reporter) = &self.progress_reporter {
                                progress_reporter.mark_tool_phase(&name, &input);
                            }
                            // Display tool call now that input is fully accumulated
                            writeln!(out, "\n{}", format_tool_call_start(&name, &input))
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(delta.usage.token_usage()));
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            push_prompt_cache_record(&mut events);

            if !saw_stop
                && events.iter().any(|event| {
                    matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                        || matches!(event, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events = response_to_events(response, out)?;
            push_prompt_cache_record(&mut events);
            Ok(events)
        })
    }
}

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

fn format_tool_call_start(name: &str, input: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));

    let detail = match name {
        "bash" | "Bash" => format_bash_call(&parsed),
        "read_file" | "Read" => {
            let path = extract_tool_path(&parsed);
            format!("📄 Reading {path}…")
        }
        "write_file" | "Write" => {
            let path = extract_tool_path(&parsed);
            let lines = parsed
                .get("content")
                .and_then(|value| value.as_str())
                .map_or(0, |content| content.lines().count());
            format!("✏️ Writing {path} ({lines} lines)")
        }
        "edit_file" | "Edit" => {
            let path = extract_tool_path(&parsed);
            let old_value = parsed
                .get("old_string")
                .or_else(|| parsed.get("oldString"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let new_value = parsed
                .get("new_string")
                .or_else(|| parsed.get("newString"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!(
                "📝 Editing {path}{}",
                format_patch_preview(old_value, new_value)
                    .map(|preview| format!("\n{preview}"))
                    .unwrap_or_default()
            )
        }
        "glob_search" | "Glob" => format_search_start("🔎 Glob", &parsed),
        "grep_search" | "Grep" => format_search_start("🔎 Grep", &parsed),
        "web_search" | "WebSearch" => parsed
            .get("query")
            .and_then(|value| value.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => summarize_tool_payload(input),
    };

    let palette = ThemePalette::default_terminal();
    let border = "─".repeat(name.len() + 8);
    let name_colored = render_with_palette(&palette, name, SemanticRole::Tool, true, true);
    format!(
        "╭─ {name_colored} ─╮\n│ {detail}\n╰{border}╯"
    )
}

fn format_tool_result(name: &str, output: &str, is_error: bool) -> String {
    let palette = ThemePalette::default_terminal();
    let role = if is_error { SemanticRole::Error } else { SemanticRole::Success };
    let icon = render_with_palette(&palette, if is_error { "✗" } else { "✓" }, role, true, true);

    if is_error {
        let summary = truncate_for_summary(output.trim(), 160);
        return if summary.is_empty() {
            format!("{icon} {name}")
        } else {
            let summary_colored = render_with_palette(&palette, &summary, SemanticRole::Error, true, false);
            format!("{icon} {name}\n{summary_colored}")
        };
    }

    let parsed: serde_json::Value =
        serde_json::from_str(output).unwrap_or(serde_json::Value::String(output.to_string()));
    match name {
        "bash" | "Bash" => format_bash_result(&icon, &parsed),
        "read_file" | "Read" => format_read_result(&icon, &parsed),
        "write_file" | "Write" => format_write_result(&icon, &parsed),
        "edit_file" | "Edit" => format_edit_result(&icon, &parsed),
        "glob_search" | "Glob" => format_glob_result(&icon, &parsed),
        "grep_search" | "Grep" => format_grep_result(&icon, &parsed),
        _ => format_generic_tool_result(&icon, name, &parsed),
    }
}

const DISPLAY_TRUNCATION_NOTICE: &str =
    "… output truncated for display; full result preserved in session.";
const READ_DISPLAY_MAX_LINES: usize = 80;
const READ_DISPLAY_MAX_CHARS: usize = 6_000;
const TOOL_OUTPUT_DISPLAY_MAX_LINES: usize = 60;
const TOOL_OUTPUT_DISPLAY_MAX_CHARS: usize = 4_000;

fn extract_tool_path(parsed: &serde_json::Value) -> String {
    parsed
        .get("file_path")
        .or_else(|| parsed.get("filePath"))
        .or_else(|| parsed.get("path"))
        .and_then(|value| value.as_str())
        .unwrap_or("?")
        .to_string()
}

fn format_search_start(label: &str, parsed: &serde_json::Value) -> String {
    let pattern = parsed
        .get("pattern")
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    let scope = parsed
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    format!("{label} {pattern}\nin {scope}")
}

fn format_patch_preview(old_value: &str, new_value: &str) -> Option<String> {
    if old_value.is_empty() && new_value.is_empty() {
        return None;
    }
    Some(format!(
        "- {}\n+ {}",
        truncate_for_summary(first_visible_line(old_value), 72),
        truncate_for_summary(first_visible_line(new_value), 72)
    ))
}

fn format_bash_call(parsed: &serde_json::Value) -> String {
    let command = parsed
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if command.is_empty() {
        String::new()
    } else {
        format!(
            "\x1b[48;5;236;38;5;255m $ {} \x1b[0m",
            truncate_for_summary(command, 160)
        )
    }
}

fn first_visible_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
}

fn format_bash_result(icon: &str, parsed: &serde_json::Value) -> String {
    use std::fmt::Write as _;

    let mut lines = vec![format!("{icon} \x1b[38;5;245mbash\x1b[0m")];
    if let Some(task_id) = parsed
        .get("backgroundTaskId")
        .and_then(|value| value.as_str())
    {
        write!(&mut lines[0], " backgrounded ({task_id})").expect("write to string");
    } else if let Some(status) = parsed
        .get("returnCodeInterpretation")
        .and_then(|value| value.as_str())
        .filter(|status| !status.is_empty())
    {
        write!(&mut lines[0], " {status}").expect("write to string");
    }

    if let Some(stdout) = parsed.get("stdout").and_then(|value| value.as_str()) {
        if !stdout.trim().is_empty() {
            lines.push(truncate_output_for_display(
                stdout,
                TOOL_OUTPUT_DISPLAY_MAX_LINES,
                TOOL_OUTPUT_DISPLAY_MAX_CHARS,
            ));
        }
    }
    if let Some(stderr) = parsed.get("stderr").and_then(|value| value.as_str()) {
        if !stderr.trim().is_empty() {
            lines.push(format!(
                "\x1b[38;5;203m{}\x1b[0m",
                truncate_output_for_display(
                    stderr,
                    TOOL_OUTPUT_DISPLAY_MAX_LINES,
                    TOOL_OUTPUT_DISPLAY_MAX_CHARS,
                )
            ));
        }
    }

    lines.join("\n\n")
}

fn format_read_result(icon: &str, parsed: &serde_json::Value) -> String {
    let file = parsed.get("file").unwrap_or(parsed);
    let path = extract_tool_path(file);
    let start_line = file
        .get("startLine")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    let num_lines = file
        .get("numLines")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let total_lines = file
        .get("totalLines")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(num_lines);
    let content = file
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let end_line = start_line.saturating_add(num_lines.saturating_sub(1));

    format!(
        "{icon} \x1b[2m📄 Read {path} (lines {}-{} of {})\x1b[0m\n{}",
        start_line,
        end_line.max(start_line),
        total_lines,
        truncate_output_for_display(content, READ_DISPLAY_MAX_LINES, READ_DISPLAY_MAX_CHARS)
    )
}

fn format_write_result(icon: &str, parsed: &serde_json::Value) -> String {
    let path = extract_tool_path(parsed);
    let kind = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("write");
    let line_count = parsed
        .get("content")
        .and_then(|value| value.as_str())
        .map_or(0, |content| content.lines().count());
    format!(
        "{icon} \x1b[1;32m✏️ {} {path}\x1b[0m \x1b[2m({line_count} lines)\x1b[0m",
        if kind == "create" { "Wrote" } else { "Updated" },
    )
}

fn format_structured_patch_preview(parsed: &serde_json::Value) -> Option<String> {
    let hunks = parsed.get("structuredPatch")?.as_array()?;
    let mut preview = Vec::new();
    for hunk in hunks.iter().take(2) {
        let lines = hunk.get("lines")?.as_array()?;
        for line in lines.iter().filter_map(|value| value.as_str()).take(6) {
            match line.chars().next() {
                Some('+') => preview.push(format!("\x1b[38;5;70m{line}\x1b[0m")),
                Some('-') => preview.push(format!("\x1b[38;5;203m{line}\x1b[0m")),
                _ => preview.push(line.to_string()),
            }
        }
    }
    if preview.is_empty() {
        None
    } else {
        Some(preview.join("\n"))
    }
}

fn format_edit_result(icon: &str, parsed: &serde_json::Value) -> String {
    let path = extract_tool_path(parsed);
    let suffix = if parsed
        .get("replaceAll")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        " (replace all)"
    } else {
        ""
    };
    let preview = format_structured_patch_preview(parsed).or_else(|| {
        let old_value = parsed
            .get("oldString")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let new_value = parsed
            .get("newString")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        format_patch_preview(old_value, new_value)
    });

    match preview {
        Some(preview) => format!("{icon} \x1b[1;33m📝 Edited {path}{suffix}\x1b[0m\n{preview}"),
        None => format!("{icon} \x1b[1;33m📝 Edited {path}{suffix}\x1b[0m"),
    }
}

fn format_glob_result(icon: &str, parsed: &serde_json::Value) -> String {
    let num_files = parsed
        .get("numFiles")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let filenames = parsed
        .get("filenames")
        .and_then(|value| value.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|value| value.as_str())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if filenames.is_empty() {
        format!("{icon} \x1b[38;5;245mglob_search\x1b[0m matched {num_files} files")
    } else {
        format!("{icon} \x1b[38;5;245mglob_search\x1b[0m matched {num_files} files\n{filenames}")
    }
}

fn format_grep_result(icon: &str, parsed: &serde_json::Value) -> String {
    let num_matches = parsed
        .get("numMatches")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let num_files = parsed
        .get("numFiles")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let content = parsed
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let filenames = parsed
        .get("filenames")
        .and_then(|value| value.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|value| value.as_str())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let summary = format!(
        "{icon} \x1b[38;5;245mgrep_search\x1b[0m {num_matches} matches across {num_files} files"
    );
    if !content.trim().is_empty() {
        format!(
            "{summary}\n{}",
            truncate_output_for_display(
                content,
                TOOL_OUTPUT_DISPLAY_MAX_LINES,
                TOOL_OUTPUT_DISPLAY_MAX_CHARS,
            )
        )
    } else if !filenames.is_empty() {
        format!("{summary}\n{filenames}")
    } else {
        summary
    }
}

fn format_generic_tool_result(icon: &str, name: &str, parsed: &serde_json::Value) -> String {
    let rendered_output = match parsed {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            serde_json::to_string_pretty(parsed).unwrap_or_else(|_| parsed.to_string())
        }
        _ => parsed.to_string(),
    };
    let preview = truncate_output_for_display(
        &rendered_output,
        TOOL_OUTPUT_DISPLAY_MAX_LINES,
        TOOL_OUTPUT_DISPLAY_MAX_CHARS,
    );

    if preview.is_empty() {
        format!("{icon} \x1b[38;5;245m{name}\x1b[0m")
    } else if preview.contains('\n') {
        format!("{icon} \x1b[38;5;245m{name}\x1b[0m\n{preview}")
    } else {
        format!("{icon} \x1b[38;5;245m{name}:\x1b[0m {preview}")
    }
}

fn summarize_tool_payload(payload: &str) -> String {
    let compact = match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(value) => value.to_string(),
        Err(_) => payload.trim().to_string(),
    };
    truncate_for_summary(&compact, 96)
}

fn truncate_for_summary(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn truncate_output_for_display(content: &str, max_lines: usize, max_chars: usize) -> String {
    let original = content.trim_end_matches('\n');
    if original.is_empty() {
        return String::new();
    }

    let mut preview_lines = Vec::new();
    let mut used_chars = 0usize;
    let mut truncated = false;

    for (index, line) in original.lines().enumerate() {
        if index >= max_lines {
            truncated = true;
            break;
        }

        let newline_cost = usize::from(!preview_lines.is_empty());
        let available = max_chars.saturating_sub(used_chars + newline_cost);
        if available == 0 {
            truncated = true;
            break;
        }

        let line_chars = line.chars().count();
        if line_chars > available {
            preview_lines.push(line.chars().take(available).collect::<String>());
            truncated = true;
            break;
        }

        preview_lines.push(line.to_string());
        used_chars += newline_cost + line_chars;
    }

    let mut preview = preview_lines.join("\n");
    if truncated {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str(DISPLAY_TRUNCATION_NOTICE);
    }
    preview
}

fn push_output_block(
    block: OutputContentBlock,
    out: &mut (impl Write + ?Sized),
    events: &mut Vec<AssistantEvent>,
    pending_tool: &mut Option<(String, String, String)>,
    streaming_tool_input: bool,
) -> Result<(), RuntimeError> {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                let rendered = TerminalRenderer::new().markdown_to_ansi(&text);
                write!(out, "{rendered}")
                    .and_then(|()| out.flush())
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            // During streaming, the initial content_block_start has an empty input ({}).
            // The real input arrives via input_json_delta events. In
            // non-streaming responses, preserve a legitimate empty object.
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(serde_json::Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            *pending_tool = Some((id, name, initial_input));
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
    Ok(())
}

fn response_to_events(
    response: MessageResponse,
    out: &mut (impl Write + ?Sized),
) -> Result<Vec<AssistantEvent>, RuntimeError> {
    let mut events = Vec::new();
    let mut pending_tool = None;

    for block in response.content {
        push_output_block(block, out, &mut events, &mut pending_tool, false)?;
        if let Some((id, name, input)) = pending_tool.take() {
            events.push(AssistantEvent::ToolUse { id, name, input });
        }
    }

    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    Ok(events)
}

fn push_prompt_cache_record(_events: &mut Vec<AssistantEvent>) {}

fn prompt_cache_record_to_runtime_event(
    record: api::PromptCacheRecord,
) -> Option<PromptCacheEvent> {
    let cache_break = record.cache_break?;
    Some(PromptCacheEvent {
        unexpected: cache_break.unexpected,
        reason: cache_break.reason,
        previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
        current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
        token_drop: cache_break.token_drop,
    })
}

struct CliToolExecutor {
    renderer: TerminalRenderer,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
}

impl CliToolExecutor {
    fn new(
        allowed_tools: Option<AllowedToolSet>,
        emit_output: bool,
        tool_registry: GlobalToolRegistry,
    ) -> Self {
        Self {
            renderer: TerminalRenderer::new(),
            emit_output,
            allowed_tools,
            tool_registry,
        }
    }
}

impl ToolExecutor for CliToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current --allowedTools setting"
            )));
        }
        let value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        match self.tool_registry.execute(tool_name, &value) {
            Ok(output) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &output, false);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|error| ToolError::new(error.to_string()))?;
                }
                Ok(output)
            }
            Err(error) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &error, true);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|stream_error| ToolError::new(stream_error.to_string()))?;
                }
                Err(ToolError::new(error))
            }
        }
    }
}

fn permission_policy(
    mode: PermissionMode,
    feature_config: &runtime::RuntimeFeatureConfig,
    tool_registry: &GlobalToolRegistry,
    profile_supports_tools: bool,
) -> Result<PermissionPolicy, String> {
    let policy =
        PermissionPolicy::new(mode).with_permission_rules(feature_config.permission_rules());
    if !profile_supports_tools {
        return Ok(policy.with_tool_use_disabled(
            "tool use is unavailable because the active profile disables tools",
        ));
    }

    Ok(tool_registry.permission_specs(None)?.into_iter().fold(
        policy,
        |policy, (name, required_permission)| {
            policy.with_tool_requirement(name, required_permission)
        },
    ))
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

fn help_profile_supports_tools(profile_override: Option<&str>) -> bool {
    load_setup_context(
        SetupMode::Config,
        None,
        profile_override,
        default_permission_mode(),
        None,
    )
    .map(|setup| setup.active_profile.profile.supports_tools)
    .unwrap_or(true)
}

#[allow(clippy::too_many_lines)]
fn print_help_to_for_profile(out: &mut impl Write, profile_supports_tools: bool) -> io::Result<()> {
    let context =
        CommandRegistryContext::for_surface(CommandSurface::CliLocal, profile_supports_tools);
    let snapshot = build_command_registry_snapshot(&context, &[]);
    let mcp_available = snapshot
        .process_commands
        .iter()
        .any(|descriptor| descriptor.name == "mcp");
    let resume_commands = snapshot
        .session_commands
        .iter()
        .filter(|descriptor| descriptor.resume_supported)
        .map(command_descriptor_usage)
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(out, "{CLI_NAME} v{VERSION}")?;
    writeln!(out)?;
    writeln!(out, "Usage:")?;
    if profile_supports_tools {
        writeln!(
            out,
            "  {CLI_NAME} [--model MODEL] [--profile PROFILE] [--allowedTools TOOL[,TOOL...]]"
        )?;
    } else {
        writeln!(out, "  {CLI_NAME} [--model MODEL] [--profile PROFILE]")?;
    }
    writeln!(out, "      Start the interactive REPL")?;
    writeln!(
        out,
        "  {CLI_NAME} [--model MODEL] [--profile PROFILE] [--output-format text|json] prompt TEXT"
    )?;
    writeln!(out, "      Send one prompt and exit")?;
    writeln!(
        out,
        "  {CLI_NAME} [--model MODEL] [--profile PROFILE] [--output-format text|json] TEXT"
    )?;
    writeln!(out, "      Shorthand non-interactive prompt mode")?;
    writeln!(
        out,
        "  {CLI_NAME} --resume [SESSION.jsonl|session-id|latest] [/status] [/compact] [...]"
    )?;
    writeln!(
        out,
        "      Inspect or maintain a saved session without entering the REPL"
    )?;
    writeln!(out, "  {CLI_NAME} help")?;
    writeln!(out, "      Alias for --help")?;
    writeln!(out, "  {CLI_NAME} version")?;
    writeln!(out, "      Alias for --version")?;
    writeln!(out, "  {CLI_NAME} status")?;
    writeln!(
        out,
        "      Show the current local workspace status snapshot"
    )?;
    writeln!(out, "  {CLI_NAME} sandbox")?;
    writeln!(out, "      Show the current sandbox isolation snapshot")?;
    writeln!(out, "  {CLI_NAME} agents")?;
    if mcp_available {
        writeln!(out, "  {CLI_NAME} mcp")?;
    }
    writeln!(out, "  {CLI_NAME} skills")?;
    writeln!(out, "  {CLI_NAME} commands [show [local|bridge]]")?;
    writeln!(out, "  {CLI_NAME} profile [list|show [name]]")?;
    writeln!(
        out,
        "  {CLI_NAME} system-prompt [--cwd PATH] [--date YYYY-MM-DD]"
    )?;
    writeln!(out, "  {CLI_NAME} login")?;
    writeln!(out, "  {CLI_NAME} logout")?;
    writeln!(out, "  {CLI_NAME} init")?;
    writeln!(out)?;
    writeln!(out, "Flags:")?;
    writeln!(
        out,
        "  --model MODEL              Override the active model"
    )?;
    writeln!(
        out,
        "  --profile PROFILE          Override the active provider profile"
    )?;
    writeln!(
        out,
        "  --output-format FORMAT     Non-interactive output format: text or json"
    )?;
    writeln!(
        out,
        "  --permission-mode MODE     Set read-only, workspace-write, or danger-full-access"
    )?;
    writeln!(
        out,
        "  --dangerously-skip-permissions  Skip all permission checks"
    )?;
    if profile_supports_tools {
        writeln!(
            out,
            "  --allowedTools TOOLS       Restrict enabled tools (repeatable; comma-separated aliases supported)"
        )?;
    }
    writeln!(
        out,
        "  --version, -V              Print version and build information locally"
    )?;
    writeln!(out)?;
    writeln!(out, "Interactive slash commands:")?;
    writeln!(out, "{}", render_slash_command_help_for_context(&context))?;
    writeln!(out)?;
    writeln!(out, "Resume-safe commands: {resume_commands}")?;
    writeln!(out)?;
    writeln!(out, "Session shortcuts:")?;
    writeln!(
        out,
        "  REPL turns auto-save to .kcode/sessions/<session-id>.{PRIMARY_SESSION_EXTENSION}"
    )?;
    writeln!(
        out,
        "  Use `{LATEST_SESSION_REFERENCE}` with --resume, /resume, or /session switch to target the newest saved session"
    )?;
    writeln!(
        out,
        "  Use /session list in the REPL to browse managed sessions"
    )?;
    writeln!(out, "Examples:")?;
    writeln!(
        out,
        "  {CLI_NAME} --model gpt-4.1-mini --profile custom \"summarize this repo\""
    )?;
    writeln!(
        out,
        "  {CLI_NAME} --output-format json prompt \"explain src/main.rs\""
    )?;
    if profile_supports_tools {
        writeln!(
            out,
            "  {CLI_NAME} --allowedTools read,glob \"summarize Cargo.toml\""
        )?;
    }
    writeln!(out, "  {CLI_NAME} --resume {LATEST_SESSION_REFERENCE}")?;
    writeln!(
        out,
        "  {CLI_NAME} --resume {LATEST_SESSION_REFERENCE} /status /diff /export notes.txt"
    )?;
    writeln!(out, "  {CLI_NAME} agents")?;
    if mcp_available {
        writeln!(out, "  {CLI_NAME} mcp show my-server")?;
    }
    writeln!(out, "  {CLI_NAME} commands show bridge")?;
    writeln!(out, "  {CLI_NAME} profile show nvidia")?;
    writeln!(out, "  {CLI_NAME} profile list")?;
    writeln!(out, "  {CLI_NAME} /skills")?;
    writeln!(out, "  {CLI_NAME} init")?;
    writeln!(out, "  {CLI_NAME} doctor")?;
    writeln!(out, "  {CLI_NAME} config show")?;
    Ok(())
}

fn print_help_to(out: &mut impl Write) -> io::Result<()> {
    print_help_to_for_profile(out, true)
}

fn print_help_to_with_profile_override(
    out: &mut impl Write,
    profile_override: Option<&str>,
) -> io::Result<()> {
    print_help_to_for_profile(out, help_profile_supports_tools(profile_override))
}

fn print_help(profile_override: Option<&str>) {
    let _ = print_help_to_with_profile_override(&mut io::stdout(), profile_override);
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_plugin_state_with_loader, build_runtime_with_plugin_state,
        create_managed_session_handle, describe_tool_progress,
        ensure_session_command_available_for_profile, filter_tool_specs, format_bughunter_report,
        format_commit_preflight_report, format_commit_skipped_report, format_compact_report,
        format_cost_report, format_internal_prompt_progress_line, format_issue_report,
        format_model_report, format_model_switch_report, format_permissions_report,
        format_permissions_switch_report, format_pr_report, format_resume_report,
        format_status_report, format_tool_call_start, format_tool_result, format_ultraplan_report,
        format_unknown_slash_command, format_unknown_slash_command_message,
        normalize_permission_mode, parse_args, parse_git_status_branch,
        parse_git_status_metadata_for, parse_git_workspace_summary, permission_policy,
        print_help_to, print_help_to_for_profile, push_output_block, render_commands_report,
        render_config_report, render_diff_report, render_doctor_report_from_setup,
        render_memory_report, render_repl_help, render_repl_help_for_profile,
        render_resume_usage, resolve_model_alias, resolve_session_reference, response_to_events,
        resume_supported_slash_commands, run_resume_command,
        slash_command_completion_candidates_with_sessions, status_context, validate_no_args,
        CliAction, CliOutputFormat, CommandReportSurfaceSelection, GitWorkspaceSummary,
        InternalPromptProgressEvent, InternalPromptProgressState, LiveCli, ProviderRuntimeClient,
        SlashCommand, StatusUsage, DEFAULT_MODEL,
    };
    use api::{MessageResponse, OutputContentBlock, Usage};
    use plugins::{
        PluginManager, PluginManagerConfig, PluginTool, PluginToolDefinition, PluginToolPermission,
    };
    use runtime::{
        AssistantEvent, ConfigLoader, ContentBlock, ConversationMessage, CredentialResolution,
        CredentialSource, MessageRole, PermissionMode, PermissionOutcome, ProviderProfile,
        ResolutionSource, ResolvedProviderProfile, Session,
    };
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tools::GlobalToolRegistry;

    fn registry_with_plugin_tool() -> GlobalToolRegistry {
        GlobalToolRegistry::with_plugin_tools(vec![PluginTool::new(
            "plugin-demo@external",
            "plugin-demo",
            PluginToolDefinition {
                name: "plugin_echo".to_string(),
                description: Some("Echo plugin payload".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            },
            "echo".to_string(),
            Vec::new(),
            PluginToolPermission::WorkspaceWrite,
            None,
        )])
        .expect("plugin tool registry should build")
    }

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("kcode-cli-{nanos}"))
    }

    fn git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git command should run");
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn with_current_dir<T>(cwd: &Path, f: impl FnOnce() -> T) -> T {
        let previous = std::env::current_dir().expect("cwd should load");
        std::env::set_current_dir(cwd).expect("cwd should change");
        let result = f();
        std::env::set_current_dir(previous).expect("cwd should restore");
        result
    }

    fn test_setup_context(workspace: &Path) -> runtime::SetupContext {
        runtime::SetupContext {
            inputs: runtime::BootstrapInputs {
                argv: vec!["kcode".to_string(), "doctor".to_string()],
                cwd: workspace.to_path_buf(),
                platform: "linux".to_string(),
                stdio_mode: runtime::StdioMode::NonInteractive,
                invocation_kind: runtime::SetupMode::Doctor,
            },
            session_id: None,
            cwd: workspace.to_path_buf(),
            project_root: workspace.to_path_buf(),
            git_root: None,
            resolved_config: runtime::ResolvedConfig {
                config_home: workspace.join(".kcode"),
                session_dir: workspace.join(".kcode").join("sessions"),
                discovered_entries: Vec::new(),
                loaded_entries: Vec::new(),
                config_file_present: false,
                model: DEFAULT_MODEL.to_string(),
                base_url: None,
                api_key_env: "KCODE_API_KEY".to_string(),
                api_key_present: false,
                oauth_credentials_present: false,
                profile: None,
                legacy_paths: Vec::new(),
            },
            active_profile: ResolvedProviderProfile {
                profile_name: "cliproxyapi".to_string(),
                profile_source: ResolutionSource::ProfileDefault,
                model: DEFAULT_MODEL.to_string(),
                model_source: ResolutionSource::ProfileDefault,
                base_url: None,
                base_url_source: ResolutionSource::Missing,
                credential: CredentialResolution {
                    source: CredentialSource::Missing,
                    env_name: "KCODE_API_KEY".to_string(),
                    api_key: None,
                },
                profile: ProviderProfile {
                    name: "cliproxyapi".to_string(),
                    base_url_env: "KCODE_BASE_URL".to_string(),
                    base_url: String::new(),
                    api_key_env: "KCODE_API_KEY".to_string(),
                    default_model: DEFAULT_MODEL.to_string(),
                    supports_tools: true,
                    supports_streaming: true,
                    request_timeout_ms: 120_000,
                    max_retries: 2,
                },
            },
            trust_policy: runtime::TrustPolicyContext {
                permission_mode: "danger-full-access".to_string(),
                workspace_writeable: true,
                config_home_writeable: true,
                trusted_workspace: true,
            },
            mode: runtime::SetupMode::Doctor,
        }
    }

    fn write_plugin_fixture(root: &Path, name: &str, include_hooks: bool, include_lifecycle: bool) {
        fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
        if include_hooks {
            fs::create_dir_all(root.join("hooks")).expect("hooks dir");
            fs::write(
                root.join("hooks").join("pre.sh"),
                "#!/bin/sh\nprintf 'plugin pre hook'\n",
            )
            .expect("write hook");
        }
        if include_lifecycle {
            fs::create_dir_all(root.join("lifecycle")).expect("lifecycle dir");
            fs::write(
                root.join("lifecycle").join("init.sh"),
                "#!/bin/sh\nprintf 'init\\n' >> lifecycle.log\n",
            )
            .expect("write init lifecycle");
            fs::write(
                root.join("lifecycle").join("shutdown.sh"),
                "#!/bin/sh\nprintf 'shutdown\\n' >> lifecycle.log\n",
            )
            .expect("write shutdown lifecycle");
        }

        let hooks = if include_hooks {
            ",\n  \"hooks\": {\n    \"PreToolUse\": [\"./hooks/pre.sh\"]\n  }"
        } else {
            ""
        };
        let lifecycle = if include_lifecycle {
            ",\n  \"lifecycle\": {\n    \"Init\": [\"./lifecycle/init.sh\"],\n    \"Shutdown\": [\"./lifecycle/shutdown.sh\"]\n  }"
        } else {
            ""
        };
        fs::write(
            root.join(".claude-plugin").join("plugin.json"),
            format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\",\n  \"description\": \"runtime plugin fixture\"{hooks}{lifecycle}\n}}"
            ),
        )
        .expect("write plugin manifest");
    }
    #[test]
    fn defaults_to_repl_when_no_args() {
        let permission_mode = super::default_permission_mode();
        assert_eq!(
            parse_args(&[]).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn default_permission_mode_uses_project_config_when_env_is_unset() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn env_permission_mode_overrides_project_config_default() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", "read-only");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::ReadOnly);
    }

    #[test]
    fn parses_prompt_subcommand() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "prompt".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn parses_bare_prompt_and_json_output_flag() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "--output-format=json".to_string(),
            "--model".to_string(),
            "claude-opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "claude-opus".to_string(),
                model_explicit: true,
                profile: None,
                output_format: CliOutputFormat::Json,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn resolves_model_aliases_in_args() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "--model".to_string(),
            "opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "claude-opus-4-6".to_string(),
                model_explicit: true,
                profile: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn resolves_known_model_aliases() {
        assert_eq!(resolve_model_alias("opus"), "claude-opus-4-6");
        assert_eq!(resolve_model_alias("sonnet"), "claude-sonnet-4-6");
        assert_eq!(resolve_model_alias("haiku"), "claude-haiku-4-5-20251213");
        assert_eq!(resolve_model_alias("claude-opus"), "claude-opus");
    }

    #[test]
    fn parses_version_flags_without_initializing_prompt_mode() {
        assert_eq!(
            parse_args(&["--version".to_string()]).expect("args should parse"),
            CliAction::Version
        );
        assert_eq!(
            parse_args(&["-V".to_string()]).expect("args should parse"),
            CliAction::Version
        );
    }

    #[test]
    fn parses_permission_mode_flag() {
        let args = vec!["--permission-mode=read-only".to_string()];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                allowed_tools: None,
                permission_mode: PermissionMode::ReadOnly,
            }
        );
    }

    #[test]
    fn parses_allowed_tools_flags_with_aliases_and_lists() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "--allowedTools".to_string(),
            "read,glob".to_string(),
            "--allowed-tools=write_file".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                allowed_tools: Some(
                    ["glob_search", "read_file", "write_file"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                ),
                permission_mode,
            }
        );
    }

    #[test]
    fn rejects_allowed_tools_when_active_profile_disables_tools() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(&cwd).expect("cwd should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            config_home.join("config.toml"),
            r#"
profile = "bridge"

[profiles.bridge]
default_model = "gpt-4.1-mini"
base_url_env = "BRIDGE_BASE_URL"
api_key_env = "BRIDGE_API_KEY"
supports_tools = false
supports_streaming = false
"#,
        )
        .expect("config should write");

        let original_config_home = std::env::var("KCODE_CONFIG_HOME").ok();
        let original_profile = std::env::var("KCODE_PROFILE").ok();
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);
        std::env::remove_var("KCODE_PROFILE");

        let error = with_current_dir(&cwd, || {
            parse_args(&["--allowedTools".to_string(), "read".to_string()])
        })
        .expect_err("tool-less profile should reject allowed tools");

        match original_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }
        match original_profile {
            Some(value) => std::env::set_var("KCODE_PROFILE", value),
            None => std::env::remove_var("KCODE_PROFILE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert!(error.contains("`--allowedTools` is unavailable"));
        assert!(error.contains("active profile `bridge`"));
    }

    #[test]
    fn allowed_tools_use_cli_profile_override_when_default_profile_is_toolless() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(&cwd).expect("cwd should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            config_home.join("config.toml"),
            r#"
profile = "bridge"

[profiles.bridge]
default_model = "gpt-4.1-mini"
base_url_env = "BRIDGE_BASE_URL"
api_key_env = "BRIDGE_API_KEY"
supports_tools = false
supports_streaming = false
"#,
        )
        .expect("config should write");

        let original_config_home = std::env::var("KCODE_CONFIG_HOME").ok();
        let original_profile = std::env::var("KCODE_PROFILE").ok();
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);
        std::env::remove_var("KCODE_PROFILE");

        let permission_mode = with_current_dir(&cwd, super::default_permission_mode);
        let action = with_current_dir(&cwd, || {
            parse_args(&[
                "--profile".to_string(),
                "cliproxyapi".to_string(),
                "--allowedTools".to_string(),
                "read".to_string(),
            ])
        })
        .expect("tool-capable profile should accept allowed tools");

        match original_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }
        match original_profile {
            Some(value) => std::env::set_var("KCODE_PROFILE", value),
            None => std::env::remove_var("KCODE_PROFILE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(
            action,
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: Some("cliproxyapi".to_string()),
                allowed_tools: Some(["read_file"].into_iter().map(str::to_string).collect()),
                permission_mode,
            }
        );
    }

    #[test]
    fn rejects_unknown_allowed_tools() {
        let error = parse_args(&[
            "--profile".to_string(),
            "cliproxyapi".to_string(),
            "--allowedTools".to_string(),
            "teleport".to_string(),
        ])
        .expect_err("tool should be rejected");
        assert!(error.contains("unsupported tool in --allowedTools: teleport"));
    }

    #[test]
    fn parses_system_prompt_options() {
        let args = vec![
            "system-prompt".to_string(),
            "--cwd".to_string(),
            "/tmp/project".to_string(),
            "--date".to_string(),
            "2026-04-01".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::PrintSystemPrompt {
                cwd: PathBuf::from("/tmp/project"),
                date: "2026-04-01".to_string(),
            }
        );
    }

    #[test]
    fn parses_login_and_logout_subcommands() {
        assert_eq!(
            parse_args(&["login".to_string()]).expect("login should parse"),
            CliAction::Login
        );
        assert_eq!(
            parse_args(&["logout".to_string()]).expect("logout should parse"),
            CliAction::Logout
        );
        assert_eq!(
            parse_args(&["init".to_string()]).expect("init should parse"),
            CliAction::Init
        );
        assert_eq!(
            parse_args(&["doctor".to_string()]).expect("doctor should parse"),
            CliAction::Doctor {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["config".to_string(), "show".to_string()])
                .expect("config show should parse"),
            CliAction::ConfigShow {
                section: None,
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&[
                "config".to_string(),
                "show".to_string(),
                "plugins".to_string(),
            ])
            .expect("config section should parse"),
            CliAction::ConfigShow {
                section: Some("plugins".to_string()),
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&[
                "commands".to_string(),
                "show".to_string(),
                "bridge".to_string()
            ])
            .expect("commands show bridge should parse"),
            CliAction::Commands {
                surface: CommandReportSurfaceSelection::Bridge,
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string()]).expect("agents should parse"),
            CliAction::Agents { args: None }
        );
        assert_eq!(
            parse_args(&["mcp".to_string()]).expect("mcp should parse"),
            CliAction::Mcp {
                args: None,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["skills".to_string()]).expect("skills should parse"),
            CliAction::Skills { args: None }
        );
        assert_eq!(
            parse_args(&["agents".to_string(), "--help".to_string()])
                .expect("agents help should parse"),
            CliAction::Agents {
                args: Some("--help".to_string())
            }
        );
    }

    #[test]
    fn parses_single_word_command_aliases_without_falling_back_to_prompt_mode() {
        let permission_mode = super::default_permission_mode();
        assert_eq!(
            parse_args(&["help".to_string()]).expect("help should parse"),
            CliAction::Help { profile: None }
        );
        assert_eq!(
            parse_args(&["version".to_string()]).expect("version should parse"),
            CliAction::Version
        );
        assert_eq!(
            parse_args(&["status".to_string()]).expect("status should parse"),
            CliAction::Status {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                permission_mode,
            }
        );
        assert_eq!(
            parse_args(&["sandbox".to_string()]).expect("sandbox should parse"),
            CliAction::Sandbox
        );
        assert_eq!(
            parse_args(&["commands".to_string()]).expect("commands should parse"),
            CliAction::Commands {
                surface: CommandReportSurfaceSelection::Local,
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
    }

    #[test]
    fn single_word_slash_command_names_return_guidance_instead_of_hitting_prompt_mode() {
        let error = parse_args(&["cost".to_string()]).expect_err("cost should return guidance");
        assert!(error.contains("slash command"));
        assert!(error.contains("/cost"));
    }

    #[test]
    fn multi_word_prompt_still_uses_shorthand_prompt_mode() {
        let permission_mode = super::default_permission_mode();
        assert_eq!(
            parse_args(&["help".to_string(), "me".to_string(), "debug".to_string()])
                .expect("prompt shorthand should still work"),
            CliAction::Prompt {
                prompt: "help me debug".to_string(),
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn parses_direct_agents_mcp_and_skills_slash_commands() {
        assert_eq!(
            parse_args(&["/agents".to_string()]).expect("/agents should parse"),
            CliAction::Agents { args: None }
        );
        assert_eq!(
            parse_args(&["/mcp".to_string(), "show".to_string(), "demo".to_string()])
                .expect("/mcp show demo should parse"),
            CliAction::Mcp {
                args: Some("show demo".to_string()),
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string()]).expect("/skills should parse"),
            CliAction::Skills { args: None }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "help".to_string()])
                .expect("/skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string())
            }
        );
        assert_eq!(
            parse_args(&[
                "/skills".to_string(),
                "install".to_string(),
                "./fixtures/help-skill".to_string(),
            ])
            .expect("/skills install should parse"),
            CliAction::Skills {
                args: Some("install ./fixtures/help-skill".to_string())
            }
        );
        let error = parse_args(&["/status".to_string()])
            .expect_err("/status should remain REPL-only when invoked directly");
        assert!(error.contains("interactive-only"));
        assert!(error.contains("kcode --resume SESSION.jsonl /status"));
    }

    #[test]
    fn process_mcp_command_preserves_profile_override() {
        assert_eq!(
            parse_args(&[
                "--profile".to_string(),
                "bridge".to_string(),
                "mcp".to_string(),
            ])
            .expect("mcp with profile should parse"),
            CliAction::Mcp {
                args: None,
                profile: Some("bridge".to_string()),
            }
        );
        assert_eq!(
            parse_args(&[
                "--profile".to_string(),
                "bridge".to_string(),
                "/mcp".to_string(),
                "list".to_string(),
            ])
            .expect("direct /mcp with profile should parse"),
            CliAction::Mcp {
                args: Some("list".to_string()),
                profile: Some("bridge".to_string()),
            }
        );
    }

    #[test]
    fn direct_slash_commands_surface_shared_validation_errors() {
        let compact_error = parse_args(&["/compact".to_string(), "now".to_string()])
            .expect_err("invalid /compact shape should be rejected");
        assert!(compact_error.contains("Unexpected arguments for /compact."));
        assert!(compact_error.contains("Usage            /compact"));

        let plugins_error = parse_args(&[
            "/plugins".to_string(),
            "list".to_string(),
            "extra".to_string(),
        ])
        .expect_err("invalid /plugins list shape should be rejected");
        assert!(plugins_error.contains("Usage: /plugin list"));
        assert!(plugins_error.contains("Aliases          /plugins, /marketplace"));
    }

    #[test]
    fn formats_unknown_slash_command_with_suggestions() {
        let report = format_unknown_slash_command_message("statsu");
        assert!(report.contains("unknown slash command: /statsu"));
        assert!(report.contains("Did you mean"));
        assert!(report.contains("Use /help"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/compact".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/compact".to_string()],
            }
        );
    }

    #[test]
    fn parses_resume_flag_without_path_as_latest_session() {
        assert_eq!(
            parse_args(&["--resume".to_string()]).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec![],
            }
        );
        assert_eq!(
            parse_args(&["--resume".to_string(), "/status".to_string()])
                .expect("resume shortcut should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec!["/status".to_string()],
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_multiple_slash_commands() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/status".to_string(),
            "/compact".to_string(),
            "/cost".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/status".to_string(),
                    "/compact".to_string(),
                    "/cost".to_string(),
                ],
            }
        );
    }

    #[test]
    fn rejects_unknown_options_with_helpful_guidance() {
        let error = parse_args(&["--resum".to_string()]).expect_err("unknown option should fail");
        assert!(error.contains("unknown option: --resum"));
        assert!(error.contains("Did you mean --resume?"));
        assert!(error.contains("kcode --help"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command_arguments() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "notes.txt".to_string(),
            "/clear".to_string(),
            "--confirm".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/export notes.txt".to_string(),
                    "/clear --confirm".to_string(),
                ],
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_absolute_export_path() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "/tmp/notes.txt".to_string(),
            "/status".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/export /tmp/notes.txt".to_string(), "/status".to_string()],
            }
        );
    }

    #[test]
    fn filtered_tool_specs_respect_allowlist() {
        let allowed = ["read_file", "grep_search"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let filtered = filter_tool_specs(&GlobalToolRegistry::builtin(), Some(&allowed));
        let names = filtered
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file", "grep_search"]);
    }

    #[test]
    fn filtered_tool_specs_include_plugin_tools() {
        let filtered = filter_tool_specs(&registry_with_plugin_tool(), None);
        let names = filtered
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"plugin_echo".to_string()));
    }

    #[test]
    fn permission_policy_uses_plugin_tool_permissions() {
        let feature_config = runtime::RuntimeFeatureConfig::default();
        let policy = permission_policy(
            PermissionMode::ReadOnly,
            &feature_config,
            &registry_with_plugin_tool(),
            true,
        )
        .expect("permission policy should build");
        let required = policy.required_mode_for("plugin_echo");
        assert_eq!(required, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn permission_policy_disables_tool_use_for_toolless_profiles() {
        let feature_config = runtime::RuntimeFeatureConfig::default();
        let policy = permission_policy(
            PermissionMode::DangerFullAccess,
            &feature_config,
            &registry_with_plugin_tool(),
            false,
        )
        .expect("permission policy should build");

        assert_eq!(
            policy.authorize("bash", "{}", None),
            PermissionOutcome::Deny {
                reason: "tool use is unavailable because the active profile disables tools"
                    .to_string(),
            }
        );
    }

    #[test]
    fn shared_help_uses_resume_annotation_copy() {
        let help = commands::render_slash_command_help();
        assert!(help.contains("Slash commands"));
        assert!(help.contains("works with --resume SESSION.jsonl"));
    }

    #[test]
    fn repl_help_includes_shared_commands_and_exit() {
        let help = render_repl_help();
        assert!(help.contains("REPL"));
        assert!(help.contains("/help"));
        assert!(help.contains("Complete commands, modes, and recent sessions"));
        assert!(help.contains("/status"));
        assert!(help.contains("/sandbox"));
        assert!(help.contains("/model [model]"));
        assert!(help.contains("/permissions [read-only|workspace-write|danger-full-access]"));
        assert!(help.contains("/clear [--confirm]"));
        assert!(help.contains("/cost"));
        assert!(help.contains("/resume <session-path>"));
        assert!(help.contains("/config [env|hooks|model|plugins]"));
        assert!(help.contains("/mcp [list|show <server>|help]"));
        assert!(help.contains("/memory"));
        assert!(help.contains("/init"));
        assert!(help.contains("/diff"));
        assert!(help.contains("/version"));
        assert!(help.contains("/export [file]"));
        assert!(help.contains("/session [list|switch <session-id>|fork [branch-name]]"));
        assert!(help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
        assert!(help.contains("aliases: /plugins, /marketplace"));
        assert!(help.contains("/agents"));
        assert!(help.contains("/skills"));
        assert!(help.contains("/exit"));
        assert!(help.contains("Auto-save            .kcode/sessions/<session-id>.jsonl"));
        assert!(help.contains("Resume latest        /resume latest"));
    }

    #[test]
    fn repl_help_hides_tool_commands_when_profile_disables_tools() {
        let help = render_repl_help_for_profile(false);
        assert!(help.contains("Start here        /doctor, /config, /status, /memory"));
        assert!(!help.contains("/mcp [list|show <server>|help]"));
        assert!(!help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
    }

    #[test]
    fn completion_candidates_include_workflow_shortcuts_and_dynamic_sessions() {
        let completions = slash_command_completion_candidates_with_sessions(
            "sonnet",
            true,
            Some("session-current"),
            vec!["session-old".to_string()],
        );

        assert!(completions.contains(&"/model claude-sonnet-4-6".to_string()));
        assert!(completions.contains(&"/permissions workspace-write".to_string()));
        assert!(completions.contains(&"/session list".to_string()));
        assert!(completions.contains(&"/session switch session-current".to_string()));
        assert!(completions.contains(&"/resume session-old".to_string()));
        assert!(completions.contains(&"/mcp list".to_string()));
    }

    #[test]
    fn completion_candidates_hide_tool_commands_when_profile_disables_tools() {
        let completions = slash_command_completion_candidates_with_sessions(
            "sonnet",
            false,
            Some("session-current"),
            vec!["session-old".to_string()],
        );

        assert!(!completions.contains(&"/mcp".to_string()));
        assert!(!completions.contains(&"/mcp list".to_string()));
        assert!(!completions.contains(&"/plugin list".to_string()));
        assert!(completions.contains(&"/session list".to_string()));
    }

    #[test]
    fn startup_banner_mentions_workflow_completions() {
        let _guard = env_lock();
        std::env::set_var("KCODE_BASE_URL", "https://router.example.test/v1");
        std::env::set_var("KCODE_API_KEY", "test-dummy-key-for-banner-test");
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");

        let banner = with_current_dir(&root, || {
            LiveCli::new(
                "claude-sonnet-4-6".to_string(),
                false,
                None,
                true,
                None,
                PermissionMode::DangerFullAccess,
            )
            .expect("cli should initialize")
            .startup_banner()
        });

        assert!(banner.contains("Tab"));
        assert!(banner.contains("workflow completions"));
        assert!(banner.contains("Profile"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
        std::env::remove_var("KCODE_BASE_URL");
        std::env::remove_var("KCODE_API_KEY");
    }

    #[test]
    fn commands_report_reflects_bridge_surface_and_profile_capability() {
        let _guard = env_lock();
        let root = temp_dir();
        let config_home = root.join("home").join(".kcode");
        fs::create_dir_all(&config_home).expect("config home");
        fs::write(
            config_home.join("config.toml"),
            r#"
profile = "bridge"
model = "gpt-4.1-mini"

[profiles.bridge]
default_model = "gpt-4.1-mini"
base_url_env = "BRIDGE_BASE_URL"
api_key_env = "BRIDGE_API_KEY"
supports_tools = false
supports_streaming = false
"#,
        )
        .expect("write config");
        let previous_config_home = std::env::var_os("KCODE_CONFIG_HOME");
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);

        let report = with_current_dir(&root, || {
            render_commands_report(CommandReportSurfaceSelection::Bridge, None, None)
                .expect("commands report should render")
        });

        match previous_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }

        assert!(report.contains("Commands"));
        assert!(report.contains("Surface           bridge"));
        assert!(report.contains("Safety profile    bridge-safe"));
        assert!(report.contains("Supports tools    false"));
        assert!(report.contains("Supports stream   false"));
        assert!(report.contains("Filtered"));
        assert!(report.contains("/mcp"));
        assert!(report.contains("active profile does not expose tool-capable commands"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn session_command_availability_rejects_tool_commands_for_toolless_profile() {
        let profile = ResolvedProviderProfile {
            profile_name: "bridge".to_string(),
            profile_source: ResolutionSource::Config("config.profile"),
            model: "gpt-4.1-mini".to_string(),
            model_source: ResolutionSource::Config("config.model"),
            base_url: None,
            base_url_source: ResolutionSource::Missing,
            credential: CredentialResolution {
                source: CredentialSource::Missing,
                env_name: "BRIDGE_API_KEY".to_string(),
                api_key: None,
            },
            profile: ProviderProfile {
                name: "bridge".to_string(),
                base_url_env: "BRIDGE_BASE_URL".to_string(),
                base_url: String::new(),
                api_key_env: "BRIDGE_API_KEY".to_string(),
                default_model: "gpt-4.1-mini".to_string(),
                supports_tools: false,
                supports_streaming: false,
                request_timeout_ms: 120_000,
                max_retries: 2,
            },
        };

        let error = ensure_session_command_available_for_profile("mcp", &profile)
            .expect_err("mcp should be blocked");
        assert!(error.contains("command `/mcp` is unavailable"));
        assert!(error.contains("active profile `bridge`"));
    }

    #[test]
    fn resume_supported_command_list_matches_expected_surface() {
        let names = resume_supported_slash_commands()
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "help", "status", "sandbox", "compact", "clear", "cost", "config", "mcp", "memory",
                "init", "diff", "version", "export", "agents", "skills", "doctor",
            ]
        );
    }

    #[test]
    fn resume_report_uses_sectioned_layout() {
        let report = format_resume_report("session.jsonl", 14, 6);
        assert!(report.contains("Session resumed"));
        assert!(report.contains("Session file     session.jsonl"));
        assert!(report.contains("Messages         14"));
        assert!(report.contains("Turns            6"));
    }

    #[test]
    fn compact_report_uses_structured_output() {
        let compacted = format_compact_report(8, 5, false);
        assert!(compacted.contains("Compact"));
        assert!(compacted.contains("Result           compacted"));
        assert!(compacted.contains("Messages removed 8"));
        let skipped = format_compact_report(0, 3, true);
        assert!(skipped.contains("Result           skipped"));
    }

    #[test]
    fn cost_report_uses_sectioned_layout() {
        let report = format_cost_report(runtime::TokenUsage {
            input_tokens: 20,
            output_tokens: 8,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 1,
        });
        assert!(report.contains("Cost"));
        assert!(report.contains("Input tokens     20"));
        assert!(report.contains("Output tokens    8"));
        assert!(report.contains("Cache create     3"));
        assert!(report.contains("Cache read       1"));
        assert!(report.contains("Total tokens     32"));
    }

    #[test]
    fn permissions_report_uses_sectioned_layout() {
        let report = format_permissions_report("workspace-write");
        assert!(report.contains("Permissions"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Modes"));
        assert!(report.contains("read-only          ○ available Read/search tools only"));
        assert!(report.contains("workspace-write    ● current   Edit files inside the workspace"));
        assert!(report.contains("danger-full-access ○ available Unrestricted tool access"));
    }

    #[test]
    fn permissions_switch_report_is_structured() {
        let report = format_permissions_switch_report("read-only", "workspace-write");
        assert!(report.contains("Permissions updated"));
        assert!(report.contains("Result           mode switched"));
        assert!(report.contains("Previous mode    read-only"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Applies to       subsequent tool calls"));
    }

    #[test]
    fn init_help_mentions_direct_subcommand() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("kcode help"));
        assert!(help.contains("kcode version"));
        assert!(help.contains("kcode status"));
        assert!(help.contains("kcode sandbox"));
        assert!(help.contains("kcode init"));
        assert!(help.contains("kcode agents"));
        assert!(help.contains("kcode mcp"));
        assert!(help.contains("kcode commands [show [local|bridge]]"));
        assert!(help.contains("kcode skills"));
        assert!(help.contains("kcode /skills"));
    }

    #[test]
    fn help_hides_tooling_for_toolless_profiles() {
        let mut help = Vec::new();
        print_help_to_for_profile(&mut help, false).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        let resume_line = help
            .lines()
            .find(|line| line.starts_with("Resume-safe commands:"))
            .expect("resume-safe commands line");

        assert!(!help.contains("--allowedTools"));
        assert!(!help.contains("kcode mcp"));
        assert!(!help.contains("kcode mcp show my-server"));
        assert!(!help.contains("/mcp [list|show <server>|help]"));
        assert!(!resume_line.contains("/mcp"));
        assert!(help.contains("kcode commands [show [local|bridge]]"));
        assert!(help.contains("/status"));
    }

    #[test]
    fn model_report_uses_sectioned_layout() {
        let report = format_model_report("claude-sonnet", "cliproxyapi", 12, 4);
        assert!(report.contains("Model"));
        assert!(report.contains("Current model    claude-sonnet"));
        assert!(report.contains("Active profile   cliproxyapi"));
        assert!(report.contains("Session messages 12"));
        assert!(report.contains("Switch models with /model <name>"));
    }

    #[test]
    fn model_switch_report_preserves_context_summary() {
        let report = format_model_switch_report("claude-sonnet", "claude-opus", "nvidia", 9);
        assert!(report.contains("Model updated"));
        assert!(report.contains("Previous         claude-sonnet"));
        assert!(report.contains("Current          claude-opus"));
        assert!(report.contains("Active profile   nvidia"));
        assert!(report.contains("Preserved msgs   9"));
    }

    #[test]
    fn status_line_reports_model_and_token_totals() {
        let profile = ResolvedProviderProfile {
            profile_name: "cliproxyapi".to_string(),
            profile_source: ResolutionSource::ProfileDefault,
            model: "claude-sonnet".to_string(),
            model_source: ResolutionSource::Cli,
            base_url: Some("https://router.example.test/v1".to_string()),
            base_url_source: ResolutionSource::Env("KCODE_BASE_URL"),
            credential: CredentialResolution {
                source: CredentialSource::PrimaryEnv,
                env_name: "KCODE_API_KEY".to_string(),
                api_key: Some("test-key".to_string()),
            },
            profile: ProviderProfile {
                name: "cliproxyapi".to_string(),
                base_url_env: "KCODE_BASE_URL".to_string(),
                base_url: String::new(),
                api_key_env: "KCODE_API_KEY".to_string(),
                default_model: "claude-sonnet".to_string(),
                supports_tools: true,
                supports_streaming: true,
                request_timeout_ms: 120_000,
                max_retries: 2,
            },
        };
        let status = format_status_report(
            "claude-sonnet",
            Some(&profile),
            StatusUsage {
                message_count: 7,
                turns: 3,
                latest: runtime::TokenUsage {
                    input_tokens: 5,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 0,
                },
                cumulative: runtime::TokenUsage {
                    input_tokens: 20,
                    output_tokens: 8,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                },
                estimated_tokens: 128,
            },
            "workspace-write",
            &super::StatusContext {
                cwd: PathBuf::from("/tmp/project"),
                session_path: Some(PathBuf::from("session.jsonl")),
                loaded_config_files: 2,
                discovered_config_files: 3,
                memory_file_count: 4,
                project_root: Some(PathBuf::from("/tmp")),
                git_branch: Some("main".to_string()),
                git_summary: GitWorkspaceSummary {
                    changed_files: 3,
                    staged_files: 1,
                    unstaged_files: 1,
                    untracked_files: 1,
                    conflicted_files: 0,
                },
                sandbox_status: runtime::SandboxStatus::default(),
            },
        );
        assert!(status.contains("Status"));
        assert!(status.contains("Profile          cliproxyapi"));
        assert!(status.contains("Model            claude-sonnet"));
        assert!(status.contains("Permission mode  workspace-write"));
        assert!(status.contains("Endpoint         https://router.example.test/v1"));
        assert!(status.contains("Supports tools   true"));
        assert!(status.contains("Supports stream  true"));
        assert!(status.contains("Messages         7"));
        assert!(status.contains("Latest total     10"));
        assert!(status.contains("Cumulative total 31"));
        assert!(status.contains("Cwd              /tmp/project"));
        assert!(status.contains("Project root     /tmp"));
        assert!(status.contains("Git branch       main"));
        assert!(
            status.contains("Git state        dirty · 3 files · 1 staged, 1 unstaged, 1 untracked")
        );
        assert!(status.contains("Changed files    3"));
        assert!(status.contains("Staged           1"));
        assert!(status.contains("Unstaged         1"));
        assert!(status.contains("Untracked        1"));
        assert!(status.contains("Session          session.jsonl"));
        assert!(status.contains("Config files     loaded 2/3"));
        assert!(status.contains("Memory files     4"));
        assert!(status.contains("Suggested flow   /status → /diff → /commit"));
    }

    #[test]
    fn commit_reports_surface_workspace_context() {
        let summary = GitWorkspaceSummary {
            changed_files: 2,
            staged_files: 1,
            unstaged_files: 1,
            untracked_files: 0,
            conflicted_files: 0,
        };

        let preflight = format_commit_preflight_report(Some("feature/ux"), summary);
        assert!(preflight.contains("Result           ready"));
        assert!(preflight.contains("Branch           feature/ux"));
        assert!(preflight.contains("Workspace        dirty · 2 files · 1 staged, 1 unstaged"));
        assert!(preflight
            .contains("Action           create a git commit from the current workspace changes"));
    }

    #[test]
    fn commit_skipped_report_points_to_next_steps() {
        let report = format_commit_skipped_report();
        assert!(report.contains("Reason           no workspace changes"));
        assert!(report
            .contains("Action           create a git commit from the current workspace changes"));
        assert!(report.contains("/status to inspect context"));
        assert!(report.contains("/diff to inspect repo changes"));
    }

    #[test]
    fn runtime_slash_reports_describe_command_behavior() {
        let bughunter = format_bughunter_report(Some("runtime"));
        assert!(bughunter.contains("Scope            runtime"));
        assert!(bughunter.contains("inspect the selected code for likely bugs"));

        let ultraplan = format_ultraplan_report(Some("ship the release"));
        assert!(ultraplan.contains("Task             ship the release"));
        assert!(ultraplan.contains("break work into a multi-step execution plan"));

        let pr = format_pr_report("feature/ux", Some("ready for review"));
        assert!(pr.contains("Branch           feature/ux"));
        assert!(pr.contains("draft or create a pull request"));

        let issue = format_issue_report(Some("flaky test"));
        assert!(issue.contains("Context          flaky test"));
        assert!(issue.contains("draft or create a GitHub issue"));
    }

    #[test]
    fn no_arg_commands_reject_unexpected_arguments() {
        assert!(validate_no_args("/commit", None).is_ok());

        let error = validate_no_args("/commit", Some("now"))
            .expect_err("unexpected arguments should fail")
            .to_string();
        assert!(error.contains("/commit does not accept arguments"));
        assert!(error.contains("Received: now"));
    }

    #[test]
    fn config_report_supports_section_views() {
        let report =
            render_config_report(Some("env"), None, None).expect("config report should render");
        assert!(report.contains("Merged section: env"));
        let plugins_report = render_config_report(Some("plugins"), None, None)
            .expect("plugins config report should render");
        assert!(plugins_report.contains("Merged section: plugins"));
    }

    #[test]
    fn memory_report_uses_sectioned_layout() {
        let report = render_memory_report().expect("memory report should render");
        // Report should contain either the memory section or the project instruction section
        assert!(
            report.contains("Memory files")
                || report.contains("memory")
                || report.contains("Project instruction")
        );
    }

    #[test]
    fn config_report_uses_sectioned_layout() {
        let report = render_config_report(None, None, None).expect("config report should render");
        assert!(report.contains("Config"));
        assert!(report.contains("Config home"));
        assert!(report.contains("Discovered files"));
        assert!(report.contains("Merged JSON"));
    }

    #[test]
    fn doctor_report_surfaces_missing_bootstrap_state() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        let report = render_doctor_report_from_setup(&test_setup_context(&root));
        assert!(report.contains("Doctor"));
        assert!(report.contains("Runtime ready    no"));
        assert!(report.contains("[fail] config file"));
        assert!(report.contains("[fail] base url"));
        assert!(report.contains("[fail] api credentials"));
        assert!(report.contains("run `kcode init`"));
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn doctor_report_warns_about_legacy_residue() {
        let root = temp_dir();
        let workspace = root.join("workspace");
        fs::create_dir_all(workspace.join(".kcode")).expect("config dir");
        fs::create_dir_all(workspace.join(".kcode").join("sessions")).expect("sessions dir");

        let mut setup = test_setup_context(&workspace);
        setup.resolved_config.config_file_present = true;
        setup.resolved_config.base_url = Some("https://router.example.test".to_string());
        setup.resolved_config.api_key_present = true;
        setup.resolved_config.legacy_paths = vec![workspace.join(".claw").join("settings.json")];

        let report = render_doctor_report_from_setup(&setup);
        assert!(report.contains("Runtime ready    yes"));
        assert!(report.contains("[warn] legacy residue"));
        assert!(report.contains(".claw/settings.json"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn doctor_report_describes_toolless_profile_capability() {
        let root = temp_dir();
        let workspace = root.join("workspace");
        fs::create_dir_all(workspace.join(".kcode")).expect("config dir");
        fs::create_dir_all(workspace.join(".kcode").join("sessions")).expect("sessions dir");

        let mut setup = test_setup_context(&workspace);
        setup.resolved_config.config_file_present = true;
        setup.resolved_config.base_url = Some("https://router.example.test".to_string());
        setup.resolved_config.api_key_present = true;
        setup.active_profile.profile_name = "bridge".to_string();
        setup.active_profile.profile.supports_tools = false;

        let report = render_doctor_report_from_setup(&setup);
        assert!(report.contains("[ok  ] tool capability"));
        assert!(report.contains("disabled by active profile `bridge`"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn nested_session_dir_can_use_existing_workspace_ancestor_for_writeability() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        let nested = root.join(".kcode").join("sessions");
        assert!(super::path_or_parent_writeable(&nested));
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn parses_git_status_metadata() {
        let _guard = env_lock();
        let temp_root = temp_dir();
        fs::create_dir_all(&temp_root).expect("root dir");
        let (project_root, branch) = parse_git_status_metadata_for(
            &temp_root,
            Some(
                "## rcc/cli...origin/rcc/cli
 M src/main.rs",
            ),
        );
        assert_eq!(branch.as_deref(), Some("rcc/cli"));
        assert!(project_root.is_none());
        fs::remove_dir_all(temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn parses_detached_head_from_status_snapshot() {
        let _guard = env_lock();
        assert_eq!(
            parse_git_status_branch(Some(
                "## HEAD (no branch)
 M src/main.rs"
            )),
            Some("detached HEAD".to_string())
        );
    }

    #[test]
    fn parses_git_workspace_summary_counts() {
        let summary = parse_git_workspace_summary(Some(
            "## feature/ux
M  src/main.rs
 M README.md
?? notes.md
UU conflicted.rs",
        ));

        assert_eq!(
            summary,
            GitWorkspaceSummary {
                changed_files: 4,
                staged_files: 2,
                unstaged_files: 2,
                untracked_files: 1,
                conflicted_files: 1,
            }
        );
        assert_eq!(
            summary.headline(),
            "dirty · 4 files · 2 staged, 2 unstaged, 1 untracked, 1 conflicted"
        );
    }

    #[test]
    fn render_diff_report_shows_clean_tree_for_committed_repo() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        let report = with_current_dir(&root, || {
            render_diff_report().expect("diff report should render")
        });
        assert!(report.contains("clean working tree"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_includes_staged_and_unstaged_sections() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        fs::write(root.join("tracked.txt"), "hello\nstaged\n").expect("update file");
        git(&["add", "tracked.txt"], &root);
        fs::write(root.join("tracked.txt"), "hello\nstaged\nunstaged\n")
            .expect("update file twice");

        let report = with_current_dir(&root, || {
            render_diff_report().expect("diff report should render")
        });
        assert!(report.contains("Staged changes:"));
        assert!(report.contains("Unstaged changes:"));
        assert!(report.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_omits_ignored_files() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join(".gitignore"), ".omx/\nignored.txt\n").expect("write gitignore");
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", ".gitignore", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::create_dir_all(root.join(".omx")).expect("write omx dir");
        fs::write(root.join(".omx").join("state.json"), "{}").expect("write ignored omx");
        fs::write(root.join("ignored.txt"), "secret\n").expect("write ignored file");
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("write tracked change");

        let report = with_current_dir(&root, || {
            render_diff_report().expect("diff report should render")
        });
        assert!(report.contains("tracked.txt"));
        assert!(!report.contains("+++ b/ignored.txt"));
        assert!(!report.contains("+++ b/.omx/state.json"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn resume_diff_command_renders_report_for_saved_session() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("modify tracked");
        let session_path = root.join("session.json");
        Session::new()
            .save_to_path(&session_path)
            .expect("session should save");

        let session = Session::load_from_path(&session_path).expect("session should load");
        let outcome = with_current_dir(&root, || {
            run_resume_command(&session_path, &session, &SlashCommand::Diff)
                .expect("resume diff should work")
        });
        let message = outcome.message.expect("diff message should exist");
        assert!(message.contains("Unstaged changes:"));
        assert!(message.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn status_context_reads_real_workspace_metadata() {
        let context = status_context(None).expect("status context should load");
        assert!(context.cwd.is_absolute());
        assert!(context.discovered_config_files >= context.loaded_config_files);
        assert!(context.loaded_config_files <= context.discovered_config_files);
    }

    #[test]
    fn normalizes_supported_permission_modes() {
        assert_eq!(normalize_permission_mode("read-only"), Some("read-only"));
        assert_eq!(
            normalize_permission_mode("workspace-write"),
            Some("workspace-write")
        );
        assert_eq!(
            normalize_permission_mode("danger-full-access"),
            Some("danger-full-access")
        );
        assert_eq!(normalize_permission_mode("unknown"), None);
    }

    #[test]
    fn clear_command_requires_explicit_confirmation_flag() {
        assert_eq!(
            SlashCommand::parse("/clear"),
            Ok(Some(SlashCommand::Clear { confirm: false }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
    }

    #[test]
    fn parses_resume_and_config_slash_commands() {
        assert_eq!(
            SlashCommand::parse("/resume saved-session.jsonl"),
            Ok(Some(SlashCommand::Resume {
                session_path: Some("saved-session.jsonl".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
        assert_eq!(
            SlashCommand::parse("/config"),
            Ok(Some(SlashCommand::Config { section: None }))
        );
        assert_eq!(
            SlashCommand::parse("/config env"),
            Ok(Some(SlashCommand::Config {
                section: Some("env".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/memory"),
            Ok(Some(SlashCommand::Memory))
        );
        assert_eq!(SlashCommand::parse("/init"), Ok(Some(SlashCommand::Init)));
        assert_eq!(
            SlashCommand::parse("/session fork incident-review"),
            Ok(Some(SlashCommand::Session {
                action: Some("fork".to_string()),
                target: Some("incident-review".to_string())
            }))
        );
    }

    #[test]
    fn help_mentions_jsonl_resume_examples() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("kcode --resume [SESSION.jsonl|session-id|latest]"));
        assert!(help.contains("Use `latest` with --resume, /resume, or /session switch"));
        assert!(help.contains("kcode --resume latest"));
        assert!(help.contains("kcode --resume latest /status /diff /export notes.txt"));
    }

    #[test]
    fn managed_sessions_default_to_jsonl_and_resolve_legacy_json() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let workspace = temp_workspace("session-resolution");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let handle = create_managed_session_handle("session-alpha").expect("jsonl handle");
        assert!(handle.path.ends_with("session-alpha.jsonl"));

        let legacy_path = workspace.join(".claw/sessions/legacy.json");
        std::fs::create_dir_all(
            legacy_path
                .parent()
                .expect("legacy path should have parent directory"),
        )
        .expect("session dir should exist");
        Session::new()
            .with_persistence_path(legacy_path.clone())
            .save_to_path(&legacy_path)
            .expect("legacy session should save");

        let resolved = resolve_session_reference("legacy").expect("legacy session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            legacy_path
                .canonicalize()
                .expect("legacy path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn latest_session_alias_resolves_most_recent_managed_session() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let workspace = temp_workspace("latest-session-alias");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let older = create_managed_session_handle("session-older").expect("older handle");
        Session::new()
            .with_persistence_path(older.path.clone())
            .save_to_path(&older.path)
            .expect("older session should save");
        std::thread::sleep(Duration::from_millis(20));
        let newer = create_managed_session_handle("session-newer").expect("newer handle");
        Session::new()
            .with_persistence_path(newer.path.clone())
            .save_to_path(&newer.path)
            .expect("newer session should save");

        let resolved = resolve_session_reference("latest").expect("latest session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            newer.path.canonicalize().expect("newer path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn unknown_slash_command_guidance_suggests_nearby_commands() {
        let message = format_unknown_slash_command("stats");
        assert!(message.contains("Unknown slash command: /stats"));
        assert!(message.contains("/status"));
        assert!(message.contains("/help"));
    }

    #[test]
    fn resume_usage_mentions_latest_shortcut() {
        let usage = render_resume_usage();
        assert!(usage.contains("/resume <session-path|session-id|latest>"));
        assert!(usage.contains(".kcode/sessions/<session-id>.jsonl"));
        assert!(usage.contains("/session list"));
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_workspace(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("claw-cli-{label}-{nanos}"))
    }

    #[test]
    fn init_template_mentions_detected_rust_workspace() {
        let workspace = temp_workspace("init-rust-workspace");
        std::fs::create_dir_all(workspace.join("rust")).expect("create rust dir");
        std::fs::write(workspace.join("rust").join("Cargo.toml"), "[workspace]\n")
            .expect("write workspace cargo");

        let rendered = crate::init::render_init_kcode_md(&workspace);
        assert!(rendered.contains("# KCODE.md"));
        assert!(rendered.contains("cargo clippy --workspace --all-targets -- -D warnings"));

        std::fs::remove_dir_all(workspace).expect("cleanup temp workspace");
    }

    #[test]
    fn converts_tool_roundtrip_messages() {
        let messages = vec![
            ConversationMessage::user_text("hello"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "bash".to_string(),
                input: "{\"command\":\"pwd\"}".to_string(),
            }]),
            ConversationMessage {
                role: MessageRole::Tool,
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "tool-1".to_string(),
                    tool_name: "bash".to_string(),
                    output: "ok".to_string(),
                    is_error: false,
                }],
                usage: None,
            },
        ];

        let converted = super::convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
    }
    #[test]
    fn repl_help_mentions_history_completion_and_multiline() {
        let help = render_repl_help();
        assert!(help.contains("Up/Down"));
        assert!(help.contains("Tab"));
        assert!(help.contains("Shift+Enter/Ctrl+J"));
    }

    #[test]
    fn tool_rendering_helpers_compact_output() {
        let start = format_tool_call_start("read_file", r#"{"path":"src/main.rs"}"#);
        assert!(start.contains("read_file"));
        assert!(start.contains("src/main.rs"));

        let done = format_tool_result(
            "read_file",
            r#"{"file":{"filePath":"src/main.rs","content":"hello","numLines":1,"startLine":1,"totalLines":1}}"#,
            false,
        );
        assert!(done.contains("📄 Read src/main.rs"));
        assert!(done.contains("hello"));
    }

    #[test]
    fn tool_rendering_truncates_large_read_output_for_display_only() {
        let content = (0..200)
            .map(|index| format!("line {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "file": {
                "filePath": "src/main.rs",
                "content": content,
                "numLines": 200,
                "startLine": 1,
                "totalLines": 200
            }
        })
        .to_string();

        let rendered = format_tool_result("read_file", &output, false);

        assert!(rendered.contains("line 000"));
        assert!(rendered.contains("line 079"));
        assert!(!rendered.contains("line 199"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("line 199"));
    }

    #[test]
    fn tool_rendering_truncates_large_bash_output_for_display_only() {
        let stdout = (0..120)
            .map(|index| format!("stdout {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "stdout": stdout,
            "stderr": "",
            "returnCodeInterpretation": "completed successfully"
        })
        .to_string();

        let rendered = format_tool_result("bash", &output, false);

        assert!(rendered.contains("stdout 000"));
        assert!(rendered.contains("stdout 059"));
        assert!(!rendered.contains("stdout 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("stdout 119"));
    }

    #[test]
    fn tool_rendering_truncates_generic_long_output_for_display_only() {
        let items = (0..120)
            .map(|index| format!("payload {index:03}"))
            .collect::<Vec<_>>();
        let output = json!({
            "summary": "plugin payload",
            "items": items,
        })
        .to_string();

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("payload 000"));
        assert!(rendered.contains("payload 040"));
        assert!(!rendered.contains("payload 080"));
        assert!(!rendered.contains("payload 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("payload 119"));
    }

    #[test]
    fn tool_rendering_truncates_raw_generic_output_for_display_only() {
        let output = (0..120)
            .map(|index| format!("raw {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("raw 000"));
        assert!(rendered.contains("raw 059"));
        assert!(!rendered.contains("raw 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("raw 119"));
    }

    #[test]
    fn ultraplan_progress_lines_include_phase_step_and_elapsed_status() {
        let snapshot = InternalPromptProgressState {
            command_label: "Ultraplan",
            task_label: "ship plugin progress".to_string(),
            step: 3,
            phase: "running read_file".to_string(),
            detail: Some("reading rust/crates/kcode-cli/src/main.rs".to_string()),
            saw_final_text: false,
        };

        let started = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Started,
            &snapshot,
            Duration::from_secs(0),
            None,
        );
        let heartbeat = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            Duration::from_secs(9),
            None,
        );
        let completed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Complete,
            &snapshot,
            Duration::from_secs(12),
            None,
        );
        let failed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Failed,
            &snapshot,
            Duration::from_secs(12),
            Some("network timeout"),
        );

        assert!(started.contains("planning started"));
        assert!(started.contains("current step 3"));
        assert!(heartbeat.contains("heartbeat"));
        assert!(heartbeat.contains("9s elapsed"));
        assert!(heartbeat.contains("phase running read_file"));
        assert!(completed.contains("completed"));
        assert!(completed.contains("3 steps total"));
        assert!(failed.contains("failed"));
        assert!(failed.contains("network timeout"));
    }

    #[test]
    fn describe_tool_progress_summarizes_known_tools() {
        assert_eq!(
            describe_tool_progress("read_file", r#"{"path":"src/main.rs"}"#),
            "reading src/main.rs"
        );
        assert!(
            describe_tool_progress("bash", r#"{"command":"cargo test -p kcode-cli"}"#)
                .contains("cargo test -p kcode-cli")
        );
        assert_eq!(
            describe_tool_progress("grep_search", r#"{"pattern":"ultraplan","path":"rust"}"#),
            "grep `ultraplan` in rust"
        );
    }

    #[test]
    fn push_output_block_renders_markdown_text() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;

        push_output_block(
            OutputContentBlock::Text {
                text: "# Heading".to_string(),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            false,
        )
        .expect("text block should render");

        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains('\u{1b}'));
    }

    #[test]
    fn push_output_block_skips_empty_object_prefix_for_tool_streams() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;

        push_output_block(
            OutputContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                input: json!({}),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            true,
        )
        .expect("tool block should accumulate");

        assert!(events.is_empty());
        assert_eq!(
            pending_tool,
            Some(("tool-1".to_string(), "read_file".to_string(), String::new(),))
        );
    }

    #[test]
    fn response_to_events_preserves_empty_object_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-1".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "read_file".to_string(),
                    input: json!({}),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{}"
        ));
    }

    #[test]
    fn response_to_events_preserves_non_empty_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-2".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-2".to_string(),
                    name: "read_file".to_string(),
                    input: json!({ "path": "rust/Cargo.toml" }),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{\"path\":\"rust/Cargo.toml\"}"
        ));
    }

    #[test]
    fn response_to_events_ignores_thinking_blocks() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-3".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![
                    OutputContentBlock::Thinking {
                        thinking: "step 1".to_string(),
                        signature: Some("sig_123".to_string()),
                    },
                    OutputContentBlock::Text {
                        text: "Final answer".to_string(),
                    },
                ],
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::TextDelta(text) if text == "Final answer"
        ));
        assert!(!String::from_utf8(out).expect("utf8").contains("step 1"));
    }

    #[test]
    fn build_runtime_plugin_state_merges_plugin_hooks_into_runtime_features() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "hook-runtime-demo", true, false);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, true)
                .expect("plugin state should load");
        let pre_hooks = state.feature_config.hooks().pre_tool_use();
        assert_eq!(pre_hooks.len(), 1);
        assert!(
            pre_hooks[0].ends_with("hooks/pre.sh"),
            "expected installed plugin hook path, got {pre_hooks:?}"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn build_runtime_plugin_state_strips_tool_runtime_for_toolless_profiles() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "hook-runtime-demo", true, false);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, false)
                .expect("plugin state should load");

        assert!(state.feature_config.hooks().pre_tool_use().is_empty());
        assert!(state.tool_registry.definitions(None).is_empty());
        assert!(state.plugin_registry.plugins().is_empty());

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn build_runtime_runs_plugin_lifecycle_init_and_shutdown() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "lifecycle-runtime-demo", false, true);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        let install = manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let log_path = install.install_path.join("lifecycle.log");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let runtime_plugin_state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, true)
                .expect("plugin state should load");
        let mut setup = test_setup_context(&workspace);
        setup.resolved_config.base_url = Some("https://router.example.test/v1".to_string());
        setup.resolved_config.api_key_present = true;
        setup.resolved_config.profile = Some("cliproxyapi".to_string());
        setup.active_profile.base_url = Some("https://router.example.test/v1".to_string());
        setup.active_profile.base_url_source = ResolutionSource::Env("KCODE_BASE_URL");
        setup.active_profile.credential = CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: "KCODE_API_KEY".to_string(),
            api_key: Some("test-dummy-key-for-plugin-lifecycle".to_string()),
        };
        let mut runtime = build_runtime_with_plugin_state(
            Session::new(),
            "runtime-plugin-lifecycle",
            DEFAULT_MODEL.to_string(),
            vec!["test system prompt".to_string()],
            true,
            false,
            None,
            PermissionMode::DangerFullAccess,
            None,
            &setup,
            runtime_plugin_state,
        )
        .expect("runtime should build");

        assert_eq!(
            fs::read_to_string(&log_path).expect("init log should exist"),
            "init\n"
        );

        runtime
            .shutdown_plugins()
            .expect("plugin shutdown should succeed");

        assert_eq!(
            fs::read_to_string(&log_path).expect("shutdown log should exist"),
            "init\nshutdown\n"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn build_runtime_skips_plugin_lifecycle_for_toolless_profiles() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "lifecycle-runtime-demo", false, true);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        let install = manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let log_path = install.install_path.join("lifecycle.log");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let runtime_plugin_state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, false)
                .expect("plugin state should load");
        let mut setup = test_setup_context(&workspace);
        setup.active_profile.profile.supports_tools = false;
        setup.resolved_config.base_url = Some("https://router.example.test/v1".to_string());
        setup.resolved_config.api_key_present = true;
        setup.resolved_config.profile = Some("bridge".to_string());
        setup.active_profile.base_url = Some("https://router.example.test/v1".to_string());
        setup.active_profile.base_url_source = ResolutionSource::Env("KCODE_BASE_URL");
        setup.active_profile.credential = CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: "KCODE_API_KEY".to_string(),
            api_key: Some("test-dummy-key-for-plugin-lifecycle".to_string()),
        };

        let mut runtime = build_runtime_with_plugin_state(
            Session::new(),
            "runtime-toolless-plugin-lifecycle",
            DEFAULT_MODEL.to_string(),
            vec!["test system prompt".to_string()],
            true,
            false,
            None,
            PermissionMode::DangerFullAccess,
            None,
            &setup,
            runtime_plugin_state,
        )
        .expect("runtime should build");

        assert!(!log_path.exists(), "plugin lifecycle should not run");

        runtime
            .shutdown_plugins()
            .expect("plugin shutdown should succeed");

        assert!(
            !log_path.exists(),
            "plugin shutdown should stay inactive for toolless profiles"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn provider_runtime_client_disables_tools_for_toolless_profiles() {
        let workspace = temp_dir();
        fs::create_dir_all(&workspace).expect("workspace");

        let mut setup = test_setup_context(&workspace);
        setup.active_profile.profile.supports_tools = false;
        setup.active_profile.base_url = Some("https://router.example.test/v1".to_string());
        setup.active_profile.base_url_source = ResolutionSource::Env("KCODE_BASE_URL");
        setup.active_profile.credential = CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: "KCODE_API_KEY".to_string(),
            api_key: Some("test-dummy-key-for-runtime".to_string()),
        };

        let client = ProviderRuntimeClient::new(
            "runtime-toolless-profile",
            DEFAULT_MODEL.to_string(),
            true,
            false,
            None,
            registry_with_plugin_tool(),
            None,
            &setup,
        )
        .expect("runtime client should build");

        assert!(!client.enable_tools);

        let _ = fs::remove_dir_all(workspace);
    }
}

#[cfg(test)]
mod sandbox_report_tests {
    use super::{format_sandbox_report, HookAbortMonitor};
    use runtime::HookAbortSignal;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn sandbox_report_renders_expected_fields() {
        let report = format_sandbox_report(&runtime::SandboxStatus::default());
        assert!(report.contains("Sandbox"));
        assert!(report.contains("Enabled"));
        assert!(report.contains("Filesystem mode"));
        assert!(report.contains("Fallback reason"));
    }

    #[test]
    fn hook_abort_monitor_stops_without_aborting() {
        let abort_signal = HookAbortSignal::new();
        let (ready_tx, ready_rx) = mpsc::channel();
        let monitor = HookAbortMonitor::spawn_with_waiter(
            abort_signal.clone(),
            move |stop_rx, abort_signal| {
                ready_tx.send(()).expect("ready signal");
                let _ = stop_rx.recv();
                assert!(!abort_signal.is_aborted());
            },
        );

        ready_rx.recv().expect("waiter should be ready");
        monitor.stop();

        assert!(!abort_signal.is_aborted());
    }

    #[test]
    fn hook_abort_monitor_propagates_interrupt() {
        let abort_signal = HookAbortSignal::new();
        let (done_tx, done_rx) = mpsc::channel();
        let monitor = HookAbortMonitor::spawn_with_waiter(
            abort_signal.clone(),
            move |_stop_rx, abort_signal| {
                abort_signal.abort();
                done_tx.send(()).expect("done signal");
            },
        );

        done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("interrupt should complete");
        monitor.stop();

        assert!(abort_signal.is_aborted());
    }
}
