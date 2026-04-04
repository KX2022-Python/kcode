//! Generic deny-rule matcher for tool filtering.
//! Following CC Source Map: filterToolsByDenyRules applies uniformly
//! across builtin and MCP tools using pattern matching.

/// A deny rule that matches tools by various criteria.
#[derive(Debug, Clone)]
pub enum ToolDenyRule {
    /// Deny tools whose name matches (substring match).
    NameMatch(String),
    /// Deny stdio tools whose command matches (substring match).
    CommandMatch(String),
    /// Deny remote tools whose URL matches (substring match).
    UrlMatch(String),
    /// Deny all tools from a specific MCP server (by name prefix).
    McpServerPrefix(String),
}

impl ToolDenyRule {
    pub fn name(pattern: &str) -> Self {
        Self::NameMatch(pattern.to_string())
    }

    pub fn command(pattern: &str) -> Self {
        Self::CommandMatch(pattern.to_string())
    }

    pub fn url(pattern: &str) -> Self {
        Self::UrlMatch(pattern.to_string())
    }

    pub fn mcp_server(prefix: &str) -> Self {
        Self::McpServerPrefix(prefix.to_string())
    }

    pub fn matches(
        &self,
        tool_name: &str,
        command: Option<&str>,
        url: Option<&str>,
        mcp_server_name: Option<&str>,
    ) -> bool {
        match self {
            Self::NameMatch(pat) => tool_name.contains(pat),
            Self::CommandMatch(pat) => command.is_some_and(|c| c.contains(pat)),
            Self::UrlMatch(pat) => url.is_some_and(|u| u.contains(pat)),
            Self::McpServerPrefix(prefix) => {
                mcp_server_name.is_some_and(|name| name.starts_with(prefix))
            }
        }
    }
}

/// Apply deny rules to a list of tool definitions, removing matched tools.
/// Returns the filtered list and the count of removed tools.
pub fn apply_deny_rules<T>(
    tools: Vec<T>,
    deny_rules: &[ToolDenyRule],
    tool_name_fn: impl Fn(&T) -> &str,
    tool_command_fn: impl Fn(&T) -> Option<&str>,
    tool_url_fn: impl Fn(&T) -> Option<&str>,
    tool_mcp_server_fn: impl Fn(&T) -> Option<&str>,
) -> (Vec<T>, usize) {
    if deny_rules.is_empty() {
        return (tools, 0);
    }

    let initial_count = tools.len();
    let filtered: Vec<T> = tools
        .into_iter()
        .filter(|tool| {
            !deny_rules.iter().any(|rule| {
                rule.matches(
                    tool_name_fn(tool),
                    tool_command_fn(tool),
                    tool_url_fn(tool),
                    tool_mcp_server_fn(tool),
                )
            })
        })
        .collect();
    let removed_count = initial_count - filtered.len();
    (filtered, removed_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestTool {
        name: String,
        command: Option<String>,
        url: Option<String>,
        mcp_server: Option<String>,
    }

    fn name_fn(t: &TestTool) -> &str {
        &t.name
    }
    fn command_fn(t: &TestTool) -> Option<&str> {
        t.command.as_deref()
    }
    fn url_fn(t: &TestTool) -> Option<&str> {
        t.url.as_deref()
    }
    fn mcp_fn(t: &TestTool) -> Option<&str> {
        t.mcp_server.as_deref()
    }

    #[test]
    fn deny_by_name_match() {
        let rule = ToolDenyRule::name("danger");
        let tools = vec![
            TestTool {
                name: "safe_tool".into(),
                command: None,
                url: None,
                mcp_server: None,
            },
            TestTool {
                name: "danger_bash".into(),
                command: None,
                url: None,
                mcp_server: None,
            },
        ];
        let (filtered, removed) =
            apply_deny_rules(tools, &[rule], name_fn, command_fn, url_fn, mcp_fn);
        assert_eq!(removed, 1);
        assert_eq!(filtered[0].name, "safe_tool");
    }

    #[test]
    fn deny_by_command_match() {
        let rule = ToolDenyRule::command("rm -rf");
        let tools = vec![
            TestTool {
                name: "bash".into(),
                command: Some("ls -la".into()),
                url: None,
                mcp_server: None,
            },
            TestTool {
                name: "bash".into(),
                command: Some("rm -rf /tmp".into()),
                url: None,
                mcp_server: None,
            },
        ];
        let (filtered, removed) =
            apply_deny_rules(tools, &[rule], name_fn, command_fn, url_fn, mcp_fn);
        assert_eq!(removed, 1);
        assert_eq!(filtered[0].command.as_deref(), Some("ls -la"));
    }

    #[test]
    fn deny_by_mcp_server_prefix() {
        let rule = ToolDenyRule::mcp_server("internal-");
        let tools = vec![
            TestTool {
                name: "tool_a".into(),
                command: None,
                url: None,
                mcp_server: Some("public-mcp".into()),
            },
            TestTool {
                name: "tool_b".into(),
                command: None,
                url: None,
                mcp_server: Some("internal-mcp".into()),
            },
        ];
        let (filtered, removed) =
            apply_deny_rules(tools, &[rule], name_fn, command_fn, url_fn, mcp_fn);
        assert_eq!(removed, 1);
        assert_eq!(filtered[0].mcp_server.as_deref(), Some("public-mcp"));
    }

    #[test]
    fn multiple_rules_combined() {
        let rules = vec![
            ToolDenyRule::name("test"),
            ToolDenyRule::url("blocked.example"),
        ];
        let tools = vec![
            TestTool {
                name: "test_tool".into(),
                command: None,
                url: None,
                mcp_server: None,
            },
            TestTool {
                name: "good_tool".into(),
                command: None,
                url: Some("https://blocked.example/api".into()),
                mcp_server: None,
            },
            TestTool {
                name: "safe_tool".into(),
                command: None,
                url: Some("https://safe.example/api".into()),
                mcp_server: None,
            },
        ];
        let (filtered, removed) =
            apply_deny_rules(tools, &rules, name_fn, command_fn, url_fn, mcp_fn);
        assert_eq!(removed, 2);
        assert_eq!(filtered[0].name, "safe_tool");
    }

    #[test]
    fn empty_rules_returns_all_tools() {
        let tools = vec![
            TestTool {
                name: "tool_a".into(),
                command: None,
                url: None,
                mcp_server: None,
            },
            TestTool {
                name: "tool_b".into(),
                command: None,
                url: None,
                mcp_server: None,
            },
        ];
        let (filtered, removed) = apply_deny_rules(tools, &[], name_fn, command_fn, url_fn, mcp_fn);
        assert_eq!(removed, 0);
        assert_eq!(filtered.len(), 2);
    }
}
