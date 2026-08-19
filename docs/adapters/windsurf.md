# Windsurf Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Windsurf**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `windsurf` |
| **Native Configs** | `.windsurfrules`, `mcp_config.json` |
| **Parsing Engine** | Markdown rules + MCP configuration JSON |
| **Projection Target** | Stamped `.windsurfrules` |
| **Symlink Support** | Direct symlink where supported, fallback to stamped generated file |

---

## 📥 Ingestion & Projection Strategy

- Reads prompt conventions from `.windsurfrules` into `~/AIBrain/rules/global.md`.
- Ingests MCP tool configurations from `mcp_config.json`.

---

[⬅️ Back to Adapters Overview](README.md)
