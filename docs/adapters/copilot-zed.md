# GitHub Copilot & Zed Adapter Specifications
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Copilot & Zed**

---

## 📋 Overview

| Property | GitHub Copilot | Zed |
|---|---|---|
| **Adapter ID** | `github-copilot` | `zed` |
| **Native Configs** | `.github/copilot-instructions.md` | `.rules`, `.zed/settings.json` or `.config/zed/settings.json`, `.config/zed/AGENTS.md` |
| **Parsing Engine** | Markdown instruction parser | Markdown + JSON settings parser |
| **Projection Target** | `.github/copilot-instructions.md` | `.rules` + `.config/zed/AGENTS.md` |
| **Symlink Support** | None — always writes a stamped plain file | None — always writes stamped plain files |

---

## 📥 Ingestion & Projection Strategy

- **Copilot**: Ingests workspace instructions from `.github/copilot-instructions.md`.
- **Zed**: Ingests rules from `.rules`, multi-agent directives from `.config/zed/AGENTS.md`, and MCP/context-server entries from its settings.json — other settings keys are not parsed.

---

[⬅️ Back to Adapters Overview](README.md)
