    #[test]
    fn default_permission_mode_reads_plan_from_project_config() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".kcode")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".kcode").join("settings.local.json"),
            r#"{"permissions":{"defaultMode":"plan"}}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("KCODE_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("KCODE_PERMISSION_MODE").ok();
        std::env::set_var("KCODE_CONFIG_HOME", &config_home);
        std::env::remove_var("KCODE_PERMISSION_MODE");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("KCODE_CONFIG_HOME", value),
            None => std::env::remove_var("KCODE_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("KCODE_PERMISSION_MODE", value),
            None => std::env::remove_var("KCODE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::Plan);
    }

    #[test]
    fn plan_command_uses_tool_backed_local_override_lifecycle() {
        let _guard = env_lock();
        let root = temp_dir();
        std::fs::create_dir_all(root.join(".kcode")).expect("workspace config dir");

        with_current_dir(&root, || {
            let enabled = run_plan_mode_command(&root, Some("on"), PermissionMode::WorkspaceWrite)
                .expect("plan should enable");
            assert!(enabled.message.contains("Status           enabled"));
            assert_eq!(enabled.next_permission_mode, Some(PermissionMode::Plan));

            let settings_path = root.join(".kcode").join("settings.local.json");
            let settings = std::fs::read_to_string(&settings_path).expect("local settings");
            assert!(settings.contains(r#""defaultMode": "plan""#));

            let disabled = run_plan_mode_command(&root, Some("off"), PermissionMode::Plan)
                .expect("plan should disable");
            assert!(disabled.message.contains("Status           disabled"));
            assert_ne!(disabled.next_permission_mode, Some(PermissionMode::Plan));

            let settings = std::fs::read_to_string(&settings_path).expect("local settings");
            assert!(!settings.contains(r#""defaultMode": "plan""#));
        });

        std::fs::remove_dir_all(root).expect("cleanup temp dir");
    }
