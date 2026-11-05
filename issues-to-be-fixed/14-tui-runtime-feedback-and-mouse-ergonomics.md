## 标题

TUI 运行中反馈、鼠标交互和滚动条机制需要按 Codex 风格重做

## 状态

`in_progress`

## 结论

这不是 Rust 语言限制，主要是当前 TUI 的执行模型和交互分层做错了。

- 现有 REPL TUI 把命令执行直接塞进 UI 主循环，同步阻塞了 redraw 和事件处理，所以 prompt 发出后无法持续做动态刷新，也无法在执行期间继续接收输入事件。
- 现有代码已经具备状态机、消息渲染、鼠标事件入口，以及 provider 侧 phase 标记能力，说明“做动态反馈”在能力上完全可行，只是没把这些能力接成一个非阻塞 UI harness。
- `Ctrl+滚轮改字体大小` 不能理解为“直接调用终端宿主 API 改真实字体像素”。终端 TUI 正确做法是控制 TUI 自己的缩放级别和信息密度，例如行距、边距、wrap 宽度、面板高度和消息压缩策略。

## 证据

- `run_repl_loop()` 在收到键盘/鼠标后，直接同步调用 `process_pending_command()`，而 `process_pending_command()` 又直接执行 `executor(...)`；执行期间 UI 主循环停住，无法持续 redraw。
  - [runtime_loop.rs](/home/ubuntu/tools/kcode/rust/crates/kcode-cli/src/tui/repl/runtime_loop.rs)
- `ReplApp` 已经有 `SessionState`、`RenderableMessage`、鼠标处理入口和 footer 状态位，说明展示层不是空白，只是没形成异步闭环。
  - [mod.rs](/home/ubuntu/tools/kcode/rust/crates/kcode-cli/src/tui/repl/mod.rs)
  - [state.rs](/home/ubuntu/tools/kcode/rust/crates/kcode-cli/src/tui/repl/state.rs)
- provider 侧已经有 `progress_reporter.mark_model_phase()` / `mark_text_phase()` / `mark_tool_phase()`，但这些 phase 只写到 stdout，不回流到 TUI 状态树。
  - [provider_runtime_client_impl.rs](/home/ubuntu/tools/kcode/rust/crates/kcode-cli/src/main_parts/provider_runtime_client_impl.rs)
  - [plugin_runtime_state.rs](/home/ubuntu/tools/kcode/rust/crates/kcode-cli/src/main_parts/plugin_runtime_state.rs)
- 现有鼠标处理只做了简单 `ScrollUp/ScrollDown` 消息滚动，既没有区分 `Ctrl+滚轮`，也没有 hover scrollbar。
  - [mod.rs](/home/ubuntu/tools/kcode/rust/crates/kcode-cli/src/tui/repl/mod.rs)

## 影响

- 用户发送 prompt 后只看到静态 processing，没有持续“进行中”反馈，像卡住而不是在工作。
- thinking / execution 细节无法稳定呈现，用户看不到当前卡在模型思考、工具执行还是收尾阶段。
- 鼠标交互和用户预期不一致：普通滚轮、Ctrl+滚轮、右侧边栏反馈都缺位。

## 当前进展

- 已修复鼠标捕获缺失，REPL TUI 现在会显式启用/关闭 mouse capture。
- 已修复普通滚轮和 `Ctrl+滚轮` 的职责分离：普通滚轮只滚主消息窗，`Ctrl+滚轮` 控制 TUI 内部缩放和密度。
- 已补右侧 hover 滚动条，鼠标进入消息区或右侧边缘时显示，离开后隐藏。
- 运行中动态 feedback、thinking/execution phase 回流仍未实施，本 issue 保持未关闭。

## 建议修复方向

- 将 TUI REPL 改为 UI 线程 + 后台执行线程 + UI 事件通道，避免把执行逻辑塞进 redraw 主循环。
- 把 provider 侧现有 phase 标记接回 TUI 状态，而不是继续写 stdout。
- 动画效果做在 UI tick/re-draw 层，不做在同步业务逻辑里。
- 普通滚轮只滚消息区；`Ctrl+滚轮` 改为控制 TUI 内部缩放级别，不去碰宿主终端字体 API。
- 右侧 hover 滚动条需要单独做消息区 hover 检测和 thumb 渲染。

## 后续执行约束

- 后续处理这个问题时，默认按本 issue 的方向执行，不再把“做不到持续动态展示”归因于 Rust。
- 若实现路径再次把后台执行塞回 UI 主循环，应视为回到错误方案，直接否决。
