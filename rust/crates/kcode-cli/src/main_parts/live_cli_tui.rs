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
        let mut messages = tui::repl::default_welcome_messages(
            &self.model,
            &self.active_profile.profile_name,
            self.permission_mode.as_str(),
            &self.session.id,
        );

        messages[0] = tui::repl::RenderableMessage::AssistantText {
            text: tui::repl::render_welcome_banner(),
            streaming: false,
        };
        messages.insert(
            2,
            tui::repl::RenderableMessage::System {
                message: format!("{branch} · {workspace} · {cwd}"),
                level: tui::repl::SysLevel::Info,
            },
        );
        messages.insert(
            3,
            tui::repl::RenderableMessage::System {
                message: format!(
                    "session {} · autosave {}",
                    tui::repl::short_session_id(&self.session.id),
                    self.session.path.display(),
                ),
                level: tui::repl::SysLevel::Info,
            },
        );
        messages
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
                    return self.tui_session_command_result(Some("list"), None);
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
                        "unsupported permission mode '{mode}'. Use read-only, plan, workspace-write, or danger-full-access."
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
            SlashCommand::Bughunter { scope } => Ok(tui_text_result(format_bug_report(
                scope.as_deref(),
                &self.session.id,
                &self.session.path,
            ))),
            SlashCommand::Commit => {
                let status = git_output(&["status", "--short", "--branch"])?;
                let summary = parse_git_workspace_summary(Some(&status));
                let branch = parse_git_status_branch(Some(&status));
                Ok(tui_text_result(if summary.is_clean() {
                    format_commit_skipped_report()
                } else {
                    format_commit_preflight_report(branch.as_deref(), summary)
                }))
            }
            SlashCommand::Pr { context } => {
                let branch = resolve_git_branch_for(&env::current_dir()?)
                    .unwrap_or_else(|| "unknown".to_string());
                Ok(tui_text_result(format_pr_report(&branch, context.as_deref())))
            }
            SlashCommand::Issue { context } => {
                Ok(tui_text_result(format_issue_report(context.as_deref())))
            }
            SlashCommand::DebugToolCall => {
                Ok(tui_text_result(render_last_tool_debug_report(self.runtime.session())?))
            }
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
            SlashCommand::Dream { mode } => {
                Ok(tui_text_result(self.handle_dream(mode.as_deref())?))
            }
            SlashCommand::Plan { mode } => {
                let cwd = env::current_dir()?;
                let outcome = run_plan_mode_command(&cwd, mode.as_deref(), self.permission_mode)?;
                if let Some(next_mode) = outcome.next_permission_mode {
                    if next_mode != self.permission_mode {
                        self.replace_tui_runtime(
                            self.runtime.session().clone(),
                            self.model.clone(),
                            self.model_explicit,
                            next_mode,
                        )?;
                    }
                }
                Ok(tui_text_result(outcome.message))
            }
            SlashCommand::Tasks { args } => Ok(tui_text_result(match args.as_deref() {
                None | Some("list") => render_todos_report(&env::current_dir()?)?,
                Some("help") => {
                    "Todos\n  Usage            /todos\n  Usage            /todos help\n  Store            .clawd-todos.json\n  Source           TodoWrite tool updates this store during longer tasks".to_string()
                }
                other => format!(
                    "Unknown todos argument: {}. Use /todos help for usage.",
                    other.unwrap_or("")
                ),
            })),
            SlashCommand::Powerup => Ok(tui_text_result(format_powerup_report())),
            SlashCommand::Btw { question } => self.tui_btw_result(question.as_deref()),
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
            SlashCommand::Session { action, target } => {
                self.tui_session_command_result(action.as_deref(), target.as_deref())
            }
            SlashCommand::Plugins { action, target } => {
                if let Err(message) =
                    ensure_session_command_available_for_profile("plugin", &self.active_profile)
                {
                    return Ok(tui_text_result(message));
                }
                self.tui_plugins_command_result(action.as_deref(), target.as_deref())
            }
            SlashCommand::Hooks { .. } => Ok(tui_text_result(render_config_report(
                Some("hooks"),
                self.model_explicit.then_some(self.model.as_str()),
                self.profile_override.as_deref(),
            )?)),
            SlashCommand::Login => Ok(tui_text_result(format_login_report(
                &self.active_profile.profile_name,
                &self.model,
            ))),
            SlashCommand::Feedback => Ok(tui_text_result(format_feedback_report(None))),
            SlashCommand::Desktop => Ok(tui_text_result(format_desktop_report())),
            SlashCommand::Schedule { args } => {
                Ok(tui_text_result(format_schedule_report(args.as_deref())))
            }
            SlashCommand::Loop { args } => {
                Ok(tui_text_result(format_loop_report(args.as_deref())))
            }
            SlashCommand::Keybindings
            | SlashCommand::PrivacySettings
            | SlashCommand::Theme { .. }
            | SlashCommand::Voice { .. }
            | SlashCommand::Color { .. }
            | SlashCommand::OutputStyle { .. } => Ok(tui_text_result(
                "Run `kcode tui appearance` to manage UI and privacy settings.".to_string(),
            )),
            SlashCommand::Unknown(name) => Err(format_unknown_slash_command(&name).into()),
            SlashCommand::Logout
            | SlashCommand::Vim
            | SlashCommand::Upgrade
            | SlashCommand::Stats
            | SlashCommand::Share
            | SlashCommand::Files
            | SlashCommand::Fast
            | SlashCommand::Exit
            | SlashCommand::Summary
            | SlashCommand::Brief
            | SlashCommand::Advisor
            | SlashCommand::Stickers
            | SlashCommand::Insights
            | SlashCommand::Thinkback
            | SlashCommand::ReleaseNotes
            | SlashCommand::SecurityReview
            | SlashCommand::Review { .. }
            | SlashCommand::Usage { .. }
            | SlashCommand::Rename { .. }
            | SlashCommand::Copy { .. }
            | SlashCommand::Context { .. }
            | SlashCommand::Effort { .. }
            | SlashCommand::Rewind { .. }
            | SlashCommand::Ide { .. }
            | SlashCommand::Tag { .. }
            | SlashCommand::AddDir { .. } => Ok(tui_text_result(
                "Command registered but not yet implemented in the TUI flow.".to_string(),
            )),
            SlashCommand::Branch { name } => {
                self.tui_session_command_result(Some("fork"), name.as_deref())
            }
        }
    }

    fn tui_session_command_result(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => Ok(tui_text_result(render_session_list(&self.session.id)?)),
            Some("switch") => {
                let Some(target) = target else {
                    return Ok(tui_text_result("Usage: /session switch <session-id>".to_string()));
                };
                let handle = resolve_session_reference(target)?;
                let session = Session::load_from_path(&handle.path)?;
                let message_count = session.messages.len();
                self.session = SessionHandle {
                    id: session.session_id.clone(),
                    path: handle.path,
                };
                self.replace_tui_runtime(
                    session,
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                Ok(tui_text_result(format!(
                    "Session switched\n  Active session   {}\n  File             {}\n  Messages         {}",
                    self.session.id,
                    self.session.path.display(),
                    message_count,
                )))
            }
            Some("fork") => {
                let forked = self.runtime.fork_session(target.map(ToOwned::to_owned));
                let parent_session_id = self.session.id.clone();
                let handle = create_managed_session_handle(&forked.session_id)?;
                let branch_name = forked.fork.as_ref().and_then(|fork| fork.branch_name.clone());
                let forked = forked.with_persistence_path(handle.path.clone());
                let message_count = forked.messages.len();
                forked.save_to_path(&handle.path)?;
                self.session = handle;
                self.replace_tui_runtime(
                    forked,
                    self.model.clone(),
                    self.model_explicit,
                    self.permission_mode,
                )?;
                Ok(tui_text_result(format!(
                    "Session forked\n  Parent session   {}\n  Active session   {}\n  Branch           {}\n  File             {}\n  Messages         {}",
                    parent_session_id,
                    self.session.id,
                    branch_name.as_deref().unwrap_or("(unnamed)"),
                    self.session.path.display(),
                    message_count,
                )))
            }
            Some(other) => Ok(tui_text_result(format!(
                "Unknown /session action '{other}'. Use /session list, /session switch <session-id>, or /session fork [branch-name]."
            ))),
        }
    }

    fn tui_plugins_command_result(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader.load()?;
        let mut manager = build_plugin_manager(&cwd, &loader, &runtime_config);
        let result = handle_plugins_slash_command(action, target, &mut manager)?;
        if result.reload_runtime {
            self.reload_runtime_features()?;
        }
        Ok(tui_text_result(result.message))
    }

    fn tui_btw_result(
        &self,
        question: Option<&str>,
    ) -> Result<tui::repl::BackendResult, Box<dyn std::error::Error>> {
        let Some(question) = question.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(tui_text_result(render_btw_usage()));
        };
        Ok(tui_text_result(self.run_internal_prompt_text(question, false)?))
    }
}
