#!/bin/sh
#
# build-installers.sh — Build all LLM Neurosurgeon installers
#
# Generates:
#   - CLI binary  (target/release/neurosurgeon)
#   - Desktop app (Tauri bundle: .deb, .AppImage, etc.)
#
# Usage:
#   ./scripts/build-installers.sh            # full build
#   ./scripts/build-installers.sh --help     # this message
#   ./scripts/build-installers.sh --skip-cli # skip CLI build
#   ./scripts/build-installers.sh --skip-desktop # skip desktop build
#
# Environment variables:
#   CARGO_PROFILE  — Cargo profile (default: release)
#   JOBS           — parallel build jobs (default: number of CPUs)
#

set -eu

# ── Paths ──────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CLI_CRATE="${PROJECT_ROOT}/apps/cli"
DESKTOP_DIR="${PROJECT_ROOT}/apps/desktop"
TAURI_DIR="${DESKTOP_DIR}/src-tauri"

# ── Defaults ───────────────────────────────────────────────────────────────────
CARGO_PROFILE="${CARGO_PROFILE:-release}"
JOBS="${JOBS:-$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)}"
BUILD_CLI=1
BUILD_DESKTOP=1

# ── Help ───────────────────────────────────────────────────────────────────────
show_help() {
    cat <<'EOF'
build-installers.sh — Build all LLM Neurosurgeon installers

USAGE:
    ./scripts/build-installers.sh [OPTIONS]

OPTIONS:
    --help           Show this help message and exit
    --skip-cli       Skip building the CLI binary
    --skip-desktop   Skip building the desktop app and Tauri bundles

WHAT IT BUILDS:
  1. CLI binary          — cargo build -p neurosurgeon --release
  2. Desktop frontend    — pnpm install && pnpm build (in apps/desktop)
  3. App icons           — python3 build-icons.py (in apps/desktop/src-tauri)
  4. Tauri desktop app   — cargo build --release (in apps/desktop)
  5. Tauri bundles       — pnpm tauri build (in apps/desktop)

OUTPUT LOCATIONS:
  CLI binary:       target/release/neurosurgeon
  Desktop frontend: apps/desktop/dist/
  Tauri binary:     apps/desktop/src-tauri/target/release/desktop-app
  Tauri bundles:    apps/desktop/src-tauri/target/release/bundle/

ENVIRONMENT:
  CARGO_PROFILE    Cargo build profile (default: release)
  JOBS             Parallel build jobs (default: number of CPUs)
EOF
    exit 0
}

# ── Parse arguments ────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "${arg}" in
        --help) show_help ;;
        --skip-cli) BUILD_CLI=0 ;;
        --skip-desktop) BUILD_DESKTOP=0 ;;
        *)
            echo "ERROR: Unknown option: ${arg}"
            echo "Run with --help for usage."
            exit 1
            ;;
    esac
done

# ── Color helpers (POSIX-safe, no tput dependency) ────────────────────────────
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    CYAN=''
    BOLD=''
    NC=''
fi

info()   { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()     { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
warn()   { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
err()    { printf "${RED}[ERROR]${NC} %s\n" "$*"; }
header() { printf "\n${BOLD}━━━ %s ━━━${NC}\n" "$*"; }

# ── Dependency checks ─────────────────────────────────────────────────────────

check_cmd() {
    cmd="$1"
    pkg="$2"
    if ! command -v "${cmd}" >/dev/null 2>&1; then
        err "'${cmd}' not found. Please install ${pkg}."
        case "${cmd}" in
            cargo)
                echo "  → https://rustup.rs/"
                ;;
            node)
                echo "  → https://nodejs.org/ (v18 or later)"
                ;;
            pnpm)
                echo "  → npm install -g pnpm  or  https://pnpm.io/installation"
                ;;
            python3)
                echo "  → https://www.python.org/downloads/"
                ;;
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
            echo "      libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev"
            echo ""
            echo "  On Fedora, install:"
            echo "    sudo dnf install gtk3-devel webkit2gtk4.1-devel \\"
            echo "      libappindicator-gtk3-devel librsvg2-devel"
            echo ""
            echo "  On Arch, install:"
            echo "    sudo pacman -S gtk3 webkit2gtk-4.1 libappindicator-gtk3 librsvg"
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
    # Rust / Cargo
    if command -v cargo >/dev/null 2>&1; then
        rust_ver=$(cargo --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        info "Rust version: ${rust_ver}"
    fi

    # Node.js
    if command -v node >/dev/null 2>&1; then
        node_ver=$(node --version 2>/dev/null || echo "unknown")
        info "Node.js version: ${node_ver}"
    fi

    # pnpm
    if command -v pnpm >/dev/null 2>&1; then
        pnpm_ver=$(pnpm --version 2>/dev/null || echo "unknown")
        info "pnpm version: ${pnpm_ver}"
    fi

    # Python 3
    if command -v python3 >/dev/null 2>&1; then
        py_ver=$(python3 --version 2>/dev/null || echo "unknown")
        info "Python3 version: ${py_ver}"
    fi
}

# ── Build steps ────────────────────────────────────────────────────────────────

build_cli() {
    header "Step 1/5: Building CLI binary (neurosurgeon)"

    cd "${PROJECT_ROOT}"

    info "Running: cargo build -p neurosurgeon --${CARGO_PROFILE} -j${JOBS}"
    cargo build -p neurosurgeon --"${CARGO_PROFILE}" -j"${JOBS}"

    echo ""
    ok "CLI binary built successfully!"

    # Determine the output path
    if [ "${CARGO_PROFILE}" = "release" ]; then
        cli_binary="${PROJECT_ROOT}/target/release/neurosurgeon"
    else
        cli_binary="${PROJECT_ROOT}/target/${CARGO_PROFILE}/neurosurgeon"
    fi

    if [ -f "${cli_binary}" ]; then
        file_size=$(ls -lh "${cli_binary}" 2>/dev/null | awk '{print $5}')
        info "Output: ${cli_binary} (${file_size})"
    else
        warn "Expected binary not found at: ${cli_binary}"
        warn "It may have been placed elsewhere depending on your Cargo configuration."
    fi
}

build_desktop_frontend() {
    header "Step 2/5: Building desktop frontend (Vite + React)"

    if [ ! -d "${DESKTOP_DIR}" ]; then
        err "Desktop directory not found: ${DESKTOP_DIR}"
        return 1
    fi

    cd "${DESKTOP_DIR}"

    info "Running: pnpm install"
    pnpm install --frozen-lockfile 2>/dev/null || pnpm install

    info "Running: pnpm build"
    pnpm build

    echo ""
    ok "Desktop frontend built successfully!"

    if [ -d "${DESKTOP_DIR}/dist" ]; then
        info "Output: ${DESKTOP_DIR}/dist/"
    fi
}

generate_icons() {
    header "Step 3/5: Generating application icons"

    if [ ! -f "${TAURI_DIR}/build-icons.py" ]; then
        err "Icon build script not found: ${TAURI_DIR}/build-icons.py"
        return 1
    fi

    if [ ! -f "${TAURI_DIR}/icons/icon.png" ]; then
        err "Source icon not found: ${TAURI_DIR}/icons/icon.png"
        err "Place a 512x512 RGBA PNG at that path and re-run."
        return 1
    fi

    cd "${TAURI_DIR}"

    info "Running: python3 build-icons.py"
    python3 build-icons.py

    echo ""
    ok "Icons generated successfully!"
    info "Output: ${TAURI_DIR}/icons/"
}

build_tauri_app() {
    header "Step 4/5: Building Tauri desktop app (Rust)"

    if [ ! -d "${TAURI_DIR}" ]; then
        err "Tauri directory not found: ${TAURI_DIR}"
        return 1
    fi

    cd "${TAURI_DIR}"

    info "Running: cargo build --${CARGO_PROFILE} -j${JOBS}"
    cargo build --"${CARGO_PROFILE}" -j"${JOBS}"

    echo ""
    ok "Tauri desktop app built successfully!"

    # Determine the output path
    if [ "${CARGO_PROFILE}" = "release" ]; then
        tauri_binary="${TAURI_DIR}/target/release/desktop-app"
    else
        tauri_binary="${TAURI_DIR}/target/${CARGO_PROFILE}/desktop-app"
    fi

    if [ -f "${tauri_binary}" ]; then
        file_size=$(ls -lh "${tauri_binary}" 2>/dev/null | awk '{print $5}')
        info "Output: ${tauri_binary} (${file_size})"
    else
        warn "Expected binary not found at: ${tauri_binary}"
    fi
}

build_tauri_bundles() {
    header "Step 5/5: Building Tauri bundles (.deb, .AppImage, etc.)"

    if [ ! -d "${DESKTOP_DIR}" ]; then
        err "Desktop directory not found: ${DESKTOP_DIR}"
        return 1
    fi

    cd "${DESKTOP_DIR}"

    info "Running: pnpm tauri build"
    pnpm tauri build

    echo ""
    ok "Tauri bundles built successfully!"

    # Report bundle output
    bundle_dir="${TAURI_DIR}/target/release/bundle"
    if [ -d "${bundle_dir}" ]; then
        info "Bundle output directory: ${bundle_dir}"
        echo ""
        for fmt_dir in "${bundle_dir}"/*/; do
            if [ -d "${fmt_dir}" ]; then
                fmt_name=$(basename "${fmt_dir}")
                info "  ${fmt_name}:"
                for artifact in "${fmt_dir}"/*; do
                    if [ -f "${artifact}" ]; then
                        artifact_size=$(ls -lh "${artifact}" 2>/dev/null | awk '{print $5}')
                        info "    $(basename "${artifact}") (${artifact_size})"
                    fi
                done
            fi
        done
    fi
}

print_summary() {
    header "Build Summary"

    if [ "${BUILD_CLI}" -eq 1 ]; then
        if [ "${CARGO_PROFILE}" = "release" ]; then
            cli_path="${PROJECT_ROOT}/target/release/neurosurgeon"
        else
            cli_path="${PROJECT_ROOT}/target/${CARGO_PROFILE}/neurosurgeon"
        fi
        if [ -f "${cli_path}" ]; then
            ok "CLI binary:       ${cli_path}"
        else
            warn "CLI binary:       NOT BUILT (or not at expected path)"
        fi
    else
        info "CLI binary:       skipped"
    fi

    if [ "${BUILD_DESKTOP}" -eq 1 ]; then
        if [ -d "${DESKTOP_DIR}/dist" ]; then
            ok "Desktop frontend: ${DESKTOP_DIR}/dist/"
        else
            warn "Desktop frontend: NOT BUILT"
        fi

        tauri_bin="${TAURI_DIR}/target/release/desktop-app"
        if [ -f "${tauri_bin}" ]; then
            ok "Tauri binary:     ${tauri_bin}"
        else
            warn "Tauri binary:     NOT BUILT (or not at expected path)"
        fi

        bundle_dir="${TAURI_DIR}/target/release/bundle"
        if [ -d "${bundle_dir}" ]; then
            ok "Tauri bundles:    ${bundle_dir}/"
        else
            warn "Tauri bundles:    NOT BUILT"
        fi
    else
        info "Desktop app:      skipped"
    fi

    echo ""
    info "All done! Install the CLI with:"
    echo "  sudo cp target/release/neurosurgeon /usr/local/bin/"
    echo ""
    info "Install the desktop app via the bundle in:"
    echo "  ${TAURI_DIR}/target/release/bundle/"
}

# ── Main ────────────────────────────────────────────────────────────────────────

main() {
    echo ""
    echo "${BOLD}╔══════════════════════════════════════════════════════════╗${NC}"
    echo "${BOLD}║       LLM Neurosurgeon — Build & Installer Generator    ║${NC}"
    echo "${BOLD}╚══════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # ── Check dependencies ────────────────────────────────────────────────────
    header "Checking system dependencies"

    all_ok=1

    check_cmd cargo "Rust (rustup)" || all_ok=0
    check_cmd node "Node.js" || all_ok=0
    check_cmd pnpm "pnpm" || all_ok=0
    check_cmd python3 "Python 3" || all_ok=0

    check_system_libs
    check_versions

    if [ "${all_ok}" -eq 0 ]; then
        echo ""
        err "One or more required dependencies are missing."
        err "Install them and re-run this script."
        exit 1
    fi

    echo ""
    ok "All required dependencies found."

    # ── Build steps ───────────────────────────────────────────────────────────
    if [ "${BUILD_CLI}" -eq 1 ]; then
        build_cli
    else
        info "Skipping CLI build (--skip-cli)"
    fi

    if [ "${BUILD_DESKTOP}" -eq 1 ]; then
        build_desktop_frontend
        generate_icons
        build_tauri_app
        build_tauri_bundles
    else
        info "Skipping desktop build (--skip-desktop)"
    fi

    # ── Summary ───────────────────────────────────────────────────────────────
    print_summary
}

main "$@"
