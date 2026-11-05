## 标题
slash command surface 暴露了大量未实现能力，命令注册表与主执行流脱节

## 状态
已修复（2026-04-06）

## 修复结果
- 一批仍会落到“registered but not yet implemented”的命令已从正式 slash surface 下线，不再出现在帮助、建议、命令面板和共享注册表里。
- parser 对这些隐藏命令统一按 unknown 处理，避免继续把未实现入口伪装成正式能力。
- 帮助建议现在只基于可见命令集合生成，`/stats` 之类旧别名不再被继续推荐。

## 验证
- `cargo test -p commands tests_registry -- --nocapture`
- `cargo test -p kcode-cli repl_help_includes_shared_supported_commands -- --nocapture`
- `cargo test -p kcode-cli command_palette -- --nocapture`
- `cargo check --workspace`

## 结论
当前 `kcode` 的 slash command registry 明显过度暴露。

命令规格、帮助信息、命令面板和解析器已经把很多命令当成正式能力对外展示，但 REPL/TUI 主执行流仍把其中一大批统一打回 `"Command registered but not yet implemented"`。这已经不是单个命令遗漏，而是 command harness 和真实执行流系统性脱节。

## 证据
- slash command spec 对外暴露了完整命令面，例如 `/plan`、`/review`、`/usage`、`/rename`、`/copy`、`/effort`、`/rewind`、`/ide`、`/tag`、`/add-dir` 等，见 [session_specs.rs](/home/ubuntu/kcode/rust/crates/commands/src/session_specs.rs#L1)
- command registry 会把这些 spec 直接转成对外描述，`description` 直接使用 `summary`，见 [registry.rs](/home/ubuntu/kcode/rust/crates/commands/src/registry.rs#L116)
- REPL 主流程里，下列命令被统一归入未实现分支:
  - `logout`
  - `vim`
  - `upgrade`
  - `stats`
  - `share`
  - `files`
  - `fast`
  - `exit`
  - `summary`
  - `brief`
  - `advisor`
  - `stickers`
  - `insights`
  - `thinkback`
  - `release-notes`
  - `security-review`
  - `plan`
  - `review`
  - `usage`
  - `rename`
  - `copy`
  - `context`
  - `effort`
  - `rewind`
  - `ide`
  - `tag`
  - `add-dir`
  - 见 [live_cli_repl_command.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/live_cli_repl_command.rs#L170)
- TUI 主流程里，同一批命令也被统一打回 `"Command registered but not yet implemented in the TUI flow."`，见 [live_cli_tui.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/live_cli_tui.rs#L426)
- resume 路径对这些命令中的大量成员同样直接返回 `unsupported resumed slash command`，见 [resume_command.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/resume_command.rs#L233)

## 影响
- 用户会在 `/` 菜单和帮助中看到这些命令，并合理地认为它们可用。
- command palette、help、parser、runtime 主执行流四层没有闭环。
- 每新增一个“先注册后实现”的命令，命令面的可信度都会继续下降。

## 建议修复方向
- 先按命令成熟度分层:
  - 真正可用
  - experimental
  - 未实现
- 未实现命令默认不应出现在正式 slash surface 中
- 如果保留展示，必须带清晰标识，不允许伪装成正式能力
- 建立约束:
  - 新命令只有在 parse + registry + main flow + tests 都齐全时才能对外暴露
