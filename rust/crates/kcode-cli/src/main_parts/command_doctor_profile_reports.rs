fn render_commands_report(
    surface: CommandReportSurfaceSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let setup = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;
    let snapshot = build_command_registry_snapshot(&command_registry_context(&setup, surface), &[]);
    let actionable_filtered = snapshot
        .filtered_out_commands
        .iter()
        .filter(|command| command.reason != "disabled")
        .collect::<Vec<_>>();

    let mut lines = vec![
        "Commands".to_string(),
        format!("  Active profile    {}", setup.active_profile.profile_name),
        format!(
            "  Selected via      {}",
            setup.active_profile.profile_source.label()
        ),
        format!("  Surface           {}", surface.label()),
        format!("  Safety profile    {}", snapshot.safety_profile),
        format!(
            "  Supports tools    {}",
            setup.active_profile.profile.supports_tools
        ),
        format!(
            "  Supports stream   {}",
            setup.active_profile.profile.supports_streaming
        ),
        format!("  Process commands  {}", snapshot.process_commands.len()),
        format!("  Session commands  {}", snapshot.session_commands.len()),
        format!("  Filtered commands {}", actionable_filtered.len()),
    ];

    lines.push("Process commands".to_string());
    for descriptor in &snapshot.process_commands {
        lines.push(format!(
            "  {:<34} {}",
            command_descriptor_usage(descriptor),
            descriptor.description
        ));
    }

    lines.push(String::new());
    lines.push("Session commands".to_string());
    for descriptor in &snapshot.session_commands {
        lines.push(format!(
            "  {:<34} {}",
            command_descriptor_usage(descriptor),
            descriptor.description
        ));
    }

    if !actionable_filtered.is_empty() {
        lines.push(String::new());
        lines.push("Filtered".to_string());
        for filtered in actionable_filtered {
            lines.push(format!(
                "  {:<34} {}",
                filtered_command_usage(filtered),
                filtered.reason
            ));
        }
    }

    Ok(lines.join("\n"))
}

fn render_doctor_report(
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let setup = load_setup_context(
        SetupMode::Doctor,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;
    Ok(render_doctor_report_from_setup(&setup))
}

fn render_doctor_report_from_setup(setup: &SetupContext) -> String {
    let checks = doctor_checks(setup);
    let runtime_ready = !checks
        .iter()
        .any(|check| check.status == DiagnosticStatus::Fail);
    let mut lines = vec![format!(
        "Doctor
  Working directory {}
  Config home      {}
  Session dir      {}
  Active profile   {}
  Runtime ready    {}",
        setup.cwd.display(),
        setup.resolved_config.config_home.display(),
        setup.resolved_config.session_dir.display(),
        setup.active_profile.profile_name,
        if runtime_ready { "yes" } else { "no" }
    )];

    lines.push("Checks".to_string());
    for check in checks {
        lines.push(format!(
            "  [{:<4}] {:<16} {}",
            check.status.label(),
            check.name,
            check.detail
        ));
    }

    lines.push(format!(
        "Next step        {}",
        doctor_next_step(setup, runtime_ready)
    ));
    lines.join("\n")
}

fn doctor_checks(setup: &SetupContext) -> Vec<DiagnosticCheck> {
    let config_file_path = setup.resolved_config.config_home.join("config.toml");
    let loaded_config_path = setup
        .resolved_config
        .loaded_entries
        .iter()
        .find(|entry| {
            entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "config.toml")
        })
        .map(|entry| entry.path.clone())
        .unwrap_or(config_file_path);

    let credentials_path = runtime::credentials_path().ok();
    let credential_detail = if setup.resolved_config.api_key_present {
        DiagnosticCheck {
            name: "api credentials".to_string(),
            status: DiagnosticStatus::Ok,
            detail: format!(
                "env `{}` is available ({})",
                setup.active_profile.credential.env_name,
                setup.active_profile.credential.source.label()
            ),
        }
    } else if setup.resolved_config.oauth_credentials_present {
        DiagnosticCheck {
            name: "api credentials".to_string(),
            status: DiagnosticStatus::Warn,
            detail: format!(
                "legacy OAuth credentials detected{}; provider profiles ignore OAuth",
                credentials_path
                    .as_ref()
                    .map(|path| format!(" at {}", path.display()))
                    .unwrap_or_default()
            ),
        }
    } else {
        DiagnosticCheck {
            name: "api credentials".to_string(),
            status: DiagnosticStatus::Fail,
            detail: format!(
                "unset; export `{}` or `{}`",
                PRIMARY_API_KEY_ENV, setup.active_profile.credential.env_name
            ),
        }
    };

    let legacy_detail = if setup.resolved_config.legacy_paths.is_empty() {
        "none detected".to_string()
    } else {
        setup
            .resolved_config
            .legacy_paths
            .iter()
            .take(3)
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    vec![
        DiagnosticCheck {
            name: "config file".to_string(),
            status: if setup.resolved_config.config_file_present {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Fail
            },
            detail: if setup.resolved_config.config_file_present {
                format!("loaded {}", loaded_config_path.display())
            } else {
                format!(
                    "missing {}; run `{CLI_NAME} init` first",
                    loaded_config_path.display()
                )
            },
        },
        DiagnosticCheck {
            name: "profile".to_string(),
            status: DiagnosticStatus::Ok,
            detail: format!(
                "{} ({})",
                setup.active_profile.profile_name,
                setup.active_profile.profile_source.label()
            ),
        },
        DiagnosticCheck {
            name: "model".to_string(),
            status: DiagnosticStatus::Ok,
            detail: format!(
                "{} ({})",
                setup.resolved_config.model,
                setup.active_profile.model_source.label()
            ),
        },
        DiagnosticCheck {
            name: "tool capability".to_string(),
            status: DiagnosticStatus::Ok,
            detail: if setup.active_profile.profile.supports_tools {
                "enabled by active profile".to_string()
            } else {
                format!(
                    "disabled by active profile `{}`; tool-capable commands stay hidden",
                    setup.active_profile.profile_name
                )
            },
        },
        DiagnosticCheck {
            name: "base url".to_string(),
            status: if setup
                .resolved_config
                .base_url
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Fail
            },
            detail: setup
                .resolved_config
                .base_url
                .clone()
                .filter(|value| !value.trim().is_empty())
                .map(|value| format!("{value} ({})", setup.active_profile.base_url_source.label()))
                .unwrap_or_else(|| {
                    format!(
                        "unset; set `{PRIMARY_BASE_URL_ENV}` or `base_url` in `~/.kcode/config.toml`"
                    )
                }),
        },
        credential_detail,
        DiagnosticCheck {
            name: "session dir".to_string(),
            status: if path_or_parent_writeable(&setup.resolved_config.session_dir) {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Fail
            },
            detail: if path_or_parent_writeable(&setup.resolved_config.session_dir) {
                format!("writeable {}", setup.resolved_config.session_dir.display())
            } else {
                format!(
                    "not writeable {}; adjust `session_dir` or `{PRIMARY_SESSION_DIR_ENV}`",
                    setup.resolved_config.session_dir.display()
                )
            },
        },
        DiagnosticCheck {
            name: "permission mode".to_string(),
            status: DiagnosticStatus::Ok,
            detail: setup.trust_policy.permission_mode.clone(),
        },
        DiagnosticCheck {
            name: "legacy residue".to_string(),
            status: if setup.resolved_config.legacy_paths.is_empty() {
                DiagnosticStatus::Ok
            } else {
                DiagnosticStatus::Warn
            },
            detail: legacy_detail,
        },
    ]
}

fn doctor_next_step(setup: &SetupContext, runtime_ready: bool) -> String {
    if !setup.resolved_config.config_file_present {
        return format!(
            "run `{CLI_NAME} init`, fill `config.toml`, then rerun `{CLI_NAME} doctor`"
        );
    }
    if setup
        .resolved_config
        .base_url
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return format!(
            "set `{PRIMARY_BASE_URL_ENV}` or `base_url` in `~/.kcode/config.toml`, then rerun `{CLI_NAME} doctor`"
        );
    }
    if !setup.resolved_config.api_key_present {
        return format!(
            "export `{PRIMARY_API_KEY_ENV}` or the env named by `api_key_env`, then rerun `{CLI_NAME} doctor`"
        );
    }
    if !path_or_parent_writeable(&setup.resolved_config.session_dir) {
        return format!(
            "fix `session_dir` or `{PRIMARY_SESSION_DIR_ENV}` so sessions can be written"
        );
    }
    if runtime_ready {
        return format!("start `{CLI_NAME}` or run `{CLI_NAME} -p \"hello\"`");
    }
    "review warnings above before starting interactive sessions".to_string()
}

fn render_resolved_profile_report(profile: &ResolvedProviderProfile) -> String {
    let launch = ProviderLauncher::prepare(profile);
    let credential_detail = if profile.credential.api_key.is_some() {
        format!(
            "present via {} ({})",
            profile.credential.env_name,
            profile.credential.source.label()
        )
    } else {
        format!("missing {}", profile.credential.env_name)
    };

    let mut lines = vec![
        "Profile".to_string(),
        format!("  Name              {}", profile.profile_name),
        format!("  Selected via      {}", profile.profile_source.label()),
        format!("  Model             {}", profile.model),
        format!("  Model source      {}", profile.model_source.label()),
        format!(
            "  Base URL          {}",
            profile
                .base_url
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<unset>")
        ),
        format!("  Base URL source   {}", profile.base_url_source.label()),
        format!("  Base URL env      {}", profile.profile.base_url_env),
        format!("  API key env       {}", profile.credential.env_name),
        format!("  Credential        {credential_detail}"),
        format!("  Default model     {}", profile.profile.default_model),
        format!("  Supports tools    {}", profile.profile.supports_tools),
        format!("  Supports stream   {}", profile.profile.supports_streaming),
        format!("  Timeout ms        {}", profile.profile.request_timeout_ms),
        format!("  Max retries       {}", profile.profile.max_retries),
        format!(
            "  Launch ready      {}",
            if launch.is_ok() { "yes" } else { "no" }
        ),
    ];
    if let Err(error) = launch {
        lines.push(format!("  Launch detail     {error}"));
    }
    lines.join("\n")
}

fn render_active_profile_report(setup: &SetupContext) -> String {
    render_resolved_profile_report(&setup.active_profile)
}
