# LLM Neurosurgeon — User Guide

Welcome to **LLM Neurosurgeon**, the local-first desktop application and CLI tool that unifies the configuration of every AI coding tool on your machine into one canonical, git-backed "Brain" — and keeps every tool in sync automatically.

---

## Table of Contents

- [1. Overview & Core Philosophy](#1-overview--core-philosophy)
- [2. Installation & Quickstart](#2-installation--quickstart)
- [3. The Canonical Brain Layout](#3-the-canonical-brain-layout)
- [4. Supported AI Tools Matrix](#4-supported-ai-tools-matrix)
- [5. Universal Import & Scanning](#5-universal-import--scanning)
- [6. Projection Engine & Policy Rules](#6-projection-engine--policy-rules)
- [7. Auto-Sync Daemon & Time Machine](#7-auto-sync-daemon--time-machine)
- [8. Marketplace & Untrusted Skill Ingestion](#8-marketplace--untrusted-skill-ingestion)
- [9. MCP Hub & Secrets Management](#9-mcp-hub--secrets-management)
- [10. Vitals & Doctor Diagnostics](#10-vitals--doctor-diagnostics)
- [11. Graphical Desktop Interface](#11-graphical-desktop-interface)
- [12. FAQ & Safety Guarantees](#12-faq--safety-guarantees)

---

## 1. Overview & Core Philosophy

Developers using multiple AI coding assistants (such as Claude Code, Cursor, Gemini CLI, Aider, GitHub Copilot, or Windsurf) often face a frustrating problem: **configuration fragmentation**.

Each tool uses its own format and location for system prompts, rule files, custom agents, memory, and MCP server configurations:
- Claude Code uses `CLAUDE.md`, `.claude/skills/`, and `.claude/agents/`
- Gemini CLI uses `GEMINI.md` and `.gemini/settings.json`
- Cursor uses `.cursorrules` and `.cursor/rules/*.mdc`
- Aider uses `CONVENTIONS.md` and `.aider.conf.yml`
- OpenAI Codex uses `.codex/config.toml`

Over time, your custom instructions drift, skills become outdated in some tools while updated in others, and there is no single source of truth.

**LLM Neurosurgeon solves this by acting as a universal neural bridge**:
1. **Scans** your project or home directory for existing tool configurations.
2. **Imports** rules, skills, agents, memory, and MCP servers into a single canonical directory: **the Brain** (`~/AIBrain` by default).
3. **Projects** the Brain back out to every tool on your machine — using symlinks where tolerated or generated files stamped with provenance headers where necessary.
4. **Monitors & Syncs** changes bidirectionally using a debounced filesystem watcher and OS scheduler background sweeps.
5. **Tracks History** via Git commits for every sync, giving you a full "Time Machine" to inspect, diff, or roll back your AI configurations.

---

## 2. Installation & Quickstart

### Prerequisites

- **Rust**: 1.75+ (installed via `rustup`)
- **Node.js**: 20+ and `pnpm` (or `npm`)
- **Git**: Installed and available in your `$PATH`
- **System Dependencies (Linux)**: `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libssl-dev`

### Building from Source

```bash
# Clone the repository
git clone https://github.com/earnerbaymalay/llm-neuro-surgeon.git
cd llm-neuro-surgeon

# Build the Rust workspace (CLI + Core + Desktop Backend)
cargo build --workspace --release

# Run CLI scan
cargo run -p neurosurgeon -- scan

# Run CLI dry-run import
cargo run -p neurosurgeon -- import --dry-run
```

### Running the Desktop GUI

```bash
# Install frontend dependencies and start Vite dev server
cd apps/desktop
pnpm install
pnpm dev

# Or launch the Tauri application wrapper
pnpm tauri dev
```

---

## 3. The Canonical Brain Layout

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

---

## 4. Supported AI Tools Matrix

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

## 5. Universal Import & Scanning

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

---

## 6. Projection Engine & Policy Rules

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

---

## 7. Auto-Sync Daemon & Time Machine

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

---

## 8. Marketplace & Untrusted Skill Ingestion

LLM Neurosurgeon allows importing community-created skills and agents from repositories like `anthropics/skills` or external Git URLs.

### Safety Model for Untrusted Skills

To protect your environment against prompt injection or malicious code execution:
1. **Disabled by Default**: Ingested marketplace skills are set to `enabled: false` upon import.
2. **Provenance Tracking**: Each imported skill stores its original source URL, author, and license note in `skill.yaml`.
3. **SHA-256 Checksums**: Content hashes are recorded and verified upon import.
4. **Executable Content Inspection**: Skills containing scripts (`.sh`, `.py`, `.js`, `.bin`) are flagged with an **Executable Content** warning card in the UI before activation.

---

## 9. MCP Hub & Secrets Management

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

---

## 10. Vitals & Doctor Diagnostics

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

---

## 11. Graphical Desktop Interface

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

## 12. FAQ & Safety Guarantees

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
