# Kcode Mirror Repair Sync Plan

1. 在 `project/kcode` 固化 Spec/Plan，并建立临时回退点。
2. 检查 `tools/kcode` 三个目标侧独有文件是否仍需吸收；否则按镜像残留删除。
3. 以 `/home/ubuntu/kcode` 为源覆盖 `/home/ubuntu/tools/kcode` 工作树，纳入 `issues-to-be-fixed/`，保留目标仓 `.git`。
4. 在覆盖后的 `tools/kcode` 中逐条验真 13 个 issue，并修补所有未闭环问题。
5. 构建 `tools/kcode`，接管 `/usr/local/bin/kcode`，落实最高运行权限语义。
6. 用真实 PTY/TUI 模拟用户完成关键验收路径。
7. 对 `tools/kcode` 做脱敏扫描并同步到 `project/kcode`。
8. 复验同步结果后删除 `/home/ubuntu/kcode`。
9. 清理本次临时备份与运行垃圾，输出最终证据与残余风险。
