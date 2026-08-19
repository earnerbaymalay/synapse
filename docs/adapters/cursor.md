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
| **Symlink Support** | Full symlink support for individual `.mdc` files |

---

## 📥 Ingestion & Projection Strategy

- Converts legacy `.cursorrules` into `~/AIBrain/rules/global.md`.
- Parses `.cursor/rules/*.mdc` frontmatter into scoped rules with file matching patterns.
- Symlinks canonical Brain rules directly into `.cursor/rules/`.

---

[⬅️ Back to Adapters Overview](README.md)
