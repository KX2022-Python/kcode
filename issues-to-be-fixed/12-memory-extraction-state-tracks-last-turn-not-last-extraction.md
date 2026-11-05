## 标题
memory extraction 状态机名义上按“自上次提取以来”计数，实际实现更接近“按上一回合增量”计数

## 状态
已修复（2026-04-06）

## 修复结果
- `MemoryExtractionState::record_turn()` 现在只记录最近一回合观测值，不再覆盖“上次提取快照”。
- `reset()` 会同时更新提取快照和最近观测值，保证提取后从新基线重新累计。
- 单测已改成验证“自上次提取以来累计达到阈值才触发”的语义，并通过定向测试：
  - `cargo test -p runtime memory_extraction -- --nocapture`

## 结论
`auto dream` / memory extraction 的状态机存在语义偏差。

`MemoryExtractionState` 的字段和注释都表明它应该记录“上一次提取时的累计快照”；但当前 `ConversationRuntime` 在每个 turn 结束时都会调用 `record_turn()` 覆盖快照，导致阈值判断更接近“当前回合相对上一回合的增量”，而不是“自上次提取以来的累计差值”。

## 证据
- `MemoryExtractionState` 的字段注释写的是:
  - `cumulative_input_tokens_at_last_extraction`
  - `cumulative_tool_calls_at_last_extraction`
  - 见 [memory_extraction.rs](/home/ubuntu/kcode/rust/crates/runtime/src/memory_extraction.rs#L21)
- `should_extract()` 也是按“当前累计值 - 上次快照”来判断阈值，见 [memory_extraction.rs](/home/ubuntu/kcode/rust/crates/runtime/src/memory_extraction.rs#L43)
- 但 `ConversationRuntime` 在每个 turn 结束时，无论是否提取，都会执行:
  - `self.memory_extraction_state.record_turn(...)`
  - 见 [conversation.rs](/home/ubuntu/kcode/rust/crates/runtime/src/conversation.rs#L552)
- 同一位置只有在触发提取时才会调用 `reset(...)`，见 [conversation.rs](/home/ubuntu/kcode/rust/crates/runtime/src/conversation.rs#L537)
- 这两者叠加后，状态机会不断被“上一回合累计值”刷新
- 现有测试也默认了这种实现方式，而不是验证“自上次提取以来累计阈值”语义，见 [memory_extraction.rs](/home/ubuntu/kcode/rust/crates/runtime/src/memory_extraction.rs#L426)

## 影响
- auto dream 可能需要单回合出现很大的 token/tool 增量才会触发
- “默认存在但很少触发”会成为常态
- 配置开关和用户感知即使补齐，也会继续被这个状态机偏差削弱

## 建议修复方向
- 明确状态机语义:
  - 以“上次提取”为基准
  - 还是以“上一回合”为基准
- 如果目标是前者:
  - 不应在每个 turn 无条件覆盖 extraction snapshot
  - 应单独维护 `last_turn_snapshot` 和 `last_extraction_snapshot`
- 为此补充更贴近真实会话的测试:
  - 多轮小增量累计后触发
  - 提取后重新累计
  - 提取失败时如何处理快照
