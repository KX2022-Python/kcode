use std::path::Path;

use plugins::PluginSummary;
use runtime::{ConfigSource, McpOAuthConfig, McpServerConfig, ScopedMcpServerConfig};

use crate::catalog_discovery::{AgentSummary, DefinitionSource, InstalledSkill, SkillSummary};

pub(crate) fn render_plugins_report(plugins: &[PluginSummary]) -> String {
    let mut lines = vec!["Plugins".to_string()];
    if plugins.is_empty() {
        lines.push("  No plugins installed.".to_string());
        return lines.join("\n");
    }
    for plugin in plugins {
        let enabled = if plugin.enabled {
            "enabled"
        } else {
            "disabled"
        };
        lines.push(format!(
            "  {name:<20} v{version:<10} {enabled}",
            name = plugin.metadata.name,
            version = plugin.metadata.version,
        ));
    }
    lines.join("\n")
}

pub(crate) fn render_plugin_install_report(
    plugin_id: &str,
    plugin: Option<&PluginSummary>,
) -> String {
    let name = plugin.map_or(plugin_id, |plugin| plugin.metadata.name.as_str());
    let version = plugin.map_or("unknown", |plugin| plugin.metadata.version.as_str());
    let enabled = plugin.is_some_and(|plugin| plugin.enabled);
    format!(
        "Plugins\n  Result           installed {plugin_id}\n  Name             {name}\n  Version          {version}\n  Status           {}",
        if enabled { "enabled" } else { "disabled" }
    )
}

pub(crate) fn render_agents_report(agents: &[AgentSummary]) -> String {
    if agents.is_empty() {
        return "No agents found.".to_string();
    }

    let total_active = agents
        .iter()
        .filter(|agent| agent.shadowed_by.is_none())
        .count();
    let mut lines = vec![
        "Agents".to_string(),
        format!("  {total_active} active agents"),
        String::new(),
    ];

    for source in [
        DefinitionSource::ProjectCodex,
        DefinitionSource::ProjectClaude,
        DefinitionSource::UserCodexHome,
        DefinitionSource::UserCodex,
        DefinitionSource::UserClaude,
    ] {
        let group = agents
            .iter()
            .filter(|agent| agent.source == source)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }

        lines.push(format!("{}:", source.label()));
        for agent in group {
            let detail = agent_detail(agent);
            match agent.shadowed_by {
                Some(winner) => lines.push(format!("  (shadowed by {}) {detail}", winner.label())),
                None => lines.push(format!("  {detail}")),
            }
        }
        lines.push(String::new());
    }

    lines.join("\n").trim_end().to_string()
}

fn agent_detail(agent: &AgentSummary) -> String {
    let mut parts = vec![agent.name.clone()];
    if let Some(description) = &agent.description {
        parts.push(description.clone());
    }
    if let Some(model) = &agent.model {
        parts.push(model.clone());
    }
    if let Some(reasoning) = &agent.reasoning_effort {
        parts.push(reasoning.clone());
    }
    parts.join(" · ")
}

pub(crate) fn render_skills_report(skills: &[SkillSummary]) -> String {
    if skills.is_empty() {
        return "No skills found.".to_string();
    }

    let total_active = skills
        .iter()
        .filter(|skill| skill.shadowed_by.is_none())
        .count();
    let mut lines = vec![
        "Skills".to_string(),
        format!("  {total_active} available skills"),
        String::new(),
    ];

    for source in [
        DefinitionSource::ProjectCodex,
        DefinitionSource::ProjectClaude,
        DefinitionSource::UserCodexHome,
        DefinitionSource::UserCodex,
        DefinitionSource::UserClaude,
    ] {
        let group = skills
            .iter()
            .filter(|skill| skill.source == source)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }

        lines.push(format!("{}:", source.label()));
        for skill in group {
            let mut parts = vec![skill.name.clone()];
            if let Some(description) = &skill.description {
                parts.push(description.clone());
            }
            if let Some(detail) = skill.origin.detail_label() {
                parts.push(detail.to_string());
            }
            let detail = parts.join(" · ");
            match skill.shadowed_by {
                Some(winner) => lines.push(format!("  (shadowed by {}) {detail}", winner.label())),
                None => lines.push(format!("  {detail}")),
            }
        }
        lines.push(String::new());
    }

    lines.join("\n").trim_end().to_string()
}

pub(crate) fn render_skill_install_report(skill: &InstalledSkill) -> String {
    let mut lines = vec![
        "Skills".to_string(),
        format!("  Result           installed {}", skill.invocation_name),
        format!("  Invoke as        ${}", skill.invocation_name),
    ];
    if let Some(display_name) = &skill.display_name {
        lines.push(format!("  Display name     {display_name}"));
    }
    lines.push(format!("  Source           {}", skill.source.display()));
    lines.push(format!(
        "  Registry         {}",
        skill.registry_root.display()
    ));
    lines.push(format!(
        "  Installed path   {}",
        skill.installed_path.display()
    ));
    lines.join("\n")
}

pub(crate) fn render_mcp_server_report(
    cwd: &Path,
    server_name: &str,
    server: Option<&ScopedMcpServerConfig>,
) -> String {
    let Some(server) = server else {
        return format!(
            "MCP\n  Working directory {}\n  Result            server `{server_name}` is not configured",
            cwd.display()
        );
    };

    let mut lines = vec![
        "MCP".to_string(),
        format!("  Working directory {}", cwd.display()),
        format!("  Name              {server_name}"),
        format!("  Scope             {}", config_source_label(server.scope)),
        format!(
            "  Transport         {}",
            mcp_transport_label(&server.config)
        ),
    ];

    match &server.config {
        McpServerConfig::Stdio(config) => {
            lines.push(format!("  Command           {}", config.command));
            lines.push(format!(
                "  Args              {}",
                format_optional_list(&config.args)
            ));
            lines.push(format!(
                "  Env keys          {}",
                format_optional_keys(config.env.keys().cloned().collect())
            ));
            lines.push(format!(
                "  Tool timeout      {}",
                config
                    .tool_call_timeout_ms
                    .map_or_else(|| "<default>".to_string(), |value| format!("{value} ms"))
            ));
        }
        McpServerConfig::Sse(config) | McpServerConfig::Http(config) => {
            lines.push(format!("  URL               {}", config.url));
            lines.push(format!(
                "  Header keys       {}",
                format_optional_keys(config.headers.keys().cloned().collect())
            ));
            lines.push(format!(
                "  Header helper     {}",
                config.headers_helper.as_deref().unwrap_or("<none>")
            ));
            lines.push(format!(
                "  OAuth             {}",
                format_mcp_oauth(config.oauth.as_ref())
            ));
        }
        McpServerConfig::Ws(config) => {
            lines.push(format!("  URL               {}", config.url));
            lines.push(format!(
                "  Header keys       {}",
                format_optional_keys(config.headers.keys().cloned().collect())
            ));
            lines.push(format!(
                "  Header helper     {}",
                config.headers_helper.as_deref().unwrap_or("<none>")
            ));
        }
        McpServerConfig::Sdk(config) => lines.push(format!("  SDK name          {}", config.name)),
        McpServerConfig::ManagedProxy(config) => {
            lines.push(format!("  URL               {}", config.url));
            lines.push(format!("  Proxy id          {}", config.id));
        }
    }

    lines.join("\n")
}

pub(crate) fn render_agents_usage(unexpected: Option<&str>) -> String {
    let mut lines = vec![
        "Agents".to_string(),
        "  Usage            /agents [list|help]".to_string(),
        "  Direct CLI       kcode agents".to_string(),
        "  Sources          .codex/agents, .claude/agents, $CODEX_HOME/agents".to_string(),
    ];
    if let Some(args) = unexpected {
        lines.push(format!("  Unexpected       {args}"));
    }
    lines.join("\n")
}

pub(crate) fn render_skills_usage(unexpected: Option<&str>) -> String {
    let mut lines = vec![
        "Skills".to_string(),
        "  Usage            /skills [list|install <path>|help]".to_string(),
        "  Direct CLI       kcode skills [list|install <path>|help]".to_string(),
        "  Install root     $CODEX_HOME/skills or ~/.codex/skills".to_string(),
        "  Sources          .codex/skills, .claude/skills, legacy /commands".to_string(),
    ];
    if let Some(args) = unexpected {
        lines.push(format!("  Unexpected       {args}"));
    }
    lines.join("\n")
}

pub(crate) fn render_mcp_usage(unexpected: Option<&str>) -> String {
    let mut lines = vec![
        "MCP".to_string(),
        "  Usage            /mcp [list|show <server>|help]".to_string(),
        "  Direct CLI       kcode mcp [list|show <server>|help]".to_string(),
        "  Sources          .kcode/settings.json, .kcode/settings.local.json".to_string(),
    ];
    if let Some(args) = unexpected {
        lines.push(format!("  Unexpected       {args}"));
    }
    lines.join("\n")
}

fn config_source_label(source: ConfigSource) -> &'static str {
    match source {
        ConfigSource::User => "user",
        ConfigSource::Project => "project",
        ConfigSource::Local => "local",
        ConfigSource::Managed => "managed",
    }
}

fn mcp_transport_label(config: &McpServerConfig) -> &'static str {
    match config {
        McpServerConfig::Stdio(_) => "stdio",
        McpServerConfig::Sse(_) => "sse",
        McpServerConfig::Http(_) => "http",
        McpServerConfig::Ws(_) => "ws",
        McpServerConfig::Sdk(_) => "sdk",
        McpServerConfig::ManagedProxy(_) => "managed-proxy",
    }
}

fn format_optional_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(" ")
    }
}

fn format_optional_keys(mut keys: Vec<String>) -> String {
    if keys.is_empty() {
        return "<none>".to_string();
    }
    keys.sort();
    keys.join(", ")
}

fn format_mcp_oauth(oauth: Option<&McpOAuthConfig>) -> String {
    let Some(oauth) = oauth else {
        return "<none>".to_string();
    };

    let mut parts = Vec::new();
    if let Some(client_id) = &oauth.client_id {
        parts.push(format!("client_id={client_id}"));
    }
    if let Some(port) = oauth.callback_port {
        parts.push(format!("callback_port={port}"));
    }
    if let Some(url) = &oauth.auth_server_metadata_url {
        parts.push(format!("metadata_url={url}"));
    }
    if let Some(xaa) = oauth.xaa {
        parts.push(format!("xaa={xaa}"));
    }
    if parts.is_empty() {
        "enabled".to_string()
    } else {
        parts.join(", ")
    }
}
