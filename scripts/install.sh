#!/usr/bin/env bash
#
# One-shot installer for ufc on macOS or Linux.
#
# Does everything in a single run:
#   1. Bootstraps Rust via rustup if `cargo` isn't already on PATH.
#   2. Installs LibreOffice via the system package manager if `soffice`
#      isn't already on PATH (needed for docx/odt/pdf-from-office routes).
#   3. Installs ufc itself: tries a prebuilt binary from the repo's latest
#      GitHub Release first, falls back to `cargo build --release`.
#
# You do not need to install anything yourself beforehand — just run this
# script. It will ask for your sudo/admin password only if it needs to
# install LibreOffice through your OS's package manager (apt/dnf/pacman on
# Linux, brew on macOS) — that's the OS's own prompt, not this script
# collecting credentials.
#
# Safe to re-run (idempotent; skips anything already installed).
set -euo pipefail

# EDIT THIS after you push to GitHub, so the prebuilt-binary path (and the
# standalone/curled source-build fallback) can find your repo:
# "yourname/universal-file-converter".
UFC_REPO="${UFC_REPO:-buildby-anish/universal-file-converter}"

# Resolve REPO_ROOT only if this script is actually running from inside a
# cloned checkout (i.e. ../Cargo.toml exists next to it). When the script
# is instead piped straight from curl — `curl ... | bash` — there is no
# on-disk checkout yet, so REPO_ROOT stays empty and build_from_source()
# clones one on demand via ensure_repo_checkout().
REPO_ROOT=""
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    CANDIDATE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    [ -f "$CANDIDATE_ROOT/Cargo.toml" ] && REPO_ROOT="$CANDIDATE_ROOT"
fi

INSTALL_DIR="${UFC_INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="ufc"
FORCE_SOURCE=false
SKIP_LIBREOFFICE=false
for arg in "$@"; do
    case "$arg" in
        --from-source)      FORCE_SOURCE=true ;;
        --skip-libreoffice) SKIP_LIBREOFFICE=true ;;
    esac
done

log() { echo "==> $*"; }

# ---------------------------------------------------------------------------
# Step 1: Rust toolchain
# ---------------------------------------------------------------------------
ensure_rust() {
    if command -v cargo >/dev/null 2>&1; then
        log "Rust toolchain already installed ($(cargo --version))."
        return
    fi
    log "Rust not found — installing via rustup (non-interactive) ..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # rustup installs to ~/.cargo/bin; source it into this script's
    # environment so the rest of this run can use `cargo` immediately
    # without requiring a new shell.
    # shellcheck disable=SC1090
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: rustup install finished but cargo still isn't on PATH. Open a new terminal and re-run this script." >&2
        exit 1
    fi
    log "Rust installed ($(cargo --version))."
}

# ---------------------------------------------------------------------------
# Step 2: LibreOffice (optional dependency, only for docx/odt/pdf-from-office)
# ---------------------------------------------------------------------------
ensure_libreoffice() {
    if [ "$SKIP_LIBREOFFICE" = true ]; then
        log "Skipping LibreOffice install (--skip-libreoffice passed)."
        return
    fi
    if command -v soffice >/dev/null 2>&1 || command -v libreoffice >/dev/null 2>&1; then
        log "LibreOffice already installed."
        return
    fi

    log "LibreOffice not found — installing (docx/odt/pdf-from-office routes need it) ..."
    if [ "$(uname)" = "Darwin" ]; then
        if ! command -v brew >/dev/null 2>&1; then
            log "Homebrew not found — installing Homebrew first ..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        fi
        brew install --cask libreoffice
    elif command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update && sudo apt-get install -y libreoffice
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y libreoffice
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -Sy --noconfirm libreoffice-fresh
    elif command -v zypper >/dev/null 2>&1; then
        sudo zypper install -y libreoffice
    else
        log "No supported package manager detected — skipping LibreOffice."
        log "Image and PDF-text conversion still work without it; install LibreOffice"
        log "manually later if you need docx/odt/pdf-from-office routes."
        return
    fi
    log "LibreOffice installed."
}

# ---------------------------------------------------------------------------
# Step 3: ufc itself
# ---------------------------------------------------------------------------
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
    [ -z "$UFC_REPO" ] && return 1
    command -v curl >/dev/null 2>&1 || return 1

    local asset="ufc-${target}.tar.gz"
    local url="https://github.com/${UFC_REPO}/releases/latest/download/${asset}"
    log "Trying prebuilt binary: $url"

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
    log "Installed prebuilt $BIN_NAME to $INSTALL_DIR/$BIN_NAME (no local compile needed)"
    return 0
}

ensure_git() {
    if command -v git >/dev/null 2>&1; then
        return
    fi
    log "git not found — installing it (needed to fetch source for the local build) ..."
    if [ "$(uname)" = "Darwin" ]; then
        command -v brew >/dev/null 2>&1 || /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        brew install git
    elif command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update && sudo apt-get install -y git
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y git
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -Sy --noconfirm git
    elif command -v zypper >/dev/null 2>&1; then
        sudo zypper install -y git
    else
        echo "error: no supported package manager found to install git automatically. Install git manually and re-run." >&2
        exit 1
    fi
}

# When running standalone (curled, no local checkout), clone one into a
# reusable cache directory so build_from_source() has something to build.
ensure_repo_checkout() {
    [ -n "$REPO_ROOT" ] && return
    if [ -z "$UFC_REPO" ]; then
        echo "error: no local checkout found and UFC_REPO is unset." >&2
        echo "Re-run with: UFC_REPO=\"owner/repo\" bash -c \"\$(curl -fsSL <raw-script-url>)\"" >&2
        exit 1
    fi
    ensure_git
    local checkout_dir="$HOME/.cache/ufc/src"
    if [ -d "$checkout_dir/.git" ]; then
        log "Updating cached source checkout at $checkout_dir ..."
        git -C "$checkout_dir" fetch --depth 1 origin main
        git -C "$checkout_dir" reset --hard origin/main
    else
        log "Cloning https://github.com/$UFC_REPO for a source build ..."
        mkdir -p "$(dirname "$checkout_dir")"
        git clone --depth 1 "https://github.com/$UFC_REPO.git" "$checkout_dir"
    fi
    REPO_ROOT="$checkout_dir"
}

build_from_source() {
    ensure_repo_checkout
    log "Building ufc from source (release) ..."
    ( cd "$REPO_ROOT" && cargo build --release --workspace )

    local built_bin="$REPO_ROOT/target/release/$BIN_NAME"
    if [ ! -f "$built_bin" ]; then
        echo "error: expected binary not found at $built_bin" >&2
        exit 1
    fi

    mkdir -p "$INSTALL_DIR"
    install -m 755 "$built_bin" "$INSTALL_DIR/$BIN_NAME"
    log "Installed $BIN_NAME to $INSTALL_DIR/$BIN_NAME"
}

install_ufc() {
    local target
    target="$(detect_target)"
    if [ "$FORCE_SOURCE" = false ] && try_prebuilt "$target"; then
        return
    fi
    [ "$FORCE_SOURCE" = false ] && log "No prebuilt binary available, falling back to source build."
    ensure_rust
    build_from_source
}

# ---------------------------------------------------------------------------
# Run everything
# ---------------------------------------------------------------------------
install_ufc
ensure_libreoffice

case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        log "$INSTALL_DIR is already on PATH."
        ;;
    *)
        SHELL_RC=""
        case "${SHELL:-}" in
            */zsh)  SHELL_RC="$HOME/.zshrc" ;;
            */bash) SHELL_RC="$HOME/.bashrc" ;;
            *)      SHELL_RC="$HOME/.profile" ;;
        esac
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_RC"
        export PATH="$INSTALL_DIR:$PATH"
        log "Added $INSTALL_DIR to PATH in $SHELL_RC and to this session."
        ;;
esac

log "Done. Verifying:"
"$INSTALL_DIR/$BIN_NAME" routes
echo ""
log "ufc is installed and ready to use in this terminal. New terminals will pick it up automatically too."
