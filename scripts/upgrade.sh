#!/usr/bin/env bash
# upgrade.sh — Upgrade Kcode to latest version
#
# Usage:
#   ./scripts/upgrade.sh              # Pull and build latest from kcode-base branch
#   ./scripts/upgrade.sh <commit>     # Upgrade to specific commit
#
# Creates a backup before upgrading.

set -euo pipefail

BINARY_NAME="kcode"
INSTALL_DIR="/usr/local/bin"
BACKUP_DIR="$HOME/.kcode/backups"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[info]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[warn]${NC} $*"; }
log_error() { echo -e "${RED}[error]${NC} $*"; }

# --- Step 1: Backup current binary ---
mkdir -p "$BACKUP_DIR"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
BACKUP_PATH="$BACKUP_DIR/$BINARY_NAME-$TIMESTAMP"

if [[ -f "$INSTALL_DIR/$BINARY_NAME" ]]; then
    cp "$INSTALL_DIR/$BINARY_NAME" "$BACKUP_PATH"
    log_info "Backed up current binary → $BACKUP_PATH"
else
    log_warn "No existing binary found, skipping backup"
fi

# --- Step 2: Pull latest ---
log_info "Pulling latest from kcode-base..."
cd "$REPO_ROOT"
git fetch origin kcode-base
if [[ $# -ge 1 ]]; then
    git checkout "$1"
else
    git checkout origin/kcode-base
fi

# --- Step 3: Build ---
log_info "Building..."
cd "$REPO_ROOT/rust"
cargo build --release

# --- Step 4: Install ---
log_info "Installing..."
cp "$REPO_ROOT/rust/target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# --- Step 5: Verify ---
log_info "Installed version:"
"$INSTALL_DIR/$BINARY_NAME" version

log_info "Upgrade complete. Backup retained at $BACKUP_PATH"
echo ""
echo "To rollback: $REPO_ROOT/scripts/rollback.sh"
