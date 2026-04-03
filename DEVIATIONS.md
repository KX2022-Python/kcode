# Kcode 参考源偏差记录

本文件记录 Kcode 与各参考源之间的偏差，确保高保真目标不会在长期迭代中逐渐漂移。

## 1. 偏差分类

| 类型 | 说明 |
|------|------|
| `INTENTIONAL` | 有意偏差（去 Claude 化、独立运行等） |
| `DEFERRED` | 延期实现（v1.1/v1.2 计划） |
| `GAP` | 尚未实现的官方能力 |
| `ENHANCEMENT` | Kcode 增强（超出官方但保持兼容） |

## 2. Bridge / Theme / MCP 偏差

### 2.1 Bridge

| 偏差项 | 类型 | 说明 | 参考源 |
|--------|------|------|--------|
| `kcode-bridge` 抽象层 | ENHANCEMENT | 官方无独立 bridge crate，Kcode 独立为 `crates/bridge/` | CC source Map |
| loopback adapter | ENHANCEMENT | 官方通过真实渠道验证，Kcode 提供进程内模拟 | CC source Map |
| bridge-safe command policy | INTENTIONAL | 官方命令面较宽，Kcode 默认收窄 | CC source Map §14 |

### 2.2 Render Theme

| 偏差项 | 类型 | 说明 | 参考源 |
|--------|------|------|--------|
| `RenderIntent` 语义层 | ENHANCEMENT | 官方渲染更直接，Kcode 增加统一语义抽象层 | CC source Map |
| `NO_COLOR` 支持 | ENHANCEMENT | 官方默认带颜色输出，Kcode 显式支持无颜色降级 | 行业标准 |
| 主题可切换性 | DEFERRED | 当前只有默认主题，v1.2+ 支持多主题 | CC source Map |

### 2.3 MCP

| 偏差项 | 类型 | 说明 | 参考源 |
|--------|------|------|--------|
| `McpRegistryAssembler` | ENHANCEMENT | 官方 MCP 配置较扁平，Kcode 增加多来源仲裁层 | CC source Map §19 |
| 去 Anthropic MCP | INTENTIONAL | 不依赖 Claude.ai connectors 和 Anthropic registry | KCODE.md |
| signature dedup | ENHANCEMENT | 官方按名称去重，Kcode 按底层连接内容去重 | CC source Map §19 |

## 3. 命令面偏差

### 3.1 已实现的官方命令

| 命令 | 官方状态 | Kcode 状态 | 偏差类型 |
|------|---------|-----------|---------|
| `/help` | ✅ | ✅ | — |
| `/compact` | ✅ | ✅ | — |
| `/memory` | ✅ | ✅ | — |
| `/mcp` | ✅ | ✅ | — |
| `/model` | ✅ | ✅ | — |
| `/permissions` | ✅ | ✅ | — |
| `/tasks` | ✅ | ✅ | — |
| `/status` | ✅ | ✅ | — |
| `/config` | ✅ | ✅ | — |
| `/resume` | ✅ | ✅ | — |
| `/init` | ✅ | ✅ | — |
| `/diff` | ✅ | ✅ | — |
| `/version` | ✅ | ✅ | — |
| `/doctor` | ENHANCEMENT | ✅ | ENHANCEMENT |

### 3.2 未实现的官方命令

| 命令 | 偏差类型 | 原因 | 计划版本 |
|------|---------|------|---------|
| `/vim` | INTENTIONAL | 与 Kcode 终端风格不符 | 不考虑 |
| `/voice` | INTENTIONAL | 依赖语音基础设施 | 不考虑 |
| `/plan` / `/ultraplan` | DEFERRED | 远程审批链路依赖 CCR | v1.2+ |
| `/review` | DEFERRED | 官方已 deprecated | 视需求 |
| `/teleport` | INTENTIONAL | 依赖远程 web 审批 | 不考虑 |
| `/agents` | ENHANCEMENT | Kcode 改为 `/tasks` | v1.2+ |

## 4. 工具偏差

### 4.1 官方有、Kcode 已实现的工具

bash, read_file, write_file, edit_file, glob_search, grep_search,
WebFetch, WebSearch, TodoWrite, Skill, Agent, ToolSearch, NotebookEdit,
Sleep, SendUserMessage, Config, EnterPlanMode, ExitPlanMode,
StructuredOutput, REPL, PowerShell, AskUserQuestion, TaskCreate/Get/List/Stop/Update/Output,
TeamCreate/Delete, CronCreate/Delete/List, LSP, MCP 相关工具,
TestingPermission, **WebBrowser** (Phase 14 新增)

### 4.2 官方有、Kcode 未实现的工具

| 工具 | 偏差类型 | 原因 | 计划版本 |
|------|---------|------|---------|
| `WebBrowser` 完整实现 | GAP | 当前为骨架，需外部浏览器 executor | v1.2+ |
| `Monitor` | DEFERRED | 监控功能非核心需求 | 待定 |
| `Workflow` | DEFERRED | 工作流编排需额外基础设施 | 待定 |

## 5. 结构偏差

| 偏差项 | 类型 | 说明 | 官方参考 |
|--------|------|------|---------|
| Workspace crate 结构 | INTENTIONAL | 官方单 crate，Kcode 拆为 9 个 crate | claw-code rust/ |
| 去 OAuth 认证 | INTENTIONAL | 改为 KCODE_* 环境变量驱动 | CC source Map §auth |
| session persistence | ENHANCEMENT | 官方用 JSONL，Kcode 同时支持 JSON | CC source Map §session |

## 6. 偏差维护规则

- 新增功能时必须评估偏差类型并更新本文件
- 每完成一个 Phase 后回顾偏差项是否仍然成立
- 被标记为 `GAP` 的项超过两个版本未填补时，评估是否转为 `DEFERRED` 或移除
- 被标记为 `INTENTIONAL` 的项不得在后续版本中无意中消除
