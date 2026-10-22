# Antigravity CLI Adapter Specification
### Synapse

[Docs Hub](../README.md) > [Adapters](README.md) > **Antigravity CLI**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `agy-cli` |
| **Native Configs** | `AGENTS.md`, `.agents/skills/`, `.agents/agents/`, `.agents/mcp_config.json`, `.gemini/config/` |
| **Parsing Engine** | Markdown agents + YAML frontmatter skill parser |
| **Projection Target** | `AGENTS.md` + `.agents/skills/`, `.agents/agents/`, `.agents/mcp_config.json` |
| **Symlink Support** | None. Skills, agents, and MCP config are written as files. |

---

## 📥 Ingestion & Projection Strategy

- Ingests skills from `.agents/skills/<slug>/SKILL.md` into the Brain's `skills/<slug>/`. Directories and `SKILL.md` files that are symlinks are skipped.
- Ingests agent definitions from `.agents/agents/` and memory from `AGENTS.md`.
- Ingests MCP servers from `.agents/mcp_config.json`.
- Under a `.gemini/config/` root, the same layout applies with `skills/`, `sidecars/`, and `mcp_config.json` relative to that root.
- Projects Brain skills, agents, and MCP servers into `.agents/`.

---

[⬅️ Back to Adapters Overview](README.md)
