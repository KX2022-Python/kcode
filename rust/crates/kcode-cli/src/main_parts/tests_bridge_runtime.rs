#[test]
fn live_cli_loads_existing_bridge_session_from_override_path() {
    let _guard = env_lock();
    std::env::set_var("KCODE_BASE_URL", "https://router.example.test/v1");
    std::env::set_var("KCODE_API_KEY", "test-dummy-key-for-bridge-session");

    let root = temp_dir();
    let workspace = root.join("workspace");
    let session_path = root
        .join("config-home")
        .join("bridge-sessions")
        .join("bridge-telegram-42.jsonl");
    fs::create_dir_all(&workspace).expect("workspace dir");
    fs::create_dir_all(
        session_path
            .parent()
            .expect("bridge session path should have parent"),
    )
    .expect("bridge session dir");

    let mut session = Session::new();
    session
        .messages
        .push(ConversationMessage::user_text("persisted inbound"));
    session.messages.push(ConversationMessage::assistant(vec![
        ContentBlock::Text {
            text: "persisted outbound".to_string(),
        },
    ]));
    session
        .save_to_path(&session_path)
        .expect("bridge session should persist");

    let cli = with_current_dir(&workspace, || {
        LiveCli::new(
            "gpt-4.1".to_string(),
            false,
            None,
            true,
            None,
            PermissionMode::DangerFullAccess,
            Some(session_path.clone()),
        )
        .expect("bridge session should load")
    });

    assert_eq!(cli.session.id, "bridge-telegram-42");
    assert_eq!(cli.session.path, session_path);
    assert_eq!(cli.runtime.session().messages.len(), 2);
    assert_eq!(
        cli.runtime.session().messages[0],
        ConversationMessage::user_text("persisted inbound")
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
    std::env::remove_var("KCODE_BASE_URL");
    std::env::remove_var("KCODE_API_KEY");
}

#[test]
fn bridge_session_manager_evicts_idle_sessions_and_clears_routes() {
    let _guard = env_lock();
    std::env::set_var("KCODE_BASE_URL", "https://router.example.test/v1");
    std::env::set_var("KCODE_API_KEY", "test-dummy-key-for-bridge-eviction");

    let root = temp_dir();
    let workspace = root.join("workspace");
    let session_dir = root.join("config-home").join("bridge-sessions");
    fs::create_dir_all(&workspace).expect("workspace dir");
    fs::create_dir_all(&session_dir).expect("session dir");

    let mut manager = SessionManager::new(session_dir.clone());
    let config = SessionConfig {
        model: "gpt-4.1".to_string(),
        model_explicit: false,
        profile: None,
        permission_mode: PermissionMode::DangerFullAccess,
    };

    with_current_dir(&workspace, || {
        let (_, route) = manager
            .get_or_create_session("42", "telegram", &config)
            .expect("session should initialize");
        assert_eq!(route.session_id, "bridge-telegram-42");
    });

    manager.mark_session_idle_for_test("bridge-telegram-42", std::time::Duration::from_secs(30 * 60));

    manager.evict_expired_sessions_for_test();

    assert_eq!(manager.active_session_count(), 0);
    assert!(!manager.has_active_route("telegram", "42"));
    let next_route = manager.route_for_test("telegram", "42");
    assert_eq!(next_route.session_id, "bridge-telegram-42");
    assert!(manager.has_active_route("telegram", "42"));
    assert!(session_dir.join("bridge-telegram-42.jsonl").exists());

    fs::remove_dir_all(root).expect("cleanup temp root");
    std::env::remove_var("KCODE_BASE_URL");
    std::env::remove_var("KCODE_API_KEY");
}
