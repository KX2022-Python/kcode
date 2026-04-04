use std::fs;

use plugins::{PluginKind, PluginMetadata, PluginSummary};
use runtime::ConfigLoader;

use crate::test_support::{temp_dir, write_agent, write_legacy_command, write_skill};
use crate::{
    handle_agents_slash_command, handle_mcp_slash_command, handle_skills_slash_command,
    load_agents_from_roots, load_skills_from_roots, parse_skill_frontmatter, render_agents_report,
    render_mcp_report_for, render_plugins_report, render_skills_report, DefinitionSource,
    SkillOrigin, SkillRoot,
};

#[test]
fn renders_plugins_report_with_name_version_and_status() {
    let rendered = render_plugins_report(&[
        PluginSummary {
            metadata: PluginMetadata {
                id: "demo@external".to_string(),
                name: "demo".to_string(),
                version: "1.2.3".to_string(),
                description: "demo plugin".to_string(),
                kind: PluginKind::External,
                source: "demo".to_string(),
                default_enabled: false,
                root: None,
            },
            enabled: true,
        },
        PluginSummary {
            metadata: PluginMetadata {
                id: "sample@external".to_string(),
                name: "sample".to_string(),
                version: "0.9.0".to_string(),
                description: "sample plugin".to_string(),
                kind: PluginKind::External,
                source: "sample".to_string(),
                default_enabled: false,
                root: None,
            },
            enabled: false,
        },
    ]);

    assert!(rendered.contains("demo"));
    assert!(rendered.contains("v1.2.3"));
    assert!(rendered.contains("enabled"));
    assert!(rendered.contains("sample"));
    assert!(rendered.contains("v0.9.0"));
    assert!(rendered.contains("disabled"));
}

#[test]
fn lists_agents_from_project_and_user_roots() {
    let workspace = temp_dir("agents-workspace");
    let project_agents = workspace.join(".codex").join("agents");
    let user_home = temp_dir("agents-home");
    let user_agents = user_home.join(".codex").join("agents");

    write_agent(
        &project_agents,
        "planner",
        "Project planner",
        "gpt-5.4",
        "medium",
    );
    write_agent(
        &user_agents,
        "planner",
        "User planner",
        "gpt-5.4-mini",
        "high",
    );
    write_agent(
        &user_agents,
        "verifier",
        "Verification agent",
        "gpt-5.4-mini",
        "high",
    );

    let roots = vec![
        (DefinitionSource::ProjectCodex, project_agents),
        (DefinitionSource::UserCodex, user_agents),
    ];
    let report =
        render_agents_report(&load_agents_from_roots(&roots).expect("agent roots should load"));

    assert!(report.contains("2 active agents"));
    assert!(report.contains("planner · Project planner · gpt-5.4 · medium"));
    assert!(report.contains("(shadowed by Project (.codex)) planner · User planner"));
    assert!(report.contains("verifier · Verification agent · gpt-5.4-mini · high"));

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn lists_skills_from_project_and_user_roots() {
    let workspace = temp_dir("skills-workspace");
    let project_skills = workspace.join(".codex").join("skills");
    let project_commands = workspace.join(".claude").join("commands");
    let user_home = temp_dir("skills-home");
    let user_skills = user_home.join(".codex").join("skills");

    write_skill(&project_skills, "plan", "Project planning guidance");
    write_legacy_command(&project_commands, "deploy", "Legacy deployment guidance");
    write_skill(&user_skills, "plan", "User planning guidance");
    write_skill(&user_skills, "help", "Help guidance");

    let roots = vec![
        SkillRoot {
            source: DefinitionSource::ProjectCodex,
            path: project_skills,
            origin: SkillOrigin::SkillsDir,
        },
        SkillRoot {
            source: DefinitionSource::ProjectClaude,
            path: project_commands,
            origin: SkillOrigin::LegacyCommandsDir,
        },
        SkillRoot {
            source: DefinitionSource::UserCodex,
            path: user_skills,
            origin: SkillOrigin::SkillsDir,
        },
    ];
    let report =
        render_skills_report(&load_skills_from_roots(&roots).expect("skill roots should load"));

    assert!(report.contains("3 available skills"));
    assert!(report.contains("plan · Project planning guidance"));
    assert!(report.contains("deploy · Legacy deployment guidance · legacy /commands"));
    assert!(report.contains("(shadowed by Project (.codex)) plan · User planning guidance"));
    assert!(report.contains("help · Help guidance"));

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn usage_commands_and_mcp_reports_render_expected_text() {
    let cwd = temp_dir("slash-usage");
    let agents_help = handle_agents_slash_command(Some("help"), &cwd).expect("agents help");
    let agents_unexpected =
        handle_agents_slash_command(Some("show planner"), &cwd).expect("agents usage");
    let skills_help = handle_skills_slash_command(Some("--help"), &cwd).expect("skills help");
    let skills_unexpected =
        handle_skills_slash_command(Some("show help"), &cwd).expect("skills usage");
    let mcp_help = handle_mcp_slash_command(Some("help"), &cwd).expect("mcp help");
    let mcp_unexpected =
        handle_mcp_slash_command(Some("show alpha beta"), &cwd).expect("mcp usage");

    assert!(agents_help.contains("Usage            /agents [list|help]"));
    assert!(agents_unexpected.contains("Unexpected       show planner"));
    assert!(skills_help.contains("Usage            /skills [list|install <path>|help]"));
    assert!(skills_unexpected.contains("Unexpected       show help"));
    assert!(mcp_help.contains("Usage            /mcp [list|show <server>|help]"));
    assert!(mcp_unexpected.contains("Unexpected       show alpha beta"));

    let _ = fs::remove_dir_all(cwd);
}

#[test]
fn renders_mcp_reports_from_loaded_config() {
    let workspace = temp_dir("mcp-config-workspace");
    let config_home = temp_dir("mcp-config-home");
    fs::create_dir_all(workspace.join(".claw")).expect("workspace config dir");
    fs::create_dir_all(&config_home).expect("config home");
    fs::write(
        workspace.join(".claw").join("settings.json"),
        r#"{
          "mcpServers": {
            "alpha": {
              "command": "uvx",
              "args": ["alpha-server"],
              "env": {"ALPHA_TOKEN": "secret"},
              "toolCallTimeoutMs": 1200
            },
            "remote": {
              "type": "http",
              "url": "https://remote.example/mcp",
              "headers": {"Authorization": "Bearer secret"},
              "headersHelper": "./bin/headers",
              "oauth": {"clientId": "remote-client", "callbackPort": 7878}
            }
          }
        }"#,
    )
    .expect("write settings");
    fs::write(
        workspace.join(".claw").join("settings.local.json"),
        r#"{"mcpServers":{"remote":{"type":"ws","url":"wss://remote.example/mcp"}}}"#,
    )
    .expect("write local settings");

    let loader = ConfigLoader::new(&workspace, &config_home);
    let list = render_mcp_report_for(&loader, &workspace, None).expect("mcp list report");
    let show = render_mcp_report_for(&loader, &workspace, Some("show alpha")).expect("mcp show");
    let remote =
        render_mcp_report_for(&loader, &workspace, Some("show remote")).expect("mcp show remote");
    let missing =
        render_mcp_report_for(&loader, &workspace, Some("show missing")).expect("missing report");

    assert!(list.contains("Active servers ("));
    assert!(list.contains("alpha-server"));
    assert!(list.contains("remote.example"));
    assert!(show.contains("Env keys          ALPHA_TOKEN"));
    assert!(show.contains("Tool timeout      1200 ms"));
    assert!(remote.contains("Transport         ws"));
    assert!(remote.contains("URL               wss://remote.example/mcp"));
    assert!(missing.contains("server `missing` is not configured"));

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(config_home);
}

#[test]
fn parses_quoted_skill_frontmatter_values() {
    let (name, description) =
        parse_skill_frontmatter("---\nname: \"hud\"\ndescription: 'Quoted description'\n---\n");
    assert_eq!(name.as_deref(), Some("hud"));
    assert_eq!(description.as_deref(), Some("Quoted description"));
}
