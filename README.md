<div align="center">

# SYNAPSE
### llm-neuro-surgeon: One Brain. All Models.

**Surgical precision. Zero friction.**

A local-first desktop app and CLI that unifies the configuration of every AI coding tool on your machine into one canonical, git-backed **Brain** — then keeps every tool in sync with it, automatically.

[![CI](https://github.com/earnerbaymalay/llm-neuro-surgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/earnerbaymalay/llm-neuro-surgeon/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-1.0.0-1d9bf0?style=flat-square)](CHANGELOG.md)
[![E2E Tests](https://img.shields.io/badge/e2e-142%2F142_passing-3fb950?style=flat-square)](packages/e2e)
[![Rust Tests](https://img.shields.io/badge/rust-179%2F179_passing-3fb950?style=flat-square)](packages/core)
[![Stack](https://img.shields.io/badge/stack-Tauri_2_·_Rust_·_React-1d9bf0?style=flat-square)](#architecture)
[![License](https://img.shields.io/badge/license-MIT-d4af37?style=flat-square)](LICENSE)

**[Quickstart](#quickstart) · [Onboarding](docs/ONBOARDING.md) · [User Guide](docs/USER_GUIDE.md) · [Adapters](#13-verified-adapters) · [Architecture](#architecture) · [Brand System](brands/synapse/)**

</div>

---

### The problem

You use more than one AI coding tool — Claude Code for deep reasoning, Cursor for rapid iteration, Gemini CLI for refactors, Windsurf for flow, Copilot for autocomplete, Antigravity CLI for agentic execution. Each one speaks its own config dialect: `CLAUDE.md`, `.cursorrules`, `GEMINI.md`, `AGENTS.md`, `.windsurfrules`. You are maintaining the same skills and rules N times, and every edit is a tax on your attention. Formats diverge. Nothing tells you what any given model actually knows.

### The solution

Four verbs, one Brain:

```text
synapse scan       discover every AI tool on this machine
synapse import     ingest configs losslessly into ~/AIBrain
synapse project    push the Brain back out to every tool
synapse sync       keep it all in lockstep, automatically
```

Edit once, in the Brain. Every model stays equally skilled. Every sync is a Git commit — a complete history you can browse, diff, and roll back.

---

## Feature pillars

| Pillar | What it does |
|---|---|
| **The Brain** | One canonical, git-backed directory (`~/AIBrain`) — the single source of truth for every model on your machine. |
| **Universal Import** | Losslessly ingests configs from 13 AI coding tools, each with a purpose-built adapter. |
| **Projection Engine** | Per-tool output: symlinks where tolerated, generated files where required, first-class `AGENTS.md` support. |
| **Auto-Sync Daemon** | Debounced filesystem watcher, 3-way merge, conflict queue. Every sync is a commit. |
| **Marketplace Import** | Ingest skills and agents from any Git repo, with license cards and SHA-256 provenance. |
| **MCP Hub** | Browse, search, and health-check MCP servers. Secrets live in the OS Keychain. |
| **Doctor** | A health matrix across every tool and capability, plus one-command auto-repair. |
| **Safety by Design** | Dry-run defaults, Git snapshots before destructive operations, zero telemetry. |

## 13 verified adapters

1. **Claude Code / Desktop** (`CLAUDE.md`, `.claude/skills/`, `.claude/agents/`, `.mcp.json`, `.claude/settings.json`)
2. **Gemini CLI** (`GEMINI.md`, `.gemini/settings.json`)
3. **OpenAI Codex CLI** (`.codex/config.toml`, `.codex/config.json`, `.codex/instructions.md`, `AGENTS.md`)
4. **Cursor** (`.cursorrules`, `.cursor/rules/*.mdc`)
5. **Windsurf** (`.windsurfrules`, `mcp_config.json`)
6. **Cline** (`.clinerules`, `cline_mcp_settings.json`, `.vscode/mcp.json`)
7. **Roo Code** (`.roomodes`, `.clinerules`)
8. **Aider** (`CONVENTIONS.md`, `.aider.conf.yml`)
9. **Continue** (`.continue/rules/*.md`, `.continue/config.json`)
10. **GitHub Copilot** (`.github/copilot-instructions.md`)
11. **Zed** (`.rules`, `.zed/settings.json`, `.config/zed/settings.json`, `.config/zed/AGENTS.md`)
12. **OpenCode** (`AGENTS.md`)
13. **Antigravity CLI / Agy CLI** (`AGENTS.md`, `.agy/skills/`, `.gemini/settings.json`)

Each adapter has a verified research brief in [`docs/research/`](docs/research/).

## Canonical Brain layout

```text
AIBrain/
├── skills/<slug>/          SKILL.md + skill.yaml
├── agents/<slug>.md        canonical agent definitions
├── rules/                  global.md + scoped/<glob>.md
├── memory/                 MEMORY.md + topic files
├── prompts/                reusable templates
├── mcp/servers/<id>.yaml   transport, env-placeholders, targets
├── .brain/                 mappings.json, state.json
└── .git/                   full history — the Time Machine
```

## Architecture

```text
apps/desktop (Tauri 2 + React/TS)     apps/cli (Rust + Clap binary: synapse / neurosurgeon)
              \                            /
               \                          /
                packages/core (Rust)
                scanner · adapters · projector · sync · doctor
                              |
              filesystem · git · OS keychain
```

## Quickstart

**Prerequisites:** Rust 1.75+, Node.js 20+, Git.  
*Linux packages:* `libdbus-1-dev`, `libjavascriptcoregtk-4.1-dev`, `libsoup-3.0-dev`, `pkg-config`.

### 4-Phase Quickstart Journey

```bash
# 1. Discover all installed AI tools
synapse scan              # or: cargo run -p neurosurgeon -- scan

# 2. Preview what would be imported (zero writes)
synapse import --dry-run  # or: cargo run -p neurosurgeon -- import --dry-run

# 3. Import configurations into ~/AIBrain and project out
synapse import            # or: cargo run -p neurosurgeon -- import
synapse project           # or: cargo run -p neurosurgeon -- project

# 4. Run background auto-sync daemon and health diagnostic
synapse sync --daemon     # or: cargo run -p neurosurgeon -- sync --daemon
synapse doctor --fix      # or: cargo run -p neurosurgeon -- doctor --fix

# Desktop app (React UI overhaul)
cd apps/desktop && pnpm install && pnpm tauri dev
```

Full 4-phase walkthrough: [docs/ONBOARDING.md](docs/ONBOARDING.md)

## Quality

142/142 (100%) E2E tests passing · 179/179 (100%) Rust tests passing · `cargo clippy` clean · `cargo fmt` compliant across Linux, macOS, and Windows.

## Documentation

- [Onboarding Journey](docs/ONBOARDING.md) — the 4-phase quickstart journey
- [User Guide](docs/USER_GUIDE.md) — CLI binary aliases (`synapse` / `neurosurgeon`), React desktop GUI, MCP hub, Doctor, rollbacks
- [Adapter Authoring Guide](docs/ADAPTER_AUTHORING_GUIDE.md) — build a new tool adapter in Rust
- [Security Report](docs/security.md) — path safety, symlink defenses, sandboxing, threat-model pass
- [Brand System](brands/synapse/) — SYNAPSE identity, design tokens, marketing pack, brand book

## Security & privacy

Zero telemetry. Local-first — every operation runs offline. API keys live in the OS Keychain; config files only ever reference them as `${VAR}` placeholders. All path joins are traversal- and symlink-escape safe.

## License

[MIT](LICENSE) © 2026 earnerbaymalay.

