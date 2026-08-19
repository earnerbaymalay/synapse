# Antigravity CLI Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Antigravity CLI**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `agy-cli` |
| **Native Configs** | `AGENTS.md`, `.agy/skills/`, `.gemini/settings.json` |
| **Parsing Engine** | Markdown agents + YAML frontmatter skill parser |
| **Projection Target** | `AGENTS.md` + `.agy/skills/` |
| **Symlink Support** | Direct symlink for `.agy/skills/` |

---

## 📥 Ingestion & Projection Strategy

- Ingests Antigravity skills from `.agy/skills/<slug>/SKILL.md` into `~/AIBrain/skills/<slug>/`.
- Ingests agent definitions from `AGENTS.md`.
- Projects canonical Brain skills into Antigravity workspaces.

---

[⬅️ Back to Adapters Overview](README.md)
