mod agent_runtime;
mod agent_spawn;
mod brief;
mod config;
mod core_specs;
mod dispatch;
mod extended_specs;
mod manifest;
mod notebook;
mod plan_mode;
mod registry;
mod repl;
mod shell;
mod specs;
mod todo_skill;
mod tool_contract;
pub mod tool_filter;
mod types;
mod web;

pub use dispatch::execute_tool;
pub use manifest::{ToolManifestEntry, ToolRegistry, ToolSource, ToolSpec};
pub use registry::GlobalToolRegistry;
pub use specs::mvp_tool_specs;
pub use tool_contract::{ToolPoolAssembler, ToolUseContext};
pub use tool_filter::{apply_deny_rules, ToolDenyRule};

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_agent;
#[cfg(test)]
mod tests_notebook_fs;
#[cfg(test)]
mod tests_registry;
#[cfg(test)]
mod tests_runtime;
#[cfg(test)]
mod tests_subagent;
#[cfg(test)]
mod tests_web;
