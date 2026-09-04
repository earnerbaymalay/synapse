# Onboarding Guide — SYNAPSE
### llm-neuro-surgeon — Four-Phase Journey

Welcome to **SYNAPSE**. This guide walks you through the four phases of setting up and taking complete control of your AI tool configurations.

```mermaid
flowchart TD
    Phase1[Phase 1: The Hook\nIdentify fragmented tool configs] --> Phase2[Phase 2: The Solution\nInitialize canonical ~/AIBrain]
    Phase2 --> Phase3[Phase 3: Immediate Value\nExecute scan -> import -> project]
    Phase3 --> Phase4[Phase 4: Long-Term Power\nDaemon sync + Time Machine snapshots]
```

---

## Phase 1 — The Hook

You have more than one AI coding tool installed. Each tool keeps its own rules file, in its own format, in its own place. You've already copy-pasted the same instructions into two or three of them this month.

That's the problem SYNAPSE exists to remove.

> [!WARNING]
> Maintaining separate `CLAUDE.md`, `.cursorrules`, `GEMINI.md`, and `.windsurfrules` files leads to configuration drift, stale skills, and fragmented model knowledge across your workspace.

---

## Phase 2 — The Solution

> **The Brain.** One canonical, git-backed directory — `~/AIBrain` — holding every skill, rule, agent, and MCP server you use. Every tool on your machine reads from it, directly or through a generated file. Edit once. Every model stays equally skilled.

```text
~/AIBrain/
├── skills/<slug>/          # SKILL.md + skill.yaml
├── agents/<slug>.md        # Canonical agent definitions
├── rules/                  # global.md + scoped/<glob>.md
├── memory/                 # MEMORY.md + topic files
├── prompts/                # Reusable command templates
├── mcp/servers/<id>.yaml   # Transports & env placeholders
├── .brain/                 # Active state & mappings.json
└── .git/                   # Full Git history (Time Machine)
```

---

## Phase 3 — Immediate Value (The Three-Command Loop)

```bash
# 1. Discover installed tools (read-only)
synapse scan

# 2. Ingest existing configs (dry-run first)
synapse import --dry-run
synapse import

# 3. Project Brain back out to all tools
synapse project
```

> [!TIP]
> Run `synapse scan` first. It is read-only and takes seconds. `synapse import --dry-run` shows you exactly what will move into the Brain before anything happens — expect it to report something like:
> `"Ingesting 7 skills, 6 agents, 2 rules, 3 MCP servers into ~/AIBrain"`.

From here, `synapse project` pushes the Brain's contents back out to every tool it found. That's the whole loop: **scan → import → project.**

---

## Phase 4 — Long-Term Power

Once your Brain is initialized, unlock long-term features:

| Feature | Command | Benefits |
|---|---|---|
| ⏳ **Time Machine** | `synapse snapshot "msg"` | Every sync is a Git commit. Revert anytime with `synapse rollback <hash>`. |
| 🔄 **Sync** | `synapse sync` | Runs one import + project reconciliation pass over the Brain and all tools and exits. No watch/daemon mode yet. |
| 🩺 **Doctor Auto-Repair** | `synapse doctor --fix` | Diagnoses broken symlinks and config drift, repairing them automatically. |
| 🔌 **MCP Registry** | `synapse mcp search <query>`, `synapse mcp health <command-or-url>` | Search the official MCP registry; health-check a server yourself before trusting it — nothing is spawned without you naming it explicitly. |
| 🛒 **Marketplace** | `synapse marketplace search`, `synapse marketplace show <slug>` | Browse skills from `anthropics/skills` with SHA-256 provenance. Metadata only for now — installing into the Brain isn't wired up yet. |

---

> [!NOTE]
> Read the full [docs/USER_GUIDE.md](USER_GUIDE.md) for complete CLI, Desktop GUI, MCP Hub, and Doctor documentation.
