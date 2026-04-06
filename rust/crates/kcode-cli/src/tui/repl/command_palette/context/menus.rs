use commands::{CommandDescriptor, CommandSource};

use super::{
    default_insert_text, enhanced_command_copy, format_usage, model_detail,
    model_short_description,
};
use super::super::{SlashCommandEntry, SlashCommandEntryAction};

pub(super) fn command_entry(descriptor: &CommandDescriptor) -> SlashCommandEntry {
    let (description, detail) = enhanced_command_copy(&descriptor.name, &descriptor.description);
    SlashCommandEntry {
        name: descriptor.name.clone(),
        usage: format_usage(&descriptor.name, descriptor.argument_hint.as_deref()),
        action: SlashCommandEntryAction::Insert(default_insert_text(
            &descriptor.name,
            descriptor.argument_hint.as_deref(),
        )),
        aliases: descriptor.aliases.clone(),
        description,
        detail,
        argument_hint: descriptor.argument_hint.clone(),
        source: descriptor.source,
    }
}

pub(super) fn command_context_entries(
    command: &str,
    available_models: &[String],
) -> Vec<SlashCommandEntry> {
    let builtin = CommandSource::Builtin;
    let mut entries = vec![navigation_entry(
        "back",
        "Back",
        "返回一级菜单",
        "回到顶层分组视图。",
        None,
    )];

    match command {
        "model" => {
            entries.push(submenu_entry(
                "current",
                "/model",
                "/model",
                "查看当前模型与会话统计",
                "显示当前 profile、生效模型和本会话的消息/turn 统计。",
                builtin,
            ));
            for model in available_models {
                if model.trim().is_empty() {
                    continue;
                }
                let detail = model_detail(model);
                entries.push(submenu_entry(
                    model,
                    &format!("/model {model}"),
                    &format!("/model {model}"),
                    &model_short_description(model),
                    &detail,
                    builtin,
                ));
            }
            entries
        }
        "permissions" => {
            entries.extend([
                submenu_entry(
                    "read-only",
                    "/permissions read-only",
                    "/permissions read-only",
                    "只读检查模式",
                    "适合先理解仓库、看日志、查配置；不会修改文件，也不会放开高风险工具。",
                    builtin,
                ),
                submenu_entry(
                    "plan",
                    "/permissions plan",
                    "/permissions plan",
                    "规划优先模式",
                    "适合先做方案、拆步骤和审视影响；保留严格权限，同时允许明确进入 planning 语义。",
                    builtin,
                ),
                submenu_entry(
                    "workspace-write",
                    "/permissions workspace-write",
                    "/permissions workspace-write",
                    "工作区写入模式",
                    "适合本地开发与修复；允许在仓库内改文件，但不放开无限制本机执行。",
                    builtin,
                ),
                submenu_entry(
                    "danger-full-access",
                    "/permissions danger-full-access",
                    "/permissions danger-full-access",
                    "完全访问模式",
                    "适合需要系统级命令或跨目录修改的场景；风险最高，只在明确需要时使用。",
                    builtin,
                ),
            ]);
            entries
        }
        "dream" => {
            entries.extend([
                submenu_entry(
                    "current",
                    "/dream",
                    "/dream",
                    "查看或切换 auto-dream",
                    "进入 auto-dream 入口，查看当前状态，或继续选择 On / Off / Status。",
                    builtin,
                ),
                submenu_entry(
                    "status",
                    "/dream status",
                    "/dream status",
                    "查看 auto-dream 状态",
                    "显示当前 worktree 的 auto-dream 开关和本地 override 状态。",
                    builtin,
                ),
                submenu_entry(
                    "on",
                    "/dream on",
                    "/dream on",
                    "开启 auto-dream",
                    "开启回合后自动 memory extraction；适合长会话或高信息密度任务。",
                    builtin,
                ),
                submenu_entry(
                    "off",
                    "/dream off",
                    "/dream off",
                    "关闭 auto-dream",
                    "关闭自动提取，减少后台提取动作；适合临时排查或希望保持最小副作用时使用。",
                    builtin,
                ),
            ]);
            entries
        }
        "plan" => {
            entries.extend([
                submenu_entry(
                    "status",
                    "/plan status",
                    "/plan status",
                    "查看 planning mode 状态",
                    "查看当前 worktree 是否启用了本地 planning mode override。",
                    builtin,
                ),
                submenu_entry(
                    "on",
                    "/plan on",
                    "/plan on",
                    "开启 planning mode",
                    "把当前 worktree 切到 planning mode，适合先规划再执行的任务流。",
                    builtin,
                ),
                submenu_entry(
                    "off",
                    "/plan off",
                    "/plan off",
                    "关闭 planning mode",
                    "清理本地 override，回到常规权限模式。",
                    builtin,
                ),
            ]);
            entries
        }
        "config" => {
            entries.extend([
                submenu_entry(
                    "env",
                    "/config env",
                    "/config env",
                    "查看环境与配置路径",
                    "适合先判断当前到底加载了哪些配置文件、环境变量和 config home。",
                    builtin,
                ),
                submenu_entry(
                    "hooks",
                    "/config hooks",
                    "/config hooks",
                    "查看 hook 配置",
                    "适合排查 hook 是否启用、来源在哪、以及当前工作区是否覆盖了默认值。",
                    builtin,
                ),
                submenu_entry(
                    "model",
                    "/config model",
                    "/config model",
                    "查看模型与 profile 配置",
                    "适合确认当前 profile、模型、base URL 和相关 override 是否生效。",
                    builtin,
                ),
                submenu_entry(
                    "plugins",
                    "/config plugins",
                    "/config plugins",
                    "查看插件配置",
                    "适合确认插件启停、来源和局部覆盖关系。",
                    builtin,
                ),
            ]);
            entries
        }
        "mcp" => {
            entries.extend([
                submenu_entry(
                    "list",
                    "/mcp list",
                    "/mcp list",
                    "列出 MCP servers",
                    "快速看当前配置里有哪些 MCP server，以及哪些被禁用、阻断或去重。",
                    builtin,
                ),
                submenu_entry(
                    "show",
                    "/mcp show <server>",
                    "/mcp show ",
                    "查看单个 MCP server",
                    "适合排查某个 server 的 transport、命令、策略或被过滤原因。",
                    builtin,
                ),
                submenu_entry(
                    "help",
                    "/mcp help",
                    "/mcp help",
                    "查看 MCP 命令帮助",
                    "当你不确定 list/show/help 的差异时，用它快速回到 MCP 子命令说明。",
                    builtin,
                ),
            ]);
            entries
        }
        "plugin" => {
            entries.extend([
                submenu_entry(
                    "list",
                    "/plugin list",
                    "/plugin list",
                    "列出已安装插件",
                    "查看当前可用插件及其启用状态。",
                    builtin,
                ),
                submenu_entry(
                    "install",
                    "/plugin install <path>",
                    "/plugin install ",
                    "安装本地插件",
                    "从本地目录安装插件；适合开发中的插件或私有插件源。",
                    builtin,
                ),
                submenu_entry(
                    "enable",
                    "/plugin enable <name>",
                    "/plugin enable ",
                    "启用插件",
                    "启用已安装插件，让它参与工具、hooks 或命令面。",
                    builtin,
                ),
                submenu_entry(
                    "disable",
                    "/plugin disable <name>",
                    "/plugin disable ",
                    "停用插件",
                    "先停掉可疑插件，再观察运行时与帮助面是否恢复正常。",
                    builtin,
                ),
                submenu_entry(
                    "uninstall",
                    "/plugin uninstall <id>",
                    "/plugin uninstall ",
                    "卸载插件",
                    "移除本地插件安装记录和文件。",
                    builtin,
                ),
                submenu_entry(
                    "update",
                    "/plugin update <id>",
                    "/plugin update ",
                    "更新插件",
                    "刷新已安装插件到最新本地源内容。",
                    builtin,
                ),
            ]);
            entries
        }
        "session" => {
            entries.extend([
                submenu_entry(
                    "list",
                    "/session list",
                    "/session list",
                    "列出已保存会话",
                    "查看当前可恢复的本地会话和 session id。",
                    builtin,
                ),
                submenu_entry(
                    "switch",
                    "/session switch <session-id>",
                    "/session switch ",
                    "切换到另一条会话",
                    "适合在多任务并行时快速跳到另一条 session 继续工作。",
                    builtin,
                ),
                submenu_entry(
                    "fork",
                    "/session fork [branch-name]",
                    "/session fork ",
                    "分叉当前会话",
                    "把当前上下文拷贝成一条新会话，适合平行方案、独立实验或隔离上下文污染。",
                    builtin,
                ),
            ]);
            entries
        }
        "agents" => {
            entries.extend([
                submenu_entry(
                    "list",
                    "/agents list",
                    "/agents list",
                    "列出 agents",
                    "查看当前配置里有哪些 agent 定义。",
                    builtin,
                ),
                submenu_entry(
                    "help",
                    "/agents help",
                    "/agents help",
                    "查看 agents 帮助",
                    "快速回到 agents 子命令说明。",
                    builtin,
                ),
            ]);
            entries
        }
        "skills" => {
            entries.extend([
                submenu_entry(
                    "list",
                    "/skills list",
                    "/skills list",
                    "列出 skills",
                    "查看本机已发现或可安装的 skills。",
                    builtin,
                ),
                submenu_entry(
                    "install",
                    "/skills install <path>",
                    "/skills install ",
                    "安装 skill",
                    "从路径或 repo 安装 skill，适合把常用工作流固化成可复用入口。",
                    builtin,
                ),
                submenu_entry(
                    "help",
                    "/skills help",
                    "/skills help",
                    "查看 skills 帮助",
                    "快速回到 skills 子命令说明。",
                    builtin,
                ),
            ]);
            entries
        }
        "schedule" => {
            entries.extend([
                submenu_entry(
                    "list",
                    "/schedule list",
                    "/schedule list",
                    "列出调度任务",
                    "查看当前已有的 recurring schedule。",
                    builtin,
                ),
                submenu_entry(
                    "create",
                    "/schedule create <cron> <prompt>",
                    "/schedule create ",
                    "创建调度任务",
                    "适合固定频率轮询、日报或定时提醒。",
                    builtin,
                ),
                submenu_entry(
                    "delete",
                    "/schedule delete <id>",
                    "/schedule delete ",
                    "删除调度任务",
                    "清理不再需要的 recurring task。",
                    builtin,
                ),
            ]);
            entries
        }
        _ => Vec::new(),
    }
}

pub(super) fn navigation_entry(
    name: &str,
    usage: &str,
    description: &str,
    detail: &str,
    context: Option<&str>,
) -> SlashCommandEntry {
    SlashCommandEntry {
        name: name.to_string(),
        usage: usage.to_string(),
        action: SlashCommandEntryAction::Navigate(context.map(ToOwned::to_owned)),
        aliases: Vec::new(),
        description: description.to_string(),
        detail: detail.to_string(),
        argument_hint: None,
        source: CommandSource::Builtin,
    }
}

fn submenu_entry(
    name: &str,
    usage: &str,
    insert_text: &str,
    description: &str,
    detail: &str,
    source: CommandSource,
) -> SlashCommandEntry {
    SlashCommandEntry {
        name: name.to_string(),
        usage: usage.to_string(),
        action: SlashCommandEntryAction::Insert(insert_text.to_string()),
        aliases: Vec::new(),
        description: description.to_string(),
        detail: detail.to_string(),
        argument_hint: None,
        source,
    }
}
