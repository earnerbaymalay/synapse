# Onboarding — SYNAPSE / llm-neuro-surgeon

Four phases, in the order a new user actually experiences them. Nothing here is a feature list — it's a journey.

---

## Phase 1 — The hook

You have more than one AI coding tool installed. Each keeps its own rules file, in its own format, in its own place. You've already copy-pasted the same instructions into two or three of them this month.

That's the problem SYNAPSE exists to remove.

## Phase 2 — The solution

> **The Brain.** One canonical, git-backed directory — `~/AIBrain` — holding every skill, rule, agent and MCP server you use. Every tool on your machine reads from it, directly or through a generated file. Edit once. Every model stays equally skilled.

## Phase 3 — Immediate value (the three-command loop)

```bash
synapse scan             # what's installed, right now
synapse import --dry-run # what an import would do — zero writes
synapse import           # commit to it
```

Run `scan` first. It's read-only and takes seconds. `import --dry-run` shows you exactly what will move into the Brain before anything happens — expect it to report something like "7 skills, 6 agents, 0 MCP servers." Only then run `import` for real.

From here, `synapse project` pushes the Brain's contents back out to every tool it found. That's the whole loop: **scan → import → project.**

## Phase 4 — Long-term power

- **Time Machine** — every sync is a Git commit. `synapse snapshot "message"` to mark a point deliberately; `synapse rollback <hash>` to return to it.
- **Auto-Sync Daemon** — `synapse sync --daemon` watches the Brain and every tool, resolving drift with a 3-way merge as it happens.
- **Doctor** — `synapse doctor` diagnoses broken symlinks and config drift; `synapse doctor --fix` repairs what it safely can.
- **MCP Hub** — browse and health-check MCP servers from one place; secrets stay in the OS Keychain, never in a config file.
- **Marketplace** — pull skills and agents from any Git repo, with license cards and SHA-256 provenance attached.

---

**Next:** [docs/USER_GUIDE.md](USER_GUIDE.md) for the full command and GUI reference.
