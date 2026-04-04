use crate::manifest::ToolSpec;

#[must_use]
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    let mut specs = crate::core_specs::core_tool_specs();
    specs.extend(crate::extended_specs::extended_tool_specs());
    specs
}
