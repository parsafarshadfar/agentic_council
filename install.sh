#!/usr/bin/env bash
# Agentic Council one-click installer for macOS and Linux.

set -Eeuo pipefail

STEP=0
TOTAL_STEPS=6
TEMP_DIR=""
TMP_BASE="${TMPDIR:-/tmp}"
TMP_BASE="${TMP_BASE%/}"
TOOLS_ROOT="${HOME}/.agentic-council/tools"
RUST_TOOLCHAIN="1.97.0"

step() {
    STEP=$((STEP + 1))
    printf '\n[STEP %s/%s] %s\n' "$STEP" "$TOTAL_STEPS" "$*"
}

ok()   { printf '  [OK] %s\n' "$*"; }
info() { printf '  [INFO] %s\n' "$*"; }
warn() { printf '  [WARNING] %s\n' "$*"; }

pause_on_error() {
    if [[ -t 0 ]]; then
        printf '\nPress Enter to close this window...'
        read -r _ || true
    fi
}

on_error() {
    local status="$1"
    local line="$2"
    trap - ERR
    printf '\nSETUP COULD NOT FINISH\n' >&2
    printf 'The installer stopped near line %s. Review the message just above.\n' "$line" >&2
    printf 'Nothing needs to be undone; fix the issue and run this installer again.\n' >&2
    pause_on_error
    exit "$status"
}

cleanup() {
    if [[ -n "$TEMP_DIR" && -d "$TEMP_DIR" && "$TEMP_DIR" == "$TMP_BASE"/agentic-council.* ]]; then
        rm -rf -- "$TEMP_DIR"
    fi
}

trap 'on_error $? $LINENO' ERR
trap cleanup EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

case "$(uname -s)" in
    Darwin) PLATFORM="macos" ;;
    Linux)  PLATFORM="linux" ;;
    *)
        printf 'Unsupported operating system: %s\n' "$(uname -s)" >&2
        exit 1
        ;;
esac

printf '\n========================================================\n'
printf '      Agentic Council - One-Click %s Setup\n' "$PLATFORM"
printf ' Everything is checked, installed, and launched for you.\n'
printf '========================================================\n'
info "App folder: $SCRIPT_DIR"

step "Checking that the app package is complete..."
missing=()
for required in package.json rust-toolchain.toml src-tauri/Cargo.toml; do
    [[ -f "$required" ]] || missing+=("$required")
done
if [[ ! -f src-tauri/tauri.conf.json && ! -f src-tauri/tauri.conf.json5 && ! -f src-tauri/Tauri.toml ]]; then
    missing+=("src-tauri/tauri.conf.json")
fi
if (( ${#missing[@]} > 0 )); then
    printf 'This is an incomplete Agentic Council folder. Missing: %s\n' "${missing[*]}" >&2
    printf 'Download or extract the complete project, then run this installer again.\n' >&2
    pause_on_error
    exit 1
fi

toolchain_value="$(sed -nE 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' rust-toolchain.toml | head -n 1)"
if [[ -n "$toolchain_value" ]]; then
    RUST_TOOLCHAIN="$toolchain_value"
fi
ok "All required application files are present."

if [[ "${AGENTIC_COUNCIL_VALIDATE_ONLY:-0}" == "1" ]]; then
    ok "Installer validation completed."
    exit 0
fi

TEMP_DIR="$(mktemp -d "$TMP_BASE/agentic-council.XXXXXX")"

step "Checking platform build tools..."
if [[ "$PLATFORM" == "macos" ]]; then
    if xcode-select -p >/dev/null 2>&1; then
        ok "Apple Command Line Tools are ready."
    else
        info "Apple will open a small installer window. Choose Install."
        xcode-select --install >/dev/null 2>&1 || true
        info "Waiting for Apple Command Line Tools to finish..."
        installed=0
        for _ in {1..360}; do
            if xcode-select -p >/dev/null 2>&1; then
                installed=1
                break
            fi
            sleep 5
        done
        if [[ "$installed" != "1" ]]; then
            printf 'Apple Command Line Tools were not installed. Run this installer again and choose Install in the Apple dialog.\n' >&2
            exit 1
        fi
        ok "Apple Command Line Tools are ready."
    fi
else
    if command -v apt-get >/dev/null 2>&1; then
        info "Installing the Linux desktop build libraries. Your password may be requested."
        sudo apt-get update -qq
        sudo apt-get install -y build-essential curl file libwebkit2gtk-4.1-dev \
            libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config patchelf
    elif command -v dnf >/dev/null 2>&1; then
        info "Installing the Linux desktop build libraries. Your password may be requested."
        sudo dnf install -y gcc gcc-c++ make curl file openssl-devel \
            webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel pkg-config
    else
        printf 'This Linux distribution is not supported by the automatic installer.\n' >&2
        exit 1
    fi
    ok "Linux desktop build tools are ready."
fi

node_version_is_compatible() {
    local node_command="$1"
    local version major minor
    version="$("$node_command" -p 'process.versions.node' 2>/dev/null)" || return 1
    IFS=. read -r major minor _ <<< "$version"
    [[ "$major" =~ ^[0-9]+$ && "$minor" =~ ^[0-9]+$ ]] || return 1
    (( major > 22 || (major == 22 && minor >= 12) ))
}

find_compatible_node() {
    local candidate
    if command -v node >/dev/null 2>&1; then
        candidate="$(command -v node)"
        if node_version_is_compatible "$candidate"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    fi
    for candidate in "$TOOLS_ROOT"/node-v*/bin/node; do
        [[ -x "$candidate" ]] || continue
        if node_version_is_compatible "$candidate"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

step "Checking Node.js..."
node_path="$(find_compatible_node || true)"
if [[ -z "$node_path" ]]; then
    case "$(uname -m)" in
        arm64|aarch64) node_arch="arm64" ;;
        x86_64|amd64) node_arch="x64" ;;
        *)
            printf 'Unsupported processor architecture: %s\n' "$(uname -m)" >&2
            exit 1
            ;;
    esac
    if [[ "$PLATFORM" == "macos" ]]; then
        node_platform="darwin"
    else
        node_platform="linux"
    fi

    info "Downloading a private Node.js LTS copy for Agentic Council..."
    checksums="$(curl --fail --silent --show-error --location --retry 3 --connect-timeout 20 \
        'https://nodejs.org/dist/latest-v22.x/SHASUMS256.txt')"
    archive="$(printf '%s\n' "$checksums" | awk -v suffix="-${node_platform}-${node_arch}.tar.gz" '$2 ~ suffix "$" { print $2; exit }')"
    if [[ -z "$archive" ]]; then
        printf 'Node.js does not provide a download for %s-%s.\n' "$node_platform" "$node_arch" >&2
        exit 1
    fi
    expected_hash="$(printf '%s\n' "$checksums" | awk -v file="$archive" '$2 == file { print $1; exit }')"
    curl --fail --show-error --location --retry 3 --connect-timeout 20 \
        "https://nodejs.org/dist/latest-v22.x/$archive" -o "$TEMP_DIR/$archive"
    if command -v shasum >/dev/null 2>&1; then
        actual_hash="$(shasum -a 256 "$TEMP_DIR/$archive" | awk '{print $1}')"
    else
        actual_hash="$(sha256sum "$TEMP_DIR/$archive" | awk '{print $1}')"
    fi
    if [[ "$actual_hash" != "$expected_hash" ]]; then
        printf 'The Node.js download failed its security checksum.\n' >&2
        exit 1
    fi
    mkdir -p "$TOOLS_ROOT"
    tar -xzf "$TEMP_DIR/$archive" -C "$TOOLS_ROOT"
    node_directory="${archive%.tar.gz}"
    node_path="$TOOLS_ROOT/$node_directory/bin/node"
    node_version_is_compatible "$node_path"
fi
export PATH="$(dirname "$node_path"):$PATH"
ok "Node.js $(node --version) is ready."

step "Checking Rust..."
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
    info "Downloading rustup from the official Rust servers..."
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location --retry 3 \
        https://sh.rustup.rs -o "$TEMP_DIR/rustup-init.sh"
    sh "$TEMP_DIR/rustup-init.sh" -y --profile minimal --default-toolchain none --no-modify-path
    export PATH="$HOME/.cargo/bin:$PATH"
fi
command -v rustup >/dev/null 2>&1
ok "Rust toolchain manager is ready."

step "Installing Rust $RUST_TOOLCHAIN..."
rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --component rustfmt --component clippy
ok "Rust $RUST_TOOLCHAIN is ready."

step "Installing application dependencies..."
if [[ -f package-lock.json && ! -d node_modules ]]; then
    npm ci --no-audit --no-fund
else
    npm install --no-audit --no-fund
fi
ok "All application dependencies are ready."

if [[ "${AGENTIC_COUNCIL_SKIP_LAUNCH:-0}" == "1" ]]; then
    ok "Setup completed. Launch was skipped as requested."
    exit 0
fi

printf '\n========================================================\n'
printf ' Setup complete. Agentic Council is starting now.\n'
printf ' The first launch compiles the app and can take a while.\n'
printf ' Keep this window open while the app is running.\n'
printf '========================================================\n\n'

npm run tauri -- dev
