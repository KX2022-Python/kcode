fn render_profile_report(
    selection: &ProfileCommandSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    let setup = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    )?;

    match selection {
        ProfileCommandSelection::List => {
            let names = ProfileResolver::available_profile_names(&runtime_config);
            let mut lines = vec![
                "Profile".to_string(),
                format!("  Active profile    {}", setup.active_profile.profile_name),
                format!(
                    "  Selected via      {}",
                    setup.active_profile.profile_source.label()
                ),
                format!(
                    "  Launch ready      {}",
                    if ProviderLauncher::prepare(&setup.active_profile).is_ok() {
                        "yes"
                    } else {
                        "no"
                    }
                ),
                format!("  Known profiles    {}", names.len()),
                String::new(),
                "Profiles".to_string(),
            ];
            for name in names {
                match ProfileResolver::resolve_named(&runtime_config, &name, None) {
                    Ok(profile) => {
                        let marker = if profile.profile_name == setup.active_profile.profile_name {
                            "*"
                        } else {
                            " "
                        };
                        lines.push(format!(
                            "  {marker} {name:<12} key={key:<18} model={model:<24} tools={tools} stream={stream}",
                            name = profile.profile_name,
                            key = profile.credential.env_name,
                            model = profile.model,
                            tools = profile.profile.supports_tools,
                            stream = profile.profile.supports_streaming,
                        ));
                    }
                    Err(error) => lines.push(format!("    {name:<12} error={error}")),
                }
            }
            Ok(lines.join("\n"))
        }
        ProfileCommandSelection::Show { profile_name: None } => {
            Ok(render_active_profile_report(&setup))
        }
        ProfileCommandSelection::Show {
            profile_name: Some(name),
        } if name.eq_ignore_ascii_case(&setup.active_profile.profile_name) => {
            Ok(render_active_profile_report(&setup))
        }
        ProfileCommandSelection::Show {
            profile_name: Some(name),
        } => {
            let resolved = ProfileResolver::resolve_named(&runtime_config, name, model_override)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            Ok(render_resolved_profile_report(&resolved))
        }
    }
}

fn render_config_report(
    section: Option<&str>,
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
    let loader = ConfigLoader::default_for(&setup.cwd);
    let discovered = loader.discover();
    let runtime_config = loader.load()?;

    let mut lines = vec![
        format!(
            "Config
  Working directory {}
  Config home      {}
  Session dir      {}
  Effective profile {}
  Effective model  {}
  Loaded files     {}
  Merged keys      {}",
            setup.cwd.display(),
            setup.resolved_config.config_home.display(),
            setup.resolved_config.session_dir.display(),
            setup.active_profile.profile_name,
            setup.resolved_config.model,
            runtime_config.loaded_entries().len(),
            runtime_config.merged().len()
        ),
        "Discovered files".to_string(),
    ];
    for entry in discovered {
        let source = match entry.source {
            ConfigSource::User => "user",
            ConfigSource::Project => "project",
            ConfigSource::Local => "local",
            ConfigSource::Managed => "managed",
        };
        let status = if runtime_config
            .loaded_entries()
            .iter()
            .any(|loaded_entry| loaded_entry.path == entry.path)
        {
            "loaded"
        } else {
            "missing"
        };
        lines.push(format!(
            "  {source:<7} {status:<7} {}",
            entry.path.display()
        ));
    }

    if let Some(section) = section {
        lines.push(format!("Merged section: {section}"));
        match section {
            "env" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("env")
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "hooks" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("hooks")
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "model" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("model")
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "plugins" => lines.push(format!(
                "  {}",
                runtime_config
                    .get("plugins")
                    .or_else(|| runtime_config.get("enabledPlugins"))
                    .map_or_else(|| "<unset>".to_string(), |value| value.render())
            )),
            "profile" => {
                lines.extend(
                    render_active_profile_report(&setup)
                        .lines()
                        .skip(1)
                        .map(|line| format!("  {line}")),
                );
            }
            "provider" => match ProviderLauncher::prepare(&setup.active_profile) {
                Ok(launch) => {
                    lines.push(format!("  Profile          {}", launch.profile_name));
                    lines.push(format!("  Provider         {}", launch.provider_label));
                    lines.push(format!("  Base URL         {}", launch.base_url));
                    lines.push(format!("  Model            {}", launch.model));
                    lines.push(format!("  Timeout ms       {}", launch.request_timeout_ms));
                    lines.push(format!("  Max retries      {}", launch.max_retries));
                    lines.push(format!("  Supports tools   {}", launch.supports_tools));
                    lines.push(format!("  Supports stream  {}", launch.supports_streaming));
                }
                Err(error) => lines.push(format!("  Launch error     {error}")),
            },
            other => {
                lines.push(format!(
                    "  Unsupported config section '{other}'. Use env, hooks, model, plugins, profile, or provider."
                ));
                return Ok(lines.join(
                    "
",
                ));
            }
        }
        return Ok(lines.join(
            "
",
        ));
    }

    lines.push("Merged JSON".to_string());
    lines.push(format!("  {}", runtime_config.as_json().render()));
    Ok(lines.join(
        "
",
    ))
}

fn render_memory_report() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let dir = default_memory_dir();
    ensure_memory_dir(&dir)?;
    ensure_memory_index(&dir.join("MEMORY.md"))?;

    let entries = list_memories(&dir)?;
    let summary = render_memory_summary(&entries);

    let project_context = ProjectContext::discover(&cwd, DEFAULT_DATE)?;
    let mut lines = vec![summary];

    if !project_context.instruction_files.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Project instruction files ({}):",
            project_context.instruction_files.len()
        ));
        for (index, file) in project_context.instruction_files.iter().enumerate() {
            let preview = file.content.lines().next().unwrap_or("").trim();
            let preview = if preview.is_empty() {
                "<empty>"
            } else {
                preview
            };
            lines.push(format!(
                "  {}. {} (lines={}, preview={})",
                index + 1,
                file.path.display(),
                file.content.lines().count(),
                preview
            ));
        }
    }

    Ok(lines.join("\n"))
}

