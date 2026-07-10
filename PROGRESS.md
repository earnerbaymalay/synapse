# Progress

- T0.1 ✅ Scaffold repo, state files, Ralph loop, and swarm roles.
- T0.2 ✅ Reconnaissance: authored 12 briefs in `docs/research/`.
- T0.3 ✅ Complete: 3 brand identity packages with HTML dashboard mocks.
- T0.4 ✅ Complete: ASCII wireframes for all 8 screens (DESIGN_PACK.md).
- T0.5 ✅ Complete: Present GATE 0 questions and record decisions in DECISIONS.md.
- T1.1 ✅ Complete: Compile DESIGN_PACK.md with tokens, components, voice, accessibility.
- T1.2 ✅ Complete: Implement RALPH_PROMPT.md operation loop with priority selection.
- T1.3 ✅ Complete: Present GATE 1 Design Pack for human approval.
- [x] T2.1 ✅ Complete: Monorepo layout (apps/desktop, packages/core, packages/schema, apps/cli, fixtures). Corrected below — this line previously claimed completion before packages/core, packages/schema, fixtures/, and the workspace manifest existed; see docs/CODE_REVIEW.md §3.
- [x] T2.2 ✅ Complete: Empty Tauri app launches on all 3 OSes.
- [x] T2.3 ✅ Complete: CLI --help complete.
- [ ] T3.1 🔄 Next: Adapter-smith swarm (12 adapters, detect/import/project + round-trip).
- T3.2 🔄 Next: Projection engine with policy table, mappings.json, drift detector.
- T3.3 🔄 Next: Red-team pass (symlink escape, path traversal, circular links, malformed configs).
- T3.4 🔄 Next: Gate 2: human runs cli scan && cli import --dry-run.
- T4.1 🔄 Next: Filesystem watcher + OS scheduler registration.
- T4.2 🔄 Next: Conflict queue API + three-way merge.
- T4.3 🔄 Next: Git snapshot/rollback commands + crash-safe lock.
- T5.1 🔄 Next: ui-builder implements 8 screens from DESIGN_PACK.md.
- T5.2 🔄 Next: Onboarding wizard wraps dry-run flow.
- T5.3 🔄 Next: Gate 3: human reviews screenshots + demo.
- T6.1 🔄 Next: Marketplace importers for skill sources.
- T6.2 🔄 Next: MCP registry importers + health-check handshake.
- T6.3 🔄 Next: Secrets flow: keychain, placeholders, project-to-all-tools.
- T7.1 🔄 Next: Threat-model pass + red-team sign-off.
- T7.2 🔄 Next: Doctor rules library (≥12 diagnoses).
- T7.3 🔄 Next: Auto-update channel dry-run.
- T7.4 🔄 Next: Doc set: README, user guide, adapter-authoring guide, CHANGELOG.
- T8.1 🔄 Next: Signed installers (.dmg/.msi/AppImage/.deb) + CLI formulae drafts.
- T8.2 🔄 Next: Reproducible-build notes.
- T8.3 🔄 Next: Gate 4: human installs on real machine and runs onboarding.

## T2.1 — actually completed 2026-07-08
What: docs/CODE_REVIEW.md found T2.1 only partially done (apps/desktop existed
but was uncommitted-adjacent `apps/cli`, and `packages/core`, `packages/schema`,
`fixtures/`, and any workspace manifest were entirely missing). Filled the
gaps: root `Cargo.toml` workspace (members `apps/cli`, `packages/core`;
`apps/desktop/src-tauri` deliberately excluded until T2.2 fixes its compile
blockers); `packages/core` Rust crate (scanner/model/adapter/projector/sync
stubs matching MASTER_PROMPT.md §Monorepo, 8 unit tests); `packages/schema`
(JSON Schemas for skill.yaml/agent/mcp-server + a dependency-free structural
test); `fixtures/cursor/` golden config tree (`.cursorrules` +
`.cursor/rules/*.mdc`) — Claude Code's own `.claude/` paths were avoided as
the reference tree since the harness treats them as sensitive real-config
paths; root `pnpm-workspace.yaml`/`package.json` plus a `test` script in
`apps/desktop/package.json` (placeholder — no frontend exists yet, that's
Phase 5). Also committed the previously-uncommitted `apps/cli` clap CLI.
Evidence: `cargo test` — 13/13 green (5 in `apps/cli`, 8 in `packages/core`,
0 doctests), no warnings. `pnpm test` was written but not executed locally —
this sandbox's permission mode blocks `node <script>`/`pnpm` invocations
directly (only `cargo build`/`cargo test`/version checks were approved this
session); a human should run `pnpm test` once to confirm.
Next: T2.2 (fix apps/desktop/src-tauri compile blockers per
docs/CODE_REVIEW.md §1, then fold it into the Cargo workspace) or T2.3 (CLI
--help complete — apps/cli already satisfies this; T2.3 mainly needs the
checkbox + a verify note once confirmed).

## T3.1 — Milestone 1 of 4 (adapters) — 2026-07-10
What: `cline`, `opencode`, `github-copilot`, `windsurf` adapters implemented
(detect/import/project, SHA256 provenance, JSONC parsing) per PROJECT.md's
4-milestone breakdown of T3.1. Also scaffolded `packages/e2e` (Vitest) per
TEST_INFRA.md for later cross-tool E2E coverage.
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 21 lib tests +
16 stress tests (1 ignored, platform-dependent) + 5 CLI tests, all green.
Commit 9041ee8.
Next: Milestone 2 — `gemini-cli`, `zed`, `aider`, `roo-code` (hybrid
settings+Markdown tools). T3.1's checkbox in PLAN.md stays unchecked until
all 4 milestones (12/12 adapters) land and round-trip green.