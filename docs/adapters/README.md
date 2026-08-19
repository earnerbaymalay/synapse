# 🔌 Tool Adapters Overview
### Synapse // LLM-NeuroSurgeon — 13 Verified Adapters

[Docs Hub](../README.md) > **Adapters**

Synapse includes 13 verified tool adapters, each purpose-built for bi-directional configuration translation.

---

## 📊 Supported Ecosystem Matrix

| Tool | Specification | Config Location | Format | Import Capabilities | Projection Target |
|---|---|---|---|---|---|
| **Claude Code / Desktop** | [Spec](claude.md) | `CLAUDE.md`, `.claude/skills/`, `.claude/agents/`, `.mcp.json` | Markdown, JSON | Skills, Agents, MCP Servers | `.claude/` structure |
| **Gemini CLI** | [Spec](gemini.md) | `GEMINI.md`, `.gemini/settings.json` | Markdown, JSON | Rules, Settings | Stamped `GEMINI.md` |
| **OpenAI Codex CLI** | [Spec](openai-codex.md) | `.codex/config.toml`, `.codex/instructions.md` | TOML, Markdown | Rules, Config | `AGENTS.md` + `.codex/` |
| **Cursor** | [Spec](cursor.md) | `.cursorrules`, `.cursor/rules/*.mdc` | MDC Frontmatter | MDC Rules & Globs | `.cursor/rules/*.mdc` |
| **Windsurf** | [Spec](windsurf.md) | `.windsurfrules`, `mcp_config.json` | Text, JSON | Rules & MCP Servers | `.windsurfrules` |
| **Cline** | [Spec](cline.md) | `.clinerules`, `cline_mcp_settings.json` | Text, JSON | Custom Rules & MCP | `.clinerules` |
| **Roo Code** | [Spec](roo-code.md) | `.roomodes`, `.clinerules` | JSON, Text | Mode Rules | `.roomodes` |
| **Aider** | [Spec](aider.md) | `CONVENTIONS.md`, `.aider.conf.yml` | Markdown, YAML | Conventions & Config | `CONVENTIONS.md` |
| **Continue** | [Spec](continue.md) | `.continue/rules/*.md`, `.continue/config.json` | MDC, JSON | MDC Rules & Config | `.continue/rules/` |
| **GitHub Copilot** | [Spec](github-copilot.md) | `.github/copilot-instructions.md` | Markdown | Scoped Instructions | `.github/copilot-instructions.md` |
| **Zed** | [Spec](zed.md) | `.rules`, `.zed/settings.json`, `AGENTS.md` | Text, JSON, Markdown | Settings & Rules | `.rules` + `AGENTS.md` |
| **OpenCode** | [Spec](opencode.md) | `AGENTS.md` | Markdown | Multi-Agent Rules | `AGENTS.md` |
| **Antigravity CLI** | [Spec](antigravity.md) | `AGENTS.md`, `.agy/skills/`, `.gemini/settings.json` | Markdown, YAML | Skills & Settings | `AGENTS.md` + `.agy/skills/` |

---

## 🛠️ Authoring a Custom Adapter

Want to add support for a new AI developer tool? Read the **[Adapter Authoring Guide](../ADAPTER_AUTHORING_GUIDE.md)** for step-by-step instructions on implementing the Rust `Adapter` trait.

---

[⬅️ Back to Docs Hub](../README.md)
