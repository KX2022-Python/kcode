# Kcode 回归检查单

每次发布前必须通过的检查项。

## 1. 编译验证

```bash
cd rust && cargo check --workspace
# 期望：编译成功，且无 warning
```

## 2. 测试验证

```bash
cd rust && cargo test --workspace -q
# 期望：工作区所有测试通过
```

## 3. Release 构建

```bash
cd rust && cargo build --release
# 期望：编译成功，生成 target/release/kcode
```

## 4. 命令可用性验证

| 命令 | 验证方式 | 期望结果 |
|------|---------|---------|
| `kcode version` | 运行命令 | 输出版本号 |
| `kcode doctor` | 运行命令 | 显示环境检查结果 |
| `kcode --help` | 运行命令 | 显示默认 TS TUI、`--headless` 与 `kcode-engine` 入口 |
| `kcode -p "hello"` | 运行命令 | 走 Rust engine 非交互 prompt 路径 |
| `kcode` | 交互终端启动 | 默认进入 TS/React/Ink TUI；若设置 `KCODE_TUI=rust` 则进入 Rust TUI fallback |
| `kcode --headless` | 交互终端启动 | 跳过 TS TUI，直接进入 Rust engine TUI |
| `kcode-engine --help` | 运行命令 | Rust engine 入口可用，不回跳 TS TUI |
| `kcode tui` | 交互终端启动 | 进入设置 TUI 并可安全退出 |
| `kcode configure bridge` | 交互终端启动 | 直接打开 bridge 设置页 |
| `kcode init` | 运行命令 | 创建配置和目录 |
| `kcode config show` | 运行命令 | 显示配置 |
| `kcode config tui bridge` | 交互终端启动 | `config show` 不受影响，TUI 入口正常 |
| `kcode resume` | 运行命令 | 显示可恢复会话 |
| `kcode /help` | 运行后输入 | 显示帮助 |
| `kcode /memory` | 运行后输入 | 显示 memory 状态 |
| `kcode /goal` | 运行后输入 | TS TUI 内展示当前 goal；`/goal <objective>` 设置，`/goal done` 完成，`/goal clear` 清空 |
| `kcode /compact` | 运行后输入 | 触发 compaction |
| `kcode /mcp` | 运行后输入 | 显示 MCP 状态 |
| `kcode /tasks` | 运行后输入 | 显示任务帮助 |
| `kcode /status` | 运行后输入 | 显示会话状态 |
| `kcode /model` | 运行后输入 | 显示/切换模型 |
| `kcode /permissions` | 运行后输入 | 显示权限状态 |

## 5. 高保真行为回归

对照 Claude Code 官方行为与 CC Source Map 结构，检查以下行为是否保持一致：

### 5.1 交互行为

- [ ] REPL 启动后显示 banner 和当前状态摘要
- [ ] Tab 补全命令名
- [ ] 输入 `/` 后显示可用命令列表
- [ ] 未知命令返回友好错误提示
- [ ] TS TUI 的 permission dialog 路径可打开并可 approve / deny
- [ ] Agent progress 以可读分组行展示，不泄漏原始后端 JSON
- [ ] `/goal` 在 TS TUI header 中可见并能切换 active / complete / none
- [ ] TS TUI 输入框支持输入法组合输入、英文、日文、符号和混合 UTF-8 文本
- [ ] 鼠标滚轮默认滚动终端上下文，不进入输入框选择或菜单状态
- [ ] 任务运行中按 ESC 可以取消当前 engine 子进程，并回到可输入状态
- [ ] `kcode tui` 能在交互终端进入、导航、保存并退出
- [ ] 非交互终端调用 `kcode tui` 时返回清晰错误并指向 `kcode config show`

### 5.2 工具行为

- [ ] `bash` 工具执行 shell 命令并返回结果
- [ ] `read_file` 读取文件内容
- [ ] `write_file` 写入文件
- [ ] `edit_file` 编辑文件（old_string → new_string）
- [ ] `glob_search` 按 glob 模式搜索文件
- [ ] `grep_search` 按正则搜索文件内容

### 5.3 会话行为

- [ ] 会话自动保存到 `~/.kcode/sessions/`
- [ ] `kcode resume` 可恢复最近会话
- [ ] 长会话自动触发 compact
- [ ] compact 后会话可继续

### 5.4 权限行为

- [ ] ReadOnly 模式下拒绝写操作
- [ ] WorkspaceWrite 模式下允许文件操作但拒绝 bash
- [ ] DangerFullAccess 允许所有操作

### 5.5 Bridge 行为

- [ ] bridge-safe 命令过滤正确
- [ ] local-ui 命令不暴露到 bridge
- [ ] loopback adapter 可模拟消息往返
- [ ] `~/.kcode/bridge.env` 或 `KCODE_BRIDGE_ENV_FILE` 中的 bridge 凭据可被 doctor / bridge 启动读取

## 6. 平台验证

| 平台 | 验证项 | 状态 |
|------|--------|------|
| Linux/VPS | 编译 + 测试 + 安装脚本 | ✅ 已验证 |
| macOS | 编译（待验证） | ⏳ |
| Windows | 编译（待验证） | ⏳ |

## 7. 安全验证

- [ ] 无硬编码 API 密钥
- [ ] 无 Anthropic/Claude 默认端点泄漏
- [ ] `kcode doctor` 正确检测残留配置
- [ ] memory 目录权限 0700
- [ ] memory 文件权限 0600

## 8. 新增 Phase 后更新

每次新增 Phase 时，更新本检查单中对应的行为验证项。
