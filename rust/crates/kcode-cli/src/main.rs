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
    AuthSource, ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest,
    MessageResponse, OpenAiCompatClient, OpenAiCompatConfig, OutputContentBlock,
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
    ensure_memory_index, generate_pkce_pair, generate_state, list_memories, load_system_prompt,
    parse_oauth_callback_request_target, render_memory_summary, resolve_sandbox_status,
    save_oauth_credentials, ApiClient, ApiRequest, AssistantEvent, BootstrapInputs,
    CompactionConfig, ConfigLoader, ConfigSource, ContentBlock, ConversationMessage,
    ConversationRuntime, DiagnosticCheck, DiagnosticStatus, MemoryType, MessageRole,
    OAuthAuthorizationRequest, OAuthConfig, OAuthTokenExchangeRequest, PermissionMode,
    PermissionPolicy, ProfileResolver, ProjectContext, PromptCacheEvent, ProviderLaunchConfig,
    ProviderLauncher, ProviderProfile, ProviderProfileError, ResolutionSource, ResolvedConfig,
    ResolvedPermissionMode, ResolvedProviderProfile, RuntimeError, Session, SetupContext,
    SetupMode, StdioMode, TokenUsage, ToolError, ToolExecutor, TrustPolicyContext, UsageTracker,
    MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES,
};
use serde_json::json;
use tools::GlobalToolRegistry;

// v1.1 Bridge Modules
mod bridge_core;
use bridge_core::{BridgeCore, BridgeMessage, SessionConfig};

use adapters::{TelegramConfig, TelegramMode, TelegramTransport};

const DEFAULT_MODEL: &str = "gpt-4.1";
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

include!("main_parts/cli_action.rs");
include!("main_parts/cli_parse.rs");
include!("main_parts/cli_parse_support.rs");

include!("main_parts/main_entry.rs");
include!("main_parts/status_basics.rs");
include!("main_parts/setup_git.rs");
include!("main_parts/resume_command.rs");
include!("main_parts/bridge_repl_types.rs");
include!("main_parts/hook_abort.rs");

include!("main_parts/live_cli_core.rs");
include!("main_parts/live_cli_repl_command.rs");
include!("main_parts/live_cli_session_state.rs");
include!("main_parts/live_cli_session_ops.rs");
include!("main_parts/live_cli_misc_actions.rs");

include!("main_parts/sessions.rs");
include!("main_parts/status_reports.rs");
include!("main_parts/doctor_fix_and_command_availability.rs");
include!("main_parts/command_doctor_profile_reports.rs");
include!("main_parts/profile_config_memory.rs");
include!("main_parts/init_diff_utils.rs");
include!("main_parts/plugin_runtime_state.rs");
include!("main_parts/runtime_builder.rs");
include!("main_parts/provider_runtime_client_impl.rs");
include!("main_parts/tool_summary_helpers.rs");
include!("main_parts/tool_formatting.rs");
include!("main_parts/tool_response.rs");
include!("main_parts/tool_executor_help.rs");

#[cfg(test)]
mod tests {
    include!("main_parts/tests_support.rs");
    include!("main_parts/tests_args_basic.rs");
    include!("main_parts/tests_args_commands.rs");
    include!("main_parts/tests_help_permissions.rs");
    include!("main_parts/tests_reports_status.rs");
    include!("main_parts/tests_render_runtime.rs");
    include!("main_parts/tests_plugin_runtime.rs");
}

#[cfg(test)]
mod sandbox_report_tests {
    include!("main_parts/tests_sandbox_report.rs");
}
