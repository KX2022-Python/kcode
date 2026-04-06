use crate::{
    build_command_registry_snapshot, render_slash_command_help, render_slash_command_help_detail,
    render_slash_command_help_for_context, slash_command_specs, suggest_slash_commands,
    v1_command_manifest, CommandRegistryContext, CommandSurface,
};

#[test]
fn renders_help_from_shared_specs() {
    let help = render_slash_command_help();
    assert!(help.contains("Start here        /doctor, /config, /status, /mcp, /memory"));
    assert!(help.contains("[resume]          also works with --resume SESSION.jsonl"));
    assert!(help.contains("Session & visibility"));
    assert!(help.contains("Workspace & git"));
    assert!(help.contains("Discovery & debugging"));
    assert!(help.contains("Core v1.0 manifest"));
    assert!(help.contains("Extended local surface"));
    assert!(help.contains("/skills [list|install <path>|help]"));
    assert!(slash_command_specs()
        .iter()
        .any(|spec| spec.name == "dream"));
}

#[test]
fn command_registry_snapshot_filters_bridge_unsafe_commands() {
    let snapshot = build_command_registry_snapshot(&CommandRegistryContext::bridge_safe(), &[]);
    assert!(snapshot
        .session_commands
        .iter()
        .all(|descriptor| descriptor.remote_safe && descriptor.channel_safe));
    assert!(snapshot
        .filtered_out_commands
        .iter()
        .any(|command| command.reason == "not bridge-safe"));
}

#[test]
fn render_slash_command_help_for_context_hides_tool_commands_when_profile_disables_tools() {
    let help = render_slash_command_help_for_context(&CommandRegistryContext::for_surface(
        CommandSurface::CliLocal,
        false,
    ));
    assert!(help.contains("Start here        /doctor, /config, /status, /memory"));
    assert!(!help.contains("/mcp [list|show <server>|help]"));
}

#[test]
fn v1_command_manifest_matches_expected_session_surface() {
    let names = v1_command_manifest()
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "help",
            "compact",
            "model",
            "permissions",
            "clear",
            "resume",
            "config",
            "mcp",
            "memory",
            "init",
            "doctor",
        ]
    );
}

#[test]
fn renders_per_command_help_detail() {
    let help = render_slash_command_help_detail("plugins").expect("detail help should exist");
    assert!(help.contains("/plugin"));
    assert!(help.contains("Summary          Manage Kcode plugins"));
    assert!(help.contains("Aliases          /plugins, /marketplace"));
    assert!(help.contains("Category         Workspace & git"));
}

#[test]
fn renders_per_command_help_detail_for_mcp() {
    let help = render_slash_command_help_detail("mcp").expect("detail help should exist");
    assert!(help.contains("/mcp"));
    assert!(help.contains("Summary          Inspect configured MCP servers"));
    assert!(help.contains("Category         Discovery & debugging"));
    assert!(help.contains("Resume           Supported with --resume SESSION.jsonl"));
}

#[test]
fn suggests_closest_slash_commands_for_typos_and_aliases() {
    assert_eq!(suggest_slash_commands("stats", 3), vec!["/status"]);
    assert_eq!(suggest_slash_commands("/plugns", 3), vec!["/plugin"]);
    assert_eq!(suggest_slash_commands("zzz", 3), Vec::<String>::new());
}
