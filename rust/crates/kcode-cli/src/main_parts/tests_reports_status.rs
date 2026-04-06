    #[test]
    fn commit_reports_surface_workspace_context() {
        let summary = GitWorkspaceSummary {
            changed_files: 2,
            staged_files: 1,
            unstaged_files: 1,
            untracked_files: 0,
            conflicted_files: 0,
        };

        let preflight = format_commit_preflight_report(Some("feature/ux"), summary);
        assert!(preflight.contains("Result           ready"));
        assert!(preflight.contains("Branch           feature/ux"));
        assert!(preflight.contains("Workspace        dirty · 2 files · 1 staged, 1 unstaged"));
        assert!(preflight
            .contains("Action           create a git commit from the current workspace changes"));
    }

    #[test]
    fn commit_skipped_report_points_to_next_steps() {
        let report = format_commit_skipped_report();
        assert!(report.contains("Reason           no workspace changes"));
        assert!(report
            .contains("Action           create a git commit from the current workspace changes"));
        assert!(report.contains("/status to inspect context"));
        assert!(report.contains("/diff to inspect repo changes"));
    }

    #[test]
    fn runtime_slash_reports_describe_command_behavior() {
        let bughunter = format_bughunter_report(Some("runtime"));
        assert!(bughunter.contains("Scope            runtime"));
        assert!(bughunter.contains("inspect the selected code for likely bugs"));

        let ultraplan = format_ultraplan_report(Some("ship the release"));
        assert!(ultraplan.contains("Task             ship the release"));
        assert!(ultraplan.contains("break work into a multi-step execution plan"));

        let pr = format_pr_report("feature/ux", Some("ready for review"));
        assert!(pr.contains("Branch           feature/ux"));
        assert!(pr.contains("draft or create a pull request"));

        let issue = format_issue_report(Some("flaky test"));
        assert!(issue.contains("Context          flaky test"));
        assert!(issue.contains("draft or create a GitHub issue"));
    }

    #[test]
    fn no_arg_commands_reject_unexpected_arguments() {
        assert!(validate_no_args("/commit", None).is_ok());

        let error = validate_no_args("/commit", Some("now"))
            .expect_err("unexpected arguments should fail")
            .to_string();
        assert!(error.contains("/commit does not accept arguments"));
        assert!(error.contains("Received: now"));
    }

    #[test]
    fn config_report_supports_section_views() {
        let report =
            render_config_report(Some("env"), None, None).expect("config report should render");
        assert!(report.contains("Merged section: env"));
        let plugins_report = render_config_report(Some("plugins"), None, None)
            .expect("plugins config report should render");
        assert!(plugins_report.contains("Merged section: plugins"));
    }

    #[test]
    fn memory_report_uses_sectioned_layout() {
        let report = render_memory_report().expect("memory report should render");
        // Report should contain either the memory section or the project instruction section
        assert!(
            report.contains("Memory files")
                || report.contains("memory")
                || report.contains("Project instruction")
        );
    }

    #[test]
    fn config_report_uses_sectioned_layout() {
        let report = render_config_report(None, None, None).expect("config report should render");
        assert!(report.contains("Config"));
        assert!(report.contains("Config home"));
        assert!(report.contains("Discovered files"));
        assert!(report.contains("Merged JSON"));
    }

    #[test]
    fn doctor_report_surfaces_missing_bootstrap_state() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        let report = render_doctor_report_from_setup(&test_setup_context(&root));
        assert!(report.contains("Doctor"));
        assert!(report.contains("Runtime ready    no"));
        assert!(report.contains("[fail] config file"));
        assert!(report.contains("[fail] base url"));
        assert!(report.contains("[fail] api credentials"));
        assert!(report.contains("run `kcode init`"));
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn doctor_report_warns_about_legacy_residue() {
        let root = temp_dir();
        let workspace = root.join("workspace");
        fs::create_dir_all(workspace.join(".kcode")).expect("config dir");
        fs::create_dir_all(workspace.join(".kcode").join("sessions")).expect("sessions dir");

        let mut setup = test_setup_context(&workspace);
        setup.resolved_config.config_file_present = true;
        setup.resolved_config.base_url = Some("https://router.example.test".to_string());
        setup.resolved_config.api_key_present = true;
        setup.resolved_config.legacy_paths = vec![workspace.join(".claw").join("settings.json")];

        let report = render_doctor_report_from_setup(&setup);
        assert!(report.contains("Runtime ready    yes"));
        assert!(report.contains("[warn] legacy residue"));
        assert!(report.contains(".claw/settings.json"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn doctor_report_describes_toolless_profile_capability() {
        let root = temp_dir();
        let workspace = root.join("workspace");
        fs::create_dir_all(workspace.join(".kcode")).expect("config dir");
        fs::create_dir_all(workspace.join(".kcode").join("sessions")).expect("sessions dir");

        let mut setup = test_setup_context(&workspace);
        setup.resolved_config.config_file_present = true;
        setup.resolved_config.base_url = Some("https://router.example.test".to_string());
        setup.resolved_config.api_key_present = true;
        setup.active_profile.profile_name = "bridge".to_string();
        setup.active_profile.profile.supports_tools = false;

        let report = render_doctor_report_from_setup(&setup);
        assert!(report.contains("[ok  ] tool capability"));
        assert!(report.contains("disabled by active profile `bridge`"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn nested_session_dir_can_use_existing_workspace_ancestor_for_writeability() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        let nested = root.join(".kcode").join("sessions");
        assert!(super::path_or_parent_writeable(&nested));
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn parses_git_status_metadata() {
        let _guard = env_lock();
        let temp_root = temp_dir();
        fs::create_dir_all(&temp_root).expect("root dir");
        let (project_root, branch) = parse_git_status_metadata_for(
            &temp_root,
            Some(
                "## rcc/cli...origin/rcc/cli
 M src/main.rs",
            ),
        );
        assert_eq!(branch.as_deref(), Some("rcc/cli"));
        assert!(project_root.is_none());
        fs::remove_dir_all(temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn parses_detached_head_from_status_snapshot() {
        let _guard = env_lock();
        assert_eq!(
            parse_git_status_branch(Some(
                "## HEAD (no branch)
 M src/main.rs"
            )),
            Some("detached HEAD".to_string())
        );
    }

    #[test]
    fn parses_git_workspace_summary_counts() {
        let summary = parse_git_workspace_summary(Some(
            "## feature/ux
M  src/main.rs
 M README.md
?? notes.md
UU conflicted.rs",
        ));

        assert_eq!(
            summary,
            GitWorkspaceSummary {
                changed_files: 4,
                staged_files: 2,
                unstaged_files: 2,
                untracked_files: 1,
                conflicted_files: 1,
            }
        );
        assert_eq!(
            summary.headline(),
            "dirty · 4 files · 2 staged, 2 unstaged, 1 untracked, 1 conflicted"
        );
    }

    #[test]
    fn render_diff_report_shows_clean_tree_for_committed_repo() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        let report = with_current_dir(&root, || {
            render_diff_report().expect("diff report should render")
        });
        assert!(report.contains("clean working tree"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_includes_staged_and_unstaged_sections() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        fs::write(root.join("tracked.txt"), "hello\nstaged\n").expect("update file");
        git(&["add", "tracked.txt"], &root);
        fs::write(root.join("tracked.txt"), "hello\nstaged\nunstaged\n")
            .expect("update file twice");

        let report = with_current_dir(&root, || {
            render_diff_report().expect("diff report should render")
        });
        assert!(report.contains("Staged changes:"));
        assert!(report.contains("Unstaged changes:"));
        assert!(report.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_omits_ignored_files() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join(".gitignore"), ".omx/\nignored.txt\n").expect("write gitignore");
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", ".gitignore", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::create_dir_all(root.join(".omx")).expect("write omx dir");
        fs::write(root.join(".omx").join("state.json"), "{}").expect("write ignored omx");
        fs::write(root.join("ignored.txt"), "secret\n").expect("write ignored file");
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("write tracked change");

        let report = with_current_dir(&root, || {
            render_diff_report().expect("diff report should render")
        });
        assert!(report.contains("tracked.txt"));
        assert!(!report.contains("+++ b/ignored.txt"));
        assert!(!report.contains("+++ b/.omx/state.json"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn resume_diff_command_renders_report_for_saved_session() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Kcode Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("modify tracked");
        let session_path = root.join("session.json");
        Session::new()
            .save_to_path(&session_path)
            .expect("session should save");

        let session = Session::load_from_path(&session_path).expect("session should load");
        let outcome = with_current_dir(&root, || {
            run_resume_command(&session_path, &session, &SlashCommand::Diff)
                .expect("resume diff should work")
        });
        let message = outcome.message.expect("diff message should exist");
        assert!(message.contains("Unstaged changes:"));
        assert!(message.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn status_context_reads_real_workspace_metadata() {
        let context = status_context(None).expect("status context should load");
        assert!(context.cwd.is_absolute());
        assert!(context.discovered_config_files >= context.loaded_config_files);
        assert!(context.loaded_config_files <= context.discovered_config_files);
    }

    #[test]
    fn normalizes_supported_permission_modes() {
        assert_eq!(normalize_permission_mode("read-only"), Some("read-only"));
        assert_eq!(normalize_permission_mode("plan"), Some("plan"));
        assert_eq!(
            normalize_permission_mode("workspace-write"),
            Some("workspace-write")
        );
        assert_eq!(
            normalize_permission_mode("danger-full-access"),
            Some("danger-full-access")
        );
        assert_eq!(normalize_permission_mode("unknown"), None);
    }

    #[test]
    fn clear_command_requires_explicit_confirmation_flag() {
        assert_eq!(
            SlashCommand::parse("/clear"),
            Ok(Some(SlashCommand::Clear { confirm: false }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
    }

    #[test]
    fn parses_resume_and_config_slash_commands() {
        assert_eq!(
            SlashCommand::parse("/resume saved-session.jsonl"),
            Ok(Some(SlashCommand::Resume {
                session_path: Some("saved-session.jsonl".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
        assert_eq!(
            SlashCommand::parse("/config"),
            Ok(Some(SlashCommand::Config { section: None }))
        );
        assert_eq!(
            SlashCommand::parse("/config env"),
            Ok(Some(SlashCommand::Config {
                section: Some("env".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/memory"),
            Ok(Some(SlashCommand::Memory))
        );
        assert_eq!(SlashCommand::parse("/init"), Ok(Some(SlashCommand::Init)));
        assert_eq!(
            SlashCommand::parse("/session fork incident-review"),
            Ok(Some(SlashCommand::Session {
                action: Some("fork".to_string()),
                target: Some("incident-review".to_string())
            }))
        );
    }

    #[test]
    fn help_mentions_jsonl_resume_examples() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("kcode --resume [SESSION.jsonl|session-id|latest]"));
        assert!(help.contains("Use `latest` with --resume, /resume, or /session switch"));
        assert!(help.contains("kcode --resume latest"));
        assert!(help.contains("kcode --resume latest /status /diff /export notes.txt"));
    }

    #[test]
    fn managed_sessions_default_to_jsonl_and_resolve_legacy_json() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let workspace = temp_workspace("session-resolution");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let handle = create_managed_session_handle("session-alpha").expect("jsonl handle");
        assert!(handle.path.ends_with("session-alpha.jsonl"));

        let legacy_path = workspace.join(".claw/sessions/legacy.json");
        std::fs::create_dir_all(
            legacy_path
                .parent()
                .expect("legacy path should have parent directory"),
        )
        .expect("session dir should exist");
        Session::new()
            .with_persistence_path(legacy_path.clone())
            .save_to_path(&legacy_path)
            .expect("legacy session should save");

        let resolved = resolve_session_reference("legacy").expect("legacy session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            legacy_path
                .canonicalize()
                .expect("legacy path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn latest_session_alias_resolves_most_recent_managed_session() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let workspace = temp_workspace("latest-session-alias");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let older = create_managed_session_handle("session-older").expect("older handle");
        Session::new()
            .with_persistence_path(older.path.clone())
            .save_to_path(&older.path)
            .expect("older session should save");
        std::thread::sleep(Duration::from_millis(20));
        let newer = create_managed_session_handle("session-newer").expect("newer handle");
        Session::new()
            .with_persistence_path(newer.path.clone())
            .save_to_path(&newer.path)
            .expect("newer session should save");

        let resolved = resolve_session_reference("latest").expect("latest session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            newer.path.canonicalize().expect("newer path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn unknown_slash_command_guidance_suggests_nearby_commands() {
        let message = format_unknown_slash_command("stats");
        assert!(message.contains("Unknown slash command: /stats"));
        assert!(message.contains("/status"));
        assert!(message.contains("/help"));
    }

    #[test]
    fn resume_usage_mentions_latest_shortcut() {
        let usage = render_resume_usage();
        assert!(usage.contains("latest"));
        assert!(usage.contains("/resume <session-path|session-id|latest>"));
        assert!(usage.contains("/session list"));
    }
