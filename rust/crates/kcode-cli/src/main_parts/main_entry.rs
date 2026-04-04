fn main() {
    if let Err(error) = run() {
        let message = error.to_string();
        if message.contains(&format!("`{CLI_NAME} --help`")) {
            eprintln!("error: {message}");
        } else {
            eprintln!(
                "error: {message}

Run `{CLI_NAME} --help` for usage.
💡 If you are experiencing persistent issues, run:
   `{CLI_NAME} doctor` to diagnose
   `{CLI_NAME} doctor --fix` to automatically repair common problems"
            );
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        CliAction::Agents { args } => LiveCli::print_agents(args.as_deref())?,
        CliAction::Mcp { args, profile } => {
            ensure_process_command_available("mcp", None, profile.as_deref())?;
            LiveCli::print_mcp(args.as_deref())?
        }
        CliAction::Skills { args } => LiveCli::print_skills(args.as_deref())?,
        CliAction::PrintSystemPrompt { cwd, date } => print_system_prompt(cwd, date),
        CliAction::Version => print_version(),
        CliAction::ResumeSession {
            session_path,
            commands,
        } => resume_session(&session_path, &commands),
        CliAction::Doctor {
            model,
            model_explicit,
            profile,
            fix,
        } => print_doctor(model_explicit.then_some(model.as_str()), profile.as_deref(), fix)?,
        CliAction::ConfigShow {
            section,
            model,
            model_explicit,
            profile,
        } => print_config_show(
            section.as_deref(),
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
        )?,
        CliAction::Commands {
            surface,
            model,
            model_explicit,
            profile,
        } => print_commands_report(
            surface,
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
        )?,
        CliAction::Profile {
            selection,
            model,
            model_explicit,
            profile,
        } => print_profile_report(
            &selection,
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
        )?,
        CliAction::Status {
            model,
            model_explicit,
            profile,
            permission_mode,
        } => print_status_snapshot(
            &model,
            model_explicit.then_some(model.as_str()),
            profile.as_deref(),
            permission_mode,
        )?,
        CliAction::Sandbox => print_sandbox_status_snapshot()?,
        CliAction::Prompt {
            prompt,
            model,
            model_explicit,
            profile,
            output_format,
            allowed_tools,
            permission_mode,
        } => LiveCli::new(
            model,
            model_explicit,
            profile,
            true,
            allowed_tools,
            permission_mode,
            None,
        )?
        .run_turn_with_output(&prompt, output_format)?,
        CliAction::Login => run_login()?,
        CliAction::Logout => run_logout()?,
        CliAction::Init => run_init()?,
        CliAction::Tui { section } => tui::run(section.as_deref())?,
        CliAction::Repl {
            model,
            model_explicit,
            profile,
            allowed_tools,
            permission_mode,
        } => run_repl(
            model,
            model_explicit,
            profile,
            allowed_tools,
            permission_mode,
        )?,
        CliAction::Bridge {
            model,
            model_explicit,
            profile,
            permission_mode,
        } => run_bridge(
            model,
            model_explicit,
            profile,
            permission_mode,
        )?,
        CliAction::Help { profile } => print_help(profile.as_deref()),
    }
    Ok(())
}

// ... (other existing code)

fn default_oauth_config() -> OAuthConfig {
    OAuthConfig {
        client_id: String::from("kcode-legacy-oauth-disabled"),
        authorize_url: String::from("https://oauth.invalid/authorize"),
        token_url: String::from("https://oauth.invalid/token"),
        callback_port: None,
        manual_redirect_url: None,
        scopes: vec![String::from("legacy:disabled")],
    }
}

fn run_login() -> Result<(), Box<dyn std::error::Error>> {
    println!("Login is retired. Kcode uses configuration-driven authentication.");
    println!("Set your credentials via KCODE_API_KEY, KCODE_BASE_URL, KCODE_MODEL env vars.");
    println!("Or run `kcode tui` to edit ~/.kcode/config.toml and bridge settings.");
    println!("Run `kcode doctor` to verify your setup.");
    Ok(())
}

fn run_logout() -> Result<(), Box<dyn std::error::Error>> {
    println!("Logout is retired. Kcode does not use OAuth authentication.");
    println!("To change your API key, update KCODE_API_KEY or run `kcode tui`.");
    Ok(())
}

fn open_browser(url: &str) -> io::Result<()> {
    let commands = if cfg!(target_os = "macos") {
        vec![("open", vec![url])]
    } else if cfg!(target_os = "windows") {
        vec![("cmd", vec!["/C", "start", "", url])]
    } else {
        vec![("xdg-open", vec![url])]
    };
    for (program, args) in commands {
        match Command::new(program).args(args).spawn() {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no supported browser opener command found",
    ))
}

fn wait_for_oauth_callback(
    port: u16,
) -> Result<runtime::OAuthCallbackParams, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let (mut stream, _) = listener.accept()?;
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing callback request line")
    })?;
    let target = request_line.split_whitespace().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "missing callback request target",
        )
    })?;
    let callback = parse_oauth_callback_request_target(target)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let body = if callback.error.is_some() {
        "Legacy OAuth callback failed. You can close this window."
    } else {
        "Legacy OAuth callback succeeded. You can close this window."
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(callback)
}

fn print_system_prompt(cwd: PathBuf, date: String) {
    match load_system_prompt(cwd, date, env::consts::OS, "unknown") {
        Ok(sections) => println!("{}", sections.join("\n\n")),
        Err(error) => {
            eprintln!("failed to build system prompt: {error}");
            std::process::exit(1);
        }
    }
}

fn print_version() {
    println!("{}", render_version_report());
}

fn resume_session(session_path: &Path, commands: &[String]) {
    let resolved_path = if session_path.exists() {
        session_path.to_path_buf()
    } else {
        match resolve_session_reference(&session_path.display().to_string()) {
            Ok(handle) => handle.path,
            Err(error) => {
                eprintln!("failed to restore session: {error}");
                std::process::exit(1);
            }
        }
    };

    let session = match Session::load_from_path(&resolved_path) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("failed to restore session: {error}");
            std::process::exit(1);
        }
    };

    if commands.is_empty() {
        println!(
            "Restored session from {} ({} messages).",
            resolved_path.display(),
            session.messages.len()
        );
        return;
    }

    let mut session = session;
    for raw_command in commands {
        let command = match SlashCommand::parse(raw_command) {
            Ok(Some(command)) => command,
            Ok(None) => {
                eprintln!("unsupported resumed command: {raw_command}");
                std::process::exit(2);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        };
        match run_resume_command(&resolved_path, &session, &command) {
            Ok(ResumeCommandOutcome {
                session: next_session,
                message,
            }) => {
                session = next_session;
                if let Some(message) = message {
                    println!("{message}");
                }
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    }
}
