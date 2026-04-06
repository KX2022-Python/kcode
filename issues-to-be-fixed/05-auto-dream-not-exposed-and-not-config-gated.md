## 标题
auto dream 已有底层触发代码，但没有用户入口，配置开关也未真正接入运行时

## 状态
已修复（2026-04-06）

## 修复结果
- `autoDreamEnabled` 已接入 runtime 配置解析，默认保持开启，设置为 `false` 时会真正阻止 auto-dream 触发。
- 新增 `/dream [on|off|status]`：
  - REPL 可直接查看和切换
  - TUI slash flow 可直接查看和切换
  - `--resume SESSION /dream ...` 也可工作
- `/dream on|off` 会写入当前工作区的 `.kcode/settings.local.json`，随后重载 runtime，使后续 turn 立即生效。
- 帮助面和共享命令规格已同步更新。

## 验证
- `cargo test -p runtime auto_dream -- --nocapture`
- `cargo test -p runtime memory_extraction -- --nocapture`
- `cargo test -p commands parses_supported_slash_commands -- --nocapture`
- `cargo test -p commands rejects_invalid_argument_values -- --nocapture`
- `cargo test -p commands renders_help_from_shared_specs -- --nocapture`
- `cargo test -p commands render_slash_command_help_for_context_hides_tool_commands_when_profile_disables_tools -- --nocapture`
- `cargo test -p kcode-cli resumed_dream_command_updates_local_settings_end_to_end -- --nocapture`
- `cargo test -p kcode-cli repl_help_includes_shared_commands_and_exit -- --nocapture`

## 原始说法
auto dream 好像在当前 `kcode` 里无法通过 `/` 命令调取开关，也不清楚是不是默认开启。

## 结论
这个判断基本正确。

当前 `kcode` 里确实存在 auto dream 的底层实现，但它还不是一个完整、可控、可验证的用户功能。更准确地说:

- 运行时里有自动触发 memory extraction 的代码
- 但没有 `/dream` 或等价的 slash 命令入口
- TUI 配置页也没有暴露这个设置
- `autoDreamEnabled` 只出现在 tools 的 config setting 列表里，我没有查到 runtime 实际读取这个开关的代码

所以现状不是“用户可控但默认开启”，而是“底层路径存在，但用户侧没有正式开关，配置项还是悬空的”。

## 证据
- `autoDreamEnabled` 只出现在配置工具支持的 setting 列表里，见 [config.rs](/home/ubuntu/kcode/rust/crates/tools/src/config.rs#L181)
- slash 命令解析表里没有 `/dream` 或同类入口，见 [parse.rs](/home/ubuntu/kcode/rust/crates/commands/src/parse.rs#L202)
- TUI runtime 配置结构里也没有 `autoDreamEnabled` 字段，见 [state.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/state.rs#L205)
- conversation runtime 在回合结束时会直接检查阈值并触发 auto-dream，没有看到对 `autoDreamEnabled` 的守卫条件，见 [conversation.rs](/home/ubuntu/kcode/rust/crates/runtime/src/conversation.rs#L537)
- 真正的 auto-dream 执行实现存在于内存提取模块，会在后台 `spawn` 提取任务，见 [memory_extraction.rs](/home/ubuntu/kcode/rust/crates/runtime/src/memory_extraction.rs#L555)
- 阈值机制也确实存在:
  - `50_000` input tokens
  - `10` tool calls
  - 见 [memory_extraction.rs](/home/ubuntu/kcode/rust/crates/runtime/src/memory_extraction.rs#L15)

## 补充观察
- 目前 `MemoryExtractionState::record_turn()` 会在每个 turn 结束后记录当前累计值，见 [conversation.rs](/home/ubuntu/kcode/rust/crates/runtime/src/conversation.rs#L552) 和 [memory_extraction.rs](/home/ubuntu/kcode/rust/crates/runtime/src/memory_extraction.rs#L36)
- 这会让阈值判断更接近“单回合增量是否足够大”，而不是“自上次提取后累计是否达到阈值”
- 也就是说，即使 auto dream 路径默认存在，它的实际触发频率也可能低于设计预期

## 影响
- 用户无法通过 slash 命令或 TUI 明确查看 / 开启 / 关闭 auto dream。
- `autoDreamEnabled` 这个配置键会制造“好像可配”的错觉，但当前运行时并不消费它。
- 该功能目前更像隐藏实现，而不是稳定的产品能力。
- 如果阈值状态记录逻辑确有偏差，auto dream 还可能表现出“默认存在但几乎不触发”的假象。

## 建议修复方向
- 先决定产品语义:
  - auto dream 是否应该默认开启
  - 是否允许按 worktree/session 配置关闭
- 把 `autoDreamEnabled` 真正接到 runtime:
  - conversation runtime 初始化时读取设置
  - 触发前显式判断开关
- 增加正式用户入口:
  - `/dream on|off|status`
  - 或在 `kcode tui` 的 Runtime/Review 配置页暴露
- 为 auto dream 增加可观测性:
  - 最近一次触发时间
  - 当前是否在运行
  - 最近一次提取结果
- 复核 `MemoryExtractionState` 的阈值累计逻辑，确保它按“自上次提取以来的累计差值”工作，而不是按“上一回合差值”工作
