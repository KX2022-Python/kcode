# Kcode 维护手册

本手册定义 Kcode 的长期维护机制，目标是稳定、低风险地迭代。

## 1. Release 节奏

### 1.1 分支模型

| 分支 | 用途 | 保护策略 |
|------|------|---------|
| `kcode-base` | 主线开发分支，所有 PR 合并到此 | 合并前必须通过全量测试 |
| `upstream/main` | 上游 `claw-code-parity` 的远程跟踪 | 只读，不直接修改 |
| `upstream-import/2026-04-03` | 初始导入快照 | 冻结，不再变更 |
| `main` | 稳定发布分支 | 从 `kcode-base` 打 tag 后同步 |

### 1.2 发布周期

- 功能开发在 `kcode-base` 上持续进行
- 每个 Phase 完成后在 `kcode-base` 上打 tag（如 `v1.0-phase9`）
- `main` 分支仅在 `v1.0` / `v1.1` / `v1.2` 等里程碑版本时从 `kcode-base` 同步

### 1.3 版本号规则

- `v<major>.<minor>.<patch>`
- Phase 0-12 → v1.0
- Phase 13 → v1.1
- Phase 14-15 → v1.2

## 2. 上游同步窗口

### 2.1 同步策略

上游 `claw-code-parity` 只在以下时机同步：

1. **定向 cherry-pick**：只选取对 Kcode 有明确价值的单个 commit
2. **定期比对**：每月检查一次 upstream/main 与 kcode-base 的差异

### 2.2 同步边界

| 允许同步 | 禁止同步 |
|---------|---------|
| 安全补丁 | 整个 feature 分支 |
| 已知 bug 的修复 | 品牌相关改动 |
| 性能优化 | Claude/Anthropic 绑定代码 |
| 新 tool spec 定义 | OAuth/登录相关代码 |

### 2.3 同步流程

```bash
# 1. 获取上游最新
git fetch upstream main

# 2. 检查差异
git log kcode-base..upstream/main --oneline

# 3. 按需 cherry-pick
git cherry-pick <commit-hash>

# 4. 解决冲突后测试
cargo test --workspace
```

## 3. 升级准则

### 3.1 从当前版本升级

```bash
./scripts/upgrade.sh          # 从 kcode-base 拉取最新并构建
./scripts/upgrade.sh <commit> # 升级到指定 commit
```

### 3.2 回滚

```bash
./scripts/rollback.sh          # 回滚到上一个备份版本
./scripts/rollback.sh <date>   # 回滚到指定日期的备份
```

### 3.3 升级前检查清单

- [ ] `cargo check` 零错误
- [ ] `cargo test` 全通过
- [ ] `kcode doctor` 在目标环境运行正常
- [ ] 备份当前二进制（upgrade.sh 自动执行）

## 4. 变更日志

每次 release 在 `docs/CHANGELOG.md` 中记录变更。格式：

```markdown
## v1.0 — 2026-04-XX

### Added
- ...

### Changed
- ...

### Fixed
- ...
```

## 5. 文档维护

| 文档 | 位置 | 更新时机 |
|------|------|---------|
| KCODE.md | `/home/ubuntu/memos/KCODE.md` | 每个 Phase 完成时更新进度 |
| Phase Spec | `/home/ubuntu/memos/KCODE-*-Spec.md` | Phase 设计阶段创建，完成后归档 |
| 维护手册 | 本文件 | 维护机制变更时更新 |
| 回归检查单 | `REGRESSION.md` | 新增 Phase 时补充测试项 |
| 偏差记录 | `DEVIATIONS.md` | 发现与参考源偏差时记录 |

## 6. 代码质量标准

- 单文件默认 ≤ 500 行，超过必须拆分
- `cargo check` 零 warning
- 所有 crate 测试通过
- 禁止硬编码 API 密钥、token 等敏感信息
- 所有工具/命令的添加必须附带测试
