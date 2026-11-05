## 标题
bridge runtime 在多通道启动、会话键设计和路径约定上存在结构性裂缝

## 状态
已修复（2026-04-06）

## 修复结果
- bridge 启动入口已经收敛到唯一实现：CLI 的 `run_bridge()` 现在直接委托给 `run_bridge_service()`，不再保留第二套近似但漂移的启动逻辑。
- `BridgeCore` 不再依赖 Telegram transport 才会启动；只要任一 channel 凭据存在，核心线程都会启动，WhatsApp / Feishu-only 场景不再天然挂空。
- bridge session 主键已经统一成 `channel + chat_id`，持久化文件名也同步切到 `bridge-<channel>-<chat_id>.jsonl`，消除了跨 channel 的 chat id 冲突面。
- bridge 会话目录现在收敛到 `config_home/bridge-sessions`，不再写死相对路径 `.kcode/bridge-sessions`。
- `LiveCli::new(..., Some(session_path))` 现在会真正加载已有会话文件；bridge 指定路径恢复不再只是“指向旧文件但实际起新 session”。
- webhook server 不再携带未参与路由的 `SessionRouter` 状态，BridgeCore 和 SessionRouter 的职责边界收紧成“SessionRouter 负责 channel-scoped session id / path，BridgeCore 负责实际 LiveCli 生命周期”。

## 验证
- 安装版 `/home/ubuntu/kcode`
  - `cargo test -p adapters session_router_scopes_same_chat_id_by_channel -- --nocapture`
  - `cargo test -p adapters session_path_matches_channel_scoped_session_id -- --nocapture`
  - `cargo test -p kcode-cli bridge_session_dir_lives_under_config_home -- --nocapture`
  - `cargo test -p kcode-cli live_cli_loads_existing_bridge_session_from_override_path -- --nocapture`
- 开发源码 `/home/ubuntu/project/kcode`
  - `cargo test -p adapters session_router_scopes_same_chat_id_by_channel -- --nocapture`
  - `cargo test -p adapters session_path_matches_channel_scoped_session_id -- --nocapture`
  - `cargo test -p kcode-cli bridge_session_dir_lives_under_config_home -- --nocapture`
  - `cargo test -p kcode-cli live_cli_loads_existing_bridge_session_from_override_path -- --nocapture`

## 原始说法
11. bridge runtime 多通道与 session 架构裂缝。

## 结论
`kcode` 的 bridge 目前不是单点 bug，而是多处架构约定不一致。

主要问题有 4 类:
- BridgeCore 的启动依赖 Telegram transport，导致非 Telegram-only 场景不稳
- bridge 有两套近似但不一致的启动逻辑
- 预检路径和实际运行路径不一致
- 会话键设计没有把 `channel` 纳入真正的持久化主键

## 证据
- `BridgeCore` 的 `SessionManager` 虽然接收 `channel` 参数，但实际 session map 和 session 文件都只按 `chat_id` 建键:
  - `sessions: HashMap<String, LiveCli>`
  - `session_path = ...join(format!(\"{}.jsonl\", chat_id))`
  - 见 [bridge_core.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/bridge_core.rs#L31)
- 这意味着不同 channel 上如果出现相同 chat id，会共享同一 session key 设计
- `run_bridge_service()` 里，BridgeCore 只在 `telegram_transport` 存在时才启动线程，见 [bridge_core.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/bridge_core.rs#L267)
- 如果只配置 WhatsApp 或 Feishu，webhook handler 仍会把事件发到 `core_tx` 并等待回复，但核心线程没有被启动，见 [bridge_core.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/bridge_core.rs#L290)
- `bridge_repl_types.rs` 里还保留了一套近似重复但行为不同的 bridge 启动逻辑:
  - Telegram 固定走 polling
  - 仍然保留 webhook 相关代码分支
  - 见 [bridge_repl_types.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/bridge_repl_types.rs#L19)
- `quick_preflight_check()` 检查的是 `~/.kcode/sessions`，但 bridge 真正用的是 `.kcode/bridge-sessions`，见 [bridge_repl_types.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/main_parts/bridge_repl_types.rs#L130)
- `SessionRouter` 单独实现了按 `channel + chat_id` 生成 `bridge-<channel>-<chat_id>` session id 的思路，见 [session_router.rs](/home/ubuntu/kcode/rust/crates/adapters/src/session_router.rs#L1)
- 但它在 webhook server 里只是被存入 state，没有实际参与 webhook handler 的路由逻辑，见 [session_router.rs](/home/ubuntu/kcode/rust/crates/adapters/src/session_router.rs#L29) 和 [webhook_server.rs](/home/ubuntu/kcode/rust/crates/adapters/src/webhook_server.rs#L31)

## 影响
- bridge 多通道行为难以推断，尤其是非 Telegram 主通道场景。
- 会话持久化主键设计不稳定，未来扩展更多 channel 时容易出冲突。
- 同一能力出现两套近似实现，会持续引入行为漂移。

## 建议修复方向
- 收敛为唯一 bridge 启动入口
- 把会话主键统一成 `channel + channel_chat_id`
- 明确 SessionRouter 和 BridgeCore 的职责边界，不要保留两套并行设计
- 让 BridgeCore 的存在不依赖 Telegram transport
- 校正预检路径、运行路径、doctor 输出和文档说明
