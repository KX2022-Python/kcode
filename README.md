# Kcode

**Kcode** is a high-performance, Rust-based terminal AI agent CLI designed for developers who need persistent memory, multi-channel support, and enterprise-grade control over their AI workflows.

![Kcode Banner](docs/images/banner.png)

---

## 🌟 Key Features

- 🦀 **Rust Core**: Built entirely in Rust for maximum safety, speed, and zero dependencies.
- 🧠 **Persistent Memory**: Automatically extracts insights from your sessions and saves them as local Markdown files.
- 🌐 **Multi-Channel Bridge**: Connect to Telegram, WhatsApp, and Feishu with a single unified engine.
- 🔄 **Native 24/7 Uptime**: Run as a robust Systemd service or via Docker with auto-restart policies.
- 📷 **Rich Media Support**: Recognize and process images, documents, and voice messages from all supported channels.
- 🔒 **Enterprise Security**: Managed configurations, deny-rule filtering, and strict permission modes.
- 🚀 **Production Ready**: Comprehensive test suite (364+ tests), automated regression checks, and maintenance playbooks.

---

## 📦 Installation & Deployment

We provide two official ways to run Kcode Bridge: as a **Native Systemd Service** (Recommended for VPS) or via **Docker**.

### Option 1: Native Systemd Service (Best Performance)

This method runs Kcode directly on your host machine as a system service, providing the best performance and easiest log management.

1.  **Clone & Install**:
    ```bash
    git clone https://github.com/KX2022-Python/kcode.git
    cd kcode
    ./scripts/install.sh
    ```
    *This script builds the binary, registers the system service, and creates a template for secrets.*

2.  **Configure Secrets**:
    Edit `/etc/kcode/bridge.env` and add your keys (e.g., `KCODE_TELEGRAM_BOT_TOKEN`, `KCODE_API_KEY`).

3.  **Start Service**:
    ```bash
    sudo systemctl start kcode-bridge
    sudo systemctl status kcode-bridge
    ```
    *The service is configured with `Restart=always`, ensuring it recovers automatically from any crash.*

### Option 2: Docker Compose

Use this method if you prefer containerization or want to isolate dependencies.

1.  **Configure Environment**:
    Copy `.env.example` to `.env` and fill in your API keys.
    
2.  **Run**:
    ```bash
    docker compose up -d --build
    ```

---

## ⚡ Quick Start (CLI)

If you just want to use the interactive terminal REPL:

```bash
export KCODE_API_KEY="your-api-key"
kcode
```

### Check Health

```bash
kcode doctor
```

---

## 🌉 Multi-Channel Bridge

Kcode supports running as a bot on multiple platforms simultaneously.

### Supported Channels
| Platform | Features | Required Env Vars |
|----------|----------|-------------------|
| **Telegram** | Text, Photos, Files, Voice, Webhook | `KCODE_TELEGRAM_BOT_TOKEN` |
| **WhatsApp** | Text, Images, Audio, Docs | `KCODE_WHATSAPP_PHONE_ID`, `KCODE_WHATSAPP_TOKEN` |
| **Feishu** | Text, Images, Files, Cards | `KCODE_FEISHU_APP_ID`, `KCODE_FEISHU_APP_SECRET` |

### Webhook Configuration
To use Webhook mode (recommended for high concurrency), set:
```bash
export KCODE_WEBHOOK_URL="https://your-domain.com/webhook/telegram"
```
Kcode will automatically configure the Telegram API and listen on port `3000` for all active channels.

---

## 🧠 Memory System

Kcode treats memory as a first-class citizen:

- **Auto-Extraction**: Detects patterns, tools used, and key files automatically.
- **Conflict Resolution**: Updates existing memories instead of creating duplicates.
- **Timestamps**: Every memory file includes creation and update timestamps.
- **Privacy**: All memory files are stored locally in `~/.kcode/memory/` with `0600` permissions.

---

## 🏗️ Architecture

Kcode is organized into a modular Rust workspace:

```text
rust/crates/
├── api/              # Provider abstraction
├── bridge/           # Unified channel event system
├── commands/         # Slash command registry
├── kcode-cli/        # CLI entry point & REPL
├── plugins/          # Plugin system
├── runtime/          # Core engine (Session, Tools, Memory)
└── tools/            # 30+ built-in tools
```

For detailed deployment instructions, see [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

We welcome contributions! Please see our [Maintenance Guide](MAINTENANCE.md) and [Regression Checklist](REGRESSION.md) for development standards.
