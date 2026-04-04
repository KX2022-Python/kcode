# Kcode 回归检查单

每次发布前必须通过的检查项。

## 1. 编译验证

```bash
cd rust && cargo check 2>&1 | grep -E "error|warning"
# 期望：零错误零警告
```

## 2. 测试验证

```bash
cd rust && cargo test --workspace 2>&1 | grep "test result"
# 期望：所有 test result 均为 ok
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
| `kcode init` | 运行命令 | 创建配置和目录 |
| `kcode config show` | 运行命令 | 显示配置 |
| `kcode resume` | 运行命令 | 显示可恢复会话 |
| `kcode /help` | 运行后输入 | 显示帮助 |
| `kcode /memory` | 运行后输入 | 显示 memory 状态 |
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
