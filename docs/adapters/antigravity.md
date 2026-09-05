# Antigravity CLI Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Antigravity CLI**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `agy-cli` |
| **Native Configs** | `AGENTS.md` (or `GEMINI.md` fallback), `.agents/skills/<name>/SKILL.md`, `.agents/agents/<slug>.md`, `.agents/mcp_config.json` |
| **Detected By** | `AGENTS.md` exists, or `.agents/` or `.gemini/config` is a directory |
| **Parsing Engine** | Skill/agent content is stored raw — no frontmatter parsing |
| **Projection Target** | `AGENTS.md` + `.agents/skills/` + `.agents/agents/` + `.agents/mcp_config.json` |
| **Symlink Support** | None — `project()` always writes plain files |

---

## 📥 Ingestion & Projection Strategy

- Ingests memory rules from `AGENTS.md` (falling back to `GEMINI.md`) into a
  single `agy-cli-memory` skill.
- Ingests custom skills from `.agents/skills/<name>/SKILL.md`.
- Ingests custom agents from `.agents/agents/<slug>.md`.
- Ingests MCP server entries from `.agents/mcp_config.json`'s `mcpServers`
  object.
- Projects canonical Brain skills/agents/MCP servers back to the same paths,
  writing plain files (no symlinking).

See `packages/core/src/adapters/agy_cli.rs` for the exact behavior.

---

[⬅️ Back to Adapters Overview](README.md)
