# Kcode Workspace Rules

更新时间：2026-05-19 JST

## 目录约定

- 权威源码目录：`/home/ubuntu/project/kcode`
- 旧活跃修复目录：`/home/ubuntu/tools/kcode`，只作差异审计来源；吸收有效内容后归档或等待明确批准清理
- 本机运行态目录：`/home/ubuntu/.kcode`
- 系统安装产物：`/usr/local/bin/kcode`、`/usr/local/bin/kcode-engine`、`/usr/local/lib/kcode/tui/dist/index.js`、`/etc/kcode/bridge.env`、`/etc/systemd/system/kcode-bridge.service`
- 当前本机最高权限对齐策略：`~/.kcode/config.toml` 使用 `permission_mode = "danger-full-access"` 且 `[sandbox].enabled = false`

## 开发规则

- 默认只在 `/home/ubuntu/project/kcode` 修改源码、文档和脚本。
- 每轮真实验收对象是 `/usr/local/bin/kcode`，不能只验证源码目录里的 debug/release binary。
- `kcode` 无参数默认进入 TS/React/Ink TUI；`kcode --headless` 或 `kcode-engine` 进入 Rust engine 模式；`KCODE_TUI=rust` 是临时 ratatui fallback。
- Git 提交、脱敏扫描与 GitHub 推送，统一从 `/home/ubuntu/project/kcode` 发起；git add 前必须扫描密钥形态。
- 不要把 `~/.kcode`、`/etc/kcode/bridge.env`、日志、会话、记忆文件提交到 GitHub。
- 不要把 `tui/node_modules`、`tui/dist`、Rust `target/` 或缓存提交到 GitHub。

## 运行态边界

- `~/.kcode` 属于本机运行态，不属于仓库源码。
- 会话目录固定为 `/home/ubuntu/.kcode/sessions`，避免把运行态写入源码仓库。
- `tui/` 只保存 TS/React/Ink 源码和协议类型；构建产物安装到 `/usr/local/lib/kcode/tui/dist/index.js`。
- Rust engine 继续负责 runtime/tools/session/provider/MCP/memory/bridge，TS TUI 负责交互、布局、输入、消息渲染、权限弹窗、goal/agent 可读性。
