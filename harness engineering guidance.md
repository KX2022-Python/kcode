# Harness Engineering Guidance

## 定位
这不是只为当前 7 个问题写的临时备注，而是 `kcode` 后续进入深入开发、结构性修复、能力扩展时应默认启用的一份基础工程准则。

它的目标不是鼓励继续堆功能，而是要求所有新能力、命令、工具、桥接通道、TUI 交互和配置项都通过统一的 harness 工程标准落地:

- 可发现
- 可解释
- 可执行
- 可验证
- 可回退
- 可观测

当 `kcode` 进入任何中等及以上复杂度的修改时，应把这份文档视为基础 skill 的行为约束。

## 何时启用
满足任一条件，即视为启用本准则:

- 修改 `/` 命令、TUI 菜单、命令面板、配置页
- 修改 tool registry、tool dispatch、permission policy、runtime builder
- 修改 bridge、webhook、session manager、channel adapter
- 修改 MCP、task、remote trigger、auth、memory、auto dream
- 新增对外暴露能力
- 重构现有执行路径
- 修复“表面可用但底层未打通”的问题

## 核心判断
在 `kcode` 里，harness engineering 的含义是:

- 不把命令、按钮、配置项、工具名当作 UI 文案
- 把它们当作正式系统接口
- 每一个接口都必须有真实绑定、清晰语义、失败路径、测试和观测

换句话说:

- 任何“已注册但未实现”的能力，都是 harness 缺口
- 任何“UI 有这个模式，但 runtime 没这个语义”的能力，都是 harness 缺口
- 任何“配置能写，但运行时不读取”的能力，都是 harness 缺口
- 任何“文档说能做，但 dispatch 只是 stub”的能力，都是 harness 缺口

## 不可妥协的原则

### 1. 暴露即承诺
只要某个能力出现在以下任一位置，就必须被视为对外承诺:

- slash command
- TUI 菜单
- tool spec / tool registry
- config key
- bridge env
- README / deployment 文档

如果还不能兑现，就必须:

- 隐藏
- 明确标记 experimental
- 或直接移除

禁止继续维持“看起来可用、实际是 stub”的状态。

### 2. 一份语义，只能有一套真源
以下对象必须共享同一个权威语义源，而不能各自解释:

- permission mode
- plan mode
- effort
- auto dream
- model metadata
- MCP server state
- task lifecycle state

严禁出现:

- CLI 一套枚举
- TUI 一套枚举
- config 一套字符串
- runtime 再偷偷折叠成另一套语义

### 3. 菜单不是皮肤，是操作 harness
`/` 命令面板、层级菜单、模型说明、权限说明，不是“以后再做的 UX 优化”，而是能力是否可正确使用的一部分。

任何高频命令都必须满足:

- 用户能找到
- 用户能理解何时该用
- 用户能看懂切换的后果
- 用户能直接完成常见操作

### 4. 配置项必须闭环
新增或保留任何 config key 时，必须回答 4 个问题:

1. 谁写它
2. 谁读它
3. 何时生效
4. 如何验证它确实生效

只写不读、只读不展示、只展示不生效，都不合格。

### 5. 失败必须是正式路径
每个能力都必须有明确失败语义，而不是靠模糊文案顶过去。

要求至少定义:

- 输入非法时怎么报错
- 依赖缺失时怎么报错
- 网络失败时怎么报错
- 权限不够时怎么报错
- 目标未配置时怎么报错
- 降级路径是什么

禁止“返回一个看起来成功的 JSON，但里面其实什么都没做”。

## 设计约束

### 命令与工具
新增或修改 `/` 命令、tool 时，必须同时具备:

- spec
- registry entry
- runtime binding
- clear help text
- at least one integration-style test

如果只有 parse/registry，没有 runtime binding，不得交付。

### TUI 与状态
TUI 中所有模式、菜单、状态标签都必须映射到真实后端状态。

禁止:

- 仅前端展示的 mode
- 仅 header 可见的状态词
- 仅 footer 可见但不会影响行为的切换

### Bridge 与通道
bridge/channel 相关能力必须区分两层:

- 本地执行能力
- 对外接入能力

文档必须明确说明:

- 是否只提供本地 receiver
- 是否需要公网入口
- 是否需要 reverse proxy
- 是否支持 polling fallback

### Auto 能力
像 auto dream、auto memory、auto compact 这样的自动能力，必须满足:

- 默认策略清晰
- 开关存在
- 状态可见
- 最近一次执行可见
- 为什么没有触发可解释

## 推荐开发流程

### 第一步：先判定是“能力问题”还是“harness 问题”
每次改动前先判断问题属于哪类:

- 底层根本没实现
- 已实现但没暴露
- 已暴露但没打通
- 已打通但不可发现
- 已可发现但不可理解
- 已可理解但不可验证

如果是后 5 类，优先按 harness 问题处理，不要急着继续加功能。

### 第二步：先收语义，再改 UI
先明确:

- 这个能力的唯一真源在哪
- 哪些输入合法
- 哪些状态存在
- 哪些状态可转移
- 哪些失败是预期失败

然后再改:

- slash menu
- TUI 标签
- 配置页
- 文案

### 第三步：把“声明层”与“执行层”同时改完
任何一项能力改动都至少涉及两层:

- 声明层
  - spec
  - menu
  - help
  - config
  - docs
- 执行层
  - runtime
  - dispatch
  - adapter
  - persistence
  - transport

只改一层不改另一层，视为未完成。

### 第四步：加验收，不只加单测
优先补这几类测试:

- parse + registry test
- runtime behavior test
- state persistence test
- end-to-end user path test
- downgrade / fallback test

如果是桥接或通道功能，还应补:

- fake server / fake webhook / fake MCP server test

## 交付前检查清单
任何中等以上 `kcode` 改动，在交付前都应自检:

- 命令是否只是“能解析”，还是真能执行
- 菜单是否只是“能看到”，还是真能完成操作
- 配置是否只是“能写入”，还是真能驱动运行时
- 状态是否只是“能显示”，还是真来自后端
- 错误是否只是“有一句文案”，还是真有明确失败语义
- 测试是否只覆盖 happy path
- 文档是否仍在过度承诺

## 当前最容易反复出现的反模式

- 注册一个命令，但主流程里返回 `not yet implemented`
- 定义一个 tool spec，但 dispatch 返回 stub JSON
- 增加一个配置键，但 runtime 根本不读
- TUI 做出一个 mode，但 CLI/runtime 没这个概念
- README 宣称支持，但部署链路还要用户自己猜
- 为了先交付，继续往特例分支里塞逻辑，而不是重建统一模型

## 对当前问题集的指导意义
以下问题都应按本准则处理，而不是逐个打补丁:

- `/effort` 未打通
- `task / mcp auth / remote trigger` stub
- Telegram webhook 对外接入语义不清
- `plan-mode` 语义碎裂
- `auto dream` 无入口且无真实配置闭环
- `/` 菜单缺少层级化与全局过滤并存的结构
- `/` 菜单缺少上下文说明和直接操作能力

这些问题共同说明:
`kcode` 当前不是单纯“缺功能”，而是 harness 工程还没有完全收口。

## 未来默认执行要求
从现在开始，只要进入 `kcode` 的深入开发，应默认遵守:

1. 不新增伪能力
2. 不保留未打通但继续对外暴露的接口
3. 不允许 UI、config、runtime 三套语义继续漂移
4. 任何新增能力必须同时补齐说明、执行、测试、观测
5. 任何复杂修复优先做统一模型，再做局部补丁

## 建议的后续形态
这份文档目前是项目内准则文件。后续如果要正式纳入 agent 工作流，建议再升级为:

- 项目级 `SKILL.md`
- 或 `kcode-harness-engineering` 基础 skill

升级后应包含:

- 标准修改流程
- 必检清单
- 常见反模式
- 典型修复模板
- 验收模板

在那之前，这份文档就是 `kcode` 深入开发时的默认行为准则。
