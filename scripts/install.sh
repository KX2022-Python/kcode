#!/usr/bin/env bash
# install.sh — Install Kcode on Linux/VPS
#
# Usage:
#   ./scripts/install.sh              # Build from source and install
#   ./scripts/install.sh --binary     # Install pre-built binary only
#
# Installs to:
#   Binary:  /usr/local/bin/kcode
#   Config:  ~/.kcode/
#   Logs:    ~/.kcode/logs/
#   Sessions: ~/.kcode/sessions/

set -euo pipefail

BINARY_NAME="kcode"
INSTALL_DIR="/usr/local/bin"
KCODE_DIR="$HOME/.kcode"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[info]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[warn]${NC} $*"; }
log_error() { echo -e "${RED}[error]${NC} $*"; }

usage() {
    echo "Usage: $0 [--binary]"
    echo "  --binary    Install pre-built release binary (skip cargo build)"
    exit 0
}

BUILD_MODE="source"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary) BUILD_MODE="binary"; shift ;;
        -h|--help) usage ;;
        *) log_error "Unknown option: $1"; exit 1 ;;
    esac
done

# --- Step 1: Build or locate binary ---
if [[ "$BUILD_MODE" == "source" ]]; then
    log_info "Building $BINARY_NAME from source..."
    cd "$REPO_ROOT/rust"
    cargo build --release
    BINARY_PATH="$REPO_ROOT/rust/target/release/$BINARY_NAME"
else
    BINARY_PATH="$REPO_ROOT/rust/target/release/$BINARY_NAME"
    if [[ ! -f "$BINARY_PATH" ]]; then
        log_error "Release binary not found at $BINARY_PATH"
        log_info "Run without --binary to build from source first"
        exit 1
    fi
fi

if [[ ! -f "$BINARY_PATH" ]]; then
    log_error "Binary not found after build"
    exit 1
fi

# --- Step 2: Install binary ---
log_info "Installing $BINARY_NAME to $INSTALL_DIR..."
if [[ -w "$INSTALL_DIR" ]]; then
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
else
    log_warn "Cannot write to $INSTALL_DIR, using sudo..."
    sudo cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
fi

log_info "Installed: $(which "$BINARY_NAME" 2>/dev/null || echo "$INSTALL_DIR/$BINARY_NAME")"

# --- Step 3: Initialize directories ---
log_info "Initializing $KCODE_DIR..."
mkdir -p "$KCODE_DIR/sessions"
mkdir -p "$KCODE_DIR/logs"
mkdir -p "$KCODE_DIR/memory"
chmod 700 "$KCODE_DIR"
chmod 700 "$KCODE_DIR/memory"

# --- Step 4: Run doctor ---
log_info "Running $BINARY_NAME doctor..."
"$INSTALL_DIR/$BINARY_NAME" doctor 2>&1 || true

# --- Step 5: Post-install summary ---
echo ""
log_info "Installation complete!"
echo ""
echo "  Binary:   $INSTALL_DIR/$BINARY_NAME"
echo "  Config:   $KCODE_DIR/config.toml"
echo "  Sessions: $KCODE_DIR/sessions/"
echo "  Logs:     $KCODE_DIR/logs/"
echo "  Memory:   $KCODE_DIR/memory/"
echo ""
echo "Next steps:"
echo "  1. Edit $KCODE_DIR/config.toml and set your API key/base URL"
echo "  2. Run \`kcode doctor\` to verify connectivity"
echo "  3. Run \`kcode\` to start the REPL"
echo ""
echo "To upgrade: run \`$REPO_ROOT/scripts/upgrade.sh\`"
echo "To rollback: run \`$REPO_ROOT/scripts/rollback.sh\`"
