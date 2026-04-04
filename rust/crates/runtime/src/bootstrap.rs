use std::path::{Path, PathBuf};

use crate::config::ConfigEntry;
use crate::provider_profile::ResolvedProviderProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapPhase {
    CliEntry,
    FastPathVersion,
    StartupProfiler,
    SystemPromptFastPath,
    ChromeMcpFastPath,
    DaemonWorkerFastPath,
    BridgeFastPath,
    DaemonFastPath,
    BackgroundSessionFastPath,
    TemplateFastPath,
    EnvironmentRunnerFastPath,
    MainRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapPlan {
    phases: Vec<BootstrapPhase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupMode {
    Interactive,
    Print,
    Resume,
    Bridge,
    Doctor,
    Init,
    Config,
    Status,
    Sandbox,
}

impl SetupMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Print => "print",
            Self::Resume => "resume",
            Self::Bridge => "bridge",
            Self::Doctor => "doctor",
            Self::Init => "init",
            Self::Config => "config",
            Self::Status => "status",
            Self::Sandbox => "sandbox",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioMode {
    Interactive,
    NonInteractive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapInputs {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub platform: String,
    pub stdio_mode: StdioMode,
    pub invocation_kind: SetupMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub config_home: PathBuf,
    pub session_dir: PathBuf,
    pub discovered_entries: Vec<ConfigEntry>,
    pub loaded_entries: Vec<ConfigEntry>,
    pub config_file_present: bool,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key_env: String,
    pub api_key_present: bool,
    pub oauth_credentials_present: bool,
    pub profile: Option<String>,
    pub legacy_paths: Vec<PathBuf>,
}

impl ResolvedConfig {
    #[must_use]
    pub fn is_runtime_ready(&self) -> bool {
        self.base_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            && self.api_key_present
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustPolicyContext {
    pub permission_mode: String,
    pub workspace_writeable: bool,
    pub config_home_writeable: bool,
    pub trusted_workspace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupContext {
    pub inputs: BootstrapInputs,
    pub session_id: Option<String>,
    pub cwd: PathBuf,
    pub project_root: PathBuf,
    pub git_root: Option<PathBuf>,
    pub resolved_config: ResolvedConfig,
    pub active_profile: ResolvedProviderProfile,
    pub trust_policy: TrustPolicyContext,
    pub mode: SetupMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStatus {
    Ok,
    Warn,
    Fail,
}

impl DiagnosticStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: DiagnosticStatus,
    pub detail: String,
}

impl BootstrapPlan {
    #[must_use]
    pub fn claude_code_default() -> Self {
        Self::from_phases(vec![
            BootstrapPhase::CliEntry,
            BootstrapPhase::FastPathVersion,
            BootstrapPhase::StartupProfiler,
            BootstrapPhase::SystemPromptFastPath,
            BootstrapPhase::ChromeMcpFastPath,
            BootstrapPhase::DaemonWorkerFastPath,
            BootstrapPhase::BridgeFastPath,
            BootstrapPhase::DaemonFastPath,
            BootstrapPhase::BackgroundSessionFastPath,
            BootstrapPhase::TemplateFastPath,
            BootstrapPhase::EnvironmentRunnerFastPath,
            BootstrapPhase::MainRuntime,
        ])
    }

    #[must_use]
    pub fn from_phases(phases: Vec<BootstrapPhase>) -> Self {
        let mut deduped = Vec::new();
        for phase in phases {
            if !deduped.contains(&phase) {
                deduped.push(phase);
            }
        }
        Self { phases: deduped }
    }

    #[must_use]
    pub fn phases(&self) -> &[BootstrapPhase] {
        &self.phases
    }
}

#[must_use]
pub fn is_path_effectively_writeable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| !metadata.permissions().readonly())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        is_path_effectively_writeable, BootstrapPhase, BootstrapPlan, DiagnosticCheck,
        DiagnosticStatus, ResolvedConfig, SetupMode,
    };
    use crate::config::ConfigEntry;
    use crate::provider_profile::{
        CredentialResolution, CredentialSource, ProviderProfile, ResolvedProviderProfile,
        ResolutionSource,
    };
    use crate::ConfigSource;
    use std::path::PathBuf;

    #[test]
    fn from_phases_deduplicates_while_preserving_order() {
        let phases = vec![
            BootstrapPhase::CliEntry,
            BootstrapPhase::FastPathVersion,
            BootstrapPhase::CliEntry,
            BootstrapPhase::MainRuntime,
            BootstrapPhase::FastPathVersion,
        ];

        let plan = BootstrapPlan::from_phases(phases);

        assert_eq!(
            plan.phases(),
            &[
                BootstrapPhase::CliEntry,
                BootstrapPhase::FastPathVersion,
                BootstrapPhase::MainRuntime,
            ]
        );
    }

    #[test]
    fn claude_code_default_covers_each_phase_once() {
        let expected = [
            BootstrapPhase::CliEntry,
            BootstrapPhase::FastPathVersion,
            BootstrapPhase::StartupProfiler,
            BootstrapPhase::SystemPromptFastPath,
            BootstrapPhase::ChromeMcpFastPath,
            BootstrapPhase::DaemonWorkerFastPath,
            BootstrapPhase::BridgeFastPath,
            BootstrapPhase::DaemonFastPath,
            BootstrapPhase::BackgroundSessionFastPath,
            BootstrapPhase::TemplateFastPath,
            BootstrapPhase::EnvironmentRunnerFastPath,
            BootstrapPhase::MainRuntime,
        ];

        let plan = BootstrapPlan::claude_code_default();

        assert_eq!(plan.phases(), &expected);
    }

    #[test]
    fn setup_mode_labels_are_stable() {
        assert_eq!(SetupMode::Doctor.as_str(), "doctor");
        assert_eq!(SetupMode::Print.as_str(), "print");
    }

    #[test]
    fn resolved_config_runtime_ready_requires_url_and_credentials() {
        let sample_entry = ConfigEntry {
            source: ConfigSource::User,
            path: PathBuf::from("/tmp/config.toml"),
        };
        let config = ResolvedConfig {
            config_home: PathBuf::from("/tmp/.kcode"),
            session_dir: PathBuf::from("/tmp/.kcode/sessions"),
            discovered_entries: vec![sample_entry.clone()],
            loaded_entries: vec![sample_entry],
            config_file_present: true,
            model: "gpt-4.1".to_string(),
            base_url: Some("https://example.test".to_string()),
            api_key_env: "KCODE_API_KEY".to_string(),
            api_key_present: true,
            oauth_credentials_present: false,
            profile: Some("default".to_string()),
            legacy_paths: Vec::new(),
        };

        assert!(config.is_runtime_ready());

        let profile = ResolvedProviderProfile {
            profile_name: "custom".to_string(),
            profile_source: ResolutionSource::ProfileDefault,
            model: "gpt-4.1-mini".to_string(),
            model_source: ResolutionSource::ProfileDefault,
            base_url: Some("https://example.test".to_string()),
            base_url_source: ResolutionSource::ProfileDefault,
            credential: CredentialResolution {
                source: CredentialSource::PrimaryEnv,
                env_name: "KCODE_API_KEY".to_string(),
                api_key: Some("test-key".to_string()),
            },
            profile: ProviderProfile {
                name: "custom".to_string(),
                base_url_env: "KCODE_BASE_URL".to_string(),
                base_url: "https://example.test".to_string(),
                api_key_env: "KCODE_API_KEY".to_string(),
                default_model: "gpt-4.1-mini".to_string(),
                supports_tools: true,
                supports_streaming: true,
                request_timeout_ms: 120_000,
                max_retries: 2,
            },
        };
        assert_eq!(profile.profile_name, "custom");
    }

    #[test]
    fn diagnostic_labels_are_stable() {
        let check = DiagnosticCheck {
            name: "config".to_string(),
            status: DiagnosticStatus::Warn,
            detail: "missing".to_string(),
        };
        assert_eq!(check.status.label(), "warn");
    }

    #[test]
    fn writeable_path_helper_handles_missing_path() {
        assert!(!is_path_effectively_writeable(std::path::Path::new(
            "/definitely/missing/path"
        )));
    }
}
