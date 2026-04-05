fn load_setup_context(
    mode: SetupMode,
    model_override: Option<&str>,
    profile_override: Option<&str>,
    permission_mode: PermissionMode,
    session_id: Option<&str>,
) -> Result<SetupContext, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let discovered_entries = loader.discover();
    let runtime_config = loader.load()?;
    let active_profile =
        ProfileResolver::resolve(&runtime_config, profile_override, model_override)?;
    let project_context = ProjectContext::discover_with_git(&cwd, DEFAULT_DATE)?;
    let git_root = find_git_root_in(&cwd).ok();
    let project_root = git_root.clone().unwrap_or_else(|| cwd.clone());
    let config_home = loader.config_home().to_path_buf();
    let session_dir = resolve_setup_session_dir(&cwd, &runtime_config);
    let oauth_credentials_present = runtime::load_oauth_credentials()?.is_some();
    let legacy_paths =
        collect_legacy_paths(&discovered_entries, &project_context.instruction_files);
    let resolved_config = ResolvedConfig {
        config_home: config_home.clone(),
        session_dir,
        discovered_entries,
        loaded_entries: runtime_config.loaded_entries().to_vec(),
        config_file_present: runtime_config.loaded_entries().iter().any(|entry| {
            entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "config.toml")
        }),
        model: active_profile.model.clone(),
        base_url: active_profile.base_url.clone(),
        api_key_env: active_profile.credential.env_name.clone(),
        api_key_present: active_profile.credential.api_key.is_some(),
        oauth_credentials_present,
        profile: Some(active_profile.profile_name.clone()),
        legacy_paths,
    };
    let trust_policy = TrustPolicyContext {
        permission_mode: permission_mode.as_str().to_string(),
        workspace_writeable: path_or_parent_writeable(&cwd),
        config_home_writeable: path_or_parent_writeable(&config_home),
        trusted_workspace: path_or_parent_writeable(&cwd),
    };

    Ok(SetupContext {
        inputs: BootstrapInputs {
            argv: env::args().collect(),
            cwd: cwd.clone(),
            platform: env::consts::OS.to_string(),
            stdio_mode: current_stdio_mode(),
            invocation_kind: mode,
        },
        session_id: session_id.map(ToOwned::to_owned),
        cwd,
        project_root,
        git_root,
        resolved_config,
        active_profile,
        trust_policy,
        mode,
    })
}

fn resolve_effective_model(
    profile_override: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    let active_profile = ProfileResolver::resolve(&runtime_config, profile_override, None)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(active_profile.model)
}

fn current_stdio_mode() -> StdioMode {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        StdioMode::Interactive
    } else {
        StdioMode::NonInteractive
    }
}

fn read_non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn config_string_value(runtime_config: &runtime::RuntimeConfig, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| runtime_config.get(key))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn resolve_setup_session_dir(cwd: &Path, runtime_config: &runtime::RuntimeConfig) -> PathBuf {
    env::var_os(PRIMARY_SESSION_DIR_ENV)
        .map(PathBuf::from)
        .or_else(|| env::var_os(LEGACY_SESSION_DIR_ENV).map(PathBuf::from))
        .or_else(|| {
            config_string_value(runtime_config, &["session_dir", "sessionDir"]).map(|value| {
                let path = PathBuf::from(value);
                if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                }
            })
        })
        .unwrap_or_else(|| cwd.join(PRIMARY_CONFIG_DIR_NAME).join("sessions"))
}

fn collect_legacy_paths(
    discovered_entries: &[runtime::ConfigEntry],
    instruction_files: &[runtime::ContextFile],
) -> Vec<PathBuf> {
    let mut legacy_paths = discovered_entries
        .iter()
        .map(|entry| entry.path.clone())
        .filter(|path| {
            let rendered = path.display().to_string();
            rendered.contains(".claw") || rendered.contains(".claude")
        })
        .collect::<Vec<_>>();

    for file in instruction_files {
        let rendered = file.path.display().to_string();
        if (rendered.contains(".claw")
            || rendered.contains(".claude")
            || rendered.ends_with("CLAUDE.md"))
            && !legacy_paths.iter().any(|path| path == &file.path)
        {
            legacy_paths.push(file.path.clone());
        }
    }

    legacy_paths
}

fn path_or_parent_writeable(path: &Path) -> bool {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return runtime::is_path_effectively_writeable(candidate);
        }
        current = candidate.parent();
    }
    false
}

fn has_explicit_bootstrap_inputs(setup: &SetupContext) -> bool {
    setup.resolved_config.config_file_present
        || setup.resolved_config.base_url.is_some()
        || setup.resolved_config.api_key_present
        || !matches!(
            setup.active_profile.profile_source,
            ResolutionSource::ProfileDefault
        )
}

fn ensure_setup_ready_for_runtime(setup: &SetupContext) -> Result<(), Box<dyn std::error::Error>> {
    if !has_explicit_bootstrap_inputs(setup) {
        return Err(format!(
            "Kcode is not initialized yet.\nRun `{CLI_NAME} init` to create `~/.kcode/config.toml`, then run `{CLI_NAME} doctor`."
        )
        .into());
    }
    if setup
        .resolved_config
        .base_url
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err(format!(
            "missing base URL.\nSet `{PRIMARY_BASE_URL_ENV}` or `base_url` in `~/.kcode/config.toml`, then rerun `{CLI_NAME} doctor`."
        )
        .into());
    }
    if !setup.resolved_config.api_key_present {
        return Err(format!(
            "missing API credentials.\nSet `{PRIMARY_API_KEY_ENV}` or the env named by `api_key_env`, then rerun `{CLI_NAME} doctor`."
        )
        .into());
    }
    if !path_or_parent_writeable(&setup.resolved_config.session_dir) {
        return Err(format!(
            "session directory is not writeable: {}\nAdjust `session_dir` or `{PRIMARY_SESSION_DIR_ENV}` before continuing.",
            setup.resolved_config.session_dir.display()
        )
        .into());
    }
    Ok(())
}

fn format_compact_report(removed: usize, resulting_messages: usize, skipped: bool) -> String {
    if skipped {
        format!(
            "Compact
  Result           skipped
  Reason           session below compaction threshold
  Messages kept    {resulting_messages}"
        )
    } else {
        format!(
            "Compact
  Result           compacted
  Messages removed {removed}
  Messages kept    {resulting_messages}"
        )
    }
}

fn format_auto_compaction_notice(removed: usize) -> String {
    format!("[auto-compacted: removed {removed} messages]")
}

fn parse_git_status_metadata(status: Option<&str>) -> (Option<PathBuf>, Option<String>) {
    parse_git_status_metadata_for(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        status,
    )
}

fn parse_git_status_branch(status: Option<&str>) -> Option<String> {
    let status = status?;
    let first_line = status.lines().next()?;
    let line = first_line.strip_prefix("## ")?;
    if line.starts_with("HEAD") {
        return Some("detached HEAD".to_string());
    }
    let branch = line.split(['.', ' ']).next().unwrap_or_default().trim();
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

fn parse_git_workspace_summary(status: Option<&str>) -> GitWorkspaceSummary {
    let mut summary = GitWorkspaceSummary::default();
    let Some(status) = status else {
        return summary;
    };

    for line in status.lines() {
        if line.starts_with("## ") || line.trim().is_empty() {
            continue;
        }

        summary.changed_files += 1;
        let mut chars = line.chars();
        let index_status = chars.next().unwrap_or(' ');
        let worktree_status = chars.next().unwrap_or(' ');

        if index_status == '?' && worktree_status == '?' {
            summary.untracked_files += 1;
            continue;
        }

        if index_status != ' ' {
            summary.staged_files += 1;
        }
        if worktree_status != ' ' {
            summary.unstaged_files += 1;
        }
        if (matches!(index_status, 'U' | 'A') && matches!(worktree_status, 'U' | 'A'))
            || index_status == 'U'
            || worktree_status == 'U'
        {
            summary.conflicted_files += 1;
        }
    }

    summary
}

fn resolve_git_branch_for(cwd: &Path) -> Option<String> {
    let branch = run_git_capture_in(cwd, &["branch", "--show-current"])?;
    let branch = branch.trim();
    if !branch.is_empty() {
        return Some(branch.to_string());
    }

    let fallback = run_git_capture_in(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let fallback = fallback.trim();
    if fallback.is_empty() {
        None
    } else if fallback == "HEAD" {
        Some("detached HEAD".to_string())
    } else {
        Some(fallback.to_string())
    }
}

fn run_git_capture_in(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn find_git_root_in(cwd: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        return Err("not a git repository".into());
    }
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    if path.is_empty() {
        return Err("empty git root".into());
    }
    Ok(PathBuf::from(path))
}

fn parse_git_status_metadata_for(
    cwd: &Path,
    status: Option<&str>,
) -> (Option<PathBuf>, Option<String>) {
    let branch = resolve_git_branch_for(cwd).or_else(|| parse_git_status_branch(status));
    let project_root = find_git_root_in(cwd).ok();
    (project_root, branch)
}
