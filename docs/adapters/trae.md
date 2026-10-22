# Trae IDE Adapter Specification
### Synapse

[Docs Hub](../README.md) > [Adapters](README.md) > **Trae IDE**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `trae` |
| **Native Configs** | `.trae/rules/*.md`, `.trae/mcp.json` |
| **Parsing Engine** | MDC Frontmatter (`globs`, `alwaysApply`) + JSONC MCP Config |
| **Projection Target** | `.trae/rules/*.md` and `.trae/mcp.json` |
| **Symlink Support** | Supported for individual `.trae/rules/*.md` files |

---

## 📥 Ingestion & Projection Strategy

- Imports `.trae/rules/*.md` rule documents into canonical `skills/trae-rule-<slug>/skill.yaml`, mapping `globs` frontmatter into canonical triggers.
- Parses `.trae/mcp.json` (`mcpServers` map) with CWE-22 path traversal and CWE-59 symlink rejection.
- Projects canonical skills and MCP servers atomically back into `.trae/rules/` and `.trae/mcp.json`.

---

[⬅️ Back to Adapters Overview](README.md)
