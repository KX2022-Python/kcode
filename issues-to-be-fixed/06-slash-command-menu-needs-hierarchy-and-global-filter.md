## 标题
`/` 命令面板需要从单层平铺升级为一级菜单/二级菜单，同时保留全量全局过滤

## 状态
已修复（2026-04-06）

## 修复结果
- `/` 默认视图已改成一级菜单，先展示 `Session`、`Runtime`、`Workspace`、`Integrations`、`Automation` 五个分组。
- 进入分组后会展示对应二级动作，并提供 `Back` 返回一级菜单。
- 用户一旦输入 filter，搜索轨会切换为对全量命令语料做全局匹配，而不是只搜索当前局部菜单。

## 验证
- `cargo test -p kcode-cli command_palette -- --nocapture`
- `cargo check --workspace`

## 原始说法
现在 `/` 命令很多，应该做更有效的菜单集合:
- 做成一级菜单和二级菜单
- 同类相关的二级菜单归并到一级菜单里
- 用户直接输入 `/` 时，如果开始 filter，仍应作用于所有 `/` 命令，方便熟练用户直接搜索命中

## 结论
这个需求合理，而且当前实现确实还没到这个层级。

目前的 `/` 命令面板基本是:
- 一个单层根命令列表
- 外加少量针对特定命令的上下文子列表

它还不是“真正的分组一级菜单/二级菜单”结构，也没有把大规模命令集按稳定的信息架构组织起来。

## 证据
- `SlashCommandPicker` 当前核心状态只有:
  - `commands`
  - `context_command`
  - `filter`
  - `selected`
  - 没有一级菜单/分组节点模型
  - 见 [command_palette.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette.rs#L36)
- root 列表来自 `slash_command_entries()`，本质上是把 command registry snapshot 平铺后按固定顺序排序，见 [context.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs#L42)
- 当前只有少数命令有“上下文子菜单”:
  - `model`
  - `permissions`
  - `config`
  - `mcp`
  - `plugin`
  - `session`
  - 见 [context.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs#L153)
- 这些上下文项是按命令名手写分支生成的，不是统一的信息架构或层级菜单系统，见 [context.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs#L153)
- 当前 filter 只对“当前 entries 集合”做匹配:
  - 如果在 root，就搜 root entries
  - 如果进入某个 context command，就只搜该命令的 context entries
  - 见 [command_palette.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/command_palette.rs#L79)
- 也就是说，目前还没有“展示上按层级分组，但搜索上仍面向全量命令”的双轨机制

## 影响
- 命令数量继续增长后，平铺列表的可发现性会下降。
- 初学者难以通过信息架构理解命令体系。
- 熟练用户虽然还能直接输入过滤，但当前 context palette 会把搜索空间缩小到局部，不符合“全局可搜”的理想体验。
- 后续继续往 `context_entries()` 里硬塞特例，会让命令面板越来越像补丁集合。

## 建议修复方向
- 为 slash palette 建立统一的数据模型:
  - 一级菜单，例如 `session`、`runtime`、`config`、`agents`、`memory`、`bridge`、`review`、`advanced`
  - 二级菜单承载具体命令或命令变体
- 让 `/` 的默认视图优先展示一级菜单，而不是直接把所有命令平铺出来
- 让进入一级菜单后展示相关二级项，但保留“返回上级/退出分组”的导航
- 过滤逻辑改成双轨:
  - 展示轨: 走分组/层级
  - 搜索轨: 用户一旦输入 filter，仍对全量 slash command corpus 做匹配
- 搜索结果里建议显示分组来源，避免命中后不知道命令属于哪个一级菜单
- 现有 `model` / `permissions` / `config` / `mcp` / `plugin` / `session` 的 hand-written context entries 可以作为新二级菜单的第一批迁移对象
