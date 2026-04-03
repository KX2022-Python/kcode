use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime::Session;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn status_command_applies_model_and_permission_mode_flags() {
    // given
    let temp_dir = unique_temp_dir("status-flags");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");

    // when
    let output = command_in(&temp_dir)
        .args([
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "status",
        ])
        .output()
        .expect("kcode should launch");

    // then
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Status"));
    assert!(stdout.contains("Model            claude-sonnet-4-6"));
    assert!(stdout.contains("Permission mode  read-only"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn init_command_bootstraps_user_config_home() {
    let temp_dir = unique_temp_dir("init-bootstrap");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");

    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .arg("init")
        .output()
        .expect("kcode should launch");

    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Init"));
    assert!(stdout.contains("Config home"));
    assert!(config_home.join("config.toml").is_file());
    assert!(config_home.join("sessions").is_dir());
    assert!(config_home.join("logs").is_dir());

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn resume_flag_loads_a_saved_session_and_dispatches_status() {
    // given
    let temp_dir = unique_temp_dir("resume-status");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");
    let session_path = write_session(&temp_dir, "resume-status");

    // when
    let output = command_in(&temp_dir)
        .args([
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/status",
        ])
        .output()
        .expect("kcode should launch");

    // then
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Status"));
    assert!(stdout.contains("Messages         1"));
    assert!(stdout.contains("Session          "));
    assert!(stdout.contains(session_path.to_str().expect("utf8 path")));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn slash_command_names_match_known_commands_and_suggest_nearby_unknown_ones() {
    // given
    let temp_dir = unique_temp_dir("slash-dispatch");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");

    // when
    let help_output = command_in(&temp_dir)
        .arg("/help")
        .output()
        .expect("kcode should launch");
    let unknown_output = command_in(&temp_dir)
        .arg("/zstats")
        .output()
        .expect("kcode should launch");

    // then
    assert_success(&help_output);
    let help_stdout = String::from_utf8(help_output.stdout).expect("stdout should be utf8");
    assert!(help_stdout.contains("Interactive slash commands:"));
    assert!(help_stdout.contains("/status"));

    assert!(
        !unknown_output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&unknown_output.stdout),
        String::from_utf8_lossy(&unknown_output.stderr)
    );
    let stderr = String::from_utf8(unknown_output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("unknown slash command outside the REPL: /zstats"));
    assert!(stderr.contains("Did you mean"));
    assert!(stderr.contains("/status"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn config_command_loads_defaults_from_standard_config_locations() {
    // given
    let temp_dir = unique_temp_dir("config-defaults");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(temp_dir.join(".kcode")).expect("project config dir should exist");
    fs::create_dir_all(&config_home).expect("home config dir should exist");

    fs::write(config_home.join("settings.json"), r#"{"model":"haiku"}"#)
        .expect("write user settings");
    fs::write(temp_dir.join(".kcode.json"), r#"{"model":"sonnet"}"#)
        .expect("write project settings");
    fs::write(
        temp_dir.join(".kcode").join("settings.local.json"),
        r#"{"model":"opus"}"#,
    )
    .expect("write local settings");
    let session_path = write_session(&temp_dir, "config-defaults");

    // when
    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .args([
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/config",
            "model",
        ])
        .output()
        .expect("kcode should launch");

    // then
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Config"));
    assert!(stdout.contains("Config home"));
    assert!(stdout.contains("Loaded files"));
    assert!(stdout.contains("Merged section: model"));
    assert!(stdout.contains("opus"));
    assert!(stdout.contains(
        config_home
            .join("settings.json")
            .to_str()
            .expect("utf8 path")
    ));
    assert!(stdout.contains(temp_dir.join(".kcode.json").to_str().expect("utf8 path")));
    assert!(stdout.contains(
        temp_dir
            .join(".kcode")
            .join("settings.local.json")
            .to_str()
            .expect("utf8 path")
    ));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn doctor_command_reports_bootstrap_gaps_without_oauth_login() {
    let temp_dir = unique_temp_dir("doctor-bootstrap");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::write(
        config_home.join("config.toml"),
        r#"
model = "claude-sonnet-4-6"
base_url = ""
api_key_env = "KCODE_API_KEY"
"#,
    )
    .expect("write config");

    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .arg("doctor")
        .output()
        .expect("kcode should launch");

    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Doctor"));
    assert!(stdout.contains("Runtime ready    no"));
    assert!(stdout.contains("[ok  ] config file"));
    assert!(stdout.contains("[fail] base url"));
    assert!(stdout.contains("[fail] api credentials"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn profile_commands_report_effective_router_settings_after_init() {
    let temp_dir = unique_temp_dir("profile-bootstrap");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");

    let init_output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .arg("init")
        .output()
        .expect("kcode should launch");
    assert_success(&init_output);

    let list_output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .args(["profile", "list"])
        .output()
        .expect("kcode should launch");
    assert_success(&list_output);
    let list_stdout = String::from_utf8(list_output.stdout).expect("stdout should be utf8");
    assert!(list_stdout.contains("* cliproxyapi"));
    assert!(list_stdout.contains("key=KCODE_API_KEY"));
    assert!(list_stdout.contains("model=claude-sonnet-4-6"));

    let show_output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .args(["profile", "show"])
        .output()
        .expect("kcode should launch");
    assert_success(&show_output);
    let show_stdout = String::from_utf8(show_output.stdout).expect("stdout should be utf8");
    assert!(show_stdout.contains("Base URL env      KCODE_BASE_URL"));
    assert!(show_stdout.contains("API key env       KCODE_API_KEY"));
    assert!(show_stdout.contains("Default model     claude-sonnet-4-6"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn commands_command_reports_bridge_surface_for_toolless_profile() {
    let temp_dir = unique_temp_dir("commands-bridge");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&config_home).expect("config home should exist");
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

    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .args(["commands", "show", "bridge"])
        .output()
        .expect("kcode should launch");

    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Commands"));
    assert!(stdout.contains("Surface           bridge"));
    assert!(stdout.contains("Safety profile    bridge-safe"));
    assert!(stdout.contains("Supports tools    false"));
    assert!(stdout.contains("Supports stream   false"));
    assert!(stdout.contains("Filtered"));
    assert!(stdout.contains("/mcp"));
    assert!(stdout.contains("active profile does not expose tool-capable commands"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn mcp_command_is_blocked_when_profile_disables_tools() {
    let temp_dir = unique_temp_dir("mcp-blocked");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&config_home).expect("config home should exist");
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

    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .arg("mcp")
        .output()
        .expect("kcode should launch");

    assert!(
        !output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("command `mcp` is unavailable"));
    assert!(stderr.contains("active profile `bridge`"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn resumed_mcp_command_reports_unavailable_when_profile_disables_tools() {
    let temp_dir = unique_temp_dir("resume-mcp-blocked");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&config_home).expect("config home should exist");
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
    let session_path = write_session(&temp_dir, "resume-mcp-blocked");

    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .args([
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/mcp",
        ])
        .output()
        .expect("kcode should launch");

    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("command `/mcp` is unavailable"));
    assert!(stdout.contains("active profile `bridge`"));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

#[test]
fn resumed_help_hides_tool_commands_when_profile_disables_tools() {
    let temp_dir = unique_temp_dir("resume-help-blocked");
    let config_home = temp_dir.join("home").join(".kcode");
    fs::create_dir_all(&config_home).expect("config home should exist");
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
    let session_path = write_session(&temp_dir, "resume-help-blocked");

    let output = command_in(&temp_dir)
        .env("KCODE_CONFIG_HOME", &config_home)
        .args([
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/help",
        ])
        .output()
        .expect("kcode should launch");

    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Start here        /doctor, /config, /status, /memory"));
    assert!(!stdout.contains("/mcp [list|show <server>|help]"));
    assert!(!stdout.contains(
        "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
    ));

    fs::remove_dir_all(temp_dir).expect("cleanup temp dir");
}

fn command_in(cwd: &Path) -> Command {
    let home = cwd.join("__home");
    fs::create_dir_all(&home).expect("isolated home should exist");
    let mut command = Command::new(env!("CARGO_BIN_EXE_kcode"));
    command.current_dir(cwd);
    command.env("HOME", &home);
    command.env_remove("KCODE_CONFIG_HOME");
    command.env_remove("CLAW_CONFIG_HOME");
    command
}

fn write_session(root: &Path, label: &str) -> PathBuf {
    let session_path = root.join(format!("{label}.jsonl"));
    let mut session = Session::new();
    session
        .push_user_text(format!("session fixture for {label}"))
        .expect("session write should succeed");
    session
        .save_to_path(&session_path)
        .expect("session should persist");
    session_path
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_millis();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "kcode-{label}-{}-{millis}-{counter}",
        std::process::id()
    ))
}
