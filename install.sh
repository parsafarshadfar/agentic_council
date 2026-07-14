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

directory_has_entries() {
    local directory="$1"
    local entry
    [[ -d "$directory" ]] || return 1
    for entry in "$directory"/* "$directory"/.[!.]* "$directory"/..?*; do
        [[ -e "$entry" || -L "$entry" ]] && return 0
    done
    return 1
}

remove_cargo_cache_path() {
    local path="$1"
    local target_root="$2"
    case "$path" in
        "$target_root"/*) rm -rf -- "$path" ;;
        *)
            printf 'Refusing to clean a path outside the Cargo target folder: %s\n' "$path" >&2
            return 1
            ;;
    esac
}

maintain_cargo_cache() {
    local target_root="$SCRIPT_DIR/src-tauri/target"
    local debug_root="$target_root/debug"
    local incremental_root="$debug_root/incremental"
    local deps_root="$debug_root/deps"
    local needs_maintenance=0
    local path
    local legacy_artifacts=(
        "$debug_root/libagentic_council_lib.a"
        "$debug_root/libagentic_council_lib.so"
        "$debug_root/libagentic_council_lib.dylib"
        "$debug_root/libagentic_council_lib.dylib.dSYM"
        "$deps_root/libagentic_council_lib.a"
        "$deps_root/libagentic_council_lib.so"
        "$deps_root/libagentic_council_lib.dylib"
        "$deps_root/libagentic_council_lib.dylib.dSYM"
    )

    [[ -d "$target_root" ]] || return 0
    if directory_has_entries "$incremental_root"; then
        needs_maintenance=1
    fi
    for path in "${legacy_artifacts[@]}"; do
        if [[ -e "$path" || -L "$path" ]]; then
            needs_maintenance=1
            break
        fi
    done
    for path in "$deps_root"/.tmp*.temp-archive; do
        if [[ -d "$path" ]]; then
            needs_maintenance=1
            break
        fi
    done
    (( needs_maintenance == 1 )) || return 0

    info 'Removing stale Rust incremental, temporary, and legacy desktop build artifacts...'

    # Cargo obtains its target lock before removing this package's development
    # artifacts, so maintenance cannot race an active compilation.
    cargo clean --manifest-path "$SCRIPT_DIR/src-tauri/Cargo.toml" \
        --package agentic-council --profile dev --quiet

    # Old crate types and interrupted linker archives may no longer be known to
    # the current Cargo manifest. Remove only their exact paths after Cargo has
    # coordinated access to the target directory.
    if [[ -d "$incremental_root" ]]; then
        remove_cargo_cache_path "$incremental_root" "$target_root"
    fi
    for path in "${legacy_artifacts[@]}"; do
        if [[ -e "$path" || -L "$path" ]]; then
            remove_cargo_cache_path "$path" "$target_root"
        fi
    done
    for path in "$deps_root"/.tmp*.temp-archive; do
        if [[ -d "$path" ]]; then
            remove_cargo_cache_path "$path" "$target_root"
        fi
    done
    ok 'Rust build-cache maintenance completed.'
}

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
    node_base_url='https://nodejs.org/dist/latest-v22.x'
    checksum_path="$TEMP_DIR/SHASUMS256.txt"
    node_download_verified=0
    archive=""

    if ! command -v shasum >/dev/null 2>&1 && ! command -v sha256sum >/dev/null 2>&1; then
        printf 'A SHA-256 checksum tool is required (shasum on macOS or sha256sum on Linux).\n' >&2
        exit 1
    fi

    # Fetch the checksum list and archive as a pair. The latest-v22.x alias can
    # briefly point at different CDN cache generations during a Node.js update.
    for attempt in 1 2 3; do
        if ! curl --fail --silent --show-error --location --retry 3 --connect-timeout 20 \
            "$node_base_url/SHASUMS256.txt" -o "$checksum_path"; then
            if [[ "$attempt" != "3" ]]; then
                info "Node.js checksum download failed on attempt $attempt; retrying both files..."
                sleep 2
                continue
            fi
            break
        fi

        archive="$(awk -v suffix="-${node_platform}-${node_arch}.tar.gz" '$2 ~ suffix "$" { print $2; exit }' "$checksum_path")"
        expected_hash="$(awk -v file="$archive" '$2 == file { print $1; exit }' "$checksum_path")"
        if [[ -z "$archive" || ! "$expected_hash" =~ ^[[:xdigit:]]{64}$ ]]; then
            if [[ "$attempt" != "3" ]]; then
                info "Node.js checksum response was invalid on attempt $attempt; retrying both files..."
                sleep 2
                continue
            fi
            break
        fi

        if ! curl --fail --show-error --location --retry 3 --connect-timeout 20 \
            "$node_base_url/$archive" -o "$TEMP_DIR/$archive"; then
            if [[ "$attempt" != "3" ]]; then
                info "Node.js archive download failed on attempt $attempt; retrying both files..."
                sleep 2
                continue
            fi
            break
        fi

        if command -v shasum >/dev/null 2>&1; then
            actual_hash="$(shasum -a 256 "$TEMP_DIR/$archive" | awk '{print $1}')"
        else
            actual_hash="$(sha256sum "$TEMP_DIR/$archive" | awk '{print $1}')"
        fi
        if [[ "$actual_hash" == "$expected_hash" ]]; then
            node_download_verified=1
            break
        fi
        if [[ "$attempt" != "3" ]]; then
            info "Node.js archive and checksum did not match on attempt $attempt; retrying both files..."
            sleep 2
        fi
    done

    if [[ "$node_download_verified" != "1" ]]; then
        printf 'Could not download a checksum-verified Node.js archive after 3 matched attempts.\n' >&2
        printf 'A proxy, VPN, antivirus web shield, or filtered network may be changing downloads from nodejs.org. Try another network, then run this installer again.\n' >&2
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

export CARGO_INCREMENTAL=0
maintain_cargo_cache

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
