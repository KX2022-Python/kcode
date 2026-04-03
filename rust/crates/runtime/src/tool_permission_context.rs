use crate::permissions::{PermissionContext, PermissionOverride};

/// Tool-specific permission context.
/// Wraps `PermissionContext` with additional tool execution metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolPermissionContext {
    pub tool_name: String,
    pub tool_input: String,
    pub base: PermissionContext,
}

impl ToolPermissionContext {
    pub fn new(tool_name: String, tool_input: String) -> Self {
        Self {
            tool_name,
            tool_input,
            base: PermissionContext::default(),
        }
    }

    pub fn with_override(
        tool_name: String,
        tool_input: String,
        override_decision: PermissionOverride,
        reason: String,
    ) -> Self {
        Self {
            tool_name,
            tool_input,
            base: PermissionContext::new(Some(override_decision), Some(reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_permission_context_new_defaults_to_empty_base() {
        let ctx = ToolPermissionContext::new("bash".to_string(), "echo hi".to_string());
        assert_eq!(ctx.tool_name, "bash");
        assert_eq!(ctx.tool_input, "echo hi");
        assert_eq!(ctx.base.override_decision(), None);
    }

    #[test]
    fn tool_permission_context_with_override() {
        let ctx = ToolPermissionContext::with_override(
            "read_file".to_string(),
            "path".to_string(),
            PermissionOverride::Allow,
            "hook approved".to_string(),
        );
        assert_eq!(ctx.base.override_decision(), Some(PermissionOverride::Allow));
        assert_eq!(ctx.base.override_reason(), Some("hook approved"));
    }
}
