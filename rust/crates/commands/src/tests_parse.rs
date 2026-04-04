use crate::test_support::parse_error_message;
use crate::{validate_slash_command_input, SlashCommand};

#[allow(clippy::too_many_lines)]
#[test]
fn parses_supported_slash_commands() {
    assert_eq!(SlashCommand::parse("/help"), Ok(Some(SlashCommand::Help)));
    assert_eq!(
        SlashCommand::parse(" /status "),
        Ok(Some(SlashCommand::Status))
    );
    assert_eq!(
        SlashCommand::parse("/sandbox"),
        Ok(Some(SlashCommand::Sandbox))
    );
    assert_eq!(
        SlashCommand::parse("/bughunter runtime"),
        Ok(Some(SlashCommand::Bughunter {
            scope: Some("runtime".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/commit"),
        Ok(Some(SlashCommand::Commit))
    );
    assert_eq!(
        SlashCommand::parse("/pr ready for review"),
        Ok(Some(SlashCommand::Pr {
            context: Some("ready for review".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/issue flaky test"),
        Ok(Some(SlashCommand::Issue {
            context: Some("flaky test".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/debug-tool-call"),
        Ok(Some(SlashCommand::DebugToolCall))
    );
    assert_eq!(
        SlashCommand::parse("/model claude-opus"),
        Ok(Some(SlashCommand::Model {
            model: Some("claude-opus".to_string()),
        }))
    );
    assert_eq!(
        SlashCommand::parse("/model"),
        Ok(Some(SlashCommand::Model { model: None }))
    );
    assert_eq!(
        SlashCommand::parse("/permissions read-only"),
        Ok(Some(SlashCommand::Permissions {
            mode: Some("read-only".to_string()),
        }))
    );
    assert_eq!(
        SlashCommand::parse("/clear"),
        Ok(Some(SlashCommand::Clear { confirm: false }))
    );
    assert_eq!(
        SlashCommand::parse("/clear --confirm"),
        Ok(Some(SlashCommand::Clear { confirm: true }))
    );
    assert_eq!(SlashCommand::parse("/cost"), Ok(Some(SlashCommand::Cost)));
    assert_eq!(
        SlashCommand::parse("/resume session.json"),
        Ok(Some(SlashCommand::Resume {
            session_path: Some("session.json".to_string()),
        }))
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
        SlashCommand::parse("/mcp"),
        Ok(Some(SlashCommand::Mcp {
            action: None,
            target: None
        }))
    );
    assert_eq!(
        SlashCommand::parse("/mcp show remote"),
        Ok(Some(SlashCommand::Mcp {
            action: Some("show".to_string()),
            target: Some("remote".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/memory"),
        Ok(Some(SlashCommand::Memory))
    );
    assert_eq!(SlashCommand::parse("/init"), Ok(Some(SlashCommand::Init)));
    assert_eq!(SlashCommand::parse("/diff"), Ok(Some(SlashCommand::Diff)));
    assert_eq!(
        SlashCommand::parse("/version"),
        Ok(Some(SlashCommand::Version))
    );
    assert_eq!(
        SlashCommand::parse("/export notes.txt"),
        Ok(Some(SlashCommand::Export {
            path: Some("notes.txt".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/session switch abc123"),
        Ok(Some(SlashCommand::Session {
            action: Some("switch".to_string()),
            target: Some("abc123".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/session fork incident-review"),
        Ok(Some(SlashCommand::Session {
            action: Some("fork".to_string()),
            target: Some("incident-review".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/plugins install demo"),
        Ok(Some(SlashCommand::Plugins {
            action: Some("install".to_string()),
            target: Some("demo".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/plugins list"),
        Ok(Some(SlashCommand::Plugins {
            action: Some("list".to_string()),
            target: None
        }))
    );
    assert_eq!(
        SlashCommand::parse("/plugins enable demo"),
        Ok(Some(SlashCommand::Plugins {
            action: Some("enable".to_string()),
            target: Some("demo".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/plugins disable demo"),
        Ok(Some(SlashCommand::Plugins {
            action: Some("disable".to_string()),
            target: Some("demo".to_string())
        }))
    );
    assert_eq!(
        SlashCommand::parse("/skills install ./fixtures/help-skill"),
        Ok(Some(SlashCommand::Skills {
            args: Some("install ./fixtures/help-skill".to_string())
        }))
    );
}

#[test]
fn rejects_unexpected_arguments_for_no_arg_commands() {
    let error = parse_error_message("/compact now");
    assert!(error.contains("Unexpected arguments for /compact."));
    assert!(error.contains("  Usage            /compact"));
    assert!(error.contains("  Summary          Compact local session history"));
}

#[test]
fn rejects_invalid_argument_values() {
    let error = parse_error_message("/permissions admin");
    assert!(error.contains(
        "Unsupported /permissions mode 'admin'. Use read-only, workspace-write, or danger-full-access."
    ));
    assert!(error.contains(
        "  Usage            /permissions [read-only|workspace-write|danger-full-access]"
    ));
}

#[test]
fn rejects_missing_required_arguments() {
    let error = parse_error_message("/resume");
    assert!(error.contains("/resume"));
}

#[test]
fn rejects_invalid_session_and_plugin_shapes() {
    let session_error = parse_error_message("/session switch");
    let plugin_error = parse_error_message("/plugins list extra");

    assert!(session_error.contains("Usage: /session switch <session-id>"));
    assert!(plugin_error.contains("Usage: /plugin list"));
    assert!(plugin_error.contains("Aliases          /plugins, /marketplace"));
}

#[test]
fn rejects_invalid_agents_and_skills_arguments() {
    let agents_error = parse_error_message("/agents show planner");
    let skills_error = parse_error_message("/skills show help");

    assert!(agents_error.contains(
        "Unexpected arguments for /agents: show planner. Use /agents, /agents list, or /agents help."
    ));
    assert!(agents_error.contains("  Usage            /agents [list|help]"));
    assert!(skills_error.contains(
        "Unexpected arguments for /skills: show help. Use /skills, /skills list, /skills install <path>, or /skills help."
    ));
    assert!(skills_error.contains("  Usage            /skills [list|install <path>|help]"));
}

#[test]
fn rejects_invalid_mcp_arguments() {
    let show_error = parse_error_message("/mcp show alpha beta");
    let action_error = parse_error_message("/mcp inspect alpha");

    assert!(show_error.contains("Unexpected arguments for /mcp show."));
    assert!(show_error.contains("  Usage            /mcp show <server>"));
    assert!(
        action_error.contains("Unknown /mcp action 'inspect'. Use list, show <server>, or help.")
    );
    assert!(action_error.contains("  Usage            /mcp [list|show <server>|help]"));
}

#[test]
fn validate_slash_command_input_rejects_extra_single_value_arguments() {
    let session_error = validate_slash_command_input("/session switch current next")
        .expect_err("session input should be rejected")
        .to_string();
    let plugin_error = validate_slash_command_input("/plugin enable demo extra")
        .expect_err("plugin input should be rejected")
        .to_string();

    assert!(session_error.contains("Unexpected arguments for /session switch."));
    assert!(session_error.contains("  Usage            /session switch <session-id>"));
    assert!(plugin_error.contains("Unexpected arguments for /plugin enable."));
    assert!(plugin_error.contains("  Usage            /plugin enable <name>"));
}
