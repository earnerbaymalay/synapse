# ⚡ Quickstart Guide

[Docs Hub](README.md) > **Quickstart**

Get up and running with **Synapse (LLM-NeuroSurgeon)** in under 60 seconds.

---

## 📦 1. Prerequisites

* **Rust**: `1.75+` ([rustup.rs](https://rustup.rs))
* **Node.js**: `v20+` & `pnpm` (required for Desktop GUI development)

### Linux (Ubuntu/Debian) System Dependencies
```bash
sudo apt-get update && sudo apt-get install -y \
  pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

---

## 🚀 2. Three-Step Launch

### Step 1: Detect Active AI Tools
```bash
cargo run -p synapse -- scan
```
*Output lists all discovered tools (Claude, Cursor, Gemini, Windsurf, Zed, etc.).*

### Step 2: Ingest Configurations
```bash
cargo run -p synapse -- import --dry-run
cargo run -p synapse -- import
```
*Creates `~/AIBrain` as a local Git repository and snapshots all existing prompt files.*

### Step 3: Run the Auto-Sync Watcher
```bash
cargo run -p synapse -- sync --daemon
```

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
