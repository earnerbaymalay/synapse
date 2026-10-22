#!/bin/sh
#
# build-installers.sh — Build Synapse release artifacts locally
#
# Mirrors the per-target steps of .github/workflows/build.yml so a local
# build produces the same files the release pipeline publishes:
#
#   target/release-assets/synapse-<host-triple>.tar.gz   CLI archive
#   target/release-assets/*.deb *.AppImage *.dmg *.msi   desktop installers
#   target/release-assets/SHA256SUMS                     checksums
#
# Both the CLI and the desktop app are built with the `os-keychain` feature,
# exactly like the release workflow.
#
# Usage:
#   ./scripts/build-installers.sh                 # full build
#   ./scripts/build-installers.sh --help          # this message
#   ./scripts/build-installers.sh --skip-cli      # skip CLI build
#   ./scripts/build-installers.sh --skip-desktop  # skip desktop build
#   ./scripts/build-installers.sh --regen-icons   # regenerate app icons first
#

set -eu

# ── Paths ──────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DESKTOP_DIR="${PROJECT_ROOT}/apps/desktop"
TAURI_DIR="${DESKTOP_DIR}/src-tauri"
# The root Cargo workspace includes apps/desktop/src-tauri, so every binary
# and every Tauri bundle lands in the workspace target directory.
RELEASE_DIR="${PROJECT_ROOT}/target/release"
BUNDLE_DIR="${RELEASE_DIR}/bundle"
STAGE_DIR="${PROJECT_ROOT}/target/release-assets"

# Same pins as CI (.github/workflows/*.yml).
EXPECTED_PNPM_VERSION="8.15.9"

BUILD_CLI=1
BUILD_DESKTOP=1
REGEN_ICONS=0

# ── Help ───────────────────────────────────────────────────────────────────────
show_help() {
    cat <<'EOF'
build-installers.sh — Build Synapse release artifacts locally

USAGE:
    ./scripts/build-installers.sh [OPTIONS]

OPTIONS:
    --help           Show this help message and exit
    --skip-cli       Skip building the CLI binary
    --skip-desktop   Skip building the desktop frontend and Tauri bundles
    --regen-icons    Regenerate icons with build-icons.py before bundling
                     (the committed icons are used otherwise, as in CI)

WHAT IT BUILDS (all cargo invocations use --locked):
  1. CLI binary        cargo build --release -p synapse --features synapse-core/os-keychain
  2. Desktop frontend  pnpm install --frozen-lockfile && pnpm --filter desktop-app build
  3. Tauri bundles     pnpm --filter desktop-app exec tauri build --features os-keychain
  4. Stage assets      target/release-assets/ + SHA256SUMS

OUTPUT LOCATIONS:
  CLI binary:     target/release/synapse
  Tauri binary:   target/release/desktop-app
  Tauri bundles:  target/release/bundle/{deb,appimage,dmg,msi}/
  Staged assets:  target/release-assets/
EOF
    exit 0
}

# ── Parse arguments ────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "${arg}" in
        --help) show_help ;;
        --skip-cli) BUILD_CLI=0 ;;
        --skip-desktop) BUILD_DESKTOP=0 ;;
        --regen-icons) REGEN_ICONS=1 ;;
        *)
            echo "ERROR: Unknown option: ${arg}"
            echo "Run with --help for usage."
            exit 1
            ;;
    esac
done

# ── Color helpers (POSIX-safe, no tput dependency) ────────────────────────────
if [ -t 1 ]; then
    RED=$(printf '\033[0;31m')
    GREEN=$(printf '\033[0;32m')
    YELLOW=$(printf '\033[1;33m')
    CYAN=$(printf '\033[0;36m')
    BOLD=$(printf '\033[1m')
    NC=$(printf '\033[0m')
else
    RED=''
    GREEN=''
    YELLOW=''
    CYAN=''
    BOLD=''
    NC=''
fi

info()   { printf '%s[INFO]%s  %s\n' "${CYAN}" "${NC}" "$*"; }
ok()     { printf '%s[OK]%s    %s\n' "${GREEN}" "${NC}" "$*"; }
warn()   { printf '%s[WARN]%s  %s\n' "${YELLOW}" "${NC}" "$*"; }
err()    { printf '%s[ERROR]%s %s\n' "${RED}" "${NC}" "$*"; }
header() { printf '\n%s━━━ %s ━━━%s\n' "${BOLD}" "$*" "${NC}"; }

file_size() { du -h "$1" 2>/dev/null | cut -f1; }

# ── Dependency checks ─────────────────────────────────────────────────────────

check_cmd() {
    cmd="$1"
    pkg="$2"
    if ! command -v "${cmd}" >/dev/null 2>&1; then
        err "'${cmd}' not found. Please install ${pkg}."
        case "${cmd}" in
            cargo)   echo "  → https://rustup.rs/" ;;
            node)    echo "  → https://nodejs.org/ (v20 or later)" ;;
            pnpm)    echo "  → npm install -g pnpm@${EXPECTED_PNPM_VERSION}" ;;
            python3) echo "  → https://www.python.org/downloads/" ;;
        esac
        return 1
    fi
    return 0
}

check_system_libs() {
    missing_libs=""

    # Linux: check common Tauri system libraries
    if [ "$(uname -s)" = "Linux" ]; then
        for lib in libgtk-3.so libwebkit2gtk-4.1.so libayatana-appindicator3.so libdbus-1.so libjavascriptcoregtk-4.1.so libsoup-3.0.so; do
            if ! ldconfig -p 2>/dev/null | grep -q "${lib}"; then
                missing_libs="${missing_libs}  ${lib}"
            fi
        done

        if [ -n "${missing_libs}" ]; then
            warn "Some Tauri system libraries may be missing:${missing_libs}"
            echo ""
            echo "  On Debian/Ubuntu, install:"
            echo "    sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev \\"
            echo "      libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev \\"
            echo "      libjavascriptcoregtk-4.1-dev libsoup-3.0-dev patchelf"
            echo ""
            echo "  On Fedora, install:"
            echo "    sudo dnf install gtk3-devel webkit2gtk4.1-devel \\"
            echo "      libappindicator-gtk3-devel librsvg2-devel dbus-devel patchelf"
            echo ""
            echo "  On Arch, install:"
            echo "    sudo pacman -S gtk3 webkit2gtk-4.1 libappindicator-gtk3 librsvg patchelf"
            echo ""
            echo "  (Continuing anyway — the build may fail if these are truly missing.)"
        fi
    fi

    # macOS: check for Xcode command-line tools
    if [ "$(uname -s)" = "Darwin" ]; then
        if ! xcode-select -p >/dev/null 2>&1; then
            warn "Xcode command-line tools not found."
            echo "  Install with: xcode-select --install"
            echo "  (Continuing anyway — the build may fail without them.)"
        fi
    fi
}

check_versions() {
    if command -v cargo >/dev/null 2>&1; then
        info "Rust version: $(cargo --version 2>/dev/null | awk '{print $2}')"
    fi

    if command -v node >/dev/null 2>&1; then
        info "Node.js version: $(node --version 2>/dev/null || echo unknown)"
    fi

    if command -v pnpm >/dev/null 2>&1; then
        pnpm_ver=$(pnpm --version 2>/dev/null || echo unknown)
        info "pnpm version: ${pnpm_ver}"
        if [ "${pnpm_ver}" != "${EXPECTED_PNPM_VERSION}" ]; then
            warn "CI uses pnpm ${EXPECTED_PNPM_VERSION}; --frozen-lockfile may behave differently on ${pnpm_ver}."
        fi
    fi
}

host_triple() {
    rustc -vV | sed -n 's/^host: //p'
}

# ── Build steps ────────────────────────────────────────────────────────────────

build_cli() {
    header "Building CLI binary (synapse, os-keychain)"
    cd "${PROJECT_ROOT}"

    info "Running: cargo build --release --locked -p synapse --features synapse-core/os-keychain"
    cargo build --release --locked -p synapse --features synapse-core/os-keychain

    ok "CLI binary built: ${RELEASE_DIR}/synapse ($(file_size "${RELEASE_DIR}/synapse"))"
}

build_desktop_frontend() {
    header "Building desktop frontend (Vite + React)"
    cd "${PROJECT_ROOT}"

    info "Running: pnpm install --frozen-lockfile"
    pnpm install --frozen-lockfile

    info "Running: pnpm --filter desktop-app build"
    pnpm --filter desktop-app --fail-if-no-match build

    ok "Desktop frontend built: ${DESKTOP_DIR}/dist/"
}

generate_icons() {
    header "Regenerating application icons"

    if [ ! -f "${TAURI_DIR}/icons/icon.png" ]; then
        err "Source icon not found: ${TAURI_DIR}/icons/icon.png"
        err "Place a 512x512 RGBA PNG at that path and re-run."
        return 1
    fi

    cd "${TAURI_DIR}"
    info "Running: python3 build-icons.py"
    python3 build-icons.py
    ok "Icons regenerated in ${TAURI_DIR}/icons/"
}

build_tauri_bundles() {
    header "Building Tauri desktop app + bundles (os-keychain)"
    cd "${PROJECT_ROOT}"

    # `tauri build` compiles the desktop-app crate itself, so no separate
    # cargo build is needed. `os-keychain` is a passthrough feature defined in
    # apps/desktop/src-tauri/Cargo.toml.
    info "Running: pnpm --filter desktop-app exec tauri build --features os-keychain"
    pnpm --filter desktop-app --fail-if-no-match exec tauri build --features os-keychain

    ok "Tauri bundles built in ${BUNDLE_DIR}/"
}

stage_assets() {
    header "Staging release assets"
    triple="$(host_triple)"

    rm -rf "${STAGE_DIR}"
    mkdir -p "${STAGE_DIR}"

    if [ "${BUILD_CLI}" -eq 1 ]; then
        if [ -f "${RELEASE_DIR}/synapse.exe" ]; then
            warn "Windows host: create synapse-${triple}.zip containing synapse.exe manually (CI uses 7z)."
        else
            tar -C "${RELEASE_DIR}" -czf "${STAGE_DIR}/synapse-${triple}.tar.gz" synapse
        fi
    fi

    if [ "${BUILD_DESKTOP}" -eq 1 ]; then
        for f in "${BUNDLE_DIR}"/deb/*.deb "${BUNDLE_DIR}"/appimage/*.AppImage \
                 "${BUNDLE_DIR}"/dmg/*.dmg "${BUNDLE_DIR}"/msi/*.msi; do
            if [ -f "${f}" ]; then
                cp "${f}" "${STAGE_DIR}/"
            fi
        done
    fi

    cd "${STAGE_DIR}"
    if [ -z "$(ls -A .)" ]; then
        warn "Nothing to stage."
        return 0
    fi
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -- * > ../SHA256SUMS
    else
        shasum -a 256 -- * > ../SHA256SUMS
    fi
    mv ../SHA256SUMS SHA256SUMS

    for f in "${STAGE_DIR}"/*; do
        ok "$(basename "${f}") ($(file_size "${f}"))"
    done
}

# ── Main ────────────────────────────────────────────────────────────────────────

main() {
    printf '\n%sSynapse — Build & Installer Generator%s\n\n' "${BOLD}" "${NC}"

    header "Checking system dependencies"
    all_ok=1
    check_cmd cargo "Rust (rustup)" || all_ok=0
    if [ "${BUILD_DESKTOP}" -eq 1 ]; then
        check_cmd node "Node.js" || all_ok=0
        check_cmd pnpm "pnpm" || all_ok=0
        check_system_libs
    fi
    if [ "${REGEN_ICONS}" -eq 1 ]; then
        check_cmd python3 "Python 3" || all_ok=0
    fi
    check_versions

    if [ "${all_ok}" -eq 0 ]; then
        echo ""
        err "One or more required dependencies are missing."
        exit 1
    fi
    ok "All required dependencies found."

    if [ "${BUILD_CLI}" -eq 1 ]; then
        build_cli
    else
        info "Skipping CLI build (--skip-cli)"
    fi

    if [ "${BUILD_DESKTOP}" -eq 1 ]; then
        build_desktop_frontend
        if [ "${REGEN_ICONS}" -eq 1 ]; then
            generate_icons
        fi
        build_tauri_bundles
    else
        info "Skipping desktop build (--skip-desktop)"
    fi

    stage_assets

    echo ""
    info "Install the CLI with:"
    echo "  sudo install -m 0755 target/release/synapse /usr/local/bin/synapse"
}

main "$@"
