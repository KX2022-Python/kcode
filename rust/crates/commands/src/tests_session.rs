use runtime::{CompactionConfig, ContentBlock, ConversationMessage, MessageRole, Session};

use crate::handle_slash_command;

#[test]
fn compacts_sessions_via_slash_command() {
    let mut session = Session::new();
    session.messages = vec![
        ConversationMessage::user_text("a ".repeat(200)),
        ConversationMessage::assistant(vec![ContentBlock::Text {
            text: "b ".repeat(200),
        }]),
        ConversationMessage::tool_result("1", "bash", "ok ".repeat(200), false),
        ConversationMessage::assistant(vec![ContentBlock::Text {
            text: "recent".to_string(),
        }]),
    ];

    let result = handle_slash_command(
        "/compact",
        &session,
        CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 1,
        },
    )
    .expect("slash command should be handled");

    assert!(result.message.contains("Compacted 2 messages"));
    assert_eq!(result.session.messages[0].role, MessageRole::System);
}

#[test]
fn help_command_is_non_mutating() {
    let session = Session::new();
    let result = handle_slash_command("/help", &session, CompactionConfig::default())
        .expect("help command should be handled");
    assert_eq!(result.session, session);
    assert!(result.message.contains("Slash commands"));
}

#[test]
fn ignores_unknown_or_runtime_bound_slash_commands() {
    let session = Session::new();
    let ignored = [
        "/unknown",
        "/status",
        "/sandbox",
        "/bughunter",
        "/commit",
        "/pr",
        "/issue",
        "/debug-tool-call",
        "/model claude",
        "/permissions read-only",
        "/clear",
        "/clear --confirm",
        "/cost",
        "/resume session.json",
        "/resume session.jsonl",
        "/config",
        "/config env",
        "/mcp list",
        "/diff",
        "/version",
        "/export note.txt",
        "/session list",
        "/plugins list",
    ];

    for input in ignored {
        assert!(handle_slash_command(input, &session, CompactionConfig::default()).is_none());
    }
}
