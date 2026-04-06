# Kcode Mirror Repair Sync Spec

## Goal

将 `/home/ubuntu/kcode` 的有效业务工作树完整迁移到 `/home/ubuntu/tools/kcode`，在目标仓中按 `issues-to-be-fixed/` 与 `Harness Engineering Guidance` 逐条验真并补齐残留修复，随后把经脱敏的最终结果同步到 `/home/ubuntu/project/kcode`，最后删除 `/home/ubuntu/kcode` 与本次临时备份。

## Scope

- 源仓：`/home/ubuntu/kcode`
- 目标仓：`/home/ubuntu/tools/kcode`
- 最终同步仓：`/home/ubuntu/project/kcode`
- 文档：`issues-to-be-fixed/`
- 命令入口：`/usr/local/bin/kcode`
- 构建、测试、PTY/TUI 拟人验收

## Constraints

- Spec/Plan 固定落在 `project/kcode`。
- 临时备份只用于本轮回退；最终验收通过后必须清理。
- 全程遵循 `harness engineering guidance.md`：
  - 不接受已暴露但未打通能力
  - 不接受 UI / config / runtime 语义漂移
  - 不接受文档宣称修复但代码未闭环
- 默认覆盖工作树，不覆盖目标仓 `.git` 元数据。
- 同步到 `project/kcode` 时必须脱敏，排除 `.git`、运行态目录、会话、构建产物与敏感数据。
- `tools/kcode` 需要具备最高运行权限语义，并通过真实运行验证。

## Acceptance

1. `/home/ubuntu/tools/kcode` 已被源仓有效内容覆盖，并包含 `issues-to-be-fixed/`。
2. 13 条 issue 在目标仓中经代码、测试与真实交互链路验证后均真实修复。
3. `/usr/local/bin/kcode` 实际运行 `/home/ubuntu/tools/kcode` 构建产物。
4. `tools/kcode` 的最高运行权限语义真实生效。
5. `/home/ubuntu/project/kcode` 已完成脱敏同步并通过复验。
6. `/home/ubuntu/kcode` 已被彻底删除。
7. 本次临时备份已清理。

## Non-Goals

- 不替换目标仓与同步仓的 `.git` 身份。
- 不保留长期镜像备份。
- 不把 issue 文档中的 `fixed` 标记直接当作完成证据。
