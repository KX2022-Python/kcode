use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::SlashCommand;

pub(crate) fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("commands-plugin-{label}-{nanos}"))
}

pub(crate) fn write_external_plugin(root: &Path, name: &str, version: &str) {
    fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
    fs::write(
        root.join(".claude-plugin").join("plugin.json"),
        format!(
            "{{\n  \"name\": \"{name}\",\n  \"version\": \"{version}\",\n  \"description\": \"commands plugin\"\n}}"
        ),
    )
    .expect("write manifest");
}

pub(crate) fn write_bundled_plugin(root: &Path, name: &str, version: &str, default_enabled: bool) {
    fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
    fs::write(
        root.join(".claude-plugin").join("plugin.json"),
        format!(
            "{{\n  \"name\": \"{name}\",\n  \"version\": \"{version}\",\n  \"description\": \"bundled commands plugin\",\n  \"defaultEnabled\": {}\n}}",
            if default_enabled { "true" } else { "false" }
        ),
    )
    .expect("write bundled manifest");
}

pub(crate) fn write_agent(
    root: &Path,
    name: &str,
    description: &str,
    model: &str,
    reasoning: &str,
) {
    fs::create_dir_all(root).expect("agent root");
    fs::write(
        root.join(format!("{name}.toml")),
        format!(
            "name = \"{name}\"\ndescription = \"{description}\"\nmodel = \"{model}\"\nmodel_reasoning_effort = \"{reasoning}\"\n"
        ),
    )
    .expect("write agent");
}

pub(crate) fn write_skill(root: &Path, name: &str, description: &str) {
    let skill_root = root.join(name);
    fs::create_dir_all(&skill_root).expect("skill root");
    fs::write(
        skill_root.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
    )
    .expect("write skill");
}

pub(crate) fn write_legacy_command(root: &Path, name: &str, description: &str) {
    fs::create_dir_all(root).expect("commands root");
    fs::write(
        root.join(format!("{name}.md")),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
    )
    .expect("write command");
}

pub(crate) fn parse_error_message(input: &str) -> String {
    SlashCommand::parse(input)
        .expect_err("slash command should be rejected")
        .to_string()
}
