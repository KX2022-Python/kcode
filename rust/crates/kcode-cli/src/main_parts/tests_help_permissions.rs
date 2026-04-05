    #[test]
    fn filtered_tool_specs_respect_allowlist() {
        let allowed = ["read_file", "grep_search"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let filtered = filter_tool_specs(&GlobalToolRegistry::builtin(), Some(&allowed));
        let names = filtered
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file", "grep_search"]);
    }

    #[test]
    fn filtered_tool_specs_include_plugin_tools() {
        let filtered = filter_tool_specs(&registry_with_plugin_tool(), None);
        let names = filtered
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"plugin_echo".to_string()));
    }

    #[test]
    fn permission_policy_uses_plugin_tool_permissions() {
        let feature_config = runtime::RuntimeFeatureConfig::default();
        let policy = permission_policy(
            PermissionMode::ReadOnly,
            &feature_config,
            &registry_with_plugin_tool(),
            true,
        )
        .expect("permission policy should build");
        let required = policy.required_mode_for("plugin_echo");
        assert_eq!(required, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn permission_policy_disables_tool_use_for_toolless_profiles() {
        let feature_config = runtime::RuntimeFeatureConfig::default();
        let policy = permission_policy(
            PermissionMode::DangerFullAccess,
            &feature_config,
            &registry_with_plugin_tool(),
            false,
        )
        .expect("permission policy should build");

        assert_eq!(
            policy.authorize("bash", "{}", None),
            PermissionOutcome::Deny {
                reason: "tool use is unavailable because the active profile disables tools"
                    .to_string(),
            }
        );
    }

    #[test]
    fn shared_help_uses_resume_annotation_copy() {
        let help = commands::render_slash_command_help();
        assert!(help.contains("Slash commands"));
        assert!(help.contains("works with --resume SESSION.jsonl"));
    }

    #[test]
    fn repl_help_includes_shared_commands_and_exit() {
        let help = render_repl_help();
        assert!(help.contains("REPL"));
        assert!(help.contains("/help"));
        assert!(help.contains("Complete commands, modes, and recent sessions"));
        assert!(help.contains("/status"));
        assert!(help.contains("/sandbox"));
        assert!(help.contains("/model [model]"));
        assert!(help.contains("/permissions [read-only|workspace-write|danger-full-access]"));
        assert!(help.contains("/clear [--confirm]"));
        assert!(help.contains("/cost"));
        assert!(help.contains("/resume <session-path>"));
        assert!(help.contains("/config [env|hooks|model|plugins]"));
        assert!(help.contains("/mcp [list|show <server>|help]"));
        assert!(help.contains("/memory"));
        assert!(help.contains("/init"));
        assert!(help.contains("/diff"));
        assert!(help.contains("/version"));
        assert!(help.contains("/export [file]"));
        assert!(help.contains("/session [list|switch <session-id>|fork [branch-name]]"));
        assert!(help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
        assert!(help.contains("aliases: /plugins, /marketplace"));
        assert!(help.contains("/agents"));
        assert!(help.contains("/skills"));
        assert!(help.contains("/exit"));
        assert!(help.contains("Auto-save            .kcode/sessions/<session-id>.jsonl"));
        assert!(help.contains("Resume latest        /resume latest"));
    }

    #[test]
    fn repl_help_hides_tool_commands_when_profile_disables_tools() {
        let help = render_repl_help_for_profile(false);
        assert!(help.contains("Start here        /doctor, /config, /status, /memory"));
        assert!(!help.contains("/mcp [list|show <server>|help]"));
        assert!(!help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
    }

    #[test]
    fn completion_candidates_include_workflow_shortcuts_and_dynamic_sessions() {
        let completions = slash_command_completion_candidates_with_sessions(
            "sonnet",
            true,
            Some("session-current"),
            vec!["session-old".to_string()],
        );

        assert!(completions.contains(&"/model gpt-4.1".to_string()));
        assert!(completions.contains(&"/permissions workspace-write".to_string()));
        assert!(completions.contains(&"/session list".to_string()));
        assert!(completions.contains(&"/session switch session-current".to_string()));
        assert!(completions.contains(&"/resume session-old".to_string()));
        assert!(completions.contains(&"/mcp list".to_string()));
    }

    #[test]
    fn completion_candidates_hide_tool_commands_when_profile_disables_tools() {
        let completions = slash_command_completion_candidates_with_sessions(
            "sonnet",
            false,
            Some("session-current"),
            vec!["session-old".to_string()],
        );

        assert!(!completions.contains(&"/mcp".to_string()));
        assert!(!completions.contains(&"/mcp list".to_string()));
        assert!(!completions.contains(&"/plugin list".to_string()));
        assert!(completions.contains(&"/session list".to_string()));
    }

    #[test]
    fn startup_banner_mentions_workflow_completions() {
        let _guard = env_lock();
        std::env::set_var("KCODE_BASE_URL", "https://router.example.test/v1");
        std::env::set_var("KCODE_API_KEY", "test-dummy-key-for-banner-test");
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");

        let banner = with_current_dir(&root, || {
            LiveCli::new(
                "gpt-4.1".to_string(),
                false,
                None,
                true,
                None,
                PermissionMode::DangerFullAccess,
                None,
            )
            .expect("cli should initialize")
            .startup_banner()
        });

        assert!(banner.contains("Tab"));
        assert!(banner.contains("workflow completions"));
        assert!(banner.contains("Profile"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
        std::env::remove_var("KCODE_BASE_URL");
        std::env::remove_var("KCODE_API_KEY");
    }

    #[test]
    fn commands_report_reflects_bridge_surface_and_profile_capability() {
        let _guard = env_lock();
        let root = temp_dir();
        let config_home = root.join("home").join(".kcode");
        fs::create_dir_all(&config_home).expect("config home");
        fs::write(
            config_home.join("config.toml"),
            r#"
profile = "bridge"
model = "gpt-4.1-mini"

[profiles.bridge]
default_model = "gpt-4.1-mini"
base_url_env = "BRIDGE_BASE_URL"
api_key_env = "BRIDGE_API_KEY"
supports_tools = false
supports_streaming = false
"#,
        )
        .expect("write config");
        let previous_config_home = std::env::var_os("KCODE_CONFIG_HOME");
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);

        let report = with_current_dir(&root, || {
            render_commands_report(CommandReportSurfaceSelection::Bridge, None, None)
                .expect("commands report should render")
        });

        match previous_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }

        assert!(report.contains("Commands"));
        assert!(report.contains("Surface           bridge"));
        assert!(report.contains("Safety profile    bridge-safe"));
        assert!(report.contains("Supports tools    false"));
        assert!(report.contains("Supports stream   false"));
        assert!(report.contains("Filtered"));
        assert!(report.contains("/mcp"));
        assert!(report.contains("active profile does not expose tool-capable commands"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn session_command_availability_rejects_tool_commands_for_toolless_profile() {
        let profile = ResolvedProviderProfile {
            profile_name: "bridge".to_string(),
            profile_source: ResolutionSource::Config("config.profile"),
            model: "gpt-4.1-mini".to_string(),
            model_source: ResolutionSource::Config("config.model"),
            base_url: None,
            base_url_source: ResolutionSource::Missing,
            credential: CredentialResolution {
                source: CredentialSource::Missing,
                env_name: "BRIDGE_API_KEY".to_string(),
                api_key: None,
            },
            profile: ProviderProfile {
                name: "bridge".to_string(),
                base_url_env: "BRIDGE_BASE_URL".to_string(),
                base_url: String::new(),
                api_key_env: "BRIDGE_API_KEY".to_string(),
                default_model: "gpt-4.1-mini".to_string(),
                supports_tools: false,
                supports_streaming: false,
                request_timeout_ms: 120_000,
                max_retries: 2,
            },
        };

        let error = ensure_session_command_available_for_profile("mcp", &profile)
            .expect_err("mcp should be blocked");
        assert!(error.contains("command `/mcp` is unavailable"));
        assert!(error.contains("active profile `bridge`"));
    }

    #[test]
    fn resume_supported_command_list_matches_expected_surface() {
        let names = resume_supported_slash_commands()
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "help", "status", "sandbox", "compact", "clear", "cost", "config", "mcp", "memory",
                "init", "diff", "version", "export", "agents", "skills", "doctor", "todos",
            ]
        );
    }

    #[test]
    fn resume_report_uses_sectioned_layout() {
        let report = format_resume_report("session.jsonl", 14, 6);
        assert!(report.contains("Session resumed"));
        assert!(report.contains("Session file     session.jsonl"));
        assert!(report.contains("Messages         14"));
        assert!(report.contains("Turns            6"));
    }

    #[test]
    fn compact_report_uses_structured_output() {
        let compacted = format_compact_report(8, 5, false);
        assert!(compacted.contains("Compact"));
        assert!(compacted.contains("Result           compacted"));
        assert!(compacted.contains("Messages removed 8"));
        let skipped = format_compact_report(0, 3, true);
        assert!(skipped.contains("Result           skipped"));
    }

    #[test]
    fn cost_report_uses_sectioned_layout() {
        let report = format_cost_report(runtime::TokenUsage {
            input_tokens: 20,
            output_tokens: 8,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 1,
        });
        assert!(report.contains("Cost"));
        assert!(report.contains("Input tokens     20"));
        assert!(report.contains("Output tokens    8"));
        assert!(report.contains("Cache create     3"));
        assert!(report.contains("Cache read       1"));
        assert!(report.contains("Total tokens     32"));
    }

    #[test]
    fn permissions_report_uses_sectioned_layout() {
        let report = format_permissions_report("workspace-write");
        assert!(report.contains("Permissions"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Modes"));
        assert!(report.contains("read-only          ○ available Read/search tools only"));
        assert!(report.contains("workspace-write    ● current   Edit files inside the workspace"));
        assert!(report.contains("danger-full-access ○ available Unrestricted tool access"));
    }

    #[test]
    fn permissions_switch_report_is_structured() {
        let report = format_permissions_switch_report("read-only", "workspace-write");
        assert!(report.contains("Permissions updated"));
        assert!(report.contains("Result           mode switched"));
        assert!(report.contains("Previous mode    read-only"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Applies to       subsequent tool calls"));
    }

    #[test]
    fn init_help_mentions_direct_subcommand() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("kcode help"));
        assert!(help.contains("kcode version"));
        assert!(help.contains("kcode status"));
        assert!(help.contains("kcode sandbox"));
        assert!(help.contains("kcode tui [section]"));
        assert!(help.contains("kcode configure [section]"));
        assert!(help.contains("kcode init"));
        assert!(help.contains("kcode agents"));
        assert!(help.contains("kcode mcp"));
        assert!(help.contains("kcode commands [show [local|bridge]]"));
        assert!(help.contains("kcode skills"));
        assert!(help.contains("kcode /skills"));
    }

    #[test]
    fn help_hides_tooling_for_toolless_profiles() {
        let mut help = Vec::new();
        print_help_to_for_profile(&mut help, false).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        let resume_line = help
            .lines()
            .find(|line| line.starts_with("Resume-safe commands:"))
            .expect("resume-safe commands line");

        assert!(!help.contains("--allowedTools"));
        assert!(!help.contains("kcode mcp"));
        assert!(!help.contains("kcode mcp show my-server"));
        assert!(!help.contains("/mcp [list|show <server>|help]"));
        assert!(!resume_line.contains("/mcp"));
        assert!(help.contains("kcode commands [show [local|bridge]]"));
        assert!(help.contains("/status"));
    }

    #[test]
    fn model_report_uses_sectioned_layout() {
        let report = format_model_report("claude-sonnet", "cliproxyapi", 12, 4);
        assert!(report.contains("Model"));
        assert!(report.contains("Current model    claude-sonnet"));
        assert!(report.contains("Active profile   cliproxyapi"));
        assert!(report.contains("Session messages 12"));
        assert!(report.contains("Switch models with /model <name>"));
    }

    #[test]
    fn model_switch_report_preserves_context_summary() {
        let report = format_model_switch_report("claude-sonnet", "claude-opus", "nvidia", 9);
        assert!(report.contains("Model updated"));
        assert!(report.contains("Previous         claude-sonnet"));
        assert!(report.contains("Current          claude-opus"));
        assert!(report.contains("Active profile   nvidia"));
        assert!(report.contains("Preserved msgs   9"));
    }

    #[test]
    fn status_line_reports_model_and_token_totals() {
        let profile = ResolvedProviderProfile {
            profile_name: "cliproxyapi".to_string(),
            profile_source: ResolutionSource::ProfileDefault,
            model: "claude-sonnet".to_string(),
            model_source: ResolutionSource::Cli,
            base_url: Some("https://router.example.test/v1".to_string()),
            base_url_source: ResolutionSource::Env("KCODE_BASE_URL"),
            credential: CredentialResolution {
                source: CredentialSource::PrimaryEnv,
                env_name: "KCODE_API_KEY".to_string(),
                api_key: Some("test-key".to_string()),
            },
            profile: ProviderProfile {
                name: "cliproxyapi".to_string(),
                base_url_env: "KCODE_BASE_URL".to_string(),
                base_url: String::new(),
                api_key_env: "KCODE_API_KEY".to_string(),
                default_model: "claude-sonnet".to_string(),
                supports_tools: true,
                supports_streaming: true,
                request_timeout_ms: 120_000,
                max_retries: 2,
            },
        };
        let status = format_status_report(
            "claude-sonnet",
            Some(&profile),
            StatusUsage {
                message_count: 7,
                turns: 3,
                latest: runtime::TokenUsage {
                    input_tokens: 5,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 0,
                },
                cumulative: runtime::TokenUsage {
                    input_tokens: 20,
                    output_tokens: 8,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                },
                estimated_tokens: 128,
            },
            "workspace-write",
            &super::StatusContext {
                cwd: PathBuf::from("/tmp/project"),
                session_path: Some(PathBuf::from("session.jsonl")),
                loaded_config_files: 2,
                discovered_config_files: 3,
                memory_file_count: 4,
                project_root: Some(PathBuf::from("/tmp")),
                git_branch: Some("main".to_string()),
                git_summary: GitWorkspaceSummary {
                    changed_files: 3,
                    staged_files: 1,
                    unstaged_files: 1,
                    untracked_files: 1,
                    conflicted_files: 0,
                },
                sandbox_status: runtime::SandboxStatus::default(),
            },
        );
        assert!(status.contains("Status"));
        assert!(status.contains("Profile          cliproxyapi"));
        assert!(status.contains("Model            claude-sonnet"));
        assert!(status.contains("Permission mode  workspace-write"));
        assert!(status.contains("Endpoint         https://router.example.test/v1"));
        assert!(status.contains("Supports tools   true"));
        assert!(status.contains("Supports stream  true"));
        assert!(status.contains("Messages         7"));
        assert!(status.contains("Latest total     10"));
        assert!(status.contains("Cumulative total 31"));
        assert!(status.contains("Cwd              /tmp/project"));
        assert!(status.contains("Project root     /tmp"));
        assert!(status.contains("Git branch       main"));
        assert!(
            status.contains("Git state        dirty · 3 files · 1 staged, 1 unstaged, 1 untracked")
        );
        assert!(status.contains("Changed files    3"));
        assert!(status.contains("Staged           1"));
        assert!(status.contains("Unstaged         1"));
        assert!(status.contains("Untracked        1"));
        assert!(status.contains("Session          session.jsonl"));
        assert!(status.contains("Config files     loaded 2/3"));
        assert!(status.contains("Memory files     4"));
        assert!(status.contains("Suggested flow   /status → /diff → /commit"));
    }
