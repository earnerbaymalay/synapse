# 🔌 Tool Adapters Overview
### Synapse // LLM-NeuroSurgeon — 13 Verified Adapters

[Docs Hub](../README.md) > **Adapters**

Synapse includes 13 verified tool adapters, each purpose-built for bi-directional configuration translation.

---

## 📊 Supported Ecosystem Matrix

| Tool | Config Location | Format | Import Capabilities | Projection Target |
|---|---|---|---|---|
| **Claude Code / Desktop** | `CLAUDE.md`, `.claude/skills/`, `.claude/agents/`, `.mcp.json` | Markdown, JSON | Skills, Agents, MCP Servers | `.claude/` structure |
| **Gemini CLI** | `GEMINI.md`, `.gemini/settings.json` | Markdown, JSON | Rules, Settings | Stamped `GEMINI.md` |
| **OpenAI Codex CLI** | `.codex/config.toml`, `.codex/instructions.md` | TOML, Markdown | Rules, Config | `AGENTS.md` + `.codex/` |
| **Cursor** | `.cursorrules`, `.cursor/rules/*.mdc` | MDC Frontmatter | MDC Rules & Globs | `.cursor/rules/*.mdc` |
| **Windsurf** | `.windsurfrules`, `mcp_config.json` | Text, JSON | Rules & MCP Servers | `.windsurfrules` |
| **Cline** | `.clinerules`, `cline_mcp_settings.json` | Text, JSON | Custom Rules & MCP | `.clinerules` |
| **Roo Code** | `.roomodes`, `.clinerules` | JSON, Text | Mode Rules | `.roomodes` |
| **Aider** | `CONVENTIONS.md`, `.aider.conf.yml` | Markdown, YAML | Conventions & Config | `CONVENTIONS.md` |
| **Continue** | `.continue/rules/*.md`, `.continue/config.json` | MDC, JSON | MDC Rules & Config | `.continue/rules/` |
| **GitHub Copilot** | `.github/copilot-instructions.md` | Markdown | Scoped Instructions | `.github/copilot-instructions.md` |
| **Zed** | `.rules`, `.zed/settings.json`, `AGENTS.md` | Text, JSON, Markdown | Settings & Rules | `.rules` + `AGENTS.md` |
| **OpenCode** | `AGENTS.md` | Markdown | Multi-Agent Rules | `AGENTS.md` |
| **Antigravity CLI** | `AGENTS.md`, `.agy/skills/`, `.gemini/settings.json` | Markdown, YAML | Skills & Settings | `AGENTS.md` + `.agy/skills/` |

---

[⬅️ Back to Docs Hub](../README.md)
