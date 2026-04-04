impl LiveCli {
    fn tui_welcome_messages(&self) -> Vec<tui::repl::RenderableMessage> {
        let cwd = env::current_dir().map_or_else(
            |_| "<unknown>".to_string(),
            |path| path.display().to_string(),
        );
        let status = status_context(Some(&self.session.path)).ok();
        let branch = status
            .as_ref()
            .and_then(|context| context.git_branch.as_deref())
            .unwrap_or("unknown");
        let workspace = status.as_ref().map_or_else(
            || "unknown".to_string(),
            |context| context.git_summary.headline(),
        );

        vec![
            tui::repl::RenderableMessage::AssistantText {
                text: format!(
                    "Interactive session ready\n\
  Model        {}\n\
  Profile      {}\n\
  Permissions  {}\n\
  Branch       {}\n\
  Workspace    {}\n\
  Directory    {}\n\
  Session      {}\n\
  Auto-save    {}",
                    self.model,
                    self.active_profile.profile_name,
                    self.permission_mode.as_str(),
                    branch,
                    workspace,
                    cwd,
                    self.session.id,
                    self.session.path.display(),
                ),
                streaming: false,
            },
            tui::repl::RenderableMessage::System {
                message:
                    "Enter 发送消息 · Shift+Enter 换行 · `/` 打开命令面板 · PgUp/PgDn 浏览历史"
                        .to_string(),
                level: tui::repl::SysLevel::Info,
            },
        ]
    }

    fn prepare_tui_turn_runtime(
        &self,
    ) -> Result<(BuiltRuntime, HookAbortMonitor), Box<dyn std::error::Error>> {
        let hook_abort_signal = runtime::HookAbortSignal::new();
        let runtime = build_tui_runtime(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.model_explicit.then_some(self.model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            self.allowed_tools.clone(),
            self.permission_mode,
        )?
        .with_hook_abort_signal(hook_abort_signal.clone());
        let hook_abort_monitor = HookAbortMonitor::spawn(hook_abort_signal);
        Ok((runtime, hook_abort_monitor))
    }

    fn replace_tui_runtime(
        &mut self,
        session: Session,
        model: String,
        model_explicit: bool,
        permission_mode: PermissionMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = build_tui_runtime(
            session,
            &self.session.id,
            model.clone(),
            model_explicit.then_some(model.as_str()),
            self.profile_override.as_deref(),
            self.system_prompt.clone(),
            self.allowed_tools.clone(),
            permission_mode,
        )?;
        self.replace_runtime(runtime)?;
        self.model = model;
        self.model_explicit = model_explicit;
        self.permission_mode = permission_mode;
        Ok(())
    }

    fn run_turn_tui(
        &mut self,
        input: &str,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        let previous_message_count = self.runtime.session().messages.len();
        let (mut runtime, hook_abort_monitor) = self.prepare_tui_turn_runtime()?;
        let mut permission_prompter = TuiPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();

        match result {
            Ok(summary) => {
                self.replace_runtime(runtime)?;
                self.persist_session()?;

                let mut backend =
                    backend_result_from_session_slice(&self.runtime.session().messages, previous_message_count);
                if let Some(event) = summary.auto_compaction {
                    backend.messages.push(tui::repl::RenderableMessage::System {
                        message: format_auto_compaction_notice(event.removed_message_count),
                        level: tui::repl::SysLevel::Info,
                    });
                }
                if summary.compaction_circuit_tripped {
                    backend.messages.push(tui::repl::RenderableMessage::System {
                        message: format!(
                            "自动压缩保护已触发：连续失败 {} 次，请运行 /compact 检查。",
                            MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES
                        ),
                        level: tui::repl::SysLevel::Warning,
                    });
                }

                let usage = self.runtime.usage().current_turn_usage();
                backend.input_tokens = Some(u64::from(usage.input_tokens));
                backend.output_tokens = Some(u64::from(usage.output_tokens));
                Ok(backend)
            }
            Err(error) => {
                runtime.shutdown_plugins()?;
                Err(Box::new(error))
            }
        }
    }

    fn handle_tui_command(
        &mut self,
        raw_command: &str,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        let command = SlashCommand::parse(raw_command)?
            .ok_or_else(|| std::io::Error::other("empty slash command"))?;

        match command {
            SlashCommand::Help => Ok(tui_text_result(render_repl_help_for_profile(
                self.active_profile.profile.supports_tools,
            ))),
            SlashCommand::Status => Ok(tui_text_result(format_status_report(
                &self.model,
                Some(&self.active_profile),
                StatusUsage {
                    message_count: self.runtime.session().messages.len(),
                    turns: self.runtime.usage().turns(),
                    latest: self.runtime.usage().current_turn_usage(),
                    cumulative: self.runtime.usage().cumulative_usage(),
                    estimated_tokens: self.runtime.estimated_tokens(),
                },
                self.permission_mode.as_str(),
                &status_context(Some(&self.session.path))?,
            ))),
            SlashCommand::Sandbox => {
                let cwd = env::current_dir()?;
                let loader = ConfigLoader::default_for(&cwd);
                let runtime_config = loader.load()?;
                Ok(tui_text_result(format_sandbox_report(
                    &resolve_sandbox_status(runtime_config.sandbox(), &cwd),
                )))
            }
            SlashCommand::Compact => {
                let result = runtime::compact_session(
                    self.runtime.session(),
                    CompactionConfig {
                        max_estimated_tokens: 0,
                        ..CompactionConfig::default()
                    },
                );
                let removed = result.removed_message_count;
                let kept = result.compacted_session.messages.len();
                let skipped = removed == 0;
                self.replace_tui_runtime(
                    result.compacted_session,
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                self.persist_session()?;
                Ok(tui_text_result(format_compact_report(removed, kept, skipped)))
            }
            SlashCommand::Clear { confirm } => {
                if !confirm {
                    return Ok(tui_text_result(
                        "clear: confirmation required; run /clear --confirm to start a fresh session."
                            .to_string(),
                    ));
                }

                let previous_session = self.session.clone();
                let session_state = Session::new();
                self.session = create_managed_session_handle(&session_state.session_id)?;
                self.replace_tui_runtime(
                    session_state.with_persistence_path(self.session.path.clone()),
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                self.persist_session()?;
                Ok(tui_text_result(format!(
                    "Session cleared\n  Mode             fresh session\n  Previous session {}\n  Resume previous  /resume {}\n  Preserved model  {}\n  Permission mode  {}\n  New session      {}\n  Session file     {}",
                    previous_session.id,
                    previous_session.id,
                    self.model,
                    self.permission_mode.as_str(),
                    self.session.id,
                    self.session.path.display(),
                )))
            }
            SlashCommand::Cost => Ok(tui_text_result(format_cost_report(
                self.runtime.usage().cumulative_usage(),
            ))),
            SlashCommand::Resume { session_path } => {
                let Some(session_ref) = session_path else {
                    return Ok(tui_text_result(render_resume_usage()));
                };

                let handle = resolve_session_reference(&session_ref)?;
                let session = Session::load_from_path(&handle.path)?;
                let message_count = session.messages.len();
                let session_id = session.session_id.clone();
                self.session = SessionHandle {
                    id: session_id,
                    path: handle.path,
                };
                self.replace_tui_runtime(
                    session,
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                Ok(tui_text_result(format_resume_report(
                    &self.session.path.display().to_string(),
                    message_count,
                    self.runtime.usage().turns(),
                )))
            }
            SlashCommand::Model { model } => {
                let Some(next_model) = model else {
                    return Ok(tui_text_result(format_model_report(
                        &self.model,
                        &self.active_profile.profile_name,
                        self.runtime.session().messages.len(),
                        self.runtime.usage().turns(),
                    )));
                };

                let next_model = resolve_model_alias(&next_model).to_string();
                if next_model == self.model {
                    return Ok(tui_text_result(format_model_report(
                        &self.model,
                        &self.active_profile.profile_name,
                        self.runtime.session().messages.len(),
                        self.runtime.usage().turns(),
                    )));
                }

                let previous = self.model.clone();
                let message_count = self.runtime.session().messages.len();
                self.replace_tui_runtime(
                    self.runtime.session().clone(),
                    next_model.clone(),
                    true,
                    self.permission_mode,
                )?;
                Ok(tui_text_result(format_model_switch_report(
                    &previous,
                    &next_model,
                    &self.active_profile.profile_name,
                    message_count,
                )))
            }
            SlashCommand::Permissions { mode } => {
                let Some(mode) = mode else {
                    return Ok(tui_text_result(format_permissions_report(
                        self.permission_mode.as_str(),
                    )));
                };

                let normalized = normalize_permission_mode(&mode).ok_or_else(|| {
                    format!(
                        "unsupported permission mode '{mode}'. Use read-only, workspace-write, or danger-full-access."
                    )
                })?;
                if normalized == self.permission_mode.as_str() {
                    return Ok(tui_text_result(format_permissions_report(normalized)));
                }

                let previous = self.permission_mode.as_str().to_string();
                self.replace_tui_runtime(
                    self.runtime.session().clone(),
                    self.model.clone(),
                    self.model_explicit,
                    permission_mode_from_label(normalized),
                )?;
                Ok(tui_text_result(format_permissions_switch_report(
                    &previous,
                    normalized,
                )))
            }
            SlashCommand::Config { section } => Ok(tui_text_result(render_config_report(
                section.as_deref(),
                self.model_explicit.then_some(self.model.as_str()),
                self.profile_override.as_deref(),
            )?)),
            SlashCommand::Mcp { action, target } => {
                if let Err(message) =
                    ensure_session_command_available_for_profile("mcp", &self.active_profile)
                {
                    return Ok(tui_text_result(message));
                }
                let cwd = env::current_dir()?;
                let args = match (action.as_deref(), target.as_deref()) {
                    (None, None) => None,
                    (Some(action), None) => Some(action.to_string()),
                    (Some(action), Some(target)) => Some(format!("{action} {target}")),
                    (None, Some(target)) => Some(target.to_string()),
                };
                Ok(tui_text_result(handle_mcp_slash_command(args.as_deref(), &cwd)?))
            }
            SlashCommand::Memory => Ok(tui_text_result(render_memory_report()?)),
            SlashCommand::Tasks { args } => Ok(tui_text_result(match args.as_deref() {
                None | Some("list") => {
                    "Tasks\n  Background tasks are managed through the Agent tool.\n  Use /help Agent to see how to create and manage tasks.".to_string()
                }
                Some("help") => {
                    "Tasks\n  Background tasks allow running multiple agent sessions in parallel.\n\n  Commands:\n    /tasks              List active tasks\n    /tasks help         Show this help\n\n  Task management is done through the Agent tool:\n    Agent(action=create, description='...')   Create a new task\n    Agent(action=list)                        List all tasks\n    Agent(action=stop, task_id='...')         Stop a running task\n    Agent(action=output, task_id='...')       Get task output".to_string()
                }
                other => format!(
                    "Unknown tasks argument: {}. Use /tasks help for usage.",
                    other.unwrap_or("")
                ),
            })),
            SlashCommand::Doctor => Ok(tui_text_result(render_doctor_report(
                self.model_explicit.then_some(self.model.as_str()),
                self.profile_override.as_deref(),
            )?)),
            SlashCommand::Init => Ok(tui_text_result(init_repo_kcode_md()?)),
            SlashCommand::Diff => Ok(tui_text_result(render_diff_report()?)),
            SlashCommand::Version => Ok(tui_text_result(render_version_report())),
            SlashCommand::Export { path } => {
                let export_path = resolve_export_path(path.as_deref(), self.runtime.session())?;
                fs::write(&export_path, render_export_text(self.runtime.session()))?;
                Ok(tui_text_result(format!(
                    "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
                    export_path.display(),
                    self.runtime.session().messages.len(),
                )))
            }
            SlashCommand::Agents { args } => {
                let cwd = env::current_dir()?;
                Ok(tui_text_result(handle_agents_slash_command(args.as_deref(), &cwd)?))
            }
            SlashCommand::Skills { args } => {
                let cwd = env::current_dir()?;
                Ok(tui_text_result(handle_skills_slash_command(args.as_deref(), &cwd)?))
            }
            SlashCommand::Hooks { .. } => Ok(tui_text_result(
                "Run `kcode tui extensions` to manage hooks and plugins.".to_string(),
            )),
            SlashCommand::Keybindings
            | SlashCommand::PrivacySettings
            | SlashCommand::Theme { .. }
            | SlashCommand::Voice { .. }
            | SlashCommand::Color { .. }
            | SlashCommand::OutputStyle { .. } => Ok(tui_text_result(
                "Run `kcode tui appearance` to manage UI and privacy settings.".to_string(),
            )),
            SlashCommand::Unknown(name) => Err(format_unknown_slash_command(&name).into()),
            SlashCommand::Bughunter { .. }
            | SlashCommand::Commit
            | SlashCommand::Pr { .. }
            | SlashCommand::Issue { .. }
            | SlashCommand::DebugToolCall { .. }
            | SlashCommand::Session { .. }
            | SlashCommand::Plugins { .. }
            | SlashCommand::Login
            | SlashCommand::Logout
            | SlashCommand::Vim
            | SlashCommand::Upgrade
            | SlashCommand::Stats
            | SlashCommand::Share
            | SlashCommand::Feedback
            | SlashCommand::Files
            | SlashCommand::Fast
            | SlashCommand::Exit
            | SlashCommand::Summary
            | SlashCommand::Desktop
            | SlashCommand::Brief
            | SlashCommand::Advisor
            | SlashCommand::Stickers
            | SlashCommand::Insights
            | SlashCommand::Thinkback
            | SlashCommand::ReleaseNotes
            | SlashCommand::SecurityReview
            | SlashCommand::Plan { .. }
            | SlashCommand::Review { .. }
            | SlashCommand::Usage { .. }
            | SlashCommand::Rename { .. }
            | SlashCommand::Copy { .. }
            | SlashCommand::Context { .. }
            | SlashCommand::Effort { .. }
            | SlashCommand::Branch { .. }
            | SlashCommand::Rewind { .. }
            | SlashCommand::Ide { .. }
            | SlashCommand::Tag { .. }
            | SlashCommand::AddDir { .. } => Ok(tui_text_result(
                "Command registered but not yet implemented in the TUI flow.".to_string(),
            )),
        }
    }
}
