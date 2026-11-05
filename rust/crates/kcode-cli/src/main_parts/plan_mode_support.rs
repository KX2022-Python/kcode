struct PlanModeCommandOutcome {
    message: String,
    next_permission_mode: Option<PermissionMode>,
}

fn run_plan_mode_command(
    cwd: &Path,
    mode: Option<&str>,
    current_mode: PermissionMode,
) -> Result<PlanModeCommandOutcome, Box<dyn std::error::Error>> {
    match mode.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("status") => Ok(PlanModeCommandOutcome {
            message: render_plan_mode_report(cwd, current_mode)?,
            next_permission_mode: None,
        }),
        Some("on") => {
            let output = execute_plan_mode_tool("EnterPlanMode")?;
            Ok(PlanModeCommandOutcome {
                message: format_plan_mode_update_report("enabled", &output, PermissionMode::Plan),
                next_permission_mode: Some(PermissionMode::Plan),
            })
        }
        Some("off") => {
            let output = execute_plan_mode_tool("ExitPlanMode")?;
            let next_mode = default_permission_mode_for(cwd);
            Ok(PlanModeCommandOutcome {
                message: format_plan_mode_update_report("disabled", &output, next_mode),
                next_permission_mode: Some(next_mode),
            })
        }
        Some(other) => Err(std::io::Error::other(format!(
            "unsupported /plan mode '{other}'"
        ))
        .into()),
    }
}

fn render_plan_mode_report(
    cwd: &Path,
    current_mode: PermissionMode,
) -> Result<String, Box<dyn std::error::Error>> {
    let settings_path = cwd.join(PRIMARY_CONFIG_DIR_NAME).join("settings.local.json");
    let state_path = cwd
        .join(PRIMARY_CONFIG_DIR_NAME)
        .join("tool-state")
        .join("plan-mode.json");
    let local_override = read_local_permission_mode(&settings_path)?;

    Ok(format!(
        "Plan mode
  Local override    {}
  Current session   {}
  Effective default {}
  Managed state     {}
  Config file       {}
  State file        {}
  Usage             /plan [on|off|status]",
        if matches!(local_override.as_deref(), Some("plan")) {
            "active"
        } else {
            "inactive"
        },
        current_mode.as_str(),
        default_permission_mode_for(cwd).as_str(),
        if state_path.exists() { "present" } else { "absent" },
        settings_path.display(),
        state_path.display(),
    ))
}

fn format_plan_mode_update_report(
    status: &str,
    output: &PlanModeToolOutput,
    next_mode: PermissionMode,
) -> String {
    format!(
        "Plan mode updated
  Status           {status}
  Current session  {}
  Config file      {}
  State file       {}
  Detail           {}
  Usage            /plan to inspect current state",
        next_mode.as_str(),
        output.settings_path,
        output.state_path,
        output.message,
    )
}

fn read_local_permission_mode(
    settings_path: &Path,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(read_settings_object(settings_path)?
        .get("permissions")
        .and_then(serde_json::Value::as_object)
        .and_then(|permissions| permissions.get("defaultMode"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned))
}

fn execute_plan_mode_tool(
    tool_name: &str,
) -> Result<PlanModeToolOutput, Box<dyn std::error::Error>> {
    let raw = tools::execute_tool(tool_name, &json!({})).map_err(std::io::Error::other)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(PlanModeToolOutput {
        message: value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        settings_path: value
            .get("settingsPath")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        state_path: value
            .get("statePath")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

struct PlanModeToolOutput {
    message: String,
    settings_path: String,
    state_path: String,
}
