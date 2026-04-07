    use super::{
        build_runtime_plugin_state_with_loader, build_runtime_with_plugin_state,
        create_managed_session_handle, describe_tool_progress,
        ensure_session_command_available_for_profile, filter_tool_specs, format_bughunter_report,
        format_commit_preflight_report, format_commit_skipped_report, format_compact_report,
        format_cost_report, format_internal_prompt_progress_line, format_issue_report,
        format_model_report, format_model_switch_report, format_permissions_report,
        format_permissions_switch_report, format_pr_report, format_resume_report,
        format_review_report, format_status_report, format_tool_call_start, format_tool_result, format_ultraplan_report,
        format_unknown_slash_command, format_unknown_slash_command_message,
        normalize_permission_mode, parse_args, parse_git_status_branch,
        parse_git_status_metadata_for, parse_git_workspace_summary, permission_policy,
        print_help_to, print_help_to_for_profile, push_output_block, render_commands_report,
        render_config_report, render_diff_report, render_doctor_report_from_setup,
        render_memory_report, render_repl_help, render_repl_help_for_profile,
        render_resume_usage, resolve_model_alias, resolve_session_reference, response_to_events,
        resume_supported_slash_commands, run_plan_mode_command, run_resume_command,
        slash_command_completion_candidates_with_sessions, status_context, validate_no_args,
        CliAction, CliOutputFormat, CommandReportSurfaceSelection, GitWorkspaceSummary,
        InternalPromptProgressEvent, InternalPromptProgressState, LiveCli, ProviderRuntimeClient,
        SlashCommand, StatusUsage, DEFAULT_MODEL,
    };
    use crate::bridge_core::{SessionConfig, SessionManager};
    use api::{MessageResponse, OutputContentBlock, Usage};
    use plugins::{
        PluginManager, PluginManagerConfig, PluginTool, PluginToolDefinition, PluginToolPermission,
    };
    use runtime::{
        AssistantEvent, ConfigLoader, ContentBlock, ConversationMessage, CredentialResolution,
        CredentialSource, MessageRole, PermissionMode, PermissionOutcome, ProviderProfile,
        ResolutionSource, ResolvedProviderProfile, Session,
    };
    use serde_json::json;
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, MutexGuard, OnceLock,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tools::GlobalToolRegistry;

    thread_local! {
        static ENV_LOCK_DEPTH: Cell<usize> = const { Cell::new(0) };
        static ENV_LOCK_GUARD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
    }

    fn registry_with_plugin_tool() -> GlobalToolRegistry {
        GlobalToolRegistry::with_plugin_tools(vec![PluginTool::new(
            "plugin-demo@external",
            "plugin-demo",
            PluginToolDefinition {
                name: "plugin_echo".to_string(),
                description: Some("Echo plugin payload".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            },
            "echo".to_string(),
            Vec::new(),
            PluginToolPermission::WorkspaceWrite,
            None,
        )])
        .expect("plugin tool registry should build")
    }

    fn temp_dir() -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("kcode-cli-{nanos}-{id}"))
    }

    fn git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git command should run");
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    struct EnvLockGuard;

    fn env_lock() -> EnvLockGuard {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK_DEPTH.with(|depth| {
            if depth.get() == 0 {
                let guard = LOCK
                    .get_or_init(|| Mutex::new(()))
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                ENV_LOCK_GUARD.with(|slot| {
                    *slot.borrow_mut() = Some(guard);
                });
            }
            depth.set(depth.get() + 1);
        });

        EnvLockGuard
    }

    impl Drop for EnvLockGuard {
        fn drop(&mut self) {
            ENV_LOCK_DEPTH.with(|depth| {
                let next = depth.get().saturating_sub(1);
                depth.set(next);
                if next == 0 {
                    ENV_LOCK_GUARD.with(|slot| {
                        slot.borrow_mut().take();
                    });
                }
            });
        }
    }

    fn with_current_dir<T>(cwd: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = env_lock();
        let previous = std::env::current_dir().unwrap_or_else(|_| fallback_cwd());
        std::env::set_current_dir(cwd).expect("cwd should change");
        let restore = CurrentDirRestore { previous };
        let result = f();
        drop(restore);
        result
    }

    fn fallback_cwd() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_dir())
            .unwrap_or_else(|| PathBuf::from("/"))
    }

    struct CurrentDirRestore {
        previous: PathBuf,
    }

    impl Drop for CurrentDirRestore {
        fn drop(&mut self) {
            let target = if self.previous.is_dir() {
                self.previous.as_path()
            } else {
                Path::new("/")
            };
            std::env::set_current_dir(target).expect("cwd should restore");
        }
    }

    fn test_setup_context(workspace: &Path) -> runtime::SetupContext {
        runtime::SetupContext {
            inputs: runtime::BootstrapInputs {
                argv: vec!["kcode".to_string(), "doctor".to_string()],
                cwd: workspace.to_path_buf(),
                platform: "linux".to_string(),
                stdio_mode: runtime::StdioMode::NonInteractive,
                invocation_kind: runtime::SetupMode::Doctor,
            },
            session_id: None,
            cwd: workspace.to_path_buf(),
            project_root: workspace.to_path_buf(),
            git_root: None,
            resolved_config: runtime::ResolvedConfig {
                config_home: workspace.join(".kcode"),
                session_dir: workspace.join(".kcode").join("sessions"),
                discovered_entries: Vec::new(),
                loaded_entries: Vec::new(),
                config_file_present: false,
                model: DEFAULT_MODEL.to_string(),
                base_url: None,
                api_key_env: "KCODE_API_KEY".to_string(),
                api_key_present: false,
                oauth_credentials_present: false,
                profile: None,
                legacy_paths: Vec::new(),
            },
            active_profile: ResolvedProviderProfile {
                profile_name: "cliproxyapi".to_string(),
                profile_source: ResolutionSource::ProfileDefault,
                model: DEFAULT_MODEL.to_string(),
                model_source: ResolutionSource::ProfileDefault,
                base_url: None,
                base_url_source: ResolutionSource::Missing,
                credential: CredentialResolution {
                    source: CredentialSource::Missing,
                    env_name: "KCODE_API_KEY".to_string(),
                    api_key: None,
                },
                profile: ProviderProfile {
                    name: "cliproxyapi".to_string(),
                    base_url_env: "KCODE_BASE_URL".to_string(),
                    base_url: String::new(),
                    api_key_env: "KCODE_API_KEY".to_string(),
                    default_model: DEFAULT_MODEL.to_string(),
                    supports_tools: true,
                    supports_streaming: true,
                    request_timeout_ms: 120_000,
                    max_retries: 2,
                },
            },
            trust_policy: runtime::TrustPolicyContext {
                permission_mode: "danger-full-access".to_string(),
                workspace_writeable: true,
                config_home_writeable: true,
                trusted_workspace: true,
            },
            mode: runtime::SetupMode::Doctor,
        }
    }

    fn write_plugin_fixture(root: &Path, name: &str, include_hooks: bool, include_lifecycle: bool) {
        fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
        if include_hooks {
            fs::create_dir_all(root.join("hooks")).expect("hooks dir");
            fs::write(
                root.join("hooks").join("pre.sh"),
                "#!/bin/sh\nprintf 'plugin pre hook'\n",
            )
            .expect("write hook");
        }
        if include_lifecycle {
            fs::create_dir_all(root.join("lifecycle")).expect("lifecycle dir");
            fs::write(
                root.join("lifecycle").join("init.sh"),
                "#!/bin/sh\nprintf 'init\\n' >> lifecycle.log\n",
            )
            .expect("write init lifecycle");
            fs::write(
                root.join("lifecycle").join("shutdown.sh"),
                "#!/bin/sh\nprintf 'shutdown\\n' >> lifecycle.log\n",
            )
            .expect("write shutdown lifecycle");
        }

        let hooks = if include_hooks {
            ",\n  \"hooks\": {\n    \"PreToolUse\": [\"./hooks/pre.sh\"]\n  }"
        } else {
            ""
        };
        let lifecycle = if include_lifecycle {
            ",\n  \"lifecycle\": {\n    \"Init\": [\"./lifecycle/init.sh\"],\n    \"Shutdown\": [\"./lifecycle/shutdown.sh\"]\n  }"
        } else {
            ""
        };
        fs::write(
            root.join(".claude-plugin").join("plugin.json"),
            format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\",\n  \"description\": \"runtime plugin fixture\"{hooks}{lifecycle}\n}}"
            ),
        )
        .expect("write plugin manifest");
    }
