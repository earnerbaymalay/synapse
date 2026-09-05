# QUESTIONS.md

## Gate 0 Decisions Required

- [x] **Brand Assignment**: A=Cortex, B=Synapse, C=Cerebra
  - Why: Based on DESIGN_PACK.md wireframe branding (CORTEX, Synapse, Cerebra)
  - How to apply: Continue with these assignments; confirm if any changes needed.
  - **Resolved**: the three-way split was abandoned. `IDENTITY.md` commits to
    one identity, SYNAPSE; DESIGN_PACK.md (which showed all three) is now
    marked superseded.

- [x] **Theme Default**: Dark theme
  - Why: Consistent with cross-platform CLI tools and DESIGN_PACK.md visual tokens
  - How to apply: Use dark theme as the default; confirm if light theme is preferred.
  - **Resolved**: dark is the only palette. `IDENTITY.md` and
    `brands/synapse/tokens.json` define no light variant, and the desktop
    app's CSS has no `prefers-color-scheme` branch.

- [x] **Stack Confirmation**: Entire tech stack:
  - Core: Rust + Tauri v2 (Rust core + React/TS frontend)
  - Frontend: React + TypeScript + Tailwind CSS
  - CLI: Clap for command line verbs
  - Monorepo layout: apps/desktop, packages/core, packages/schema, apps/cli, fixtures
  - Verify: Existing CLAUDE.md states default stack candidate is "Tauri 2 (Rust core + React/TS/Tailwind UI) + clap CLI"
  - How to apply: Adopt full default stack; ask for any deviations or additional constraints.
  - **Resolved**: this is exactly what shipped — confirmed by the monorepo
    layout and dependencies as built.

- [x] **Brain Location Default**:
  - Current context: Key technical concepts say "The Brain: canonical `~/AIBrain` directory for skills/agents/rules/memory/prompts/MCP"
  - How to apply: Confirm if `~/AIBrain` is the intended location for the brain structure; if not, specify alternative path.
  - **Resolved**: `~/AIBrain` is the shipped default (see
    `resolve_brain_root` in `apps/cli/src/main.rs`), overridable via env var.

## Pending Choices
All Gate 0 items above are resolved; none currently open.
