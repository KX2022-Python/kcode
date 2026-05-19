fn command_matches_query(descriptor: &CommandDescriptor, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    let query = query.to_ascii_lowercase();
    descriptor.name.to_ascii_lowercase().contains(&query)
        || descriptor.description.to_ascii_lowercase().contains(&query)
        || descriptor
            .aliases
            .iter()
            .any(|alias| alias.to_ascii_lowercase().contains(&query))
}

fn tool_status_label(status: tui::repl::ToolStatus) -> &'static str {
    match status {
        tui::repl::ToolStatus::Pending => "pending",
        tui::repl::ToolStatus::Running => "running",
        tui::repl::ToolStatus::Completed => "completed",
        tui::repl::ToolStatus::Denied => "denied",
    }
}

fn sys_level_label(level: tui::repl::SysLevel) -> &'static str {
    match level {
        tui::repl::SysLevel::Info => "info",
        tui::repl::SysLevel::Warning => "warning",
        tui::repl::SysLevel::Error => "error",
        tui::repl::SysLevel::Success => "success",
    }
}
