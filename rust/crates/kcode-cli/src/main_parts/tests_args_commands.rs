    #[test]
    fn parses_login_and_logout_subcommands() {
        assert_eq!(
            parse_args(&["login".to_string()]).expect("login should parse"),
            CliAction::Login
        );
        assert_eq!(
            parse_args(&["logout".to_string()]).expect("logout should parse"),
            CliAction::Logout
        );
        assert_eq!(
            parse_args(&["init".to_string()]).expect("init should parse"),
            CliAction::Init
        );
        assert_eq!(
            parse_args(&["doctor".to_string()]).expect("doctor should parse"),
            CliAction::Doctor {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                fix: false,
            }
        );
        assert_eq!(
            parse_args(&["config".to_string(), "show".to_string()])
                .expect("config show should parse"),
            CliAction::ConfigShow {
                section: None,
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&[
                "config".to_string(),
                "show".to_string(),
                "plugins".to_string(),
            ])
            .expect("config section should parse"),
            CliAction::ConfigShow {
                section: Some("plugins".to_string()),
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&[
                "commands".to_string(),
                "show".to_string(),
                "bridge".to_string()
            ])
            .expect("commands show bridge should parse"),
            CliAction::Commands {
                surface: CommandReportSurfaceSelection::Bridge,
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string()]).expect("agents should parse"),
            CliAction::Agents { args: None }
        );
        assert_eq!(
            parse_args(&["mcp".to_string()]).expect("mcp should parse"),
            CliAction::Mcp {
                args: None,
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["skills".to_string()]).expect("skills should parse"),
            CliAction::Skills { args: None }
        );
        assert_eq!(
            parse_args(&["tui".to_string()]).expect("tui should parse"),
            CliAction::Tui { section: None }
        );
        assert_eq!(
            parse_args(&["configure".to_string(), "bridge".to_string()])
                .expect("configure bridge should parse"),
            CliAction::Tui {
                section: Some("bridge".to_string())
            }
        );
        assert_eq!(
            parse_args(&["config".to_string(), "tui".to_string(), "appearance".to_string()])
                .expect("config tui appearance should parse"),
            CliAction::Tui {
                section: Some("appearance".to_string())
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string(), "--help".to_string()])
                .expect("agents help should parse"),
            CliAction::Agents {
                args: Some("--help".to_string())
            }
        );
    }

    #[test]
    fn parses_single_word_command_aliases_without_falling_back_to_prompt_mode() {
        let permission_mode = super::default_permission_mode();
        assert_eq!(
            parse_args(&["help".to_string()]).expect("help should parse"),
            CliAction::Help { profile: None }
        );
        assert_eq!(
            parse_args(&["version".to_string()]).expect("version should parse"),
            CliAction::Version
        );
        assert_eq!(
            parse_args(&["configure".to_string()]).expect("configure should parse"),
            CliAction::Tui { section: None }
        );
        assert_eq!(
            parse_args(&["status".to_string()]).expect("status should parse"),
            CliAction::Status {
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
                permission_mode,
            }
        );
        assert_eq!(
            parse_args(&["sandbox".to_string()]).expect("sandbox should parse"),
            CliAction::Sandbox
        );
        assert_eq!(
            parse_args(&["commands".to_string()]).expect("commands should parse"),
            CliAction::Commands {
                surface: CommandReportSurfaceSelection::Local,
                model: DEFAULT_MODEL.to_string(),
                model_explicit: false,
                profile: None,
            }
        );
    }

    #[test]
    fn single_word_slash_command_names_return_guidance_instead_of_hitting_prompt_mode() {
        let error = parse_args(&["cost".to_string()]).expect_err("cost should return guidance");
        assert!(error.contains("slash command"));
        assert!(error.contains("/cost"));
    }

    #[test]
    fn multi_word_prompt_still_uses_shorthand_prompt_mode() {
        let permission_mode = super::default_permission_mode();
        assert_eq!(
            parse_args(&["help".to_string(), "me".to_string(), "debug".to_string()])
                .expect("prompt shorthand should still work"),
            CliAction::Prompt {
                prompt: "help me debug".to_string(),
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
    fn parses_direct_agents_mcp_and_skills_slash_commands() {
        assert_eq!(
            parse_args(&["/agents".to_string()]).expect("/agents should parse"),
            CliAction::Agents { args: None }
        );
        assert_eq!(
            parse_args(&["/mcp".to_string(), "show".to_string(), "demo".to_string()])
                .expect("/mcp show demo should parse"),
            CliAction::Mcp {
                args: Some("show demo".to_string()),
                profile: None,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string()]).expect("/skills should parse"),
            CliAction::Skills { args: None }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "help".to_string()])
                .expect("/skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string())
            }
        );
        assert_eq!(
            parse_args(&[
                "/skills".to_string(),
                "install".to_string(),
                "./fixtures/help-skill".to_string(),
            ])
            .expect("/skills install should parse"),
            CliAction::Skills {
                args: Some("install ./fixtures/help-skill".to_string())
            }
        );
        let error = parse_args(&["/status".to_string()])
            .expect_err("/status should remain REPL-only when invoked directly");
        assert!(error.contains("interactive-only"));
        assert!(error.contains("kcode --resume SESSION.jsonl /status"));
    }

    #[test]
    fn process_mcp_command_preserves_profile_override() {
        assert_eq!(
            parse_args(&[
                "--profile".to_string(),
                "bridge".to_string(),
                "mcp".to_string(),
            ])
            .expect("mcp with profile should parse"),
            CliAction::Mcp {
                args: None,
                profile: Some("bridge".to_string()),
            }
        );
        assert_eq!(
            parse_args(&[
                "--profile".to_string(),
                "bridge".to_string(),
                "/mcp".to_string(),
                "list".to_string(),
            ])
            .expect("direct /mcp with profile should parse"),
            CliAction::Mcp {
                args: Some("list".to_string()),
                profile: Some("bridge".to_string()),
            }
        );
    }

    #[test]
    fn direct_slash_commands_surface_shared_validation_errors() {
        let compact_error = parse_args(&["/compact".to_string(), "now".to_string()])
            .expect_err("invalid /compact shape should be rejected");
        assert!(compact_error.contains("Unexpected arguments for /compact."));
        assert!(compact_error.contains("Usage            /compact"));

        let plugins_error = parse_args(&[
            "/plugins".to_string(),
            "list".to_string(),
            "extra".to_string(),
        ])
        .expect_err("invalid /plugins list shape should be rejected");
        assert!(plugins_error.contains("Usage: /plugin list"));
        assert!(plugins_error.contains("Aliases          /plugins, /marketplace"));
    }

    #[test]
    fn formats_unknown_slash_command_with_suggestions() {
        let report = format_unknown_slash_command_message("statsu");
        assert!(report.contains("unknown slash command: /statsu"));
        assert!(report.contains("Did you mean"));
        assert!(report.contains("Use /help"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/compact".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/compact".to_string()],
            }
        );
    }

    #[test]
    fn parses_resume_flag_without_path_as_latest_session() {
        assert_eq!(
            parse_args(&["--resume".to_string()]).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec![],
            }
        );
        assert_eq!(
            parse_args(&["--resume".to_string(), "/status".to_string()])
                .expect("resume shortcut should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec!["/status".to_string()],
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_multiple_slash_commands() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/status".to_string(),
            "/compact".to_string(),
            "/cost".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/status".to_string(),
                    "/compact".to_string(),
                    "/cost".to_string(),
                ],
            }
        );
    }

    #[test]
    fn rejects_unknown_options_with_helpful_guidance() {
        let error = parse_args(&["--resum".to_string()]).expect_err("unknown option should fail");
        assert!(error.contains("unknown option: --resum"));
        assert!(error.contains("Did you mean --resume?"));
        assert!(error.contains("kcode --help"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command_arguments() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "notes.txt".to_string(),
            "/clear".to_string(),
            "--confirm".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/export notes.txt".to_string(),
                    "/clear --confirm".to_string(),
                ],
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_absolute_export_path() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "/tmp/notes.txt".to_string(),
            "/status".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/export /tmp/notes.txt".to_string(), "/status".to_string()],
            }
        );
    }
