# Cline & Roo Code Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Cline & Roo Code**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter IDs** | `cline`, `roo-code` |
| **Native Configs** | `.clinerules`, `.roomodes`, `cline_mcp_settings.json` |
| **Parsing Engine** | Markdown rules + Custom Mode JSON schema |
| **Projection Target** | Stamped `.clinerules` + `.roomodes` |
| **Symlink Support** | Direct symlink for `.clinerules` |

---

## 📥 Ingestion & Projection Strategy

- Extracts agent modes from `.roomodes` into `~/AIBrain/agents/<slug>.md`.
- Extracts system prompts from `.clinerules` into `~/AIBrain/rules/global.md`.

---

[⬅️ Back to Adapters Overview](README.md)
