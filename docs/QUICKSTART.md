# ⚡ Quickstart Guide

[Docs Hub](README.md) > **Quickstart**

Get up and running with **Synapse** in under 60 seconds.

---

## 📦 1. Prerequisites

* **Rust**: install [rustup](https://rustup.rs). `rust-toolchain.toml` pins Rust 1.97.1, and rustup installs that toolchain on the first build.
* **Node.js**: `v20+` & `pnpm` (required for Desktop GUI development)

### Linux (Ubuntu/Debian) System Dependencies
```bash
sudo apt-get update && sudo apt-get install -y \
  pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

---

## 🚀 2. Four-Step Launch

### Step 0: Install the CLI
```bash
cargo install --path apps/cli --locked
synapse --version
```
*To run without installing, replace `synapse` with `cargo run -p synapse --` in every command below.*

### Step 1: Detect Active AI Tools
```bash
synapse scan
```
*Output lists the AI tools found in your home directory.*

### Step 2: Import Configurations
```bash
synapse import --dry-run
synapse import
```
*Imports the detected configs into the Brain, a local Git repository, and commits a snapshot. The Brain is `~/AIBrain` by default, or `~/Desktop/AI BRAIN` when that directory exists and `~/AIBrain` does not. Set `SYNAPSE_BRAIN_PATH` to use another location.*

### Step 3: Project and Keep Tools in Step
```bash
synapse project --dry-run
synapse project
synapse sync --once
```
*`sync` runs one pass and exits. It does not watch files in the background. To keep tools current, schedule it, for example with `crontab -e`:*

```bash
*/5 * * * * /usr/local/bin/synapse sync >> "$HOME/.synapse-sync.log" 2>&1
```

> [!TIP]
> **Per-project runs:** the default tool root is your home directory. To project into the current project instead, set `SYNAPSE_TOOL_ROOT=. synapse scan`. `doctor` and `onboard` also accept `--tool-root <PATH>`.

### ⚡ Bonus: Synaptic Auto-Compression for Agent Commands
```bash
# Save 95% tokens when running verbose test suites inside AI coding agents:
synapse exec -- cargo test
synapse exec -- pnpm test
```

---

## 🖥️ 3. Launching the Desktop UI

To run the Tauri v2 Desktop GUI:

```bash
pnpm install
pnpm --filter desktop-app tauri dev
```

---

[⬅️ Back to Docs Hub](README.md) • [Proceed to User Guide ➡️](USER_GUIDE.md)
