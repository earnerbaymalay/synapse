# Progress

- T0.1 ✅ Scaffold repo, state files, Ralph loop, and swarm roles.
- T0.2 ✅ Reconnaissance: authored 12 briefs in `docs/research/`.
- T0.3 ✅ Complete: 3 brand identity packages with HTML dashboard mocks.
- T0.4 ✅ Complete: ASCII wireframes for all 8 screens (DESIGN_PACK.md).
- T0.5 ✅ Complete: Present GATE 0 questions and record decisions in DECISIONS.md.
- T1.1 ✅ Complete: Compile DESIGN_PACK.md with tokens, components, voice, accessibility.
- T1.2 ✅ Complete: Implement RALPH_PROMPT.md operation loop with priority selection.
- T1.3 ✅ Complete: Present GATE 1 Design Pack for human approval.
- **T2.1 ⚠️ PARTIAL (reopened): only apps/desktop scaffolded, and it does not compile. Missing packages/core, packages/schema, apps/cli, fixtures/, and the React frontend.**
- T2.2 🔄 Next: Empty Tauri app launches on all 3 OSes.
- T2.3 🔄 Next: CLI --help complete.
- T3.1 🔄 Next: Adapter-smith swarm (12 adapters, detect/import/project + round-trip).
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

---

## 2026-07-08 — CORRECTION: T2.1 reopened (read this before selecting a task)

T2.1 was marked complete but is only PARTIAL. Current reality:
- Only `apps/desktop` exists; `packages/core`, `packages/schema`, `apps/cli`,
  and `fixtures/` were never created.
- `apps/desktop/src-tauri` does NOT compile (no `build.rs`/`tauri.conf.json`,
  no frontend, missing `#[tauri::command]` attrs, `tauri::Result` misuse,
  duplicate `main()`/`AppState`). Full list: `docs/CODE_REVIEW.md`.
- The invalid `Cargo.toml` has been fixed (valid TOML + chrono/log deps); Rust
  formatting is normalized and CI enforces it.

NEXT PRIORITY (highest first):
1. T2.1a — scaffold `packages/core` (Rust lib) + `apps/cli` (clap) with unit
   tests. No system deps ⇒ a real green `cargo test` on all 3 OSes. Do this
   first; it is the fastest path to a passing verify.
2. Then fix the desktop app compile blockers in `docs/CODE_REVIEW.md §1`.
3. Then T2.2 (app launches) and T2.3 (CLI `--help`).
Update PLAN.md/PROGRESS.md honestly as each lands.