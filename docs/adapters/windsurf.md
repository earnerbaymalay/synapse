# Windsurf Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Windsurf**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `windsurf` |
| **Native Configs** | `.windsurfrules`, `.codeium/windsurf/mcp.json` (project or `~/.codeium/windsurf/mcp.json`; also checks `~/.devin/mcp.json`) |
| **Parsing Engine** | Markdown rules + MCP configuration JSON |
| **Projection Target** | Stamped `.windsurfrules` |
| **Symlink Support** | None — always writes a stamped plain file |

---

## 📥 Ingestion & Projection Strategy

- Reads prompt conventions from `.windsurfrules` as a `windsurf-rules` skill (there is no `rules/` directory in the Brain today — see `.ai/MASTER_PROMPT.md`'s implementation-status note).
- Ingests MCP tool configurations from `.codeium/windsurf/mcp.json` (project-local, falling back to the user's global Codeium/Devin config).

---

[⬅️ Back to Adapters Overview](README.md)
