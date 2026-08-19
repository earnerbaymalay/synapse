# Continue Adapter Specification
### Synapse // LLM-NeuroSurgeon

[Docs Hub](../README.md) > [Adapters](README.md) > **Continue**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `continue` |
| **Native Configs** | `.continue/rules/*.md`, `.continue/config.json`, `.continue/prompts/*.prompt` |
| **Parsing Engine** | MDC Markdown frontmatter + JSON config parser |
| **Projection Target** | Stamped `.continue/rules/*.md` |
| **Symlink Support** | Full symlink support for individual rule files |

---

## 📥 Ingestion Strategy

- Ingests rule files with glob patterns from `.continue/rules/*.md` into `~/AIBrain/rules/scoped/`.
- Reads custom prompt templates and model configurations from `.continue/config.json`.

---

## 📤 Projection Output

When `synapse project` executes, it projects rule files into `.continue/rules/` and stamps generated files.

---

[⬅️ Back to Adapters Overview](README.md)
