    #[test]
    fn defaults_to_repl_when_no_args() {
        let permission_mode = super::default_permission_mode();
        assert_eq!(
            parse_args(&[]).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn default_permission_mode_uses_project_config_when_env_is_unset() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn env_permission_mode_overrides_project_config_default() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", "read-only");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::ReadOnly);
    }

    #[test]
    fn parses_prompt_subcommand() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "prompt".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn parses_bare_prompt_and_json_output_flag() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "--output-format=json".to_string(),
            "--model".to_string(),
            "claude-opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "claude-opus".to_string(),
                model_explicit: true,
                profile: None,
                output_format: CliOutputFormat::Json,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn resolves_model_aliases_in_args() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "--model".to_string(),
            "opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "gpt-4.1".to_string(),
                model_explicit: true,
                profile: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode,
            }
        );
    }

    #[test]
    fn resolves_known_model_aliases() {
        assert_eq!(resolve_model_alias("opus"), "gpt-4.1");
        assert_eq!(resolve_model_alias("sonnet"), "gpt-4.1");
        assert_eq!(resolve_model_alias("haiku"), "gpt-4.1-mini");
        assert_eq!(resolve_model_alias("claude-opus"), "claude-opus");
    }

    #[test]
    fn parses_version_flags_without_initializing_prompt_mode() {
        assert_eq!(
            parse_args(&["--version".to_string()]).expect("args should parse"),
            CliAction::Version
        );
        assert_eq!(
            parse_args(&["-V".to_string()]).expect("args should parse"),
            CliAction::Version
        );
    }

    #[test]
    fn parses_permission_mode_flag() {
        let args = vec!["--permission-mode=read-only".to_string()];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                allowed_tools: None,
                permission_mode: PermissionMode::ReadOnly,
            }
        );
    }

    #[test]
    fn parses_allowed_tools_flags_with_aliases_and_lists() {
        let permission_mode = super::default_permission_mode();
        let args = vec![
            "--allowedTools".to_string(),
            "read,glob".to_string(),
            "--allowed-tools=write_file".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                allowed_tools: Some(
                    ["glob_search", "read_file", "write_file"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                ),
                permission_mode,
            }
        );
    }

    #[test]
    fn rejects_allowed_tools_when_active_profile_disables_tools() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(&cwd).expect("cwd should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            config_home.join("config.toml"),
            r#"
profile = "bridge"

[profiles.bridge]
default_model = "gpt-4.1-mini"
base_url_env = "BRIDGE_BASE_URL"
api_key_env = "BRIDGE_API_KEY"
supports_tools = false
supports_streaming = false
"#,
        )
        .expect("config should write");

        let original_config_home = std::env::var("KCODE_CONFIG_HOME").ok();
        let original_profile = std::env::var("KCODE_PROFILE").ok();
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);
        std::env::remove_var("KCODE_PROFILE");

        let error = with_current_dir(&cwd, || {
            parse_args(&["--allowedTools".to_string(), "read".to_string()])
        })
        .expect_err("tool-less profile should reject allowed tools");

        match original_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }
        match original_profile {
            Some(value) => std::env::set_var("KCODE_PROFILE", value),
            None => std::env::remove_var("KCODE_PROFILE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert!(error.contains("`--allowedTools` is unavailable"));
        assert!(error.contains("active profile `bridge`"));
    }

    #[test]
    fn allowed_tools_use_cli_profile_override_when_default_profile_is_toolless() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(&cwd).expect("cwd should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            config_home.join("config.toml"),
            r#"
profile = "bridge"

[profiles.bridge]
default_model = "gpt-4.1-mini"
base_url_env = "BRIDGE_BASE_URL"
api_key_env = "BRIDGE_API_KEY"
supports_tools = false
supports_streaming = false
"#,
        )
        .expect("config should write");

        let original_config_home = std::env::var("KCODE_CONFIG_HOME").ok();
        let original_profile = std::env::var("KCODE_PROFILE").ok();
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);
        std::env::remove_var("KCODE_PROFILE");

        let permission_mode = with_current_dir(&cwd, super::default_permission_mode);
        let action = with_current_dir(&cwd, || {
            parse_args(&[
                "--profile".to_string(),
                "cliproxyapi".to_string(),
                "--allowedTools".to_string(),
                "read".to_string(),
            ])
        })
        .expect("tool-capable profile should accept allowed tools");

        match original_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }
        match original_profile {
            Some(value) => std::env::set_var("KCODE_PROFILE", value),
            None => std::env::remove_var("KCODE_PROFILE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(
            action,
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: Some("cliproxyapi".to_string()),
                allowed_tools: Some(["read_file"].into_iter().map(str::to_string).collect()),
                permission_mode,
            }
        );
    }

    #[test]
    fn rejects_unknown_allowed_tools() {
        let error = parse_args(&[
            "--profile".to_string(),
            "cliproxyapi".to_string(),
            "--allowedTools".to_string(),
            "teleport".to_string(),
        ])
        .expect_err("tool should be rejected");
        assert!(error.contains("unsupported tool in --allowedTools: teleport"));
    }

    #[test]
    fn parses_system_prompt_options() {
        let args = vec![
            "system-prompt".to_string(),
            "--cwd".to_string(),
            "/tmp/project".to_string(),
            "--date".to_string(),
            "2026-04-01".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::PrintSystemPrompt {
                cwd: PathBuf::from("/tmp/project"),
                date: "2026-04-01".to_string(),
            }
        );
    }
