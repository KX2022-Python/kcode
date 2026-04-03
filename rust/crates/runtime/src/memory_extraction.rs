//! Background memory extraction — periodically extracts session insights
//! into the memory file system, following CC Source Map principles.

use std::path::Path;

use crate::memory::{
    create_memory, ensure_memory_dir, list_memories, read_memory, update_memory, MemoryType,
};
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
/// Following CC Source Map principles: extracts insights from session, detects patterns,
/// and handles conflicts with existing memories (updates instead of duplicating).
pub fn extract_memory_from_session(
    messages: &[ConversationMessage],
    memory_dir: &Path,
    session_id: &str,
) -> Result<Option<String>, std::io::Error> {
    ensure_memory_dir(memory_dir)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // 1. Detect explicit user memory instructions ("记住", "remember", "以后...")
    let explicit_memories = detect_explicit_memory_instructions(messages);

    // 2. Detect tool usage patterns
    let tool_patterns = detect_tool_usage_patterns(messages);

    // 3. Detect key file references (files mentioned multiple times)
    let key_files = detect_key_file_references(messages);

    // 4. Detect error patterns and solutions
    let error_patterns = detect_error_patterns(messages);

    // 5. Gather recent user requests for context
    let user_requests: Vec<String> = messages
        .iter()
        .rev()
        .filter(|m| m.role == MessageRole::User)
        .take(3)
        .filter_map(|m| first_text_block(m))
        .map(|t| t.chars().take(100).collect::<String>())
        .collect();

    if explicit_memories.is_empty()
        && tool_patterns.is_empty()
        && key_files.is_empty()
        && error_patterns.is_empty()
    {
        return Ok(None);
    }

    // Check for conflicts with existing memories and update if needed
    if let Some(existing) = find_conflicting_memory(memory_dir, &tool_patterns, &key_files) {
        // Update existing memory instead of creating duplicate
        let updated_description = build_conflict_resolution_description(
            &existing.description,
            &tool_patterns,
            &key_files,
        );
        let updated_body = build_updated_memory_body(
            &existing.body,
            &tool_patterns,
            &key_files,
            &error_patterns,
        );

        update_memory(memory_dir, &existing.name, &updated_description, &updated_body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        return Ok(Some(format!("updated:{}", existing.name)));
    }

    // Create new memory
    let name = format!("session-{}", &session_id[..8.min(session_id.len())]);
    let description = build_extraction_description(&tool_patterns, &key_files, &user_requests);
    let body = build_extraction_body(
        session_id,
        &explicit_memories,
        &tool_patterns,
        &key_files,
        &error_patterns,
    );

    create_memory(memory_dir, &name, &description, MemoryType::Project, &body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(Some(name))
}

/// Detect explicit user memory instructions like "记住这个", "remember this", "以后都..."
fn detect_explicit_memory_instructions(messages: &[ConversationMessage]) -> Vec<String> {
    let memory_keywords = [
        "记住", "记下", "保存", "remember", "note that", "以后都", "always",
        "don't forget", "keep in mind", "重要", "important",
    ];

    messages
        .iter()
        .rev()
        .take(10)
        .filter(|m| m.role == MessageRole::User)
        .filter_map(|m| first_text_block(m))
        .filter(|text| {
            memory_keywords
                .iter()
                .any(|keyword| text.to_lowercase().contains(keyword))
        })
        .map(|text| text.chars().take(100).collect())
        .collect()
}

/// Detect tool usage patterns (repeated tool combinations)
fn detect_tool_usage_patterns(messages: &[ConversationMessage]) -> Vec<String> {
    let mut tool_counts = std::collections::HashMap::new();

    for msg in messages.iter().rev().take(30) {
        for block in &msg.blocks {
            if let ContentBlock::ToolUse { name, .. } = block {
                *tool_counts.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }

    tool_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(name, count)| format!("{name} (used {count} times)"))
        .collect()
}

/// Detect key file references (files mentioned multiple times)
fn detect_key_file_references(messages: &[ConversationMessage]) -> Vec<String> {
    let mut file_counts = std::collections::HashMap::new();

    for msg in messages.iter().rev().take(30) {
        for block in &msg.blocks {
            let text = match block {
                ContentBlock::Text { text } => text.as_str(),
                ContentBlock::ToolUse { input, .. } => input.as_str(),
                ContentBlock::ToolResult { output, .. } => output.as_str(),
            };

            for token in text.split_whitespace() {
                let candidate = token.trim_matches(|c: char| {
                    matches!(c, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\'' | '`')
                });
                if candidate.contains('/') && has_interesting_extension(candidate) {
                    *file_counts.entry(candidate.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    file_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(path, _)| path)
        .collect()
}

/// Detect error patterns (failed tool calls and their context)
fn detect_error_patterns(messages: &[ConversationMessage]) -> Vec<String> {
    messages
        .iter()
        .rev()
        .take(20)
        .filter_map(|msg| {
            if msg.role == MessageRole::Tool {
                for block in &msg.blocks {
                    if let ContentBlock::ToolResult {
                        tool_name,
                        output,
                        is_error: true,
                        ..
                    } = block
                    {
                        let summary = output.chars().take(80).collect::<String>();
                        return Some(format!("{tool_name}: {summary}"));
                    }
                }
            }
            None
        })
        .collect()
}

/// Check for conflicts with existing memories (similar tool patterns or key files)
fn find_conflicting_memory(
    memory_dir: &Path,
    tool_patterns: &[String],
    key_files: &[String],
) -> Option<crate::memory::MemoryEntry> {
    if tool_patterns.is_empty() && key_files.is_empty() {
        return None;
    }

    let memories = list_memories(memory_dir).ok()?;
    for mem in memories {
        let entry_path = memory_dir.join(&mem.file_name);
        if let Ok(existing) = read_memory(&entry_path) {
            // Check if any key file is already mentioned in existing memory
            if key_files.iter().any(|f| existing.body.contains(f)) {
                return Some(existing);
            }
        }
    }
    None
}

fn build_conflict_resolution_description(
    existing_desc: &str,
    tool_patterns: &[String],
    key_files: &[String],
) -> String {
    let mut desc = existing_desc.to_string();
    if !tool_patterns.is_empty() {
        desc.push_str("; New patterns: ");
        desc.push_str(&tool_patterns.join(", "));
    }
    if !key_files.is_empty() {
        desc.push_str("; New files: ");
        desc.push_str(&key_files.join(", "));
    }
    desc
}

fn build_updated_memory_body(
    existing_body: &str,
    tool_patterns: &[String],
    key_files: &[String],
    error_patterns: &[String],
) -> String {
    let mut body = existing_body.to_string();
    body.push_str("\n\n## Updated Patterns\n\n");

    if !tool_patterns.is_empty() {
        body.push_str("### Tool Patterns\n");
        for p in tool_patterns {
            body.push_str(&format!("- {p}\n"));
        }
    }

    if !key_files.is_empty() {
        body.push_str("\n### Key Files\n");
        for f in key_files {
            body.push_str(&format!("- {f}\n"));
        }
    }

    if !error_patterns.is_empty() {
        body.push_str("\n### Error Patterns\n");
        for e in error_patterns {
            body.push_str(&format!("- {e}\n"));
        }
    }

    body
}

fn build_extraction_description(
    tool_patterns: &[String],
    key_files: &[String],
    user_requests: &[String],
) -> String {
    let mut parts = Vec::new();
    if !tool_patterns.is_empty() {
        parts.push(format!("Tools: {}", tool_patterns.join(", ")));
    }
    if !key_files.is_empty() {
        parts.push(format!("Files: {}", key_files.join(", ")));
    }
    if let Some(req) = user_requests.first() {
        parts.push(format!("Context: {req}"));
    }
    parts.join("; ")
}

fn build_extraction_body(
    session_id: &str,
    explicit_memories: &[String],
    tool_patterns: &[String],
    key_files: &[String],
    error_patterns: &[String],
) -> String {
    let mut body = String::from("## Session Memory Extract\n\n");
    body.push_str(&format!("**Session ID:** {}\n\n", session_id));

    if !explicit_memories.is_empty() {
        body.push_str("### User Instructions\n");
        for m in explicit_memories {
            body.push_str(&format!("- {m}\n"));
        }
        body.push('\n');
    }

    if !tool_patterns.is_empty() {
        body.push_str("### Tool Patterns\n");
        for p in tool_patterns {
            body.push_str(&format!("- {p}\n"));
        }
        body.push('\n');
    }

    if !key_files.is_empty() {
        body.push_str("### Key Files\n");
        for f in key_files {
            body.push_str(&format!("- {f}\n"));
        }
        body.push('\n');
    }

    if !error_patterns.is_empty() {
        body.push_str("### Error Patterns\n");
        for e in error_patterns {
            body.push_str(&format!("- {e}\n"));
        }
        body.push('\n');
    }

    body
}

fn has_interesting_extension(candidate: &str) -> bool {
    std::path::Path::new(candidate)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext,
                "rs" | "ts" | "tsx" | "js" | "json" | "md" | "toml" | "yaml" | "yml" | "py"
                    | "go" | "java" | "c" | "cpp" | "h" | "rb" | "sh"
            )
        })
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
            ContentBlock::ToolUse { id: "2".into(), name: "glob_search".into(), input: "{}".into() },
        ]));
        session.messages.push(ConversationMessage::tool_result("2", "glob_search", "src/lib.rs", false));

        let result = extract_memory_from_session(&session.messages, &dir, "test-session-12345")
            .expect("extraction should succeed");
        assert!(result.is_some());

        let name = result.unwrap();
        assert!(name.starts_with("session-"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
