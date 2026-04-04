mod catalog_discovery;
mod catalog_handlers;
mod catalog_install;
mod catalog_render;
mod help;
mod model;
mod parse;
mod parse_support;
mod process_specs;
mod registry;
mod session_commands;
mod session_specs;
mod slash;
mod specs;
mod types;

pub use catalog_handlers::{
    handle_agents_slash_command, handle_mcp_slash_command, handle_plugins_slash_command,
    handle_skills_slash_command,
};
pub use help::resume_supported_slash_commands;
pub use help::{
    render_slash_command_help, render_slash_command_help_detail,
    render_slash_command_help_for_context, suggest_slash_commands,
};
pub use model::{
    slash_command_specs, CommandAvailability, CommandDescriptor, CommandKind, CommandManifestEntry,
    CommandRegistry, CommandRegistryContext, CommandRegistrySnapshot, CommandScope, CommandSource,
    CommandSurface, FilteredCommand, PluginsCommandResult, SlashCommand, SlashCommandParseError,
    SlashCommandResult, SlashCommandSpec,
};
pub use parse::validate_slash_command_input;
pub use registry::{
    build_command_registry_snapshot, build_command_registry_snapshot_with_cwd, v1_command_manifest,
};
pub use session_commands::handle_slash_command;

#[cfg(test)]
pub(crate) use catalog_discovery::{
    load_agents_from_roots, load_skills_from_roots, DefinitionSource, SkillOrigin, SkillRoot,
};
#[cfg(test)]
pub(crate) use catalog_handlers::render_mcp_report_for;
#[cfg(test)]
pub(crate) use catalog_install::{install_skill_into, parse_skill_frontmatter};
#[cfg(test)]
pub(crate) use catalog_render::{
    render_agents_report, render_plugins_report, render_skill_install_report, render_skills_report,
};

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_discovery;
#[cfg(test)]
mod tests_install;
#[cfg(test)]
mod tests_parse;
#[cfg(test)]
mod tests_registry;
#[cfg(test)]
mod tests_session;
