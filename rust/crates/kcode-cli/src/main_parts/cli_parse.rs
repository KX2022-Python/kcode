#[allow(clippy::too_many_lines)]
fn parse_args(args: &[String]) -> Result<CliAction, String> {
    let mut model = DEFAULT_MODEL.to_string();
    let mut model_explicit = false;
    let mut profile = None;
    let mut output_format = CliOutputFormat::Text;
    let mut permission_mode = default_permission_mode();
    let mut wants_help = false;
    let mut wants_version = false;
    let mut allowed_tool_values = Vec::new();
    let mut rest = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" if rest.is_empty() => {
                wants_help = true;
                index += 1;
            }
            "--version" | "-V" => {
                wants_version = true;
                index += 1;
            }
            "--model" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --model".to_string())?;
                model = resolve_model_alias(value).to_string();
                model_explicit = true;
                index += 2;
            }
            "--profile" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --profile".to_string())?;
                profile = Some(value.trim().to_string());
                index += 2;
            }
            flag if flag.starts_with("--model=") => {
                model = resolve_model_alias(&flag[8..]).to_string();
                model_explicit = true;
                index += 1;
            }
            flag if flag.starts_with("--profile=") => {
                profile = Some(flag[10..].trim().to_string());
                index += 1;
            }
            "--output-format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --output-format".to_string())?;
                output_format = CliOutputFormat::parse(value)?;
                index += 2;
            }
            "--permission-mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --permission-mode".to_string())?;
                permission_mode = parse_permission_mode_arg(value)?;
                index += 2;
            }
            flag if flag.starts_with("--output-format=") => {
                output_format = CliOutputFormat::parse(&flag[16..])?;
                index += 1;
            }
            flag if flag.starts_with("--permission-mode=") => {
                permission_mode = parse_permission_mode_arg(&flag[18..])?;
                index += 1;
            }
            "--dangerously-skip-permissions" => {
                permission_mode = PermissionMode::DangerFullAccess;
                index += 1;
            }
            "-p" => {
                let prompt = args[index + 1..].join(" ");
                if prompt.trim().is_empty() {
                    return Err("-p requires a prompt string".to_string());
                }
                return Ok(CliAction::Prompt {
                    prompt,
                    model: resolve_model_alias(&model).to_string(),
                    model_explicit,
                    profile: profile.clone(),
                    output_format,
                    allowed_tools: normalize_allowed_tools(
                        &allowed_tool_values,
                        profile.as_deref(),
                    )?,
                    permission_mode,
                });
            }
            "--print" => {
                output_format = CliOutputFormat::Text;
                index += 1;
            }
            "--resume" if rest.is_empty() => {
                rest.push("--resume".to_string());
                index += 1;
            }
            flag if rest.is_empty() && flag.starts_with("--resume=") => {
                rest.push("--resume".to_string());
                rest.push(flag[9..].to_string());
                index += 1;
            }
            "--allowedTools" | "--allowed-tools" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --allowedTools".to_string())?;
                allowed_tool_values.push(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--allowedTools=") => {
                allowed_tool_values.push(flag[15..].to_string());
                index += 1;
            }
            flag if flag.starts_with("--allowed-tools=") => {
                allowed_tool_values.push(flag[16..].to_string());
                index += 1;
            }
            other if rest.is_empty() && other.starts_with('-') => {
                return Err(format_unknown_option(other))
            }
            other => {
                rest.push(other.to_string());
                index += 1;
            }
        }
    }

    if wants_help {
        return Ok(CliAction::Help {
            profile: profile.clone(),
        });
    }

    if wants_version {
        return Ok(CliAction::Version);
    }

    let allowed_tools = normalize_allowed_tools(&allowed_tool_values, profile.as_deref())?;

    if rest.is_empty() {
        return Ok(CliAction::Repl {
            model,
            model_explicit,
            profile,
            allowed_tools,
            permission_mode,
        });
    }
    if rest.first().map(String::as_str) == Some("--resume") {
        return parse_resume_args(&rest[1..]);
    }
    if let Some(action) = parse_single_word_command_alias(
        &rest,
        &model,
        model_explicit,
        profile.as_deref(),
        permission_mode,
    ) {
        return action;
    }

    match rest[0].as_str() {
        "agents" => Ok(CliAction::Agents {
            args: join_optional_args(&rest[1..]),
        }),
        "mcp" => Ok(CliAction::Mcp {
            args: join_optional_args(&rest[1..]),
            profile,
        }),
        "skills" => Ok(CliAction::Skills {
            args: join_optional_args(&rest[1..]),
        }),
        "system-prompt" => parse_system_prompt_args(&rest[1..]),
        "doctor" => {
            let fix = rest.iter().any(|arg| arg == "--fix");
            Ok(CliAction::Doctor {
                model,
                model_explicit,
                profile,
                fix,
            })
        }
        "config" => parse_config_args(&rest[1..], &model, model_explicit, profile.clone()),
        "commands" => parse_commands_args(&rest[1..], &model, model_explicit, profile.clone()),
        "profile" => parse_profile_args(&rest[1..], &model, model_explicit, profile.clone()),
        "login" => Ok(CliAction::Login),
        "logout" => Ok(CliAction::Logout),
        "init" => Ok(CliAction::Init),
        "bridge" => Ok(CliAction::Bridge {
            model,
            model_explicit,
            profile,
            permission_mode,
        }),
        "prompt" => {
            let prompt = rest[1..].join(" ");
            if prompt.trim().is_empty() {
                return Err("prompt subcommand requires a prompt string".to_string());
            }
            Ok(CliAction::Prompt {
                prompt,
                model,
                model_explicit,
                profile,
                output_format,
                allowed_tools,
                permission_mode,
            })
        }
        other if other.starts_with('/') => parse_direct_slash_cli_action(&rest, profile.clone()),
        _ => Ok(CliAction::Prompt {
            prompt: rest.join(" "),
            model,
            model_explicit,
            profile,
            output_format,
            allowed_tools,
            permission_mode,
        }),
    }
}

fn parse_single_word_command_alias(
    rest: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<&str>,
    permission_mode: PermissionMode,
) -> Option<Result<CliAction, String>> {
    if rest.len() != 1 {
        return None;
    }

    match rest[0].as_str() {
        "help" => Some(Ok(CliAction::Help {
            profile: profile.map(ToOwned::to_owned),
        })),
        "version" => Some(Ok(CliAction::Version)),
        "doctor" => Some(Ok(CliAction::Doctor {
            model: model.to_string(),
            model_explicit,
            profile: profile.map(ToOwned::to_owned),
            fix: false,
        })),
        "profile" => Some(Ok(CliAction::Profile {
            selection: ProfileCommandSelection::Show { profile_name: None },
            model: model.to_string(),
            model_explicit,
            profile: profile.map(ToOwned::to_owned),
        })),
        "status" => Some(Ok(CliAction::Status {
            model: model.to_string(),
            model_explicit,
            profile: profile.map(ToOwned::to_owned),
            permission_mode,
        })),
        "sandbox" => Some(Ok(CliAction::Sandbox)),
        other => bare_slash_command_guidance(other).map(Err),
    }
}

fn bare_slash_command_guidance(command_name: &str) -> Option<String> {
    if matches!(
        command_name,
        "agents"
            | "mcp"
            | "skills"
            | "profile"
            | "system-prompt"
            | "doctor"
            | "config"
            | "login"
            | "logout"
            | "init"
            | "prompt"
    ) {
        return None;
    }
    let slash_command = slash_command_specs()
        .iter()
        .find(|spec| spec.name == command_name)?;
    let guidance = if slash_command.resume_supported {
        format!(
            "`{CLI_NAME} {command_name}` is a slash command. Use `{CLI_NAME} --resume SESSION.jsonl /{command_name}` or start `{CLI_NAME}` and run `/{command_name}`."
        )
    } else {
        format!(
            "`{CLI_NAME} {command_name}` is a slash command. Start `{CLI_NAME}` and run `/{command_name}` inside the REPL."
        )
    };
    Some(guidance)
}

fn parse_config_args(
    args: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<String>,
) -> Result<CliAction, String> {
    match args {
        [] => Ok(CliAction::ConfigShow {
            section: None,
            model: model.to_string(),
            model_explicit,
            profile,
        }),
        [subcommand] if subcommand == "show" => Ok(CliAction::ConfigShow {
            section: None,
            model: model.to_string(),
            model_explicit,
            profile,
        }),
        [subcommand, section] if subcommand == "show" => Ok(CliAction::ConfigShow {
            section: Some(section.clone()),
            model: model.to_string(),
            model_explicit,
            profile,
        }),
        _ => Err("usage: kcode config show [env|hooks|model|plugins|profile|provider]".to_string()),
    }
}

fn parse_profile_args(
    args: &[String],
    model: &str,
    model_explicit: bool,
    profile: Option<String>,
) -> Result<CliAction, String> {
    let selection = match args {
        [] => ProfileCommandSelection::Show { profile_name: None },
        [subcommand] if subcommand == "list" => ProfileCommandSelection::List,
        [subcommand] if subcommand == "show" => {
            ProfileCommandSelection::Show { profile_name: None }
        }
        [subcommand, name] if subcommand == "show" => ProfileCommandSelection::Show {
            profile_name: Some(name.clone()),
        },
        _ => return Err("usage: kcode profile [list|show [name]]".to_string()),
    };

    Ok(CliAction::Profile {
        selection,
        model: model.to_string(),
        model_explicit,
        profile,
    })
}
