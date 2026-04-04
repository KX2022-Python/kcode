# Kcode Workspace Rules

更新时间：2026-04-04 16:59:34 JST

## 目录约定

- 开发主目录：`/home/ubuntu/project/kcode`
- 安装/部署副本：`/home/ubuntu/tools/kcode`
- 本机运行态目录：`/home/ubuntu/.kcode`
- 系统安装产物：`/usr/local/bin/kcode`、`/etc/kcode/bridge.env`、`/etc/systemd/system/kcode-bridge.service`
- 当前本机最高权限对齐策略：`~/.kcode/config.toml` 使用 `permission_mode = "danger-full-access"` 且 `[sandbox].enabled = false`

## 开发规则

- 所有源码修改、文档修改、测试、Git 提交与推送，统一在开发主目录 `/home/ubuntu/project/kcode` 内完成。
- 不要在 `/home/ubuntu/tools/kcode` 直接做长期源码修改；该目录只用于安装、部署验证和对照。
- 不要把 `~/.kcode`、`/etc/kcode/bridge.env`、日志、会话、记忆文件提交到 GitHub。
- 需要重新安装本机 `kcode` 时，从 `/home/ubuntu/tools/kcode` 执行安装脚本。

## 运行态边界

- `~/.kcode` 属于本机运行态，不属于仓库源码。
- 会话目录固定为 `/home/ubuntu/.kcode/sessions`，避免把运行态写入源码仓库。
- 若要测试新功能，先在 `/home/ubuntu/project/kcode` 完成改动，再同步到 `/home/ubuntu/tools/kcode` 做安装验证。
