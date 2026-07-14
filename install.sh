#!/usr/bin/env bash
# ============================================================
#  Agentic Council — One-Click macOS / Linux Installer
#  Open Terminal, paste this command, and press Enter:
#
#    bash install.sh
#
#  Or make it executable and double-click from Finder:
#    chmod +x install.sh
#
#  The script installs everything needed automatically.
#  First run takes 15-25 minutes (Rust compilation).
#  Subsequent runs launch the app instantly.
# ============================================================

set -euo pipefail

# ── Colours for friendly output ─────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Colour

STEP=0
total_steps=6

step() {
    STEP=$((STEP + 1))
    echo ""
    echo -e "${BLUE}${BOLD}[STEP $STEP/$total_steps]${NC} $*"
}

ok()    { echo -e "  ${GREEN}✓${NC} $*"; }
warn()  { echo -e "  ${YELLOW}⚠${NC}  $*"; }
info()  { echo -e "  ${BLUE}ℹ${NC}  $*"; }
fail()  { echo -e "  ${RED}✗ ERROR:${NC} $*"; }

# ── Script location (works even when called from Finder) ────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║       Agentic Council — One-Click Installer          ║${NC}"
echo -e "${BOLD}║  This script sets up everything and launches the app ║${NC}"
echo -e "${BOLD}║  Grab a coffee — first run takes 15-25 minutes.      ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════╝${NC}"
echo ""
info "Project folder: $SCRIPT_DIR"

# ── Detect OS ───────────────────────────────────────────────
OS="$(uname -s)"
if [[ "$OS" == "Darwin" ]]; then
    PLATFORM="macos"
elif [[ "$OS" == "Linux" ]]; then
    PLATFORM="linux"
else
    fail "Unsupported operating system: $OS"
    exit 1
fi
info "Detected platform: $PLATFORM"

# ── 1. macOS: Xcode Command Line Tools ──────────────────────
step "Checking platform build tools..."
if [[ "$PLATFORM" == "macos" ]]; then
    if xcode-select -p &>/dev/null; then
        ok "Xcode Command Line Tools already installed."
    else
        info "Installing Xcode Command Line Tools (Apple's free C compiler)..."
        info "A dialog will pop up — click 'Install', not 'Get Xcode'."
        xcode-select --install 2>/dev/null || true
        # Wait for it to complete
        until xcode-select -p &>/dev/null; do
            sleep 5
        done
        ok "Xcode Command Line Tools installed."
    fi
else
    # Linux: ensure basic build essentials
    if command -v apt-get &>/dev/null; then
        info "Installing build-essential (requires sudo)..."
        sudo apt-get update -qq
        sudo apt-get install -y build-essential curl wget libwebkit2gtk-4.1-dev \
            libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config \
            patchelf >>/dev/null 2>&1
        ok "Linux build dependencies installed."
    elif command -v dnf &>/dev/null; then
        sudo dnf install -y gcc gcc-c++ make openssl-devel \
            webkit2gtk4.1-devel librsvg2-devel pkg-config >>/dev/null 2>&1
        ok "Linux build dependencies installed."
    else
        warn "Could not auto-install Linux build tools."
        warn "Please install: gcc, make, openssl-devel, webkit2gtk-devel"
        warn "Then re-run this script."
    fi
fi

# ── 2. Homebrew (macOS only) ─────────────────────────────────
if [[ "$PLATFORM" == "macos" ]]; then
    step "Checking Homebrew (macOS package manager)..."
    if command -v brew &>/dev/null; then
        ok "Homebrew already installed."
    else
        info "Installing Homebrew (this is safe and free)..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        # Add brew to PATH for Apple Silicon
        if [[ -f /opt/homebrew/bin/brew ]]; then
            eval "$(/opt/homebrew/bin/brew shellenv)"
            echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> "$HOME/.zprofile"
        fi
        ok "Homebrew installed."
    fi
else
    total_steps=5  # No Homebrew step on Linux
fi

# ── 3. Node.js ───────────────────────────────────────────────
step "Checking Node.js..."
if command -v node &>/dev/null; then
    NODE_VER="$(node --version)"
    ok "Node.js already installed: $NODE_VER"
else
    if [[ "$PLATFORM" == "macos" ]]; then
        info "Installing Node.js via Homebrew..."
        brew install node
    else
        # Linux: use NodeSource
        info "Installing Node.js via NodeSource..."
        curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
        sudo apt-get install -y nodejs
    fi
    ok "Node.js installed: $(node --version)"
fi

# ── 4. Rust ──────────────────────────────────────────────────
step "Checking Rust..."
# Ensure .cargo/bin is in PATH for this session
export PATH="$HOME/.cargo/bin:$PATH"

if command -v rustup &>/dev/null; then
    ok "Rust toolchain manager (rustup) already installed."
else
    info "Installing Rust (the programming language runtime for the app core)..."
    info "This is the official installer from rust-lang.org."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    export PATH="$HOME/.cargo/bin:$PATH"
    # Persist for future shell sessions
    if [[ -f "$HOME/.zprofile" ]]; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$HOME/.zprofile"
    fi
    if [[ -f "$HOME/.bash_profile" ]]; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$HOME/.bash_profile"
    fi
    ok "Rust installed."
fi

# ── 5. Exact Rust toolchain version ──────────────────────────
step "Setting up required Rust version (1.97.0)..."
rustup toolchain install 1.97.0 --profile minimal --component rustfmt clippy
rustup default 1.97.0
ok "Rust 1.97.0 ready."

# ── 6. npm dependencies ───────────────────────────────────────
step "Installing application dependencies..."
cd "$SCRIPT_DIR"
if [[ ! -d "node_modules" ]]; then
    info "Running npm install (downloads ~500 MB once)..."
    npm install
    ok "Dependencies installed."
else
    ok "Dependencies already installed (skipping)."
fi

# ── Launch ────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║  All set! Launching Agentic Council...               ║${NC}"
echo -e "${BOLD}║  The FIRST launch compiles the Rust backend.         ║${NC}"
echo -e "${BOLD}║  This takes 5-15 minutes. Please be patient!         ║${NC}"
echo -e "${BOLD}║  Subsequent launches are instant.                    ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════╝${NC}"
echo ""

cd "$SCRIPT_DIR"
npm run tauri -- dev
