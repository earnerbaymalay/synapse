# QUESTIONS.md

## Gate 0 Decisions Required

- [ ] **Brand Assignment**: A=Cortex, B=Synapse, C=Cerebra
  - Why: Based on DESIGN_PACK.md wireframe branding (CORTEX, Synapse, Cerebra)
  - How to apply: Continue with these assignments; confirm if any changes needed.

- [ ] **Theme Default**: Dark theme
  - Why: Consistent with cross-platform CLI tools and DESIGN_PACK.md visual tokens
  - How to apply: Use dark theme as the default; confirm if light theme is preferred.

- [ ] **Stack Confirmation**: Entire tech stack:
  - Core: Rust + Tauri v2 (Rust core + React/TS frontend)
  - Frontend: React + TypeScript + Tailwind CSS
  - CLI: Clap for command line verbs
  - Monorepo layout: apps/desktop, packages/core, packages/schema, apps/cli, fixtures
  - Verify: Existing CLAUDE.md states default stack candidate is "Tauri 2 (Rust core + React/TS/Tailwind UI) + clap CLI"
  - How to apply: Adopt full default stack; ask for any deviations or additional constraints.

- [ ] **Brain Location Default**:
  - Current context: Key technical concepts say "The Brain: canonical `~/AIBrain` directory for skills/agents/rules/memory/prompts/MCP"
  - How to apply: Confirm if `~/AIBrain` is the intended location for the brain structure; if not, specify alternative path.

## Pending Choices
Confirm these to proceed to T0.5 completion and Phase 1.