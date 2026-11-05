use std::collections::BTreeSet;
use std::path::Path;

use commands::{
    build_command_registry_snapshot_with_cwd, CommandDescriptor, CommandRegistryContext,
    CommandSource, CommandSurface,
};

mod menus;

use self::menus::{command_context_entries, command_entry, navigation_entry};
use super::SlashCommandEntry;

const CC_COMMAND_ORDER: &[&str] = &[
    "help",
    "clear",
    "resume",
    "compact",
    "config",
    "model",
    "permissions",
    "memory",
    "dream",
    "plan",
    "init",
    "plugin",
    "agents",
    "skills",
    "doctor",
    "mcp",
    "status",
    "cost",
    "todos",
    "commit",
    "issue",
    "pr",
    "branch",
    "session",
    "diff",
    "schedule",
    "loop",
    "powerup",
    "btw",
    "version",
    "export",
];

const SEARCHABLE_CONTEXTS: &[&str] = &[
    "model",
    "permissions",
    "dream",
    "plan",
    "config",
    "mcp",
    "plugin",
    "session",
    "agents",
    "skills",
    "schedule",
];

pub(super) fn slash_command_entries(
    profile_supports_tools: bool,
    cwd: &Path,
) -> Vec<SlashCommandEntry> {
    let snapshot = build_command_registry_snapshot_with_cwd(
        &CommandRegistryContext::for_surface(CommandSurface::CliLocal, profile_supports_tools),
        &[],
        cwd,
    );
    let mut ordered = snapshot
        .session_commands
        .into_iter()
        .enumerate()
        .map(|(index, descriptor)| (command_rank(&descriptor, index), descriptor))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(rank, _)| *rank);

    ordered
        .into_iter()
        .map(|(_, descriptor)| command_entry(&descriptor))
        .collect()
}

pub(super) fn palette_entries(
    root_commands: &[SlashCommandEntry],
    context_command: Option<&str>,
    available_models: &[String],
) -> Vec<SlashCommandEntry> {
    match context_command {
        Some(context) if context.starts_with("group:") => group_entries(
            context.trim_start_matches("group:"),
            root_commands,
            available_models,
        ),
        Some(command) => command_context_entries(command, available_models),
        None => root_group_entries(),
    }
}

pub(super) fn search_entries(
    root_commands: &[SlashCommandEntry],
    available_models: &[String],
) -> Vec<SlashCommandEntry> {
    let mut entries = root_commands.to_vec();
    for command in SEARCHABLE_CONTEXTS {
        entries.extend(command_context_entries(command, available_models));
    }
    dedup_entries(entries)
}

pub(super) fn context_title(context_command: Option<&str>) -> String {
    match context_command {
        Some(context) if context.starts_with("group:") => {
            match context.trim_start_matches("group:") {
                "session" => "Session".to_string(),
                "runtime" => "Runtime".to_string(),
                "workspace" => "Workspace".to_string(),
                "integrations" => "Integrations".to_string(),
                "automation" => "Automation".to_string(),
                other => other.to_string(),
            }
        }
        Some(command) => format!("/{command}"),
        None => "Commands".to_string(),
    }
}

pub(super) fn extract_palette_filter(
    input: &str,
    available_models: &[String],
) -> Option<(Option<String>, String)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('/') {
        return None;
    }

    let body = trimmed.trim_start_matches('/');
    if body.is_empty() {
        return Some((None, String::new()));
    }

    if !body.contains(' ') {
        let command = body.to_ascii_lowercase();
        if exact_context_palette_command(&command, available_models) {
            return Some((Some(command), String::new()));
        }
        return Some((None, command));
    }

    let mut parts = body.splitn(3, ' ');
    let command = parts.next().unwrap_or_default().to_ascii_lowercase();
    let second = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        return None;
    }

    if command_context_entries(&command, available_models).is_empty() {
        return None;
    }

    Some((Some(command), second.trim().to_ascii_lowercase()))
}

fn command_rank(descriptor: &CommandDescriptor, original_index: usize) -> (usize, usize, usize) {
    let cc_rank = CC_COMMAND_ORDER
        .iter()
        .position(|name| *name == descriptor.name)
        .unwrap_or(CC_COMMAND_ORDER.len() + original_index);
    let source_rank = match descriptor.source {
        CommandSource::Builtin => 0,
        CommandSource::Skills => 1,
        CommandSource::Plugins => 2,
        CommandSource::Workflow => 3,
        CommandSource::Mcp => 4,
    };
    (cc_rank, source_rank, original_index)
}

fn exact_context_palette_command(command: &str, available_models: &[String]) -> bool {
    !command_context_entries(command, available_models).is_empty()
}

fn root_group_entries() -> Vec<SlashCommandEntry> {
    vec![
        navigation_entry(
            "session",
            "Session",
            "恢复、清理、导出和切换会话",
            "查看状态、恢复旧会话、清空当前会话，或切到另一条会话分支继续工作。",
            Some("group:session"),
        ),
        navigation_entry(
            "runtime",
            "Runtime",
            "模型、权限、memory 与 planning 控制",
            "切换模型和权限模式，查看 memory / cost，并直接调整 auto-dream 与 plan mode。",
            Some("group:runtime"),
        ),
        navigation_entry(
            "workspace",
            "Workspace",
            "仓库初始化、diff、分支与提交流程",
            "查看当前改动、初始化 KCODE.md、切分支、生成 commit / PR / issue 文案。",
            Some("group:workspace"),
        ),
        navigation_entry(
            "integrations",
            "Integrations",
            "配置、MCP、插件、agents 与技能入口",
            "查看配置与 MCP，管理插件、技能和 agents，并用 doctor 做环境诊断。",
            Some("group:integrations"),
        ),
        navigation_entry(
            "automation",
            "Automation",
            "轮询、调度和引导型快捷入口",
            "处理 schedule / loop 等自动化动作，并保留 BTW / Powerup 这类辅助入口。",
            Some("group:automation"),
        ),
    ]
}

fn group_entries(
    group: &str,
    root_commands: &[SlashCommandEntry],
    available_models: &[String],
) -> Vec<SlashCommandEntry> {
    let mut entries = vec![navigation_entry(
        "back",
        "Back",
        "返回一级菜单",
        "回到顶层分组视图。",
        None,
    )];

    match group {
        "session" => {
            entries.extend(select_commands(
                root_commands,
                &[
                    "help", "status", "resume", "clear", "session", "export", "version",
                ],
            ));
            entries.extend(command_context_entries("session", available_models));
        }
        "runtime" => {
            entries.extend(select_commands(
                root_commands,
                &["status", "cost", "memory"],
            ));
            entries.extend(command_context_entries("model", available_models));
            entries.extend(command_context_entries("permissions", available_models));
            entries.extend(command_context_entries("dream", available_models));
            entries.extend(command_context_entries("plan", available_models));
        }
        "workspace" => {
            entries.extend(select_commands(
                root_commands,
                &["init", "diff", "branch", "commit", "pr", "issue"],
            ));
        }
        "integrations" => {
            entries.extend(command_context_entries("config", available_models));
            entries.extend(command_context_entries("mcp", available_models));
            entries.extend(command_context_entries("plugin", available_models));
            entries.extend(command_context_entries("skills", available_models));
            entries.extend(command_context_entries("agents", available_models));
            entries.extend(select_commands(root_commands, &["doctor"]));
        }
        "automation" => {
            entries.extend(command_context_entries("schedule", available_models));
            entries.extend(select_commands(
                root_commands,
                &["loop", "todos", "powerup", "btw"],
            ));
        }
        _ => {}
    }

    dedup_entries(entries)
}

fn select_commands(root_commands: &[SlashCommandEntry], names: &[&str]) -> Vec<SlashCommandEntry> {
    names
        .iter()
        .filter_map(|name| {
            root_commands
                .iter()
                .find(|entry| entry.name == *name)
                .cloned()
        })
        .collect()
}

fn dedup_entries(entries: Vec<SlashCommandEntry>) -> Vec<SlashCommandEntry> {
    let mut seen = BTreeSet::new();
    entries
        .into_iter()
        .filter(|entry| seen.insert(entry.usage.clone()))
        .collect()
}

fn format_usage(name: &str, argument_hint: Option<&str>) -> String {
    match argument_hint {
        Some(argument_hint) => format!("/{name} {argument_hint}"),
        None => format!("/{name}"),
    }
}

fn default_insert_text(name: &str, argument_hint: Option<&str>) -> String {
    match argument_hint {
        Some(_) => format!("/{name} "),
        None => format!("/{name}"),
    }
}

fn enhanced_command_copy(name: &str, summary: &str) -> (String, String) {
    match name {
        "model" => (
            "切换或查看当前模型".to_string(),
            "先看当前模型，再根据任务切到更偏代码、推理或速度的模型。".to_string(),
        ),
        "permissions" => (
            "切换权限模式".to_string(),
            "按风险调整本地工具权限；从只读到完全访问逐级提升。".to_string(),
        ),
        "dream" => (
            "控制 auto-dream 提取".to_string(),
            "查看或切换自动 memory extraction，适合长会话的信息回收。".to_string(),
        ),
        "plan" => (
            "控制 planning mode".to_string(),
            "查看或切换当前 worktree 的 planning mode，本地规划任务优先入口。".to_string(),
        ),
        "config" => (
            "查看配置与来源".to_string(),
            "快速确认当前到底加载了哪些配置、hook、profile 和插件配置。".to_string(),
        ),
        "mcp" => (
            "查看 MCP servers".to_string(),
            "列出或检查 MCP server 状态，适合排查 transport、策略和启停结果。".to_string(),
        ),
        "session" => (
            "列出、切换或分叉会话".to_string(),
            "适合多任务并行时管理本地会话，把不同路径隔离开。".to_string(),
        ),
        "doctor" => (
            "检查环境与配置健康".to_string(),
            "优先用它判断当前环境、配置和运行状态是否处于可用基线。".to_string(),
        ),
        "schedule" => (
            "管理定时任务".to_string(),
            "创建、列出或删除 recurring schedule，适合固定频率自动化。".to_string(),
        ),
        "loop" => (
            "间隔轮询一个 prompt".to_string(),
            "用固定间隔重复同一提示，适合轻量 polling workflow。".to_string(),
        ),
        "btw" => (
            "问一个不写入主会话的旁支问题".to_string(),
            "适合临时澄清背景或查边角问题，避免污染主任务上下文。".to_string(),
        ),
        _ => (summary.to_string(), summary.to_string()),
    }
}

fn model_short_description(model: &str) -> String {
    let lowered = model.to_ascii_lowercase();
    if lowered.contains("mini") {
        "更快、更省，适合轻任务".to_string()
    } else if lowered.contains("gpt-5") || lowered.contains("5.4") {
        "更强推理与代码能力".to_string()
    } else if lowered.contains("4.1") {
        "稳健通用，适合日常开发".to_string()
    } else {
        "切换到这个模型".to_string()
    }
}

fn model_detail(model: &str) -> String {
    let lowered = model.to_ascii_lowercase();
    if lowered.contains("mini") {
        "更偏速度和成本控制，适合短回复、轻量排查和高频交互。".to_string()
    } else if lowered.contains("gpt-5") || lowered.contains("5.4") {
        "更偏复杂推理、代码修复和长上下文综合判断；适合高难度开发任务。".to_string()
    } else if lowered.contains("4.1") {
        "偏稳健通用，适合日常代码问答、文档整理和中等复杂度任务。".to_string()
    } else {
        "切换到该模型，并在后续会话回合中使用它。".to_string()
    }
}
