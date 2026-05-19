#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "🛠️ Installing Kcode Systemd Service..."

# 1. Build frontend
if command -v npm >/dev/null 2>&1; then
    echo "🖥️ Building TypeScript TUI..."
    npm --prefix "$REPO_ROOT/tui" install --include=dev --package-lock=false
    npm --prefix "$REPO_ROOT/tui" run build
else
    echo "❌ npm is required to build the TypeScript TUI." >&2
    exit 1
fi

# 2. Install Binary
echo "📦 Installing binary..."
cd "$REPO_ROOT/rust"
cargo build --release -p kcode-cli
sudo mkdir -p /usr/local/bin
sudo install -m 755 "$REPO_ROOT/rust/target/release/kcode" /usr/local/bin/kcode
sudo install -m 755 "$REPO_ROOT/rust/target/release/kcode" /usr/local/bin/kcode-engine

# 3. Install frontend bundle
echo "🧩 Installing TypeScript TUI bundle..."
sudo mkdir -p /usr/local/lib/kcode/tui/dist
sudo install -m 644 "$REPO_ROOT/tui/dist/index.js" /usr/local/lib/kcode/tui/dist/index.js

# 4. Install Service
echo "🔧 Registering kcode-bridge.service..."
sudo cp "$REPO_ROOT/deploy/kcode-bridge.service" /etc/systemd/system/

# 5. Create Env File if not exists
if [ ! -f /etc/kcode/bridge.env ]; then
    echo "📝 Creating /etc/kcode/bridge.env template..."
    sudo mkdir -p /etc/kcode
    sudo bash -c 'cat > /etc/kcode/bridge.env << EOF
KCODE_API_KEY=your_key
KCODE_TELEGRAM_BOT_TOKEN=your_token
# KCODE_WEBHOOK_URL=https://your-domain.com
EOF'
fi

# 6. Reload and Enable
sudo systemctl daemon-reload
sudo systemctl enable kcode-bridge

echo "✅ Installation complete!"
echo "👉 Edit /etc/kcode/bridge.env with your secrets."
echo "🚀 Run: sudo systemctl start kcode-bridge"
