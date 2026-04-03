#!/usr/bin/env bash
# rollback.sh — Rollback Kcode to previous version
#
# Usage:
#   ./scripts/rollback.sh              # Rollback to most recent backup
#   ./scripts/rollback.sh <timestamp>  # Rollback to specific backup (e.g. 20260403-193000)

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

# --- Step 1: Find backup ---
if [[ $# -ge 1 ]]; then
    BACKUP_PATH="$BACKUP_DIR/$BINARY_NAME-$1"
else
    BACKUP_PATH="$(ls -t "$BACKUP_DIR"/$BINARY_NAME-* 2>/dev/null | head -1)"
fi

if [[ -z "${BACKUP_PATH:-}" ]] || [[ ! -f "$BACKUP_PATH" ]]; then
    log_error "No backup found"
    echo ""
    echo "Available backups:"
    ls -1 "$BACKUP_DIR"/$BINARY_NAME-* 2>/dev/null || echo "  (none)"
    exit 1
fi

# --- Step 2: Restore ---
log_info "Rolling back to: $(basename "$BACKUP_PATH")"
cp "$BACKUP_PATH" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# --- Step 3: Verify ---
log_info "Restored version:"
"$INSTALL_DIR/$BINARY_NAME" version

log_info "Rollback complete"
