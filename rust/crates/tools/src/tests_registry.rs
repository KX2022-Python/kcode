use serde_json::json;

use super::registry::permission_mode_from_plugin;
use super::{execute_tool, mvp_tool_specs, GlobalToolRegistry};

#[test]
fn exposes_mvp_tools() {
    let names = mvp_tool_specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"bash"));
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"WebFetch"));
    assert!(names.contains(&"WebSearch"));
    assert!(names.contains(&"TodoWrite"));
    assert!(names.contains(&"Skill"));
    assert!(names.contains(&"Agent"));
    assert!(names.contains(&"ToolSearch"));
    assert!(names.contains(&"NotebookEdit"));
    assert!(names.contains(&"Sleep"));
    assert!(names.contains(&"SendUserMessage"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"EnterPlanMode"));
    assert!(names.contains(&"ExitPlanMode"));
    assert!(names.contains(&"StructuredOutput"));
    assert!(names.contains(&"REPL"));
    assert!(names.contains(&"PowerShell"));
    assert!(!names.contains(&"TaskCreate"));
    assert!(!names.contains(&"LSP"));
    assert!(!names.contains(&"MCP"));
}

#[test]
fn rejects_unknown_tool_names() {
    let error = execute_tool("nope", &json!({})).expect_err("tool should be rejected");
    assert!(error.contains("unsupported tool"));
}

#[test]
fn rejects_hidden_stub_tool_names() {
    let task = execute_tool("TaskCreate", &json!({ "prompt": "hello" }))
        .expect_err("hidden task tool should be rejected");
    assert!(task.contains("unsupported tool"));

    let lsp = execute_tool("LSP", &json!({ "action": "symbols" }))
        .expect_err("hidden LSP tool should be rejected");
    assert!(lsp.contains("unsupported tool"));
}

#[test]
fn permission_mode_from_plugin_rejects_invalid_inputs() {
    assert_eq!(
        permission_mode_from_plugin("plan").expect("plan permission should map"),
        runtime::PermissionMode::Plan
    );

    let unknown_permission =
        permission_mode_from_plugin("admin").expect_err("unknown plugin permission should fail");
    assert!(unknown_permission.contains("unsupported plugin permission: admin"));

    let empty_permission =
        permission_mode_from_plugin("").expect_err("empty plugin permission should fail");
    assert!(empty_permission.contains("unsupported plugin permission: "));
}

#[test]
fn simple_mode_only_exposes_core_tools() {
    let registry = GlobalToolRegistry::builtin().simple_mode();
    let defs = registry.definitions(None);
    let names: Vec<&str> = defs
        .iter()
        .map(|definition| definition.name.as_str())
        .collect();

    assert!(names.contains(&"bash"));
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
    assert!(names.contains(&"edit_file"));
    assert!(!names.contains(&"glob_search"));
    assert!(!names.contains(&"grep_search"));
    assert!(!names.contains(&"WebFetch"));
    assert_eq!(defs.len(), 4);
}
