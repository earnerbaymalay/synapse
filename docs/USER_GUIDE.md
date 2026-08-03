# LLM Neurosurgeon — User Guide

> **Surgical precision. Zero friction.**  
> **One Brain. All Models.**  
> *The Minimalist Approach*

Welcome to **LLM Neurosurgeon** — the local-first desktop application and CLI tool that unifies the configuration of every AI coding tool on your machine into one canonical, git-backed **Brain**, and keeps every tool in sync automatically.

---

## Table of Contents

- [Phase 1: The Hook — Why Your AI Configs Are Bleeding Out](#phase-1-the-hook--why-your-ai-configs-are-bleeding-out)
- [Phase 2: The Solution — One Brain to Rule Them All](#phase-2-the-solution--one-brain-to-rule-them-all)
  - [The Canonical Brain Layout](#the-canonical-brain-layout)
  - [Supported AI Tools Matrix](#supported-ai-tools-matrix)
- [Phase 3: Immediate Value — Scan, Import, Project](#phase-3-immediate-value--scan-import-project)
  - [Quick Start](#quick-start)
  - [Universal Import & Scanning](#universal-import--scanning)
  - [Projection Engine & Policy Rules](#projection-engine--policy-rules)
  - [Graphical Desktop Interface](#graphical-desktop-interface)
- [Phase 4: Long-term Power — Time Machine, MCP Hub, Doctor](#phase-4-long-term-power--time-machine-mcp-hub-doctor)
  - [Auto-Sync Daemon & Time Machine](#auto-sync-daemon--time-machine)
  - [MCP Hub & Secrets Management](#mcp-hub--secrets-management)
  - [Vitals & Doctor Diagnostics](#vitals--doctor-diagnostics)
  - [Marketplace & Untrusted Skill Ingestion](#marketplace--untrusted-skill-ingestion)
  - [FAQ & Safety Guarantees](#faq--safety-guarantees)

---

# Phase 1: The Hook — Why Your AI Configs Are Bleeding Out

Picture this: it's 2 AM. You've just crafted the perfect system prompt for Claude Code — 47 lines of surgical precision that captures your entire Rust coding philosophy. You test it. It works. You close your laptop.

Next morning, you open Cursor to refactor a module. Your custom instructions are gone. Cursor doesn't know about your Rust conventions. You spend 20 minutes hunting down where you saved that prompt, then manually copy-paste it into `.cursorrules`. By Friday, you've done this dance five times — once for each tool. The prompts drift. The skills diverge. Your carefully curated AI personality fragments across a dozen config files, each in its own format, its own directory, its own reality.

> [!WARNING]
> **Configuration Fragmentation** — Every AI coding tool speaks its own config language. Claude Code reads `CLAUDE.md` and `.claude/skills/`. Cursor wants `.cursorrules`. Gemini CLI expects `GEMINI.md`. Aider looks for `CONVENTIONS.md`. OpenAI Codex parses `.codex/config.toml`. You maintain the same rules in five different formats, in five different places, and they *will* drift apart. This is the fragmentation tax — and you're paying it every day.

Over time, your custom instructions rot. Skills become outdated in some tools while updated in others. There is no single source of truth. Your AI tools are speaking different dialects, and you're the translator.

**There is a better way.**

---

# Phase 2: The Solution — One Brain to Rule Them All

LLM Neurosurgeon solves fragmentation by acting as a **universal neural bridge** — a single, canonical source of truth for every AI tool on your machine.

> [!TIP]
> **One Brain. All Models.** — Define your rules, skills, agents, memory, and MCP servers once in the Brain. Every tool on your machine reads from the same source. No drift. No duplication. No 2 AM copy-paste sessions.

Here's how it works:

1. **Scan** — Inspects your project or home directory for every existing AI tool configuration it can find.
2. **Import** — Ingests rules, skills, agents, memory, and MCP servers into a single canonical directory: **the Brain** (`~/AIBrain` by default).
3. **Project** — Emits the Brain back out to every tool on your machine — using symlinks where tolerated, or generated files stamped with provenance headers where necessary.
4. **Monitor & Sync** — Watches for changes bidirectionally using a debounced filesystem watcher and OS scheduler background sweeps.
5. **Track History** — Every sync is a Git commit, giving you a full **Time Machine** to inspect, diff, or roll back your AI configurations.

## The Canonical Brain Layout

The **Brain** is a human-readable, plain-text directory (defaulting to `~/AIBrain`, configurable via `$NEUROSURGEON_BRAIN` or settings). It is organized as follows:

```text
AIBrain/
├── skills/
│   └── <slug>/
│       ├── SKILL.md              # Instruction content & prompt definition
│       └── skill.yaml            # Metadata: id, version, triggers, targets, source, sha256
├── agents/
│   └── <slug>.md                 # Canonical agent definitions (frontmatter: tools, model, targets)
├── rules/
│   ├── global.md                 # System-wide global rules applicable to all tools
│   └── scoped/
│       └── <glob>.md             # Rules scoped to file patterns (e.g. *.rs, *.tsx)
├── memory/
│   ├── MEMORY.md                 # Main persistent project memory
│   └── topic/                    # Topic-specific memory notes
├── prompts/
│   └── <name>.md                 # Reusable prompt templates & custom commands
├── mcp/
│   └── servers/
│       └── <id>.yaml             # Transport (stdio/remote), command/url, env placeholders, targets
├── .brain/
│   ├── mappings.json             # Source ↔ Canonical ↔ Projection mappings & SHA256 hashes
│   ├── state.json                # Runtime state & sync locks
│   └── backups/                  # Pre-migration backup archives
└── .git/                         # Git repository for historical snapshot & rollback
```

> [!NOTE]
> Every file in the Brain is plain text. No binary blobs, no proprietary formats. Your configuration is yours — readable, diffable, and portable.

## Supported AI Tools Matrix

LLM Neurosurgeon includes **12 built-in adapters** covering major AI development tools:

| Tool ID | Tool Name | Primary Config Paths | Projection Mode |
|---|---|---|---|
| `claude-code` | Claude Code | `CLAUDE.md`, `.claude/skills/`, `.claude/agents/`, `.mcp.json` | Symlink / Direct write |
| `gemini-cli` | Gemini CLI | `GEMINI.md`, `.gemini/settings.json` | Merged JSON / Symlink |
| `openai-codex` | OpenAI Codex CLI | `.codex/config.toml` | Merged TOML |
| `cursor` | Cursor IDE | `.cursorrules`, `.cursor/rules/*.mdc` | Symlink / Frontmatter MDC |
| `windsurf` | Windsurf | `.windsurfrules`, `$HOME/.codeium/windsurf/mcp_config.json` | Symlink / Merged JSON |
| `cline` | Cline | `.clinerules`, `.vscode/mcp.json` | Symlink / Merged JSON |
| `roo-code` | Roo Code | `.roomodes` | Merged JSON Modes |
| `aider` | Aider | `CONVENTIONS.md`, `.aider.conf.yml` | Symlink / Merged YAML |
| `continue` | Continue | `.continue/rules/*.md`, `.continue/config.json` | Frontmatter MDC / Merged JSON |
| `github-copilot` | GitHub Copilot | `.github/copilot-instructions.md` | Generated markdown header |
| `zed` | Zed Editor | `.rules`, `.zed/settings.json` | Symlink / Merged JSON |
| `opencode` | OpenCode | `AGENTS.md` | Generated markdown header |

---

# Phase 3: Immediate Value — Scan, Import, Project

You don't need to read the rest of this guide to get value. Here's your 5-minute path to a unified Brain.

## Quick Start

### Prerequisites

- **Rust**: 1.75+ (installed via `rustup`)
- **Node.js**: 20+ and `pnpm` (or `npm`)
- **Git**: Installed and available in your `$PATH`
- **System Dependencies (Linux)**: `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libssl-dev`, `libdbus-1-dev`, `libjavascriptcoregtk-4.1-dev`, `libsoup-3.0-dev`

### Build from Source

```bash
# Clone the repository
git clone https://github.com/earnerbaymalay/llm-neuro-surgeon.git
cd llm-neuro-surgeon

# Build the Rust workspace (CLI + Core + Desktop Backend)
cargo build --workspace --release
```

### The 3-Command Onboarding Loop

```bash
# 1. See what's out there
cargo run -p neurosurgeon -- scan

# 2. Preview what would be imported (zero writes)
cargo run -p neurosurgeon -- import --dry-run

# 3. When you're ready, run import for real
cargo run -p neurosurgeon -- import
```

> [!TIP]
> **Dry-run is your safety net.** The `--dry-run` flag reports every file that would be ingested without touching your disk. Run it early, run it often.

### Desktop GUI (Optional)

```bash
# Install frontend dependencies and start Vite dev server
cd apps/desktop
pnpm install
pnpm dev

# Or launch the Tauri application wrapper
pnpm tauri dev
```

## Universal Import & Scanning

### Scanning for Tools

Run `neurosurgeon scan` to inspect your current directory or host environment for supported AI tool configurations:

```bash
$ neurosurgeon scan
Detected 4 AI tool configuration(s):
  • claude-code   (CLAUDE.md, .claude/agents/coder.md)
  • cursor        (.cursorrules, .cursor/rules/rust.mdc)
  • gemini-cli    (GEMINI.md)
  • zed           (.rules)
```

To format output as JSON for scripting:
```bash
$ neurosurgeon scan --json
```

### Ingesting Configs (Dry Run)

Before modifying your filesystem, run an import dry-run to preview what will be ingested into the Brain:

```bash
$ neurosurgeon import --dry-run
Migration Report (Dry Run):
  Skills found:     5
  Agents found:     2
  Rules found:      4
  MCP Servers:      1
  Status: Dry run clean — 0 files written to disk.
```

## Projection Engine & Policy Rules

Once configs are stored in the Brain, the **Projection Engine** emits them back to each tool's preferred location.

### Projection Modes

1. **Symlink Candidate (`Symlink`)**:
   Used when a tool reads standard, isolated Markdown files (e.g. `.cursorrules`, `.rules`, `CLAUDE.md`). LLM Neurosurgeon creates a relative or absolute symlink directly pointing to the canonical file in `AIBrain/rules/`.
   *Windows Fallback*: When symlink creation fails due to unprivileged user mode, the projection engine falls back to Directory Junctions, Hardlinks, or Copy+Watch.

2. **Merged Configuration (`Generate / Merge`)**:
   Used when a tool stores AI rules or MCP servers inside a multi-purpose configuration file (such as `.zed/settings.json` or `.aider.conf.yml`). LLM Neurosurgeon reads the existing file, updates only the AI-managed sections (preserving all unrelated user settings), and writes the file back.

3. **Generated Header Stamping**:
   Generated files written by LLM Neurosurgeon include a top-level provenance header:
   ```markdown
   <!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->
   ```
   This prevents accidental manual edits in target files and alerts users to edit the Brain source file instead.

## Graphical Desktop Interface

The Tauri-powered Desktop GUI provides an intuitive visual management console divided into 8 core screens:

1. **Main Dashboard (Vitals)**: Overview of Brain health, total skills/agents/rules, capability coverage matrix (Tool × Capability), and quick sync status.
2. **Configuration Manager**: Tree view and editor for skills, rules, agents, and prompts with tool target toggles.
3. **Adapter Inspector**: Detailed status of all 12 tool adapters, detected file paths, active projection policies, and drift status.
4. **Status Monitor**: Real-time sync event log, watcher status, active background schedules, and file change indicators.
5. **Debug Console**: Monospace diagnostic logs, raw IPC payload inspector, and daemon status controls.
6. **Onboarding Wizard**: Step-by-step guided setup (Environment Select → Scan & Dry Run Report → Brain Creation & Tool Link).
7. **Marketplace**: Browse community skills (`anthropics/skills`), view diff previews, provenance metadata, and security warnings.
8. **MCP Hub**: Search MCP registries, toggle active servers, view health check indicators, and manage Keychain credentials.

---

# Phase 4: Long-term Power — Time Machine, MCP Hub, Doctor

Once your Brain is live, these are the tools that keep it healthy, secure, and evolving.

## Auto-Sync Daemon & Time Machine

### Background Watching & Schedulers

LLM Neurosurgeon runs a debounced background filesystem watcher (`notify` crate) combined with native OS background schedulers:
- **macOS**: `launchd` plist (`~/Library/LaunchAgents/com.llmneurosurgeon.sync.plist`)
- **Linux**: `systemd` user unit (`~/.config/systemd/user/llm-neurosurgeon-sync.timer`)
- **Windows**: Task Scheduler (`schtasks /create /tn "LLMNeurosurgeonSync" ...`)

### Three-Way Merge Engine

When both the Brain and a target tool config are modified simultaneously, LLM Neurosurgeon executes a 3-way merge using the `diffy` engine:
- **Disjoint Markdown Edits**: Merged automatically without user intervention.
- **Overlapping Conflicts**: Pushed to the **Conflict Queue** in the Desktop GUI or CLI for manual resolution. No file is corrupted or overwritten during a conflict.

### Time Machine (Git Snapshots & Rollback)

Every sync operation automatically creates a Git commit inside `AIBrain/.git`. You can take manual snapshots or instantly restore your entire AI configuration to any previous state:

```bash
# Record a snapshot with a custom message
$ neurosurgeon snapshot "Updated Rust coding conventions"
Recorded snapshot a1b2c3d: Updated Rust coding conventions

# Roll back to a previous commit or tag
$ neurosurgeon rollback a1b2c3d
Rolled back Brain state to snapshot a1b2c3d. Working tree restored byte-identically.
```

> [!NOTE]
> **Snapshot Before Destroy** — A Git snapshot is automatically committed prior to any destructive operation. You can always go back.

## MCP Hub & Secrets Management

The **MCP Hub** centralizes Model Context Protocol (MCP) server management across all installed AI tools.

### Registry Integration & Health Checks

- **Registry Search**: Connects to official MCP registries (`registry.modelcontextprotocol.io`) to discover servers.
- **Health Check Handshake**:
  - **Stdio Transport**: Spawns the server binary and performs a JSON-RPC `initialize` handshake over stdin/stdout, verifying responsiveness.
  - **Remote Transport**: Sends HTTP POST/SSE handshake requests to remote MCP endpoints.

### OS Keychain Integration & Env Placeholders

LLM Neurosurgeon ensures API keys and secrets are **never hardcoded in plain-text configuration files**:
1. When importing MCP configs, secret values are harvested into the OS Keychain (Gnome Keyring on Linux, Security.framework on macOS, Credential Manager on Windows).
2. The config file written to disk contains environment placeholders:
   ```json
   {
     "mcpServers": {
       "github": {
         "command": "npx",
         "args": ["-y", "@modelcontextprotocol/server-github"],
         "env": {
           "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_PERSONAL_ACCESS_TOKEN}"
         }
       }
     }
   }
   ```
3. At execution runtime, environment variables are dynamically injected from the Keychain.

> [!WARNING]
> Secrets are **never** written to disk in plain text. They live in your OS Keychain. Config files contain only environment variable placeholders.

## Vitals & Doctor Diagnostics

The **Doctor Engine** continuously monitors your Brain and tool projections for drift, broken symlinks, checksum mismatches, or missing files.

### Running Doctor Diagnoses

Run `neurosurgeon doctor` to analyze system health:

```bash
$ neurosurgeon doctor
[WARN]  missing-projection: Projection for 'cursor' rule 'global.md' missing at .cursorrules
[INFO]  detached-symlink: Symlink .rules does not point to AIBrain/rules/global.md
[HINT]  Run 'neurosurgeon doctor --fix' to automatically resolve fixable issues.
```

### Automated Repair

Run `neurosurgeon doctor --fix` to automatically repair all fixable diagnoses:

```bash
$ neurosurgeon doctor --fix
Recreated missing symlink: .cursorrules -> ~/AIBrain/rules/global.md
Re-projected updated generated rule: .github/copilot-instructions.md
Updated mappings.json checksums.
Brain health restored: 0 critical errors remaining.
```

## Marketplace & Untrusted Skill Ingestion

LLM Neurosurgeon allows importing community-created skills and agents from repositories like `anthropics/skills` or external Git URLs.

### Safety Model for Untrusted Skills

To protect your environment against prompt injection or malicious code execution:
1. **Disabled by Default**: Ingested marketplace skills are set to `enabled: false` upon import.
2. **Provenance Tracking**: Each imported skill stores its original source URL, author, and license note in `skill.yaml`.
3. **SHA-256 Checksums**: Content hashes are recorded and verified upon import.
4. **Executable Content Inspection**: Skills containing scripts (`.sh`, `.py`, `.js`, `.bin`) are flagged with an **Executable Content** warning card in the UI before activation.

> [!WARNING]
> Marketplace skills are **disabled by default** and require explicit activation. Every skill carries provenance metadata and SHA-256 checksums so you know exactly what you're importing and where it came from.

## FAQ & Safety Guarantees

### Safety Commitments

- **No Telemetry**: LLM Neurosurgeon does not collect, transmit, or report telemetry data.
- **Offline First**: All scanning, importing, projecting, and sync operations take place locally on your machine. External network access is strictly limited to explicit user actions (fetching marketplace skills or checking MCP registries).
- **Snapshot Before Destroy**: A Git snapshot is automatically committed prior to any destructive operation, ensuring full rollback capability.
- **Dry-Run Default**: Initial import and projection operations default to dry-run reporting until explicitly confirmed.

### Common Questions

**Q: Will LLM Neurosurgeon modify my existing tool configurations without asking?**  
A: No. Initial operations require explicit confirmation or `--dry-run` review. When projections are created, existing user files are backed up to `AIBrain/.brain/backups/`.

**Q: What happens if I edit a rule file directly in Cursor instead of in the Brain?**  
A: If the file is a symlink, your edit modifies the file in the Brain directly! If it is a generated file, the background watcher detects the change, performs a 3-way merge into the Brain, and re-projects the result to all other tools.

**Q: How do I change the default Brain directory?**  
A: Set the `$NEUROSURGEON_BRAIN` environment variable or specify `--brain /path/to/custom/brain` in CLI commands.

**Q: Is this safe for production use?**  
A: Yes. Every destructive operation is preceded by a Git snapshot. The default mode is dry-run. No telemetry. No network calls unless you explicitly trigger them. Your configs never leave your machine.
