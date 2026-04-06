impl LiveCli {
    fn handle_repl_command(
        &mut self,
        command: SlashCommand,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(match command {
            SlashCommand::Help => {
                println!(
                    "{}",
                    render_repl_help_for_profile(self.active_profile.profile.supports_tools)
                );
                false
            }
            SlashCommand::Status => {
                self.print_status();
                false
            }
            SlashCommand::Bughunter { scope } => {
                self.run_bughunter(scope.as_deref())?;
                false
            }
            SlashCommand::Commit => {
                self.run_commit(None)?;
                false
            }
            SlashCommand::Pr { context } => {
                self.run_pr(context.as_deref())?;
                false
            }
            SlashCommand::Issue { context } => {
                self.run_issue(context.as_deref())?;
                false
            }
            SlashCommand::DebugToolCall => {
                self.run_debug_tool_call(None)?;
                false
            }
            SlashCommand::Sandbox => {
                Self::print_sandbox_status();
                false
            }
            SlashCommand::Compact => {
                self.compact()?;
                false
            }
            SlashCommand::Tasks { args } => {
                self.print_tasks(args.as_deref())?;
                false
            }
            SlashCommand::Powerup => {
                println!("{}", format_powerup_report());
                false
            }
            SlashCommand::Btw { question } => {
                let Some(question) = question.as_deref().map(str::trim).filter(|value| !value.is_empty()) else {
                    println!("{}", render_btw_usage());
                    return Ok(false);
                };
                println!("{}", self.run_internal_prompt_text(question, false)?);
                false
            }
            SlashCommand::Model { model } => self.set_model(model)?,
            SlashCommand::Permissions { mode } => self.set_permissions(mode)?,
            SlashCommand::Clear { confirm } => self.clear_session(confirm)?,
            SlashCommand::Cost => {
                self.print_cost();
                false
            }
            SlashCommand::Resume { session_path } => self.resume_session(session_path)?,
            SlashCommand::Config { section } => {
                self.print_config(section.as_deref())?;
                false
            }
            SlashCommand::Mcp { action, target } => {
                if let Err(message) =
                    ensure_session_command_available_for_profile("mcp", &self.active_profile)
                {
                    eprintln!("{message}");
                    return Ok(false);
                }
                let args = match (action.as_deref(), target.as_deref()) {
                    (None, None) => None,
                    (Some(action), None) => Some(action.to_string()),
                    (Some(action), Some(target)) => Some(format!("{action} {target}")),
                    (None, Some(target)) => Some(target.to_string()),
                };
                Self::print_mcp(args.as_deref())?;
                false
            }
            SlashCommand::Memory => {
                Self::print_memory()?;
                false
            }
            SlashCommand::Dream { mode } => {
                println!("{}", self.handle_dream(mode.as_deref())?);
                false
            }
            SlashCommand::Plan { mode } => {
                println!("{}", self.handle_plan(mode.as_deref())?);
                false
            }
            SlashCommand::Doctor => {
                self.print_doctor()?;
                false
            }
            SlashCommand::Init => {
                println!("{}", init_repo_kcode_md()?);
                false
            }
            SlashCommand::Diff => {
                Self::print_diff()?;
                false
            }
            SlashCommand::Version => {
                Self::print_version();
                false
            }
            SlashCommand::Export { path } => {
                self.export_session(path.as_deref())?;
                false
            }
            SlashCommand::Session { action, target } => {
                self.handle_session_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Plugins { action, target } => {
                if let Err(message) =
                    ensure_session_command_available_for_profile("plugin", &self.active_profile)
                {
                    eprintln!("{message}");
                    return Ok(false);
                }
                self.handle_plugins_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Agents { args } => {
                Self::print_agents(args.as_deref())?;
                false
            }
            SlashCommand::Skills { args } => {
                Self::print_skills(args.as_deref())?;
                false
            }
            SlashCommand::Login => {
                println!(
                    "{}",
                    format_login_report(&self.active_profile.profile_name, &self.model)
                );
                false
            }
            SlashCommand::Feedback => {
                println!("{}", format_feedback_report(None));
                false
            }
            SlashCommand::Desktop => {
                println!("{}", format_desktop_report());
                false
            }
            SlashCommand::Schedule { args } => {
                println!("{}", format_schedule_report(args.as_deref()));
                false
            }
            SlashCommand::Loop { args } => {
                println!("{}", format_loop_report(args.as_deref()));
                false
            }
            SlashCommand::Keybindings
            | SlashCommand::PrivacySettings
            | SlashCommand::Theme { .. }
            | SlashCommand::Voice { .. }
            | SlashCommand::Color { .. }
            | SlashCommand::OutputStyle { .. } => {
                eprintln!("Run `kcode tui appearance` to manage UI and privacy settings.");
                false
            }
            SlashCommand::Hooks { .. } => {
                self.print_config(Some("hooks"))?;
                false
            }
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
            | SlashCommand::AddDir { .. } => {
                eprintln!("Command registered but not yet implemented.");
                false
            }
            SlashCommand::Branch { name } => {
                self.handle_session_command(Some("fork"), name.as_deref())?
            }
            SlashCommand::Unknown(name) => {
                eprintln!("{}", format_unknown_slash_command(&name));
                false
            }
        })
    }
}
