use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::web::WebSearchResultItem;

#[derive(Debug, Deserialize)]
pub(crate) struct ReadFileInput {
    pub(crate) path: String,
    pub(crate) offset: Option<usize>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WriteFileInput {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditFileInput {
    pub(crate) path: String,
    pub(crate) old_string: String,
    pub(crate) new_string: String,
    pub(crate) replace_all: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GlobSearchInputValue {
    pub(crate) pattern: String,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebFetchInput {
    pub(crate) url: String,
    pub(crate) prompt: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebSearchInput {
    pub(crate) query: String,
    pub(crate) allowed_domains: Option<Vec<String>>,
    pub(crate) blocked_domains: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TodoWriteInput {
    pub(crate) todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct TodoItem {
    pub(crate) content: String,
    #[serde(rename = "activeForm")]
    pub(crate) active_form: String,
    pub(crate) status: TodoStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SkillInput {
    pub(crate) skill: String,
    pub(crate) args: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentInput {
    pub(crate) description: String,
    pub(crate) prompt: String,
    pub(crate) subagent_type: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ToolSearchInput {
    pub(crate) query: String,
    pub(crate) max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NotebookEditInput {
    pub(crate) notebook_path: String,
    pub(crate) cell_id: Option<String>,
    pub(crate) new_source: Option<String>,
    pub(crate) cell_type: Option<NotebookCellType>,
    pub(crate) edit_mode: Option<NotebookEditMode>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NotebookCellType {
    Code,
    Markdown,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NotebookEditMode {
    Replace,
    Insert,
    Delete,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SleepInput {
    pub(crate) duration_ms: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BriefInput {
    pub(crate) message: String,
    pub(crate) attachments: Option<Vec<String>>,
    pub(crate) status: BriefStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BriefStatus {
    Normal,
    Proactive,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigInput {
    pub(crate) setting: String,
    pub(crate) value: Option<ConfigValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct EnterPlanModeInput {}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ExitPlanModeInput {}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ConfigValue {
    String(String),
    Bool(bool),
    Number(f64),
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct StructuredOutputInput(pub(crate) BTreeMap<String, Value>);

#[derive(Debug, Deserialize)]
pub(crate) struct ReplInput {
    pub(crate) code: String,
    pub(crate) language: String,
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PowerShellInput {
    pub(crate) command: String,
    pub(crate) timeout: Option<u64>,
    pub(crate) description: Option<String>,
    pub(crate) run_in_background: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AskUserQuestionInput {
    pub(crate) question: String,
    #[serde(default)]
    pub(crate) options: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskCreateInput {
    pub(crate) prompt: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskIdInput {
    pub(crate) task_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskUpdateInput {
    pub(crate) task_id: String,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamCreateInput {
    pub(crate) name: String,
    pub(crate) tasks: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamDeleteInput {
    pub(crate) team_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CronCreateInput {
    pub(crate) schedule: String,
    pub(crate) prompt: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CronDeleteInput {
    pub(crate) cron_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LspInput {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) line: Option<u32>,
    #[serde(default)]
    pub(crate) character: Option<u32>,
    #[serde(default)]
    pub(crate) query: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpResourceInput {
    #[serde(default)]
    pub(crate) server: Option<String>,
    #[serde(default)]
    pub(crate) uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpAuthInput {
    pub(crate) server: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RemoteTriggerInput {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) method: Option<String>,
    #[serde(default)]
    pub(crate) headers: Option<Value>,
    #[serde(default)]
    pub(crate) body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpToolInput {
    pub(crate) server: String,
    pub(crate) tool: String,
    #[serde(default)]
    pub(crate) arguments: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TestingPermissionInput {
    pub(crate) action: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct WebBrowserInput {
    pub(crate) action: String,
    pub(crate) url: Option<String>,
    pub(crate) selector: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) scroll_amount: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebBrowserOutput {
    pub(crate) action: String,
    pub(crate) status: String,
    pub(crate) url: Option<String>,
    #[serde(rename = "pageTitle", skip_serializing_if = "Option::is_none")]
    pub(crate) page_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) message: Option<String>,
    #[serde(rename = "domLength", skip_serializing_if = "Option::is_none")]
    pub(crate) dom_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) screenshot: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebFetchOutput {
    pub(crate) bytes: usize,
    pub(crate) code: u16,
    #[serde(rename = "codeText")]
    pub(crate) code_text: String,
    pub(crate) result: String,
    #[serde(rename = "durationMs")]
    pub(crate) duration_ms: u128,
    pub(crate) url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebSearchOutput {
    pub(crate) query: String,
    pub(crate) results: Vec<WebSearchResultItem>,
    #[serde(rename = "durationSeconds")]
    pub(crate) duration_seconds: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TodoWriteOutput {
    #[serde(rename = "oldTodos")]
    pub(crate) old_todos: Vec<TodoItem>,
    #[serde(rename = "newTodos")]
    pub(crate) new_todos: Vec<TodoItem>,
    #[serde(rename = "verificationNudgeNeeded")]
    pub(crate) verification_nudge_needed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SkillOutput {
    pub(crate) skill: String,
    pub(crate) path: String,
    pub(crate) args: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentOutput {
    #[serde(rename = "agentId")]
    pub(crate) agent_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    #[serde(rename = "subagentType")]
    pub(crate) subagent_type: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) status: String,
    #[serde(rename = "outputFile")]
    pub(crate) output_file: String,
    #[serde(rename = "manifestFile")]
    pub(crate) manifest_file: String,
    #[serde(rename = "createdAt")]
    pub(crate) created_at: String,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(rename = "completedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentJob {
    pub(crate) manifest: AgentOutput,
    pub(crate) prompt: String,
    pub(crate) system_prompt: Vec<String>,
    pub(crate) allowed_tools: std::collections::BTreeSet<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ToolSearchOutput {
    pub(crate) matches: Vec<String>,
    pub(crate) query: String,
    pub(crate) normalized_query: String,
    #[serde(rename = "total_deferred_tools")]
    pub(crate) total_deferred_tools: usize,
    #[serde(rename = "pending_mcp_servers")]
    pub(crate) pending_mcp_servers: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NotebookEditOutput {
    pub(crate) new_source: String,
    pub(crate) cell_id: Option<String>,
    pub(crate) cell_type: Option<NotebookCellType>,
    pub(crate) language: String,
    pub(crate) edit_mode: String,
    pub(crate) error: Option<String>,
    pub(crate) notebook_path: String,
    pub(crate) original_file: String,
    pub(crate) updated_file: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SleepOutput {
    pub(crate) duration_ms: u64,
    pub(crate) message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BriefOutput {
    pub(crate) message: String,
    pub(crate) attachments: Option<Vec<ResolvedAttachment>>,
    #[serde(rename = "sentAt")]
    pub(crate) sent_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResolvedAttachment {
    pub(crate) path: String,
    pub(crate) size: u64,
    #[serde(rename = "isImage")]
    pub(crate) is_image: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConfigOutput {
    pub(crate) success: bool,
    pub(crate) operation: Option<String>,
    pub(crate) setting: Option<String>,
    pub(crate) value: Option<Value>,
    #[serde(rename = "previousValue")]
    pub(crate) previous_value: Option<Value>,
    #[serde(rename = "newValue")]
    pub(crate) new_value: Option<Value>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlanModeState {
    #[serde(rename = "hadLocalOverride")]
    pub(crate) had_local_override: bool,
    #[serde(rename = "previousLocalMode")]
    pub(crate) previous_local_mode: Option<Value>,
}

#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct PlanModeOutput {
    pub(crate) success: bool,
    pub(crate) operation: String,
    pub(crate) changed: bool,
    pub(crate) active: bool,
    pub(crate) managed: bool,
    pub(crate) message: String,
    #[serde(rename = "settingsPath")]
    pub(crate) settings_path: String,
    #[serde(rename = "statePath")]
    pub(crate) state_path: String,
    #[serde(rename = "previousLocalMode")]
    pub(crate) previous_local_mode: Option<Value>,
    #[serde(rename = "currentLocalMode")]
    pub(crate) current_local_mode: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct StructuredOutputResult {
    pub(crate) data: String,
    pub(crate) structured_output: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReplOutput {
    pub(crate) language: String,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    #[serde(rename = "exitCode")]
    pub(crate) exit_code: i32,
    #[serde(rename = "durationMs")]
    pub(crate) duration_ms: u128,
}
