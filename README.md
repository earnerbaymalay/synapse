<div align="center">

```text
  ____  __   __ _   _    _    ____  ____  _____ 
 / ___| \ \ / /| \ | |  / \  |  _ \/ ___|| ____|
 \___ \  \ V / |  \| | / _ \ | |_) \___ \|  _|  
  ___) |  | |  | |\  |/ ___ \|  __/ ___) | |___ 
 |____/   |_|  |_| \_/_/   \_\_|   |____/|_____|
               LLM-NEURO-SURGEON
```

### **One Brain. All Models. Surgical Precision. Zero Friction.**

*The local-first configuration engine that keeps every AI coding tool on your machine in permanent lockstep.*

[![Live Site](https://img.shields.io/badge/Website-Live_Landing_Page-00F0FF?style=flat-square&logo=githubpages&logoColor=black)](https://earnerbaymalay.github.io/llm-neuro-surgeon/)
[![Docs Hub](https://img.shields.io/badge/Docs-Central_Index-10B981?style=flat-square&logo=gitbook&logoColor=white)](docs/README.md)
[![Rust 1.75+](https://img.shields.io/badge/Rust-1.75+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tauri v2](https://img.shields.io/badge/GUI-Tauri_v2-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)

[🌐 View Live Website](https://earnerbaymalay.github.io/llm-neuro-surgeon/) • [⚡ 60s Quickstart](docs/QUICKSTART.md) • [📚 Documentation Hub](docs/README.md) • [🔧 13 Tool Adapters](docs/adapters/README.md) • [🤝 Contributing](docs/development/CONTRIBUTING.md)

---

</div>

## 💡 What is Synapse (LLM-NeuroSurgeon)?

Every AI coding companion stores configuration in different formats and locations:
* **Claude Code**: `.claude/skills/`, `CLAUDE.md`, `.mcp.json`
* **Cursor**: `.cursorrules`, `.cursor/rules/*.mdc`
* **Gemini CLI**: `GEMINI.md`, `.gemini/settings.json`
* **Windsurf, Codex, Cline, Roo Code, Copilot, Zed**: all have their own proprietary schema.

**Synapse** scans your machine, imports everything into a unified Git repository at `~/AIBrain`, and projects changes bi-directionally across your entire toolchain.

```text
                     ┌──────────────┐
                     │ AI Toolchain │ (Claude, Cursor, Gemini, etc.)
                     └──────┬───────┘
                            │ synapse scan & import
                            ▼
                ┌─────────────────────────┐
                │   ~/AIBrain (Git Repo)  │ <── Single Source of Truth
                └───────────┬─────────────┘
                            │ synapse sync --daemon
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
      .cursorrules      CLAUDE.md       AGENTS.md
```

---

## ⚡ 60-Second Quickstart

### 1. Run the Diagnostic Scan
```bash
cargo run -p neurosurgeon -- scan
```

### 2. Ingest Rules into `~/AIBrain`
```bash
cargo run -p neurosurgeon -- import --dry-run
cargo run -p neurosurgeon -- import
```

### 3. Start the Real-Time Background Sync Daemon
```bash
cargo run -p neurosurgeon -- sync --daemon
```

*For complete desktop setup, Linux prerequisites, and binary installs, view the [Quickstart Guide](docs/QUICKSTART.md).*

---

## 🧭 Repository Documentation Index

All documentation is interconnected with unified navigation and breadcrumbs:

| Document | Purpose | Link |
| :--- | :--- | :--- |
| 🌐 **Live Website** | Interactive feature tour & landing page | [Open Web App](https://earnerbaymalay.github.io/llm-neuro-surgeon/) |
| 📚 **Docs Hub** | Central documentation navigation index | [docs/README.md](docs/README.md) |
| ⚡ **Quickstart** | Fast onboarding guide for macOS, Linux, Windows | [docs/QUICKSTART.md](docs/QUICKSTART.md) |
| 🏛️ **Architecture** | 3-Way Merge, Daemon, & Tauri IPC internals | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| 📖 **User Guide** | Day-to-day workflows, MCP tools & Keychain | [docs/USER_GUIDE.md](docs/USER_GUIDE.md) |
| 🔌 **Adapters Matrix** | Specifications for all 13 supported AI tools | [docs/adapters/README.md](docs/adapters/README.md) |
| 🩺 **Doctor Guide** | Automated troubleshooting and broken link repair | [docs/USER_GUIDE.md#synapse-doctor](docs/USER_GUIDE.md#synapse-doctor) |
| 🤝 **Contributing** | Monorepo conventions, test suites & CI setup | [docs/development/CONTRIBUTING.md](docs/development/CONTRIBUTING.md) |

---

## 🩺 Synapse Doctor Self-Healing

Verify health across symlinks, MCP configurations, and project rules in one command:

```bash
# Diagnose state parity
cargo run -p neurosurgeon -- doctor

# Fix all broken symlinks and missing configs automatically
cargo run -p neurosurgeon -- doctor --fix
```

---

<div align="center">
  <sub>Maintained by <a href="https://github.com/earnerbaymalay">earnerbaymalay</a>. Open source under the MIT License.</sub>
</div>
