<div align="center">

<img src="./assets/hero.svg" width="100%" alt="SYNAPSE // LLM-NeuroSurgeon Hero Banner">

[![Live Site](https://img.shields.io/badge/Live_Site-GitHub_Pages-00f0ff?style=flat-square&logo=github)](https://earnerbaymalay.github.io/llm-neuro-surgeon/)
[![Tests](https://img.shields.io/github/actions/workflow/status/earnerbaymalay/llm-neuro-surgeon/ci.yml?branch=main&label=tests&style=flat-square)](https://github.com/earnerbaymalay/llm-neuro-surgeon/actions)
[![GitHub release](https://img.shields.io/github/v/release/earnerbaymalay/llm-neuro-surgeon?style=flat-square)](https://github.com/earnerbaymalay/llm-neuro-surgeon/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Zero Telemetry](https://img.shields.io/badge/telemetry-zero-10B981?style=flat-square)](docs/security.md)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-dea584?style=flat-square&logo=rust)](https://www.rust-lang.org/)

**[🌐 Live Site](https://earnerbaymalay.github.io/llm-neuro-surgeon/)** • **[📚 Docs Hub](docs/README.md)** • **[⚡ Quickstart](docs/QUICKSTART.md)** • **[📖 User Guide](docs/USER_GUIDE.md)** • **[🔌 13 Adapters](docs/adapters/README.md)** • **[🏛️ Architecture](docs/ARCHITECTURE.md)** • **[🤝 Contributing](docs/development/CONTRIBUTING.md)**

---

</div>

## 💡 What is SYNAPSE (LLM-NeuroSurgeon)?

**SYNAPSE (LLM-NeuroSurgeon)** is the local-first configuration engine and synchronizer that keeps Claude Code, Cursor, Gemini CLI, Windsurf, Zed, and 8+ other AI coding companions in permanent lockstep.

### 🩺 Clinical Operating Identity
A developer's AI tooling environment is treated like a vital clinical system:
- **The Organ (`~/AIBrain`)**: A single, Git-backed source of truth holding every skill, rule, agent, and MCP server.
- **The Grafts (13 Adapters)**: Lossless bi-directional translators that project canonical rules into each tool's native dialect.
- **The Surgeon (`synapse`)**: Accountable for every modification — with dry-run verification, pre-op snapshots, and instant rollbacks.
- **The Doctor (`synapse doctor`)**: Continuous diagnostic self-healing to detect configuration drift and heal broken symlinks.

```text
                     ┌────────────────────────┐
                     │       ~/AIBrain        │
                     │  (Single Source Truth) │
                     └───────────┬────────────┘
                                 │
        ┌──────────────┬─────────┴────────┬──────────────┐
        ▼              ▼                  ▼              ▼
   Claude Code       Cursor           Gemini CLI      Windsurf / Zed
  (`.claude/`)   (`.cursorrules`)    (`GEMINI.md`)    (`.windsurfrules`)
```

---

## ⚡ 60-Second Quickstart

```bash
# 1. Detect active AI coding tools on your workstation
synapse scan

# 2. Ingest configurations into ~/AIBrain (Git-backed repository)
synapse import --dry-run
synapse import

# 3. Project Brain configurations out to all tools
synapse project

# 4. Launch background auto-sync daemon with 3-way merge resolution
synapse sync --daemon
```

> [!TIP]
> You can also run the CLI via Cargo in development: 
`cargo run -p synapse -- scan`.  

For full setup prerequisites across Linux, macOS, and Windows, read the **[Quickstart Guide](docs/QUICKSTART.md)**.

---

## 🔌 13 Verified Tool Adapters

| Tool | Specification | Config Location | Format | Import Capabilities |
|---|---|---|---|---|
| **Claude Code / Desktop** | [Spec](docs/adapters/claude.md) | `CLAUDE.md`, `.claude/skills/`, `.mcp.json` | Markdown, JSON | Skills, Agents, MCP Servers |
| **Cursor** | [Spec](docs/adapters/cursor.md) | `.cursorrules`, `.cursor/rules/*.mdc` | MDC Frontmatter | MDC Rules & Globs |
| **Gemini CLI** | [Spec](docs/adapters/gemini.md) | `GEMINI.md`, `.gemini/settings.json` | Markdown, JSON | Rules, Settings |
| **Windsurf** | [Spec](docs/adapters/windsurf.md) | `.windsurfrules`, `mcp_config.json` | Text, JSON | Rules & MCP Servers |
| **Cline** | [Spec](docs/adapters/cline.md) | `.clinerules`, `cline_mcp_settings.json` | Text, JSON | Custom Rules & MCP |
| **Roo Code** | [Spec](docs/adapters/roo-code.md) | `.roomodes`, `.clinerules` | JSON, Text | Mode Rules |
| **Aider** | [Spec](docs/adapters/aider.md) | `CONVENTIONS.md`, `.aider.conf.yml` | Markdown, YAML | Conventions & Config |
| **Continue** | [Spec](docs/adapters/continue.md) | `.continue/rules/*.md`, `.continue/config.json` | MDC, JSON | MDC Rules & Config |
| **GitHub Copilot** | [Spec](docs/adapters/github-copilot.md) | `.github/copilot-instructions.md` | Markdown | Scoped Instructions |
| **Zed** | [Spec](docs/adapters/zed.md) | `.rules`, `.zed/settings.json`, `AGENTS.md` | Text, JSON, Markdown | Settings & Rules |
| **OpenAI Codex CLI** | [Spec](docs/adapters/openai-codex.md) | `.codex/config.toml`, `.codex/instructions.md` | TOML, Markdown | Rules, Config |
| **OpenCode** | [Spec](docs/adapters/opencode.md) | `AGENTS.md` | Markdown | Multi-Agent Rules |
| **Antigravity CLI (AGY)** | [Spec](docs/adapters/antigravity.md) | `AGENTS.md`, `.agy/skills/`, `.gemini/` | Markdown, YAML | Skills & Settings |

---

## 🩺 The Doctor: Self-Healing Configurations

When tool configurations drift or symlinks break, Synapse detects and repairs the issue automatically:

```bash
# Diagnose configuration drift & broken symlinks
synapse doctor

# Apply automatic remediation
synapse doctor --fix
```

---

## 📚 Documentation Index

| Guide | Description | Target |
|---|---|---|
| **[Docs Hub](docs/README.md)** | Centralized documentation navigation & command reference | All users & contributors |
| **[Quickstart](docs/QUICKSTART.md)** | Step-by-step setup in under 60 seconds | First-time setup |
| **[User Guide](docs/USER_GUIDE.md)** | Day-to-day workflow, daemon sync, MCP hub & Doctor self-healing | Daily development |
| **[Onboarding Journey](docs/ONBOARDING.md)** | 4-phase journey from fragmented configs to permanent lockstep | Getting started |
| **[Architecture](docs/ARCHITECTURE.md)** | 3-way merge engine, file system watcher & monorepo layout | Engine internals |
| **[Adapters Hub](docs/adapters/README.md)** | Complete matrix and individual adapter specifications | Tool dialect reference |
| **[Security Audit](docs/security.md)** | Threat model, path traversal defense, and OS Keychain | Security & compliance |
| **[Contributing](docs/development/CONTRIBUTING.md)** | PR lifecycle, test requirements & coding standards | Open source contributors |
| **[Release Packaging](docs/packaging/RELEASE_PACKAGING.md)** | Tauri v2 desktop installers (.deb, .dmg, .msi) & scripts | Release engineering |

---

<div align="center">
<sub>Built with Rust, Tauri 2, and React. Open source under the MIT License.</sub>
</div>

