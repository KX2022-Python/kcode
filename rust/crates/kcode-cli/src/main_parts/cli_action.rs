#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    Agents {
        args: Option<String>,
    },
    Mcp {
        args: Option<String>,
        profile: Option<String>,
    },
    Skills {
        args: Option<String>,
    },
    PrintSystemPrompt {
        cwd: PathBuf,
        date: String,
    },
    Version,
    ResumeSession {
        session_path: PathBuf,
        commands: Vec<String>,
    },
    Doctor {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        fix: bool,
    },
    ConfigShow {
        section: Option<String>,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    Commands {
        surface: CommandReportSurfaceSelection,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    Profile {
        selection: ProfileCommandSelection,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
    },
    Status {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        permission_mode: PermissionMode,
    },
    Sandbox,
    Prompt {
        prompt: String,
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        output_format: CliOutputFormat,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    Login,
    Logout,
    Init,
    Repl {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    Bridge {
        model: String,
        model_explicit: bool,
        profile: Option<String>,
        permission_mode: PermissionMode,
    },
    Help {
        profile: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfileCommandSelection {
    List,
    Show { profile_name: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandReportSurfaceSelection {
    Local,
    Bridge,
}

impl CommandReportSurfaceSelection {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "bridge" => Ok(Self::Bridge),
            other => Err(format!(
                "unsupported commands surface: {other} (expected local or bridge)"
            )),
        }
    }

    const fn command_surface(self) -> CommandSurface {
        match self {
            Self::Local => CommandSurface::CliLocal,
            Self::Bridge => CommandSurface::Bridge,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Bridge => "bridge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliOutputFormat {
    Text,
    Json,
}

impl CliOutputFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unsupported value for --output-format: {other} (expected text or json)"
            )),
        }
    }
}
