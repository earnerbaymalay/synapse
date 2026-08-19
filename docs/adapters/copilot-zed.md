# GitHub Copilot & Zed Adapter Specifications
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Copilot & Zed**

---

## 📋 Overview

| Property | GitHub Copilot | Zed |
|---|---|---|
| **Adapter ID** | `github-copilot` | `zed` |
| **Native Configs** | `.github/copilot-instructions.md` | `.rules`, `.zed/settings.json`, `AGENTS.md` |
| **Parsing Engine** | Markdown instruction parser | Markdown + JSON settings parser |
| **Projection Target** | `.github/copilot-instructions.md` | `.rules` + `AGENTS.md` |
| **Symlink Support** | Direct symlink supported | Direct symlink for `.rules` |

---

## 📥 Ingestion & Projection Strategy

- **Copilot**: Ingests workspace instructions from `.github/copilot-instructions.md`.
- **Zed**: Ingests rules from `.rules` and multi-agent directives from `AGENTS.md`.

---

[⬅️ Back to Adapters Overview](README.md)
