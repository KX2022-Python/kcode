# Kcode Deployment Guide

This guide covers how to deploy Kcode Bridge as a production-grade, 24/7 running service on your VPS or server.

## Option 1: Native Systemd Service (Recommended)

This method runs Kcode directly on your host machine as a system service, providing the best performance and easiest log management.

### 1. Run the Installer

The installation script automatically builds the binary, registers the Systemd service, and creates a configuration template.

```bash
./scripts/install.sh
```

### 2. Configure Secrets

Edit the generated environment file:
```bash
sudo nano /etc/kcode/bridge.env
```

Add your credentials (do not quote the values):
```ini
# Required
KCODE_TELEGRAM_BOT_TOKEN=your_bot_token_here
KCODE_API_KEY=your_api_key
KCODE_MODEL=your_model

# Optional: WhatsApp
# KCODE_WHATSAPP_PHONE_ID=...
# KCODE_WHATSAPP_TOKEN=...
# KCODE_WHATSAPP_APP_SECRET=...

# Optional: Feishu
# KCODE_FEISHU_APP_ID=...
# KCODE_FEISHU_APP_SECRET=...
```

### 3. Start and Monitor

```bash
# Start the service
sudo systemctl start kcode-bridge

# Check status
sudo systemctl status kcode-bridge

# View logs in real-time
journalctl -u kcode-bridge -f
```

## Option 2: Docker Compose

Use this method if you prefer containerization or want to isolate dependencies.

### 1. Prerequisites

Ensure you have Docker and Docker Compose installed.

### 2. Configure Environment

Create a `.env` file in the repository root:
```bash
cp .env.example .env
nano .env
```

Add your keys to `.env`.

### 3. Start the Container

```bash
docker compose up -d --build
```

### 4. View Logs

```bash
docker compose logs -f kcode-bot
```

## Rich Media Support

Kcode Bridge now supports receiving and processing:
*   📷 **Images**: Photos from Telegram, WhatsApp, Feishu.
*   📄 **Files**: Documents and generic files.
*   🎤 **Voice**: Audio messages (transcription depends on AI model capabilities).

When a user sends a media file, Kcode passes the metadata and placeholder text (e.g., `[Received a photo]`) to the AI model's context.

## Troubleshooting

*   **Service fails to start**: Check logs with `journalctl -u kcode-bridge -e`. Ensure `/etc/kcode/bridge.env` has valid syntax (no spaces around `=`).
*   **Webhook not receiving messages**: Kcode only hosts the local receiver on port `3000`; it does not provide a managed public ingress. Ensure your server's port `3000` is open and, if using a domain, that your reverse proxy (Nginx/Caddy) forwards `/webhook/*` to `http://localhost:3000`. If you cannot provide public HTTPS ingress, remove `KCODE_WEBHOOK_URL` and use Telegram Long Polling instead.
