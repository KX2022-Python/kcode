## 标题
Telegram webhook 已有本地接收器，但公网接入仍依赖外部 server / 反向代理

## 状态
已修复（2026-04-06）

## 修复结果
- `KCODE_WEBHOOK_URL` 现在会被明确校验为 Telegram 专用的公网 HTTPS URL；把它指向 `localhost`、`127.0.0.1`、非 HTTPS，或不匹配 `/webhook/telegram` 路径时，bridge 启动前就会给出明确错误。
- 如果设置了 `KCODE_WEBHOOK_URL` 但没有配置 Telegram token，现在也会直接报配置错误，避免误以为这个 setting 对 WhatsApp / Feishu 生效。
- bridge 启动时会明确打印当前 Telegram 处于 `Webhook` 还是 `Long Polling`，以及 webhook 模式下“只内建本地 receiver，仍需你自己提供公网 HTTPS 入口”的部署要求和 polling fallback。
- README 与 DEPLOYMENT 文档已同步收紧，不再给出“只要设置 URL 就开箱即用”的暗示。

## 验证
- 安装版 `/home/ubuntu/kcode`
  - `cargo test -p adapters rejects_non_public_telegram_webhook_urls -- --nocapture`
  - `cargo test -p adapters rejects_webhook_url_without_telegram_channel -- --nocapture`
  - `cargo test -p kcode-cli bridge_session_dir_lives_under_config_home -- --nocapture`
- 开发源码 `/home/ubuntu/project/kcode`
  - `cargo test -p adapters rejects_non_public_telegram_webhook_urls -- --nocapture`
  - `cargo test -p adapters rejects_webhook_url_without_telegram_channel -- --nocapture`
  - `cargo test -p kcode-cli bridge_session_dir_lives_under_config_home -- --nocapture`

## 原始说法
3. telegram webhook 还需要外部 server 配合，而不是像 openclaw 那样的机制。

## 结论
这个说法大体成立，但更准确的表述应是:

`kcode` 已经内置了本地 webhook HTTP server，也能调用 Telegram `setWebhook`；但是它没有提供类似 OpenClaw gateway / tunnel / relay 的“公网入口托管机制”。因此在实际部署时，仍然需要你自己提供公网 HTTPS 可达地址，通常靠开放端口、域名和反向代理来完成。

## 证据
- Telegram transport 明确把 webhook 模式标注为“requires public HTTPS endpoint”，见 [telegram_transport.rs](/home/ubuntu/kcode/rust/crates/adapters/src/telegram_transport.rs#L25)
- `set_webhook()` 确实已经实现，会调用 Telegram Bot API 的 `setWebhook`，见 [telegram_transport.rs](/home/ubuntu/kcode/rust/crates/adapters/src/telegram_transport.rs#L87)
- 但 transport 自己的 `run()` 对 webhook 模式直接报错:
  - `"Webhook mode requires external HTTP server on port {}"`
  - 见 [telegram_transport.rs](/home/ubuntu/kcode/rust/crates/adapters/src/telegram_transport.rs#L155)
- `kcode` bridge 侧确实内置了本地 axum server，并监听:
  - `/webhook/telegram`
  - `/webhook/whatsapp`
  - `/webhook/feishu`
  - 见 [webhook_server.rs](/home/ubuntu/kcode/rust/crates/adapters/src/webhook_server.rs#L43)
- bridge service 会把 Telegram 配成 `Webhook { url, port: 3000 }`，并绑定 `0.0.0.0:3000` 启动本地 webhook server，见 [bridge_core.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/bridge_core.rs#L243) 和 [bridge_core.rs](/home/ubuntu/kcode/rust/crates/kcode-cli/src/bridge_core.rs#L319)
- README 明确要求设置:
  - `KCODE_WEBHOOK_URL="https://your-domain.com/webhook/telegram"`
  - 见 [README.md](/home/ubuntu/kcode/README.md#L133)
- 部署文档明确要求你自己保证公网入口:
  - 打开 `3000` 端口，或者
  - 用 Nginx/Caddy 把 `/webhook/*` 反代到 `http://localhost:3000`
  - 见 [DEPLOYMENT.md](/home/ubuntu/kcode/docs/DEPLOYMENT.md#L96)

## 影响
- 如果没有公网 HTTPS URL，Telegram webhook 模式无法工作。
- 当前 bridge 更像“本地 webhook receiver + 手动公网暴露”，不是“自带对外接入机制”。
- 用户对“像 OpenClaw 一样开箱即用接收 webhook”会产生错误预期。

## 建议修复方向
- 如果目标是接近 OpenClaw 体验，需要补一层 ingress 方案:
  - 内建 tunnel / relay
  - 或提供受控的 pair / gateway 注册机制
- 如果暂时不做这层，就把产品表述收紧成:
  - “内置本地 webhook server，但部署 webhook 模式仍需外部公网入口”
- 为 bridge 启动流程补更清晰的诊断:
  - 未设置公网 URL 时建议切 polling
  - 设置了 webhook URL 但本地不可达时给出明确错误和检查项
