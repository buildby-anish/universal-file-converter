#!/usr/bin/env bash
#
# Uninstall Universal File Converter (ufc) and its macOS Quick Action
#
set -euo pipefail

log() { echo "==> $*"; }

log "Uninstalling UFC..."

# Remove binaries
for bin_path in \
    "/opt/homebrew/bin/ufc" \
    "/usr/local/bin/ufc" \
    "$HOME/.local/bin/ufc" \
    "$HOME/.cargo/bin/ufc"; do
    if [ -f "$bin_path" ] || [ -L "$bin_path" ]; then
        rm -f "$bin_path"
        log "Removed $bin_path"
    fi
done

# Remove macOS Quick Action / Service workflow
SERVICE_PATH="$HOME/Library/Services/Convert with UFC.workflow"
if [ -d "$SERVICE_PATH" ]; then
    rm -rf "$SERVICE_PATH"
    log "Removed $SERVICE_PATH"
    /System/Library/CoreServices/pbs -flush 2>/dev/null || true
fi

# Clean cache directory if present
CACHE_DIR="$HOME/.cache/ufc"
if [ -d "$CACHE_DIR" ]; then
    rm -rf "$CACHE_DIR"
    log "Removed $CACHE_DIR"
fi

log "UFC has been completely uninstalled from your system."
