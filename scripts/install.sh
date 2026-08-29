#!/usr/bin/env bash
#
# Install ufc on macOS or Linux.
#
# By default, tries to download a prebuilt binary from the repo's latest
# GitHub Release (published by .github/workflows/release.yml). Falls back
# to building from source with cargo if no matching release asset is
# found (e.g. no releases published yet, or --from-source was passed).
#
# Safe to re-run (idempotent overwrite of the installed binary).
set -euo pipefail

# EDIT THIS after you push to GitHub, so the prebuilt-binary path can find
# your release assets: "yourname/universal-file-converter".
UFC_REPO="${UFC_REPO:-CHANGEME/universal-file-converter}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${UFC_INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="ufc"
FORCE_SOURCE=false
[ "${1:-}" = "--from-source" ] && FORCE_SOURCE=true

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os-$arch" in
        Linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
        Darwin-x86_64)  echo "x86_64-apple-darwin" ;;
        Darwin-arm64)   echo "aarch64-apple-darwin" ;;
        *) echo "" ;;
    esac
}

try_prebuilt() {
    local target="$1"
    [ -z "$target" ] && return 1
    [ "$UFC_REPO" = "CHANGEME/universal-file-converter" ] && return 1
    command -v curl >/dev/null 2>&1 || return 1

    local asset="ufc-${target}.tar.gz"
    local url="https://github.com/${UFC_REPO}/releases/latest/download/${asset}"
    echo "==> Trying prebuilt binary: $url"

    local tmp
    tmp="$(mktemp -d)"
    if ! curl -fsSL -o "$tmp/$asset" "$url" 2>/dev/null; then
        rm -rf "$tmp"
        return 1
    fi

    tar -xzf "$tmp/$asset" -C "$tmp"
    local extracted
    extracted="$(find "$tmp" -type f -name "$BIN_NAME" | head -n1)"
    if [ -z "$extracted" ]; then
        rm -rf "$tmp"
        return 1
    fi

    mkdir -p "$INSTALL_DIR"
    install -m 755 "$extracted" "$INSTALL_DIR/$BIN_NAME"
    rm -rf "$tmp"
    echo "==> Installed prebuilt $BIN_NAME to $INSTALL_DIR/$BIN_NAME (no Rust toolchain needed)"
    return 0
}

build_from_source() {
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo not found on PATH." >&2
        echo "Install Rust first: https://rustup.rs (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)" >&2
        exit 1
    fi

    echo "==> Building ufc from source (release) ..."
    ( cd "$REPO_ROOT" && cargo build --release --workspace )

    local built_bin="$REPO_ROOT/target/release/$BIN_NAME"
    if [ ! -f "$built_bin" ]; then
        echo "error: expected binary not found at $built_bin" >&2
        exit 1
    fi

    mkdir -p "$INSTALL_DIR"
    install -m 755 "$built_bin" "$INSTALL_DIR/$BIN_NAME"
    echo "==> Installed $BIN_NAME to $INSTALL_DIR/$BIN_NAME"
}

TARGET="$(detect_target)"
if [ "$FORCE_SOURCE" = false ] && try_prebuilt "$TARGET"; then
    : # prebuilt install succeeded
else
    [ "$FORCE_SOURCE" = false ] && echo "==> No prebuilt binary available, falling back to source build."
    build_from_source
fi

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
