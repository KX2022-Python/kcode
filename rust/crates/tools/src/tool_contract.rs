use std::collections::BTreeSet;

use runtime::PermissionMode;

use crate::ToolSpec;

/// Context passed to a tool when it is executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUseContext {
    pub tool_name: String,
    pub tool_input: String,
    pub permission_mode: PermissionMode,
}

impl ToolUseContext {
    pub fn new(tool_name: String, tool_input: String, permission_mode: PermissionMode) -> Self {
        Self {
            tool_name,
            tool_input,
            permission_mode,
        }
    }
}

/// Assemble a tool pool from specs with allow-list filtering.
pub struct ToolPoolAssembler {
    specs: Vec<ToolSpec>,
    allowed_tools: Option<BTreeSet<String>>,
}

impl ToolPoolAssembler {
    pub fn new() -> Self {
        Self {
            specs: Vec::new(),
            allowed_tools: None,
        }
    }

    pub fn with_specs(mut self, specs: Vec<ToolSpec>) -> Self {
        self.specs.extend(specs);
        self
    }

    pub fn with_allowed_tools(mut self, tools: Option<BTreeSet<String>>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Assemble the final set of allowed tool names.
    pub fn assemble(&self) -> BTreeSet<String> {
        self.specs
            .iter()
            .filter(|spec| {
                self.allowed_tools
                    .as_ref()
                    .map(|allowed| allowed.contains(spec.name))
                    .unwrap_or(true)
            })
            .map(|spec| spec.name.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_spec(name: &'static str, mode: PermissionMode) -> ToolSpec {
        ToolSpec {
            name,
            description: "test",
            input_schema: json!({}),
            required_permission: mode,
        }
    }

    #[test]
    fn tool_use_context_holds_execution_metadata() {
        let ctx = ToolUseContext::new(
            "bash".to_string(),
            "ls -la".to_string(),
            PermissionMode::DangerFullAccess,
        );
        assert_eq!(ctx.tool_name, "bash");
        assert_eq!(ctx.tool_input, "ls -la");
        assert_eq!(ctx.permission_mode, PermissionMode::DangerFullAccess);
    }

    #[test]
    fn tool_pool_assembler_filters_by_allow_list() {
        let specs = vec![
            test_spec("bash", PermissionMode::DangerFullAccess),
            test_spec("read_file", PermissionMode::ReadOnly),
        ];
        let allowed = BTreeSet::from(["read_file".to_string()]);

        let pool = ToolPoolAssembler::new()
            .with_specs(specs)
            .with_allowed_tools(Some(allowed))
            .assemble();

        assert!(pool.contains("read_file"));
        assert!(!pool.contains("bash"));
    }

    #[test]
    fn tool_pool_assembler_allows_all_when_no_filter() {
        let specs = vec![
            test_spec("bash", PermissionMode::DangerFullAccess),
            test_spec("read_file", PermissionMode::ReadOnly),
        ];

        let pool = ToolPoolAssembler::new()
            .with_specs(specs)
            .assemble();

        assert_eq!(pool.len(), 2);
        assert!(pool.contains("bash"));
        assert!(pool.contains("read_file"));
    }
}
