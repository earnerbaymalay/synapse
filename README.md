<div align="center">

# SYNAPSE
### llm-neuro-surgeon

**One Brain. All Models.**
Surgical precision. Zero friction.

A local-first desktop app and CLI that unifies the configuration of every AI coding tool on your machine into one canonical, git-backed **Brain** — then keeps every tool in sync with it, automatically.

[![CI](https://github.com/earnerbaymalay/llm-neuro-surgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/earnerbaymalay/llm-neuro-surgeon/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-1.0.0-1d9bf0?style=flat-square)](CHANGELOG.md)
[![Rust Tests](https://img.shields.io/badge/rust-211%2F211_passing-3fb950?style=flat-square)](packages/core)
[![E2E Tests](https://img.shields.io/badge/e2e-142%2F142_passing-3fb950?style=flat-square)](packages/e2e)
[![Stack](https://img.shields.io/badge/stack-Tauri_2_·_Rust_·_React-1d9bf0?style=flat-square)](#architecture)
[![License](https://img.shields.io/badge/license-MIT-d4af37?style=flat-square)](LICENSE)

**[Quickstart](#quickstart) · [User Guide](docs/USER_GUIDE.md) · [Adapters](#13-verified-adapters) · [Architecture](#architecture)**

</div>

---

### The problem

You use more than one AI coding tool — Claude Code for deep reasoning, Cursor for rapid iteration, Gemini CLI for refactors, Windsurf for flow, Copilot for autocomplete. Each one speaks its own config dialect: `CLAUDE.md`, `.cursorrules`, `GEMINI.md`, `AGENTS.md`, `.windsurfrules`. You are maintaining the same skills and rules N times, and every edit is a tax on your attention. Formats diverge. Nothing tells you what any given model actually knows.

### The solution

Four verbs, one Brain:

```
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

Claude Code / Desktop · Cursor · Windsurf · Gemini CLI · OpenAI Codex CLI · Zed · Cline · Continue · Aider · GitHub Copilot · OpenCode · Roo Code · Antigravity CLI

Each adapter has a verified research brief in [`docs/research/`](docs/research/).

## Canonical Brain layout

```
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

```
apps/desktop (Tauri 2 + React/TS)     apps/cli (Rust + Clap, binary: synapse)
              \                            /
               \                          /
                packages/core (Rust)
                scanner · adapters · projector · sync · doctor
                              |
              filesystem · git · OS keychain
```

## Quickstart

**Prerequisites:** Rust 1.75+, Node.js 20+, Git.

```bash
# CLI (package/binary name is `synapse`)
cargo run -p synapse -- scan
cargo run -p synapse -- import --dry-run
cargo run -p synapse -- import
cargo run -p synapse -- project
cargo run -p synapse -- sync --once
cargo run -p synapse -- doctor --fix

# Desktop app
cd apps/desktop && pnpm install && pnpm tauri dev
```

Full walkthrough: [docs/ONBOARDING.md](docs/ONBOARDING.md)

## Quality

211/211 Rust tests passing · 142/142 E2E tests passing (on a non-root runner — chmod-based write-protection tests can't pass as root) · `cargo clippy` clean · `cargo fmt` compliant, across Linux, macOS and Windows.

## Documentation

- [User Guide](docs/USER_GUIDE.md) — CLI, desktop GUI, MCP hub, Doctor, rollbacks
- [Onboarding](docs/ONBOARDING.md) — the four-phase quickstart journey
- [Adapter Authoring Guide](docs/ADAPTER_AUTHORING_GUIDE.md) — build a new adapter in Rust
- [Security Report](docs/security.md) — path safety, symlink defenses, sandboxing
- [Brand System](brands/synapse/) — identity, tokens, marketing pack

## Security & privacy

Zero telemetry. Local-first — every operation runs offline. API keys live in the OS Keychain; config files only ever reference them as `${VAR}` placeholders. All path joins are traversal- and symlink-escape safe.

## License

[MIT](LICENSE) © 2026 earnerbaymalay. Imported third-party skills and agents retain their upstream licenses — the Marketplace importer surfaces license cards and SHA-256 provenance for anything it ingests.
