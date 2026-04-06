## 标题
plan-mode 目前只有局部开关和展示残片，命令层/配置层/运行时语义没有形成闭环

## 状态
已修复（2026-04-06）

## 修复结果
- `plan` 现在是一个真正的一等权限模式，不再在配置解析阶段被直接折叠成 `read-only`。
- `/permissions` 现在接受 `plan`，REPL/TUI 都会切换到同一套 session 运行时权限模式。
- `/plan [on|off|status]` 已真正实现，并复用 `EnterPlanMode` / `ExitPlanMode` 工具逻辑管理当前 worktree 的本地 override。
- `ExitPlanMode` 现在也能清理“非工具创建但当前本地仍是 plan”的悬空 override，避免 `/plan off` 无效。
- 共享 slash help、参数校验、状态展示和默认权限模式解析已同步更新。

## 验证
- 安装版 `/home/ubuntu/kcode`
  - `cargo test -p runtime permission_mode_aliases_resolve_to_expected_modes -- --nocapture`
  - `cargo test -p commands parses_supported_slash_commands -- --nocapture`
  - `cargo test -p commands rejects_invalid_argument_values -- --nocapture`
  - `cargo test -p commands renders_help_from_shared_specs -- --nocapture`
  - `cargo test -p tools enter_and_exit_plan_mode_round_trip_existing_local_override -- --nocapture`
  - `cargo test -p tools exit_plan_mode_clears_override_when_enter_created_it_from_empty_local_state -- --nocapture`
  - `cargo test -p tools exit_plan_mode_clears_unmanaged_local_plan_override -- --nocapture`
  - `cargo test -p kcode-cli default_permission_mode_reads_plan_from_project_config -- --nocapture`
  - `cargo test -p kcode-cli plan_command_uses_tool_backed_local_override_lifecycle -- --nocapture`
  - `cargo test -p kcode-cli repl_help_includes_shared_commands_and_exit -- --nocapture`
  - `cargo test -p kcode-cli normalizes_supported_permission_modes -- --nocapture`
- 开发源码 `/home/ubuntu/project/kcode`
  - `cargo test -p runtime permission_mode_aliases_resolve_to_expected_modes -- --nocapture`
  - `cargo test -p commands parses_supported_slash_commands -- --nocapture`
  - `cargo test -p commands rejects_invalid_argument_values -- --nocapture`
  - `cargo test -p tools exit_plan_mode_clears_unmanaged_local_plan_override -- --nocapture`
  - `cargo test -p kcode-cli plan_command_uses_tool_backed_local_override_lifecycle -- --nocapture`
  - `cargo test -p kcode-cli repl_help_includes_shared_commands_and_exit -- --nocapture`

## 原始说法
4. plan-mode 的逻辑有问题。

## 结论
这个说法属实，而且问题不是一个点，而是整条链路不一致。

当前代码里同时存在 4 套互相没有完全打通的 “plan mode” 概念:
- slash command catalog 里有 `/plan`
- tools 层有 `EnterPlanMode` / `ExitPlanMode`
- runtime config 允许读到 `"plan"`
- TUI 里还有一个带 `Plan` 枚举的权限 UI

但它们没有收敛成统一行为，因此现状更像“写配置 + 做展示”，不是“真正可用的 planning mode”。

## 证据
- `/plan` 在命令规格里被宣称为 “Toggle or inspect planning mode”，见 [session_specs.rs](/home/ubuntu/kcode/rust/crates/commands/src/session_specs.rs#L216)
- 但 REPL 主流程里，`SlashCommand::Plan` 被直接归入 “Command registered but not yet implemented.”，见 [live_cli_repl_command.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/live_cli_repl_command.rs#L186)
- TUI 主流程里，`SlashCommand::Plan` 同样是 “not yet implemented in the TUI flow.”，见 [live_cli_tui.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/live_cli_tui.rs#L442)
- `EnterPlanMode` / `ExitPlanMode` 真正做的事情，只是改写工作区本地配置:
  - 把 `.kcode/settings.local.json` 的 `permissions.defaultMode` 写成 `"plan"`
  - 再写一份 `.kcode/tool-state/plan-mode.json` 保存旧值
  - 见 [plan_mode.rs](/home/ubuntu/kcode/rust/crates/tools/src/plan_mode.rs#L11)
- runtime config 解析 `"plan"` 时，并不会生成独立的 plan 语义，而是直接折叠成 `ResolvedPermissionMode::ReadOnly`，见 [config.rs](/home/ubuntu/kcode/rust/crates/runtime/src/config.rs#L859)
- CLI 侧真正可用的权限枚举只有 3 种:
  - `ReadOnly`
  - `WorkspaceWrite`
  - `DangerFullAccess`
  - 根本没有 `Plan`
  - 见 [args.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/args.rs#L42)
- 从配置加载权限模式时，也只会把 resolved mode 映射回这 3 种 CLI 模式，见 [cli_parse_support.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/cli_parse_support.rs#L237)
- `/permissions` 命令也明确只接受:
  - `read-only`
  - `workspace-write`
  - `danger-full-access`
  - 见 [live_cli_session_state.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/live_cli_session_state.rs#L112)
- TUI 里虽然单独定义了 `Prompt / Plan / Auto / BypassDanger` 这套 UI 枚举，但这是另一套前端展示模型，不等于 runtime 真有对应语义，见 [permission_enhanced.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/permission_enhanced.rs#L5)
- 现有测试只覆盖了 `EnterPlanMode` / `ExitPlanMode` 的配置文件往返，不覆盖 slash `/plan`、runtime 权限策略变化、或真实执行行为，见 [tests_runtime.rs](/home/ubuntu/kcode/rust/crates/tools/src/tests_runtime.rs#L132)

## 影响
- 用户会以为 `/plan` 可以切换 planning mode，但实际命令不可用。
- tool 层把模式写成 `"plan"`，运行时却把它当作 `read-only`，语义已经发生折损。
- TUI 可能展示 “plan mode” 风格状态，但底层权限与执行策略并没有对应模式，容易造成 UI/行为不一致。
- 这会直接影响审批策略、工具调用预期、以及用户对“先规划再执行”的理解。

## 建议修复方向
- 先决定 `plan mode` 的唯一语义:
  - 它到底是 `read-only` 别名
  - 还是独立的“先计划、后执行”交互模式
- 如果只是 `read-only` 别名:
  - 删除 `/plan` 命令和 TUI 的独立 Plan UI
  - 避免继续制造第四套命名
- 如果要做成独立模式:
  - 给 runtime 权限模型增加明确的 `Plan`
  - 让 `/plan` 真正调用同一条切换逻辑
  - 让 CLI/TUI/config/tool 都共享同一个 mode 枚举和转换函数
  - 为真实行为补集成测试，而不是只测配置文件写回
