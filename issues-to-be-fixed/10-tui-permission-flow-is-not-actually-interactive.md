## 标题
TUI 权限模式和增强审批 UI 已有外观结构，但真实权限审批流程没有打通

## 状态
已修复（2026-04-06）

## 修复结果
- TUI 运行时的 `TuiPermissionPrompter` 不再一律拒绝权限请求，而是会在终端内弹出真正可操作的审批对话框。
- 当前审批流已支持 `Allow once`、`Allow turn`、`Deny once`、`Deny turn` 四种决策，并把 “本轮剩余时间” 的 allow/deny 结果缓存到当前 turn 内复用。
- 权限提示会展示工具名、当前权限模式、所需权限模式、可选 reason 和输入预览，避免用户盲批。
- 这次修复保持最短链路，没有引入新的异步状态机；只把阻塞式交互收敛到单独的 `tui_permission_prompt.rs`，避免继续膨胀现有 TUI 主文件。
- 开发源码仓与安装仓已对齐；开发仓额外补齐了 `kcode-cli` 已实际使用但缺失的 `reqwest` 与 `unicode-width` 依赖，消除了两仓分叉导致的编译失败。

## 验证
- 安装版 `/home/ubuntu/kcode`
  - `cargo test -p kcode-cli permission_prompt_keybinds_cover_shortcuts_and_focus_selection -- --nocapture`
  - `cargo test -p kcode-cli cached_tui_permission_decisions_apply_for_rest_of_turn -- --nocapture`
  - `cargo test -p kcode-cli plan_command_uses_tool_backed_local_override_lifecycle -- --nocapture`
- 开发源码 `/home/ubuntu/project/kcode`
  - `cargo test -p kcode-cli permission_prompt_keybinds_cover_shortcuts_and_focus_selection -- --nocapture`
  - `cargo test -p kcode-cli cached_tui_permission_decisions_apply_for_rest_of_turn -- --nocapture`
  - `cargo test -p kcode-cli plan_command_uses_tool_backed_local_override_lifecycle -- --nocapture`
  - `cargo test -p kcode-cli repl_help_includes_shared_commands_and_exit -- --nocapture`

## 原始说法
10. TUI permission flow 并不真正交互。

## 结论
当前 TUI 的权限体验存在明显的“UI 有模式，执行没闭环”问题。

一方面，TUI 里已经定义了 `Prompt / Plan / Auto / BypassDanger` 这一套权限模式和增强审批 UI；另一方面，真正参与 runtime 权限决策的 `TuiPermissionPrompter` 仍然是“一律拒绝”。这说明权限 harness 还停在展示层，没有形成真实交互。

## 证据
- TUI 里定义了独立的权限模式枚举:
  - `Prompt`
  - `Plan`
  - `Auto`
  - `BypassDanger`
  - 见 [permission_enhanced.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/permission_enhanced.rs#L5)
- TUI app 状态里也保留了 `enhanced_permission: Option<EnhancedPermissionRequest>`，见 [mod.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/repl/mod.rs#L82)
- 但对 `enhanced_permission` 的检索结果只出现在字段定义和初始化位置，没有看到真正的渲染、事件接线或 runtime 决策联动
- 真正接入 runtime 的 `TuiPermissionPrompter` 在收到权限请求时会直接返回 `Deny`，并提示用户先用 `/permissions workspace-write` 或 `/permissions danger-full-access`，见 [live_cli_tui_support.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/live_cli_tui_support.rs#L11)
- runtime 权限系统本身是支持通过 `PermissionPrompter` 做真实交互决策的，见 [permissions.rs](/home/ubuntu/kcode/rust/crates/runtime/src/permissions.rs#L80)

## 影响
- TUI 用户会看到更丰富的权限模式和 UI 结构，但真正需要审批时无法在 TUI 内完成。
- `prompt` / `plan` / `auto` 这些模式在 TUI 里容易形成误导性预期。
- 这会直接削弱 TUI 作为主工作界面的可信度。

## 建议修复方向
- 先决定 TUI 权限流的唯一目标形态:
  - 真正做交互式审批
  - 或者明确删掉这套 UI-only 权限模式
- 如果保留，应补齐:
  - 权限请求渲染
  - 键盘决策
  - allow once / allow always / deny once / deny always
  - 与 runtime `PermissionPrompter` 的双向联动
- 在闭环完成前，不应继续扩大 TUI 里的权限 UI 语义
