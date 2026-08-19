# Gemini CLI Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Gemini CLI**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `gemini` |
| **Native Configs** | `GEMINI.md`, `.gemini/settings.json` |
| **Parsing Engine** | Markdown system prompt + JSON settings parser |
| **Projection Target** | Stamped `GEMINI.md` + merged `.gemini/settings.json` |
| **Symlink Support** | None (uses generated files with provenance comments) |

---

## 📥 Ingestion & Projection Strategy

- Reads instructions from `GEMINI.md` into `~/AIBrain/rules/global.md`.
- Safely merges MCP and tool settings into `.gemini/settings.json`.

---

[⬅️ Back to Adapters Overview](README.md)
