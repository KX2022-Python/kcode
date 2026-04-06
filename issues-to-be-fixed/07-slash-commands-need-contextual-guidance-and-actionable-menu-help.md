## 标题
所有 `/` 命令需要上下文感知的简洁说明，并在层级菜单中直接完成常见操作

## 状态
已修复（2026-04-06）

## 修复结果
- slash palette 现在区分“列表摘要”和“选中项详情”，选中命令时会显示一段更可执行的简洁说明。
- `/model`、`/permissions`、`/dream`、`/plan`、`/config`、`/mcp`、`/plugin`、`/session`、`/skills`、`/agents`、`/schedule` 都补了上下文说明和可直接插入的常见动作。
- `/model` 子菜单现在会给出不同模型的简短适用场景说明，不再统一显示空泛文案。

## 验证
- `cargo test -p kcode-cli command_palette -- --nocapture`
- `cargo check --workspace`

## 原始说法
给所有 `/` 命令添加自动的简洁说明。  
例如用户通过 `/model` 选择模型切换时，应简要说明各个模型的强项特点和适用场景，并且在 `/` 命令层级菜单里直接完成模型切换。其他 `/` 命令也应有对应说明。

## 结论
这个需求合理，而且当前实现明显不够。

现在的 `/` 命令面板已经有“描述”字段，但它基本只是静态一句话摘要，不是你要的“自动、简洁、面向决策的说明”。  
`/model` 的子菜单虽然已经能直接插入并切换模型，但说明文案仍然是统一的 `"Switch to this model"`，没有模型差异说明，也没有适用场景提示。

## 证据
- command registry 里的 `description` 直接来自命令 spec 的 `summary`，说明当前描述体系本质上是静态摘要，不带上下文推导，见 [registry.rs](/home/ubuntu/kcode/rust/crates/commands/src/registry.rs#L124)
- slash palette 在构建 `SlashCommandEntry` 时直接复用这个 `description` 字段，没有进一步增强，见 [context.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs#L61)
- `session_specs` 里多数命令只有一句 summary，例如:
  - `/model`: `"Show or switch the active model"`
  - `/permissions`: `"Show or switch the active permission mode"`
  - 见 [session_specs.rs](/home/ubuntu/kcode/rust/crates/commands/src/session_specs.rs#L28)
- `/model` 当前确实已经有上下文子项，并且能直接插入 `/model <name>` 完成切换，见 [context.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs#L156)
- 但这些模型项的说明全部是统一文案 `"Switch to this model"`，没有任何“强项/弱项/适用场景/成本倾向/速度倾向”信息，见 [context.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs#L168)

## 影响
- 初学者即使看到命令名，也不知道什么时候该用这个命令。
- `/model` 这种决策型入口缺少模型差异说明，会让切换行为像“盲选”。
- 随着 `/` 命令越来越多，单纯一行摘要不足以支撑发现和正确使用。
- 即使做成了层级菜单，如果说明层仍然只有静态一句话，信息密度还是不够。

## 建议修复方向
- 为 slash command palette 增加“帮助说明层”:
  - 默认展示一句极简摘要
  - 选中时展示一段更有用的说明，例如“适合什么、不适合什么、常见使用场景”
- 为 `/model` 建立模型元数据:
  - 速度倾向
  - 代码能力
  - 长文本/推理能力
  - 成本倾向
  - 典型适用场景
- `/model` 在层级菜单里应支持直接完成切换，同时在选中项旁展示这些元数据摘要
- 其他高频命令也应有结构化说明，例如:
  - `/permissions`: 说明每种权限模式的风险和适用场景
  - `/config`: 说明每个子项查看的是什么
  - `/mcp`: 说明 list/show/help 的差异
  - `/session`: 说明 list/switch/fork 的典型使用场景
- 说明生成策略建议分层:
  - 基础层: 来自命令 spec 的静态摘要
  - 增强层: 来自命令类型、参数、运行环境、模型元数据的自动补全说明
- 该需求应与层级菜单改造协同推进，见 [06-slash-command-menu-needs-hierarchy-and-global-filter.md](/home/ubuntu/kcode/issues-to-be-fixed/06-slash-command-menu-needs-hierarchy-and-global-filter.md)
