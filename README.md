# Kcode

**Kcode** is a high-performance, Rust-based terminal AI agent CLI designed for developers who need persistent memory, multi-channel support, and enterprise-grade control over their AI workflows.

![Kcode Banner](docs/images/banner.png)

---

## 🌟 Key Features

- 🦀 **Rust Core**: Built entirely in Rust for maximum safety, speed, and zero dependencies.
- 🧠 **Persistent Memory**: Automatically extracts insights from your sessions and saves them as local Markdown files.
- 🌐 **Multi-Channel Bridge**: Connect to Telegram, WhatsApp, and Feishu with a single unified engine.
- 🔒 **Enterprise Security**: Managed configurations, deny-rule filtering, and strict permission modes.
- 🛠️ **Rich Tool Ecosystem**: 30+ built-in tools including Bash, Git, Web Fetch, and MCP integration.
- 🚀 **Production Ready**: Comprehensive test suite (364+ tests), automated regression checks, and maintenance playbooks.

---

## 📦 Installation

### From Source (Recommended)

```bash
git clone https://github.com/KX2022-Python/kcode.git
cd kcode/rust
cargo install --path crates/kcode-cli
```

### One-Liner Install

```bash
curl -fsSL https://raw.githubusercontent.com/KX2022-Python/kcode/main/scripts/install.sh | bash
```

---

## ⚡ Quick Start

### 1. Configuration

Set your API credentials in the environment:

```bash
export KCODE_API_KEY="your-api-key"
export KCODE_MODEL="your-model"
# Optional: export KCODE_BASE_URL="your-custom-endpoint"
```

### 2. Run the REPL

```bash
kcode
```

You will enter the interactive mode. Kcode automatically loads your project context, memory files, and plugins.

### 3. Check Health

```bash
kcode doctor
```

Verifies configuration, connectivity, and permission settings.

---

## 🌉 Multi-Channel Bridge

Kcode supports running as a bot on multiple platforms simultaneously.

### Telegram Example

```bash
export KCODE_TELEGRAM_BOT_TOKEN="123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"

# Long Polling Mode (Default)
kcode bridge

# Webhook Mode (High Concurrency)
export KCODE_WEBHOOK_URL="https://your-domain.com/webhook/telegram"
kcode bridge
```

The bridge automatically isolates user sessions and persists memory per user.

---

## 🧠 Memory System

Kcode treats memory as a first-class citizen:

- **Auto-Extraction**: Detects patterns, tools used, and key files automatically.
- **Conflict Resolution**: Updates existing memories instead of creating duplicates.
- **Timestamps**: Every memory file includes creation and update timestamps.
- **Privacy**: All memory files are stored locally in `~/.kcode/memory/` with `0600` permissions.

### Manual Inspection

```bash
kcode /memory
```

Lists all loaded memories and their summaries.

---

## 🔧 Command Reference

| Command | Description |
|---------|-------------|
| `kcode` | Start interactive REPL |
| `kcode bridge` | Start multi-channel bot bridge |
| `kcode doctor` | Run environment & config diagnosis |
| `kcode init` | Initialize a new Kcode project |
| `kcode config show` | Display current configuration |
| `kcode /help` | Show available slash commands |
| `kcode /status` | Show current session status |

---

## 🏗️ Architecture

Kcode is organized into a modular Rust workspace:

```text
rust/crates/
├── api/              # Provider abstraction (OpenAI, etc.)
├── bridge/           # Unified channel event system
├── commands/         # Slash command registry
├── kcode-cli/        # CLI entry point & REPL
├── plugins/          # Plugin system
├── runtime/          # Core engine (Session, Tools, Memory)
└── tools/            # 30+ built-in tools
```

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

We welcome contributions! Please see our [Maintenance Guide](MAINTENANCE.md) and [Regression Checklist](REGRESSION.md) for development standards.
