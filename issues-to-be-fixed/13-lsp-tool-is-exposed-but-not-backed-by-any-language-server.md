## 标题
LSP 工具已经公开暴露，但没有连接任何真实 Language Server

## 状态
已修复（2026-04-06）

## 修复结果
- `LSP` 已从正式工具面下线，不再作为可用 code intelligence 工具对外暴露。
- builtin dispatch 不再接受 `LSP`；调用时统一返回 `unsupported tool`，避免继续制造“已连上语言服务器”的错觉。

## 验证
- `cargo test -p tools tests_registry -- --nocapture`
- `cargo check --workspace`

## 结论
`LSP` 是一个典型的 harness 缺口。

它已经被作为正式工具注册，对外宣称可以做 `symbols / references / diagnostics / definition / hover` 这类代码智能查询；但实际 dispatch 只会回一个空 `results` 和 `"LSP server not connected"`。这说明工具面和真实后端没有打通。

## 证据
- `LSP` 已经出现在扩展工具 spec 中，描述为:
  - `"Query Language Server Protocol for code intelligence (symbols, references, diagnostics)."`
  - 见 [extended_specs.rs](/home/ubuntu/kcode/rust/crates/tools/src/extended_specs.rs#L240)
- tool dispatch 也把 `"LSP"` 作为正式工具入口处理，见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L53)
- 但 `run_lsp()` 的实际返回只是:
  - 回显输入参数
  - `results: []`
  - `message: "LSP server not connected"`
  - 见 [dispatch.rs](/home/ubuntu/kcode/rust/crates/tools/src/dispatch.rs#L175)

## 影响
- 用户和 agent 都会把 `LSP` 当作正式只读代码智能工具来使用。
- 但实际上它不会连接项目语言服务器，也不会产生真正的 symbol/reference/definition 结果。
- 这会进一步削弱工具面的可信度。

## 建议修复方向
- 如果短期不做真实 LSP backend:
  - 从正式工具面隐藏 `LSP`
  - 或明确标记为 experimental/unavailable
- 如果要保留:
  - 接入真实语言服务器生命周期管理
  - 定义 workspace / language / project root 发现逻辑
  - 为 `symbols / references / diagnostics / definition / hover` 补真实集成测试
