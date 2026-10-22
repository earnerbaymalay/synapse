# AWS Kiro IDE Adapter Specification
### Synapse

[Docs Hub](../README.md) > [Adapters](README.md) > **AWS Kiro**

---

## 📋 Overview

| Property | Value |
|---|---|
| **Adapter ID** | `kiro` |
| **Native Configs** | `.kiro/steering/*.md`, `.kiro/settings/mcp.json` |
| **Parsing Engine** | YAML Frontmatter (`inclusion: always \| fileMatch`, `fileMatchPattern`) + JSONC MCP Config |
| **Projection Target** | `.kiro/steering/*.md` and `.kiro/settings/mcp.json` |
| **Symlink Support** | Supported for individual `.kiro/steering/*.md` files |

---

## 📥 Ingestion & Projection Strategy

- Imports `.kiro/steering/*.md` steering rules into canonical `skills/kiro-steering-<slug>/skill.yaml`, preserving `fileMatchPattern` globs as skill triggers.
- Parses `.kiro/settings/mcp.json` (`mcpServers` map) with CWE-22 path traversal and CWE-59 symlink rejection.
- Projects canonical skills and MCP servers atomically back into `.kiro/steering/` and `.kiro/settings/mcp.json`.

---

[⬅️ Back to Adapters Overview](README.md)
