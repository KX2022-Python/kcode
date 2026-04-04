use crate::help::render_slash_command_help_detail;
use crate::model::SlashCommandParseError;

pub(crate) fn normalize_optional_args(args: Option<&str>) -> Option<&str> {
    args.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) fn parse_list_or_help_args(
    command: &str,
    args: Option<String>,
) -> Result<Option<String>, SlashCommandParseError> {
    match normalize_optional_args(args.as_deref()) {
        None | Some("list" | "help" | "-h" | "--help") => Ok(args),
        Some(unexpected) => Err(command_error(
            &format!(
                "Unexpected arguments for /{command}: {unexpected}. Use /{command}, /{command} list, or /{command} help."
            ),
            command,
            &format!("/{command} [list|help]"),
        )),
    }
}

pub(crate) fn parse_skills_args(
    args: Option<&str>,
) -> Result<Option<String>, SlashCommandParseError> {
    let Some(args) = normalize_optional_args(args) else {
        return Ok(None);
    };

    if matches!(args, "list" | "help" | "-h" | "--help") {
        return Ok(Some(args.to_string()));
    }

    if args == "install" {
        return Err(command_error(
            "Usage: /skills install <path>",
            "skills",
            "/skills install <path>",
        ));
    }

    if let Some(target) = args.strip_prefix("install").map(str::trim) {
        if !target.is_empty() {
            return Ok(Some(format!("install {target}")));
        }
    }

    Err(command_error(
        &format!(
            "Unexpected arguments for /skills: {args}. Use /skills, /skills list, /skills install <path>, or /skills help."
        ),
        "skills",
        "/skills [list|install <path>|help]",
    ))
}

pub(crate) fn usage_error(command: &str, argument_hint: &str) -> SlashCommandParseError {
    let usage = format!("/{command} {argument_hint}");
    let usage = usage.trim_end().to_string();
    command_error(
        &format!("Usage: {usage}"),
        command_root_name(command),
        &usage,
    )
}

pub(crate) fn command_error(message: &str, command: &str, usage: &str) -> SlashCommandParseError {
    let detail = render_slash_command_help_detail(command)
        .map(|detail| format!("\n\n{detail}"))
        .unwrap_or_default();
    SlashCommandParseError::new(format!("{message}\n  Usage            {usage}{detail}"))
}

pub(crate) fn remainder_after_command(input: &str, command: &str) -> Option<String> {
    input
        .trim()
        .strip_prefix(&format!("/{command}"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn command_root_name(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or(command)
}
