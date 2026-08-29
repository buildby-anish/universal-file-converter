#!/usr/bin/env bash
#
# Build ufc in release mode and install the binary onto PATH.
# Works on macOS and Linux. Safe to re-run (idempotent overwrite of the
# installed binary).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${UFC_INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="ufc"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found on PATH." >&2
    echo "Install Rust first: https://rustup.rs (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)" >&2
    exit 1
fi

echo "==> Building ufc (release) ..."
( cd "$REPO_ROOT" && cargo build --release --workspace )

BUILT_BIN="$REPO_ROOT/target/release/$BIN_NAME"
if [ ! -f "$BUILT_BIN" ]; then
    echo "error: expected binary not found at $BUILT_BIN" >&2
    exit 1
fi

mkdir -p "$INSTALL_DIR"
install -m 755 "$BUILT_BIN" "$INSTALL_DIR/$BIN_NAME"
echo "==> Installed $BIN_NAME to $INSTALL_DIR/$BIN_NAME"

case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        echo "==> $INSTALL_DIR is already on PATH."
        ;;
    *)
        SHELL_RC=""
        case "${SHELL:-}" in
            */zsh)  SHELL_RC="$HOME/.zshrc" ;;
            */bash) SHELL_RC="$HOME/.bashrc" ;;
            *)      SHELL_RC="$HOME/.profile" ;;
        esac
        echo ""
        echo "NOTE: $INSTALL_DIR is not on your PATH yet."
        echo "Add this line to $SHELL_RC, then restart your terminal:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac

echo "==> Done. Verify with: $BIN_NAME routes"

if ! command -v soffice >/dev/null 2>&1 && ! command -v libreoffice >/dev/null 2>&1; then
    echo ""
    echo "NOTE: LibreOffice not found — docx/odt/pdf-from-office routes will be"
    echo "unavailable until it's installed (image and PDF-text routes work regardless)."
    if [ "$(uname)" = "Darwin" ]; then
        echo "  macOS: brew install --cask libreoffice"
    else
        echo "  Debian/Ubuntu: sudo apt install libreoffice"
        echo "  Fedora:        sudo dnf install libreoffice"
        echo "  Arch:          sudo pacman -S libreoffice-fresh"
    fi
fi
