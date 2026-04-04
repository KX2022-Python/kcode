/// Self-healing mode: Automatically fix common configuration and environment issues.
fn run_doctor_fix(
    _model_override: Option<&str>,
    _profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Kcode Doctor: Self-Healing Mode\n");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let kcode_dir = format!("{}/.kcode", home);
    let mut fixed_count = 0;

    // 1. Ensure ~/.kcode directory exists with correct permissions
    if !std::path::Path::new(&kcode_dir).exists() {
        println!("📁 Creating {}...", kcode_dir);
        std::fs::create_dir_all(&kcode_dir)?;
        fixed_count += 1;
    }
    
    // Fix permissions if needed
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&kcode_dir) {
            let current_mode = meta.permissions().mode() & 0o777;
            if current_mode != 0o700 {
                println!("🔒 Fixing permissions on {} (was {:o}, setting to 700)", kcode_dir, current_mode);
                std::fs::set_permissions(&kcode_dir, std::fs::Permissions::from_mode(0o700))?;
                fixed_count += 1;
            }
        }
    }

    // 2. Ensure subdirectories exist
    for subdir in &["sessions", "memory", "bridge-sessions", "bridge-media"] {
        let path = format!("{}/{}", kcode_dir, subdir);
        if !std::path::Path::new(&path).exists() {
            println!("📁 Creating {}...", path);
            std::fs::create_dir_all(&path)?;
            fixed_count += 1;
        }
    }

    // 3. Scan and isolate corrupted session files
    let sessions_dir = format!("{}/sessions", kcode_dir);
    if std::path::Path::new(&sessions_dir).is_dir() {
        for entry in std::fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.is_empty() {
                        let backup = path.with_extension("jsonl.corrupted");
                        println!("⚠ Isolating empty session: {} -> {}", path.display(), backup.display());
                        std::fs::rename(&path, &backup)?;
                        fixed_count += 1;
                    }
                }
            }
        }
    }

    // 4. Clean up legacy residue from upstream import
    let legacy_files = [
        format!("{}/.claw.json", home),
        format!("{}/.claw/settings.json", home),
        format!("{}/.claw/settings.local.json", home),
    ];
    for file in &legacy_files {
        if std::path::Path::new(file).exists() {
            println!("🧹 Cleaning legacy residue: {}", file);
            let _ = std::fs::remove_file(file);
            fixed_count += 1;
        }
    }
    // Clean up .claw directory if empty
    let claw_dir = format!("{}/.claw", home);
    if std::path::Path::new(&claw_dir).is_dir() {
        if std::fs::read_dir(&claw_dir).map(|d| d.count() == 0).unwrap_or(false) {
            println!("🧹 Removing empty .claw directory");
            let _ = std::fs::remove_dir(&claw_dir);
            fixed_count += 1;
        }
    }

    // 5. Generate .env template if missing
    let env_template_path = format!("{}/.kcode/.env.example", home);
    if !std::path::Path::new(&env_template_path).exists() {
        println!("📝 Generating .env.example template...");
        let env_content = r#"# Kcode Bridge Environment Variables
# Copy this file to .env and fill in your values

# Core API Configuration
KCODE_API_KEY=your_api_key_here
KCODE_MODEL=your_model_name
# KCODE_BASE_URL=https://your-custom-endpoint

# Telegram Bot (Optional)
# KCODE_TELEGRAM_BOT_TOKEN=123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11

# WhatsApp Cloud API (Optional)
# KCODE_WHATSAPP_PHONE_ID=your_phone_id
# KCODE_WHATSAPP_TOKEN=your_access_token
# KCODE_WHATSAPP_APP_SECRET=your_app_secret

# Feishu/Lark (Optional)
# KCODE_FEISHU_APP_ID=your_app_id
# KCODE_FEISHU_APP_SECRET=your_app_secret

# Webhook Configuration (Optional, for production use)
# KCODE_WEBHOOK_URL=https://your-domain.com/webhook/telegram
# KCODE_WEBHOOK_VERIFY_TOKEN=your_verify_token
"#;
        std::fs::write(&env_template_path, env_content)?;
        fixed_count += 1;
    }

    // 6. Check Environment Variables and provide setup hints
    let mut missing_vars = Vec::new();
    let required_vars = ["KCODE_API_KEY", "KCODE_MODEL"];
    for var in required_vars {
        if std::env::var(var).is_err() {
            missing_vars.push(var.to_string());
        }
    }

    if !missing_vars.is_empty() {
        println!("\n⚠ Missing required environment variables:");
        for var in &missing_vars {
            println!("   • {}", var);
        }
        println!("\n💡 Quick Fix:");
        println!("   1. Copy ~/.kcode/.env.example to ~/.kcode/.env");
        println!("   2. Edit ~/.kcode/.env and fill in your values");
        println!("   3. Run: source ~/.kcode/.env");
    } else {
        println!("\n✅ All required environment variables are set.");
    }

    // 7. Check Bridge Channel Configuration
    let has_channel = ["KCODE_TELEGRAM_BOT_TOKEN", "KCODE_WHATSAPP_PHONE_ID", "KCODE_FEISHU_APP_ID"]
        .iter()
        .any(|v| std::env::var(v).is_ok());
    
    if !has_channel {
        println!("\nℹ No bridge channels configured.");
        println!("   → This is normal for REPL-only usage.");
        println!("   → To enable multi-channel bot mode, set KCODE_TELEGRAM_BOT_TOKEN or others.");
    } else {
        println!("\n✅ Bridge channels detected.");
    }

    println!("\n✅ Self-healing complete. Fixed {} issue(s).", fixed_count);
    println!("💡 Run 'kcode doctor' again to verify the system status.");
    
    Ok(())
}

fn print_config_show(
    section: Option<&str>,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_config_report(section, model_override, profile_override)?
    );
    Ok(())
}

fn print_commands_report(
    surface: CommandReportSurfaceSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_commands_report(surface, model_override, profile_override)?
    );
    Ok(())
}

fn print_profile_report(
    selection: &ProfileCommandSelection,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        render_profile_report(selection, model_override, profile_override)?
    );
    Ok(())
}

fn command_registry_context(
    setup: &SetupContext,
    surface: CommandReportSurfaceSelection,
) -> CommandRegistryContext {
    CommandRegistryContext::for_surface(
        surface.command_surface(),
        setup.active_profile.profile.supports_tools,
    )
}

fn command_descriptor_usage(descriptor: &CommandDescriptor) -> String {
    match (&descriptor.scope, &descriptor.argument_hint) {
        (CommandScope::Session, Some(argument_hint)) => {
            format!("/{} {}", descriptor.name, argument_hint)
        }
        (CommandScope::Session, None) => format!("/{}", descriptor.name),
        (CommandScope::Process, Some(argument_hint)) => {
            format!("{} {}", descriptor.name, argument_hint)
        }
        (CommandScope::Process, None) => descriptor.name.clone(),
    }
}

fn filtered_command_usage(filtered: &FilteredCommand) -> String {
    let name = filtered
        .id
        .rsplit('.')
        .next()
        .unwrap_or(filtered.id.as_str());
    match filtered.scope {
        CommandScope::Process => name.to_string(),
        CommandScope::Session => format!("/{name}"),
    }
}

fn filtered_command_name(filtered: &FilteredCommand) -> &str {
    filtered
        .id
        .rsplit('.')
        .next()
        .unwrap_or(filtered.id.as_str())
}

fn local_command_block_reason(
    profile: &ResolvedProviderProfile,
    scope: CommandScope,
    name: &str,
) -> Option<String> {
    let snapshot = build_command_registry_snapshot(
        &CommandRegistryContext::for_surface(
            CommandSurface::CliLocal,
            profile.profile.supports_tools,
        ),
        &[],
    );
    snapshot
        .filtered_out_commands
        .iter()
        .find(|command| {
            command.scope == scope
                && filtered_command_name(command) == name
                && command.reason == "active profile does not expose tool-capable commands"
        })
        .map(|command| command.reason.clone())
}

fn command_blocked_message(
    scope: CommandScope,
    name: &str,
    profile_name: &str,
    reason: &str,
) -> String {
    let rendered_name = match scope {
        CommandScope::Process => name.to_string(),
        CommandScope::Session => format!("/{name}"),
    };
    format!(
        "command `{rendered_name}` is unavailable for active profile `{profile_name}`: {reason}\nRun `{CLI_NAME} commands show local` to inspect the current command surface."
    )
}

fn ensure_process_command_available(
    name: &str,
    model_override: Option<&str>,
    profile_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(setup) = load_setup_context(
        SetupMode::Config,
        model_override,
        profile_override,
        default_permission_mode(),
        None,
    ) else {
        return Ok(());
    };
    if let Some(reason) =
        local_command_block_reason(&setup.active_profile, CommandScope::Process, name)
    {
        return Err(command_blocked_message(
            CommandScope::Process,
            name,
            &setup.active_profile.profile_name,
            &reason,
        )
        .into());
    }
    Ok(())
}

fn ensure_session_command_available_for_profile(
    command_name: &str,
    profile: &ResolvedProviderProfile,
) -> Result<(), String> {
    if let Some(reason) = local_command_block_reason(profile, CommandScope::Session, command_name) {
        return Err(command_blocked_message(
            CommandScope::Session,
            command_name,
            &profile.profile_name,
            &reason,
        ));
    }
    Ok(())
}
