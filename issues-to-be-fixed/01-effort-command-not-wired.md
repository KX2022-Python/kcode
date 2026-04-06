# Issue To Be Fixed: `/effort` 只停留在命令面，未打通到底层请求

## 状态

已修复（2026-04-06）。

## 修复结果

- `/effort` 已从正式 slash command surface 下线，不再出现在共享命令注册表、帮助、建议和命令面板里。
- parser 对这类“仍有 spec 但被隐藏”的命令统一按 unknown 处理，不再落到“已注册但未实现”的假入口。
- 在真正打通 runtime/provider 之前，不再把 `/effort` 伪装成可用能力。

## 验证

- `cargo test -p commands tests_registry -- --nocapture`
- `cargo test -p kcode-cli repl_help_includes_shared_supported_commands -- --nocapture`
- `cargo test -p kcode-cli command_palette -- --nocapture`
- `cargo check --workspace`

## 原始说法

`kcode` 的 `/effort` 只有命令面，底层请求没有真正打通。

## 证据

1. 命令已注册并出现在帮助/命令规格中：
   - `rust/crates/commands/src/parse.rs`
   - `rust/crates/commands/src/session_specs.rs`
   - `rust/crates/kcode-cli/src/tui/repl/command_palette/context.rs`

2. 但在 CLI REPL 实际处理时，它被归类到“已注册但未实现”：
   - `rust/crates/kcode-cli/src/main_parts/live_cli_repl_command.rs`
   - 现有行为：`Command registered but not yet implemented.`

3. 在 TUI 流程里同样没有实际执行逻辑：
   - `rust/crates/kcode-cli/src/main_parts/live_cli_tui.rs`
   - 现有行为：`Command registered but not yet implemented in the TUI flow.`

4. 在 resume 路径中也被明确列为不支持：
   - `rust/crates/kcode-cli/src/main_parts/resume_command.rs`

5. 更关键的是，请求构建/运行时层与 provider 层中没有发现 `reasoning_effort` 或等价字段下传：
   - 检索范围：`rust/crates/api`、`rust/crates/runtime`、`rust/crates/kcode-cli/src/main_parts`
   - 结论：未发现 effort 被写入模型请求 payload 或 runtime 配置

## 影响

- 用户能看到 `/effort`，但无法真正改变模型推理强度。
- 命令面与运行时行为不一致，会造成明显的“功能已存在”错觉。
- 如果后续 UI 或文档继续暴露 `/effort`，会持续制造回归性误解。

## 建议修复方向

1. 先决定 effort 的真实语义和支持范围：
   - 是映射到 provider 原生 `reasoning_effort`
   - 还是映射到 profile / runtime config 的会话级状态

2. 把 `/effort` 从命令面打通到：
   - 当前会话状态
   - runtime 构建参数
   - provider 请求 payload

3. 若当前 provider 不支持，应做显式降级：
   - 返回“当前模型/提供商不支持 effort”
   - 不要继续保留假入口

4. 增加覆盖测试：
   - CLI REPL
   - TUI slash command
   - resume 行为
   - provider request payload 断言
