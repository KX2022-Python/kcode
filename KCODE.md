# Kcode Workspace Rules

更新时间：2026-04-06 23:12:00 JST

## 目录约定

- 活跃修复与验证目录：`/home/ubuntu/tools/kcode`
- 开发源码目录：`/home/ubuntu/project/kcode`
- 本机运行态目录：`/home/ubuntu/.kcode`
- 系统安装产物：`/usr/local/bin/kcode`、`/etc/kcode/bridge.env`、`/etc/systemd/system/kcode-bridge.service`
- 当前本机最高权限对齐策略：`~/.kcode/config.toml` 使用 `permission_mode = "danger-full-access"` 且 `[sandbox].enabled = false`

## 开发规则

- 默认先在活跃目录 `/home/ubuntu/tools/kcode` 完成源码修改与真实验证，优先保证改动能被本机实际运行路径立刻验收。
- `/home/ubuntu/tools/kcode` 是源码工作区，不是可直接执行的单一二进制；源码改动只有在该工作区内重新构建后，才会体现在新的构建产物里。
- 本机用 `kcode` 命令体验 `tools/kcode` 最新修改时，唯一正确链路是：先在 `/home/ubuntu/tools/kcode` 构建，再用该次构建产物覆盖 `/usr/local/bin/kcode`，然后从 shell 里执行 `kcode`。
- 不要把 `rust/target/debug/kcode` 视为“重复造轮子”；它是 Rust 工程的标准本地构建产物，作用是把 `tools/kcode` 的源码变成可执行文件。
- 只有 `/usr/local/bin/kcode` 被新的 `tools/kcode` 构建产物覆盖后，终端里直接输入 `kcode` 才会看到这次源码修改的真实效果。
- 只有当活跃目录验证通过后，才把对应改动同步到开发源码目录 `/home/ubuntu/project/kcode`。
- Git 提交、脱敏扫描与 GitHub 推送，统一从 `/home/ubuntu/project/kcode` 发起。
- 不要把未经活跃目录验证的改动直接从开发源码目录推到 GitHub。
- 不要把 `~/.kcode`、`/etc/kcode/bridge.env`、日志、会话、记忆文件提交到 GitHub。
- 需要重新安装或覆盖本机 `kcode` 时，以 `/home/ubuntu/tools/kcode` 当前验证通过的构建产物为准。
- 若用户正在 `tools/kcode` 阶段做交互验收，默认先停在 `tools/kcode -> /usr/local/bin/kcode` 这一层，不提前同步到 `/home/ubuntu/project/kcode`，更不提前推 GitHub。

## 运行态边界

- `~/.kcode` 属于本机运行态，不属于仓库源码。
- 会话目录固定为 `/home/ubuntu/.kcode/sessions`，避免把运行态写入源码仓库。
- 若要测试新功能，先在 `/home/ubuntu/tools/kcode` 完成改动并做本机真实验证，再同步到 `/home/ubuntu/project/kcode` 做脱敏、提交与推送。
