# Cursor MDC Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Cursor**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `cursor` |
| **Native Configs** | `.cursorrules`, `.cursor/rules/*.mdc` |
| **Parsing Engine** | YAML Frontmatter + Scoped File Globs (`globs: ["*.rs"]`) |
| **Projection Target** | `.cursor/rules/*.mdc` (MDC rule files) |
| **Symlink Support** | None — always writes stamped plain files |

---

## 📥 Ingestion & Projection Strategy

- Converts legacy `.cursorrules` into `~/AIBrain/rules/global.md`.
- Parses `.cursor/rules/*.mdc` frontmatter into scoped rules with file matching patterns.
- Projects canonical Brain rules as stamped plain files into `.cursor/rules/` (no symlinking).

---

[⬅️ Back to Adapters Overview](README.md)
