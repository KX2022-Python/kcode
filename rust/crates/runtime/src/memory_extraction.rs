//! Background memory extraction — periodically extracts session insights
//! into the memory file system, following CC Source Map principles.

use std::path::Path;

use crate::memory::{create_memory, ensure_memory_dir, MemoryType};
use crate::session::{ContentBlock, ConversationMessage, MessageRole};

/// Minimum input tokens between memory extractions.
pub const MEMORY_EXTRACTION_TOKEN_THRESHOLD: u32 = 50_000;

/// Minimum tool calls between memory extractions.
pub const MEMORY_EXTRACTION_TOOL_CALL_THRESHOLD: usize = 10;

/// Tracks usage since last memory extraction.
/// Uses cumulative usage snapshots for threshold comparison.
#[derive(Debug, Clone, Default)]
pub struct MemoryExtractionState {
    /// Cumulative input tokens at the point of last extraction (0 = never extracted).
    cumulative_input_tokens_at_last_extraction: u32,
    /// Cumulative tool call count at the point of last extraction.
    cumulative_tool_calls_at_last_extraction: usize,
}

impl MemoryExtractionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the current cumulative usage snapshot.
    /// Called at the end of each turn.
    pub fn record_turn(&mut self, cumulative_input_tokens: u32, cumulative_tool_calls: usize) {
        self.cumulative_input_tokens_at_last_extraction = cumulative_input_tokens;
        self.cumulative_tool_calls_at_last_extraction = cumulative_tool_calls;
    }

    /// Check if memory extraction should be triggered.
    pub fn should_extract(&self, cumulative_input_tokens: u32, cumulative_tool_calls: usize) -> bool {
        let token_delta = cumulative_input_tokens.saturating_sub(self.cumulative_input_tokens_at_last_extraction);
        let tool_call_delta = cumulative_tool_calls.saturating_sub(self.cumulative_tool_calls_at_last_extraction);

        token_delta >= MEMORY_EXTRACTION_TOKEN_THRESHOLD
            || tool_call_delta >= MEMORY_EXTRACTION_TOOL_CALL_THRESHOLD
    }

    /// Reset counters after extraction.
    pub fn reset(&mut self, cumulative_input_tokens: u32, cumulative_tool_calls: usize) {
        self.cumulative_input_tokens_at_last_extraction = cumulative_input_tokens;
        self.cumulative_tool_calls_at_last_extraction = cumulative_tool_calls;
    }
}

/// Extract memory from the current session messages and write to the memory directory.
pub fn extract_memory_from_session(
    messages: &[ConversationMessage],
    memory_dir: &Path,
    session_id: &str,
) -> Result<Option<String>, std::io::Error> {
    ensure_memory_dir(memory_dir)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Gather recent assistant messages (last 5)
    let recent_assistant: Vec<&ConversationMessage> = messages
        .iter()
        .rev()
        .filter(|m| m.role == MessageRole::Assistant)
        .take(5)
        .collect();

    if recent_assistant.is_empty() {
        return Ok(None);
    }

    // Collect tool usage summary
    let mut tool_names = Vec::new();
    let mut user_requests = Vec::new();

    for msg in messages.iter().rev().take(20) {
        match msg.role {
            MessageRole::User => {
                if let Some(text) = first_text_block(msg) {
                    if user_requests.len() < 3 {
                        user_requests.push(text.chars().take(100).collect::<String>());
                    }
                }
            }
            MessageRole::Assistant | MessageRole::Tool => {
                for block in &msg.blocks {
                    if let ContentBlock::ToolUse { name, .. } = block {
                        if !tool_names.contains(&name.as_str()) {
                            tool_names.push(name.as_str());
                        }
                    }
                }
            }
            MessageRole::System => {}
        }
    }

    if tool_names.is_empty() && user_requests.is_empty() {
        return Ok(None);
    }

    // Build memory content
    let name = format!("session-{}", &session_id[..8.min(session_id.len())]);
    let description = format!(
        "Tools used: {}; Requests: {}",
        tool_names.join(", "),
        user_requests.first().map(|s| s.as_str()).unwrap_or("")
    );

    let mut body = String::from("## Session Memory Extract\n\n");
    body.push_str(&format!("**Session ID:** {}\n\n", session_id));

    if !tool_names.is_empty() {
        body.push_str("### Tools Used\n");
        for tool in &tool_names {
            body.push_str(&format!("- {}\n", tool));
        }
        body.push('\n');
    }

    if !user_requests.is_empty() {
        body.push_str("### Recent Requests\n");
        for req in user_requests.iter().rev() {
            body.push_str(&format!("- {}\n", req));
        }
        body.push('\n');
    }

    create_memory(memory_dir, &name, &description, MemoryType::Project, &body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(Some(name))
}

fn first_text_block(msg: &ConversationMessage) -> Option<&str> {
    msg.blocks.iter().find_map(|block| match block {
        ContentBlock::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    #[test]
    fn extraction_state_tracks_usage() {
        let mut state = MemoryExtractionState::new();
        assert!(!state.should_extract(30_000, 5));

        // Record first turn
        state.record_turn(30_000, 5);
        // Not yet at threshold (delta = 0 from snapshot)
        assert!(!state.should_extract(30_000, 5));

        // After accumulating 25k more tokens (total 55k, delta = 25k from last extraction)
        // Still below 50k token threshold
        assert!(!state.should_extract(55_000, 5));

        // Now delta = 75k - 30k = 45k, still below 50k
        assert!(!state.should_extract(75_000, 5));

        // Delta = 80k - 30k = 50k, hits threshold
        assert!(state.should_extract(80_000, 5));

        // After reset at current cumulative
        state.reset(80_000, 5);
        assert!(!state.should_extract(80_000, 5));
    }

    #[test]
    fn extraction_state_triggers_on_tool_calls() {
        let mut state = MemoryExtractionState::new();
        state.record_turn(100, 0);
        // Delta = 12 - 0 = 12, above tool call threshold of 10
        assert!(state.should_extract(100, 12));
    }

    #[test]
    fn extract_memory_from_session_with_tools() {
        let dir = std::env::temp_dir().join(format!(
            "kcode_mem_extract_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);

        let mut session = Session::new();
        session.messages.push(ConversationMessage::user_text("Find all rust files"));
        session.messages.push(ConversationMessage::assistant(vec![
            ContentBlock::ToolUse { id: "1".into(), name: "glob_search".into(), input: "{}".into() },
        ]));
        session.messages.push(ConversationMessage::tool_result("1", "glob_search", "*.rs", false));
        session.messages.push(ConversationMessage::assistant(vec![
            ContentBlock::Text { text: "Found 3 files".into() },
        ]));

        let result = extract_memory_from_session(&session.messages, &dir, "test-session-12345")
            .expect("extraction should succeed");
        assert!(result.is_some());

        let name = result.unwrap();
        assert!(name.starts_with("session-"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
