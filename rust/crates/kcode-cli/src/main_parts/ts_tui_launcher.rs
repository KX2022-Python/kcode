fn invoked_as_engine() -> bool {
    env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|name| name.to_string_lossy().to_string()))
        .is_some_and(|name| name == "kcode-engine")
}

fn launch_ts_tui_default() -> Result<bool, Box<dyn std::error::Error>> {
    if env::var("KCODE_TUI").is_ok_and(|value| value.eq_ignore_ascii_case("rust")) {
        return Ok(false);
    }
    let Some(entry) = resolve_ts_tui_entry() else {
        eprintln!(
            "warning: TS TUI bundle not found; falling back to Rust TUI. Run ./scripts/install.sh to install the bundled frontend."
        );
        return Ok(false);
    };
    let Some(node) = resolve_node_runtime() else {
        eprintln!("warning: node runtime not found; falling back to Rust TUI.");
        return Ok(false);
    };

    let mut command = Command::new(node);
    command.arg(entry);
    configure_engine_env(&mut command)?;
    let status = command.status()?;
    if status.success() {
        Ok(true)
    } else {
        Err(format!("TS TUI exited with status {status}").into())
    }
}

fn resolve_node_runtime() -> Option<String> {
    env::var("KCODE_NODE").ok().filter(|value| !value.trim().is_empty()).or_else(|| {
        Command::new("node")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|_| "node".to_string())
    })
}

fn resolve_ts_tui_entry() -> Option<PathBuf> {
    if let Some(path) = env::var_os("KCODE_TS_TUI_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    let installed = PathBuf::from("/usr/local/lib/kcode/tui/dist/index.js");
    if installed.is_file() {
        return Some(installed);
    }
    source_tree_tui_entry().filter(|path| path.is_file())
}

fn source_tree_tui_entry() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(|root| root.join("tui").join("dist").join("index.js"))
}

fn configure_engine_env(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    if env::var_os("KCODE_ENGINE_BIN").is_some() {
        return Ok(());
    }
    let installed_engine = PathBuf::from("/usr/local/bin/kcode-engine");
    if installed_engine.is_file() {
        command.env("KCODE_ENGINE_BIN", installed_engine);
        return Ok(());
    }
    let current = env::current_exe()?;
    command.env("KCODE_ENGINE_BIN", current);
    command.env("KCODE_ENGINE_ARGS", "--headless");
    Ok(())
}
