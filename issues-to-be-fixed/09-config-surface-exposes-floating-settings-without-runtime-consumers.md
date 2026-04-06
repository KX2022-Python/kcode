## 标题
config surface 暴露了多项悬空设置键，但 runtime/TUI 没有消费它们

## 状态
已修复（2026-04-06）

## 修复结果
- `Config` 工具支持列表已收敛到真实有消费方的设置键。
- 以下悬空键已从正式配置面移除：`preferredNotifChannel`、`autoMemoryEnabled`、`fileCheckpointingEnabled`、`terminalProgressBarEnabled`、`todoFeatureEnabled`、`alwaysThinkingEnabled`、`teammateMode`。
- `autoDreamEnabled` 保留，因为它已经在 runtime 中接通并有对应验证。

## 验证
- `cargo test -p tools tests_registry -- --nocapture`
- `cargo check --workspace`

## 结论
`kcode` 当前的配置面存在一批“能写不能用”的设置键。

这些键已经进入 `Config` 工具支持列表，意味着用户可以正式读写它们；但在仓库中看不到对应的 runtime/TUI 消费路径。这类悬空 setting 会持续制造“好像支持配置、其实没有行为变化”的假象。

## 证据
- `Config` 工具支持以下设置键:
  - `preferredNotifChannel`
  - `autoMemoryEnabled`
  - `autoDreamEnabled`
  - `fileCheckpointingEnabled`
  - `terminalProgressBarEnabled`
  - `todoFeatureEnabled`
  - `alwaysThinkingEnabled`
  - `teammateMode`
  - 见 [config.rs](/home/ubuntu/kcode/rust/crates/tools/src/config.rs#L176)
- 对上述键做 repo 级检索时，除 `tools/src/config.rs` 和已写的 issue 文档外，没有查到 runtime 或 TUI 读取它们的代码路径
- TUI 的 runtime 设置结构只包含:
  - `permission_mode`
  - `session_dir`
  - `permission_allow`
  - `permission_deny`
  - `permission_ask`
  - 没有这些设置字段
  - 见 [state.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/tui/state.rs#L205)

## 影响
- 用户可以成功写入设置，但行为不发生变化。
- config surface 继续扩大后，会越来越难分辨“正式配置项”和“遗留占位项”。
- 这类问题和 `/effort`、`auto dream` 的悬空状态属于同一类 harness 缺口。

## 建议修复方向
- 把所有设置键按状态分成:
  - 已生效
  - 预留但隐藏
  - 待删除
- 对没有 runtime 消费方的键，默认不要继续暴露
- 保留的键必须补齐:
  - runtime 读取
  - TUI 展示
  - 生效验证
  - 集成测试
