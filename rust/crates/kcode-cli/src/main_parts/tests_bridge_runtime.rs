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
