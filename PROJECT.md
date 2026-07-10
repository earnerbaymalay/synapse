# Project: LLM Neurosurgeon Core Engine Adapters

## Architecture
- All 12 adapters implement the `Adapter` trait defined in `packages/core/src/adapter.rs`.
- Each adapter maps between its specific file format (JSON, YAML, Markdown) and the canonical `Skill`, `Agent`, and `McpServer` models defined in `packages/core/src/model.rs`.
- The engine uses these adapters to `detect` tool configurations, `import` them into the canonical brain model, and `project` them back to the tool's filesystem layout.

## Code Layout
- `packages/core/src/adapter.rs`: Trait definition and error types.
- `packages/core/src/adapters/`: Folder containing individual adapter implementations:
  - `aider.rs`
  - `claude_code.rs`
  - `cline.rs`
  - `continue_adapter.rs`
  - `cursor.rs`
  - `gemini_cli.rs`
  - `github_copilot.rs`
  - `openai_codex.rs`
  - `opencode.rs`
  - `roo_code.rs`
  - `windsurf.rs`
  - `zed.rs`
  - `mod.rs`: Registry listing all 12 adapters.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Simple Markdown Adapters | Implement: `cline`, `opencode`, `github-copilot`, `windsurf` | none | PLANNED |
| 2 | Hybrid Settings & Markdown | Implement: `gemini-cli`, `zed`, `aider`, `roo-code` | M1 | PLANNED |
| 3 | Advanced Multi-file/Settings | Implement: `cursor`, `continue`, `claude-code`, `openai-codex` | M2 | PLANNED |
| 4 | E2E test verification | Final pass of 12/12 adapter round-trip tests | M3 | PLANNED |

## Interface Contracts
- Each adapter implements the `Adapter` trait:
  - `id(&self) -> &'static str`
  - `detect(&self, root: &Path) -> bool`
  - `import(&self, root: &Path) -> Result<ImportResult, AdapterError>`
  - `project(&self, root: &Path, skills: &[Skill], agents: &[Agent], mcp_servers: &[McpServer]) -> Result<ProjectResult, AdapterError>`
