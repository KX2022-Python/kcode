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
- 只有当活跃目录验证通过后，才把对应改动同步到开发源码目录 `/home/ubuntu/project/kcode`。
- Git 提交、脱敏扫描与 GitHub 推送，统一从 `/home/ubuntu/project/kcode` 发起。
- 不要把未经活跃目录验证的改动直接从开发源码目录推到 GitHub。
- 不要把 `~/.kcode`、`/etc/kcode/bridge.env`、日志、会话、记忆文件提交到 GitHub。
- 需要重新安装或覆盖本机 `kcode` 时，以 `/home/ubuntu/tools/kcode` 当前验证通过的构建产物为准。

## 运行态边界

- `~/.kcode` 属于本机运行态，不属于仓库源码。
- 会话目录固定为 `/home/ubuntu/.kcode/sessions`，避免把运行态写入源码仓库。
- 若要测试新功能，先在 `/home/ubuntu/tools/kcode` 完成改动并做本机真实验证，再同步到 `/home/ubuntu/project/kcode` 做脱敏、提交与推送。
