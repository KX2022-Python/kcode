use std::collections::{BTreeMap, BTreeSet};

use plugins::PluginTool;
use runtime::PermissionMode;
use serde_json::Value;

use crate::specs::mvp_tool_specs;

/// Tools allowed in simple mode (CC Source Map: Bash/Read/Edit only).
const SIMPLE_MODE_TOOLS: &[&str] = &["bash", "read_file", "write_file", "edit_file"];

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalToolRegistry {
    include_builtin_tools: bool,
    simple_mode: bool,
    plugin_tools: Vec<PluginTool>,
}

impl Default for GlobalToolRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl GlobalToolRegistry {
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            include_builtin_tools: true,
            simple_mode: false,
            plugin_tools: Vec::new(),
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            include_builtin_tools: false,
            simple_mode: false,
            plugin_tools: Vec::new(),
        }
    }

    #[must_use]
    pub fn simple_mode(mut self) -> Self {
        self.simple_mode = true;
        self
    }

    pub fn with_plugin_tools(plugin_tools: Vec<PluginTool>) -> Result<Self, String> {
        let builtin_names = mvp_tool_specs()
            .into_iter()
            .map(|spec| spec.name.to_string())
            .collect::<BTreeSet<_>>();
        let mut seen_plugin_names = BTreeSet::new();

        for tool in &plugin_tools {
            let name = tool.definition().name.clone();
            if builtin_names.contains(&name) {
                return Err(format!(
                    "plugin tool `{name}` conflicts with a built-in tool name"
                ));
            }
            if !seen_plugin_names.insert(name.clone()) {
                return Err(format!("duplicate plugin tool name `{name}`"));
            }
        }

        Ok(Self {
            include_builtin_tools: true,
            simple_mode: false,
            plugin_tools,
        })
    }

    pub fn normalize_allowed_tools(
        &self,
        values: &[String],
    ) -> Result<Option<BTreeSet<String>>, String> {
        if values.is_empty() {
            return Ok(None);
        }

        let builtin_specs = if self.include_builtin_tools {
            mvp_tool_specs()
        } else {
            Vec::new()
        };
        let canonical_names = builtin_specs
            .iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .collect::<Vec<_>>();
        if canonical_names.is_empty() {
            return Err("the current tool registry does not expose any tools".to_string());
        }

        let mut name_map = canonical_names
            .iter()
            .map(|name| (normalize_tool_name(name), name.clone()))
            .collect::<BTreeMap<_, _>>();
        for (alias, canonical) in [
            ("read", "read_file"),
            ("write", "write_file"),
            ("edit", "edit_file"),
            ("glob", "glob_search"),
            ("grep", "grep_search"),
        ] {
            name_map.insert(alias.to_string(), canonical.to_string());
        }

        let mut allowed = BTreeSet::new();
        for value in values {
            for token in value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .filter(|token| !token.is_empty())
            {
                let normalized = normalize_tool_name(token);
                let canonical = name_map.get(&normalized).ok_or_else(|| {
                    format!(
                        "unsupported tool in --allowedTools: {token} (expected one of: {})",
                        canonical_names.join(", ")
                    )
                })?;
                allowed.insert(canonical.clone());
            }
        }

        Ok(Some(allowed))
    }

    #[must_use]
    pub fn definitions(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
    ) -> Vec<api::ToolDefinition> {
        let builtin = self
            .include_builtin_tools
            .then(mvp_tool_specs)
            .into_iter()
            .flatten()
            .filter(|spec| {
                (!self.simple_mode || SIMPLE_MODE_TOOLS.contains(&spec.name))
                    && allowed_tools.is_none_or(|allowed| allowed.contains(spec.name))
            })
            .map(|spec| api::ToolDefinition {
                name: spec.name.to_string(),
                description: Some(spec.description.to_string()),
                input_schema: spec.input_schema,
            });
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                !self.simple_mode
                    && allowed_tools
                        .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| api::ToolDefinition {
                name: tool.definition().name.clone(),
                description: tool.definition().description.clone(),
                input_schema: tool.definition().input_schema.clone(),
            });
        builtin.chain(plugin).collect()
    }

    pub fn permission_specs(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
    ) -> Result<Vec<(String, PermissionMode)>, String> {
        let builtin = self
            .include_builtin_tools
            .then(mvp_tool_specs)
            .into_iter()
            .flatten()
            .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
            .map(|spec| (spec.name.to_string(), spec.required_permission));
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                allowed_tools
                    .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| {
                permission_mode_from_plugin(tool.required_permission())
                    .map(|permission| (tool.definition().name.clone(), permission))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(builtin.chain(plugin).collect())
    }

    pub fn execute(&self, name: &str, input: &Value) -> Result<String, String> {
        if self.include_builtin_tools && mvp_tool_specs().iter().any(|spec| spec.name == name) {
            return crate::dispatch::execute_tool(name, input);
        }
        self.plugin_tools
            .iter()
            .find(|tool| tool.definition().name == name)
            .ok_or_else(|| format!("unsupported tool: {name}"))?
            .execute(input)
            .map_err(|error| error.to_string())
    }
}

pub(crate) fn normalize_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

pub(crate) fn permission_mode_from_plugin(value: &str) -> Result<PermissionMode, String> {
    match value {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "plan" => Ok(PermissionMode::Plan),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        other => Err(format!("unsupported plugin permission: {other}")),
    }
}
