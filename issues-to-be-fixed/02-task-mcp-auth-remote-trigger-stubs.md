## 标题
task / MCP auth / remote trigger 在 agent tools dispatch 层仍是占位实现

## 状态
已修复（2026-04-06）

## 修复结果
- `TaskCreate/TaskGet/TaskList/TaskStop/TaskUpdate/TaskOutput`、`ListMcpResources/ReadMcpResource/McpAuth/MCP`、`RemoteTrigger` 已从正式内建工具面下线。
- builtin dispatch 不再为这些名称提供占位实现；直接调用会返回 `unsupported tool`，不再伪造成功、空结果或占位文案。
- 对应的占位输入类型也已删除，避免继续扩散伪能力接口。

## 验证
- `cargo test -p tools tests_registry -- --nocapture`
- `cargo check --workspace`

## 原始说法
1. 一些 `task` / `mcp auth` / `remote trigger` 孩子是 stub。

## 结论
这个说法基本属实，而且范围比原说法更大。

在 `tools` 内建工具链里，`TaskCreate/TaskGet/TaskList/TaskStop/TaskUpdate/TaskOutput`、`ListMcpResources/ReadMcpResource/McpAuth/MCP`、`RemoteTrigger` 都已经注册到 tool catalog，但实际执行仍走占位返回，不会连到真实后台任务运行时、MCP client/auth 流程或真实远程触发。

## 证据
- 内建 tool registry 会把这些名字直接路由到 builtin dispatch，而不是插件或另一条真实执行链:
  - [registry.rs](/home/ubuntu/kcode/rust/crates/tools/src/registry.rs#L197)
- tool spec 对外宣称的是“真实能力”:
  - `TaskCreate`: “Create a background task that runs in a separate subprocess.” 见 [extended_specs.rs](/home/ubuntu/kcode/rust/crates/tools/src/extended_specs.rs#L98)
  - `TaskGet/TaskList/TaskStop/TaskUpdate/TaskOutput` 见 [extended_specs.rs](/home/ubuntu/kcode/rust/crates/tools/src/extended_specs.rs#L112)
  - `ListMcpResources/ReadMcpResource/McpAuth/MCP` 见 [extended_specs.rs](/home/ubuntu/kcode/rust/crates/tools/src/extended_specs.rs#L257)
  - `RemoteTrigger` 见 [extended_specs.rs](/home/ubuntu/kcode/rust/crates/tools/src/extended_specs.rs#L292)
- 实际 dispatch 返回的是占位 JSON:
  - `TaskCreate` 只生成一个按时间戳拼出来的 `task_id`，没有启动子进程或注册运行时状态，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L81)
  - `TaskGet` 直接回 `"Task runtime not yet implemented"`，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L94)
  - `TaskList` 固定回空数组，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L102)
  - `TaskStop` 直接回 `"Task stop requested"`，没有真实停止逻辑，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L106)
  - `TaskUpdate` 只是把输入消息回显，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L114)
  - `TaskOutput` 固定回空输出，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L122)
  - `ListMcpResources` 固定回空 `resources`，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L187)
  - `ReadMcpResource` 固定回空内容，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L195)
  - `McpAuth` 直接回 `"MCP authentication not yet implemented"`，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L204)
  - `RemoteTrigger` 直接回 `"Remote trigger stub response"`，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L212)
  - `MCP` 直接回 `"MCP tool proxy not yet connected"`，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L223)

## 影响
- agent 看到 catalog 会以为这些工具可用，但真正调用时拿到的是伪成功或空结果。
- `task` 语义目前不能支撑后台执行、查询状态、读取输出、停止任务这一整套生命周期。
- MCP 配置和 OAuth 字段虽然在 runtime/config/TUI 侧已有模型，但 agent tools 层并没有真正接进去。
- `RemoteTrigger` 当前不会发真实 HTTP 请求，任何依赖 webhook/远程动作的自动化都会产生误判。

## 建议修复方向
- 为 `task` 建立真实的后台任务注册表、状态持久化、输出缓冲和 stop/update 协议。
- 把 `McpAuth` 接到现有 runtime OAuth / MCP client 初始化链，而不是返回文案。
- 把 `ListMcpResources`、`ReadMcpResource`、`MCP` 接到真实 MCP transport/client。
- `RemoteTrigger` 要么实现真实 HTTP 调用和权限控制，要么在 catalog 中先下线，避免“伪可用”。
- 为这些工具增加集成测试，要求校验真实副作用而不是只看 JSON 字段存在。
