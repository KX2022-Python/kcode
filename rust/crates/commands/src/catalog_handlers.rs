use std::path::Path;

use plugins::{PluginError, PluginManager, PluginSummary};
use runtime::{
    ConfigLoader, McpRegistryAssembler, McpRegistrySnapshot, McpServerConfig, ScopedMcpServerConfig,
};

use crate::catalog_discovery::{
    discover_definition_roots, discover_skill_roots, load_agents_from_roots, load_skills_from_roots,
};
use crate::catalog_install::install_skill;
use crate::catalog_render::{
    render_agents_report, render_agents_usage, render_mcp_server_report, render_mcp_usage,
    render_plugin_install_report, render_plugins_report, render_skill_install_report,
    render_skills_report, render_skills_usage,
};
use crate::model::PluginsCommandResult;
use crate::parse_support::normalize_optional_args;

#[allow(clippy::too_many_lines)]
pub fn handle_plugins_slash_command(
    action: Option<&str>,
    target: Option<&str>,
    manager: &mut PluginManager,
) -> Result<PluginsCommandResult, PluginError> {
    match action {
        None | Some("list") => Ok(PluginsCommandResult {
            message: render_plugins_report(&manager.list_installed_plugins()?),
            reload_runtime: false,
        }),
        Some("install") => {
            let Some(target) = target else {
                return Ok(PluginsCommandResult {
                    message: "Usage: /plugins install <path>".to_string(),
                    reload_runtime: false,
                });
            };
            let install = manager.install(target)?;
            let plugin = manager
                .list_installed_plugins()?
                .into_iter()
                .find(|plugin| plugin.metadata.id == install.plugin_id);
            Ok(PluginsCommandResult {
                message: render_plugin_install_report(&install.plugin_id, plugin.as_ref()),
                reload_runtime: true,
            })
        }
        Some("enable") => set_plugin_enabled(manager, target, true),
        Some("disable") => set_plugin_enabled(manager, target, false),
        Some("uninstall") => mutate_plugin(
            target,
            "Usage: /plugins uninstall <plugin-id>",
            |manager, target| {
                manager.uninstall(target)?;
                Ok(format!("Plugins\n  Result           uninstalled {target}"))
            },
            manager,
        ),
        Some("update") => mutate_plugin(
            target,
            "Usage: /plugins update <plugin-id>",
            |manager, target| {
                let update = manager.update(target)?;
                let plugin = manager
                    .list_installed_plugins()?
                    .into_iter()
                    .find(|plugin| plugin.metadata.id == update.plugin_id);
                Ok(format!(
                    "Plugins\n  Result           updated {}\n  Name             {}\n  Old version      {}\n  New version      {}\n  Status           {}",
                    update.plugin_id,
                    plugin
                        .as_ref()
                        .map_or_else(|| update.plugin_id.clone(), |plugin| plugin.metadata.name.clone()),
                    update.old_version,
                    update.new_version,
                    plugin
                        .as_ref()
                        .map_or("unknown", |plugin| if plugin.enabled { "enabled" } else { "disabled" }),
                ))
            },
            manager,
        ),
        Some(other) => Ok(PluginsCommandResult {
            message: format!(
                "Unknown /plugins action '{other}'. Use list, install, enable, disable, uninstall, or update."
            ),
            reload_runtime: false,
        }),
    }
}

pub fn handle_agents_slash_command(args: Option<&str>, cwd: &Path) -> std::io::Result<String> {
    match normalize_optional_args(args) {
        None | Some("list") => {
            let roots = discover_definition_roots(cwd, "agents");
            Ok(render_agents_report(&load_agents_from_roots(&roots)?))
        }
        Some("-h" | "--help" | "help") => Ok(render_agents_usage(None)),
        Some(args) => Ok(render_agents_usage(Some(args))),
    }
}

pub fn handle_mcp_slash_command(
    args: Option<&str>,
    cwd: &Path,
) -> Result<String, runtime::ConfigError> {
    render_mcp_report_for(&ConfigLoader::default_for(cwd), cwd, args)
}

pub fn handle_skills_slash_command(args: Option<&str>, cwd: &Path) -> std::io::Result<String> {
    match normalize_optional_args(args) {
        None | Some("list") => {
            let roots = discover_skill_roots(cwd);
            Ok(render_skills_report(&load_skills_from_roots(&roots)?))
        }
        Some("install") => Ok(render_skills_usage(Some("install"))),
        Some(args) if args.starts_with("install ") => {
            let target = args["install ".len()..].trim();
            if target.is_empty() {
                return Ok(render_skills_usage(Some("install")));
            }
            Ok(render_skill_install_report(&install_skill(target, cwd)?))
        }
        Some("-h" | "--help" | "help") => Ok(render_skills_usage(None)),
        Some(args) => Ok(render_skills_usage(Some(args))),
    }
}

pub(crate) fn render_mcp_report_for(
    loader: &ConfigLoader,
    cwd: &Path,
    args: Option<&str>,
) -> Result<String, runtime::ConfigError> {
    match normalize_optional_args(args) {
        None | Some("list") => {
            Ok(build_mcp_snapshot_from_config(loader.load()?.mcp()).render_summary())
        }
        Some("-h" | "--help" | "help") => Ok(render_mcp_usage(None)),
        Some("show") => Ok(render_mcp_usage(Some("show"))),
        Some(args) if args.split_whitespace().next() == Some("show") => {
            let mut parts = args.split_whitespace();
            let _ = parts.next();
            let Some(server_name) = parts.next() else {
                return Ok(render_mcp_usage(Some("show")));
            };
            if parts.next().is_some() {
                return Ok(render_mcp_usage(Some(args)));
            }
            let runtime_config = loader.load()?;
            Ok(render_mcp_server_report(
                cwd,
                server_name,
                runtime_config.mcp().get(server_name),
            ))
        }
        Some(args) => Ok(render_mcp_usage(Some(args))),
    }
}

fn build_mcp_snapshot_from_config(
    mcp_config: &runtime::McpConfigCollection,
) -> McpRegistrySnapshot {
    let mut assembler = McpRegistryAssembler::new();
    let servers = mcp_config
        .servers()
        .iter()
        .map(|(_name, scoped)| ScopedMcpServerConfig {
            scope: scoped.scope,
            config: match &scoped.config {
                McpServerConfig::Stdio(s) => McpServerConfig::Stdio(s.clone()),
                McpServerConfig::Sse(s) => McpServerConfig::Sse(s.clone()),
                McpServerConfig::Http(s) => McpServerConfig::Http(s.clone()),
                McpServerConfig::Ws(s) => McpServerConfig::Ws(s.clone()),
                McpServerConfig::Sdk(s) => McpServerConfig::Sdk(s.clone()),
                McpServerConfig::ManagedProxy(s) => McpServerConfig::ManagedProxy(s.clone()),
            },
        })
        .collect::<Vec<_>>();

    if !servers.is_empty() {
        assembler.add_source(runtime::ConfigSource::User, servers);
    }
    assembler.assemble()
}

fn set_plugin_enabled(
    manager: &mut PluginManager,
    target: Option<&str>,
    enabled: bool,
) -> Result<PluginsCommandResult, PluginError> {
    let usage = if enabled {
        "Usage: /plugins enable <name>"
    } else {
        "Usage: /plugins disable <name>"
    };
    mutate_plugin(
        target,
        usage,
        |manager, target| {
            let plugin = resolve_plugin_target(manager, target)?;
            if enabled {
                manager.enable(&plugin.metadata.id)?;
            } else {
                manager.disable(&plugin.metadata.id)?;
            }
            Ok(format!(
                "Plugins\n  Result           {} {}\n  Name             {}\n  Version          {}\n  Status           {}",
                if enabled { "enabled" } else { "disabled" },
                plugin.metadata.id,
                plugin.metadata.name,
                plugin.metadata.version,
                if enabled { "enabled" } else { "disabled" },
            ))
        },
        manager,
    )
}

fn mutate_plugin<F>(
    target: Option<&str>,
    usage: &str,
    op: F,
    manager: &mut PluginManager,
) -> Result<PluginsCommandResult, PluginError>
where
    F: FnOnce(&mut PluginManager, &str) -> Result<String, PluginError>,
{
    let Some(target) = target else {
        return Ok(PluginsCommandResult {
            message: usage.to_string(),
            reload_runtime: false,
        });
    };
    Ok(PluginsCommandResult {
        message: op(manager, target)?,
        reload_runtime: true,
    })
}

fn resolve_plugin_target(
    manager: &PluginManager,
    target: &str,
) -> Result<PluginSummary, PluginError> {
    let mut matches = manager
        .list_installed_plugins()?
        .into_iter()
        .filter(|plugin| plugin.metadata.id == target || plugin.metadata.name == target)
        .collect::<Vec<_>>();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(PluginError::NotFound(format!(
            "plugin `{target}` is not installed or discoverable"
        ))),
        _ => Err(PluginError::InvalidManifest(format!(
            "plugin name `{target}` is ambiguous; use the full plugin id"
        ))),
    }
}
