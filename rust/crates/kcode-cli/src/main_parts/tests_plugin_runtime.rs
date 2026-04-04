    #[test]
    fn build_runtime_plugin_state_merges_plugin_hooks_into_runtime_features() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "hook-runtime-demo", true, false);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, true)
                .expect("plugin state should load");
        let pre_hooks = state.feature_config.hooks().pre_tool_use();
        assert_eq!(pre_hooks.len(), 1);
        assert!(
            pre_hooks[0].ends_with("hooks/pre.sh"),
            "expected installed plugin hook path, got {pre_hooks:?}"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn build_runtime_plugin_state_strips_tool_runtime_for_toolless_profiles() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "hook-runtime-demo", true, false);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, false)
                .expect("plugin state should load");

        assert!(state.feature_config.hooks().pre_tool_use().is_empty());
        assert!(state.tool_registry.definitions(None).is_empty());
        assert!(state.plugin_registry.plugins().is_empty());

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn build_runtime_runs_plugin_lifecycle_init_and_shutdown() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "lifecycle-runtime-demo", false, true);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        let install = manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let log_path = install.install_path.join("lifecycle.log");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let runtime_plugin_state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, true)
                .expect("plugin state should load");
        let mut setup = test_setup_context(&workspace);
        setup.resolved_config.base_url = Some("https://router.example.test/v1".to_string());
        setup.resolved_config.api_key_present = true;
        setup.resolved_config.profile = Some("cliproxyapi".to_string());
        setup.active_profile.base_url = Some("https://router.example.test/v1".to_string());
        setup.active_profile.base_url_source = ResolutionSource::Env("KCODE_BASE_URL");
        setup.active_profile.credential = CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: "KCODE_API_KEY".to_string(),
            api_key: Some("test-dummy-key-for-plugin-lifecycle".to_string()),
        };
        let mut runtime = build_runtime_with_plugin_state(
            Session::new(),
            "runtime-plugin-lifecycle",
            DEFAULT_MODEL.to_string(),
            vec!["test system prompt".to_string()],
            true,
            false,
            None,
            PermissionMode::DangerFullAccess,
            None,
            &setup,
            runtime_plugin_state,
        )
        .expect("runtime should build");

        assert_eq!(
            fs::read_to_string(&log_path).expect("init log should exist"),
            "init\n"
        );

        runtime
            .shutdown_plugins()
            .expect("plugin shutdown should succeed");

        assert_eq!(
            fs::read_to_string(&log_path).expect("shutdown log should exist"),
            "init\nshutdown\n"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn build_runtime_skips_plugin_lifecycle_for_toolless_profiles() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "lifecycle-runtime-demo", false, true);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        let install = manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let log_path = install.install_path.join("lifecycle.log");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let runtime_plugin_state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config, false)
                .expect("plugin state should load");
        let mut setup = test_setup_context(&workspace);
        setup.active_profile.profile.supports_tools = false;
        setup.resolved_config.base_url = Some("https://router.example.test/v1".to_string());
        setup.resolved_config.api_key_present = true;
        setup.resolved_config.profile = Some("bridge".to_string());
        setup.active_profile.base_url = Some("https://router.example.test/v1".to_string());
        setup.active_profile.base_url_source = ResolutionSource::Env("KCODE_BASE_URL");
        setup.active_profile.credential = CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: "KCODE_API_KEY".to_string(),
            api_key: Some("test-dummy-key-for-plugin-lifecycle".to_string()),
        };

        let mut runtime = build_runtime_with_plugin_state(
            Session::new(),
            "runtime-toolless-plugin-lifecycle",
            DEFAULT_MODEL.to_string(),
            vec!["test system prompt".to_string()],
            true,
            false,
            None,
            PermissionMode::DangerFullAccess,
            None,
            &setup,
            runtime_plugin_state,
        )
        .expect("runtime should build");

        assert!(!log_path.exists(), "plugin lifecycle should not run");

        runtime
            .shutdown_plugins()
            .expect("plugin shutdown should succeed");

        assert!(
            !log_path.exists(),
            "plugin shutdown should stay inactive for toolless profiles"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    fn provider_runtime_client_disables_tools_for_toolless_profiles() {
        let workspace = temp_dir();
        fs::create_dir_all(&workspace).expect("workspace");

        let mut setup = test_setup_context(&workspace);
        setup.active_profile.profile.supports_tools = false;
        setup.active_profile.base_url = Some("https://router.example.test/v1".to_string());
        setup.active_profile.base_url_source = ResolutionSource::Env("KCODE_BASE_URL");
        setup.active_profile.credential = CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: "KCODE_API_KEY".to_string(),
            api_key: Some("test-dummy-key-for-runtime".to_string()),
        };

        let client = ProviderRuntimeClient::new(
            "runtime-toolless-profile",
            DEFAULT_MODEL.to_string(),
            true,
            false,
            None,
            registry_with_plugin_tool(),
            None,
            &setup,
        )
        .expect("runtime client should build");

        assert!(!client.enable_tools);

        let _ = fs::remove_dir_all(workspace);
    }
