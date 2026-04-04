#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessCommandSpec {
    pub(crate) name: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) summary: &'static str,
    pub(crate) argument_hint: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashCommandSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub summary: &'static str,
    pub argument_hint: Option<&'static str>,
    pub resume_supported: bool,
}

pub(crate) use crate::process_specs::PROCESS_COMMAND_SPECS;
use crate::session_specs::SLASH_COMMAND_SPECS;

#[must_use]
pub fn slash_command_specs() -> &'static [SlashCommandSpec] {
    SLASH_COMMAND_SPECS
}
