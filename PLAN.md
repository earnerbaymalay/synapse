# LLM Neurosurgeon — PLAN.md

## Phase 0 — Discovery & Design Elicitation → GATE 0 (human)
- [x] T0.1  Initialize repo + state files + subagent roles :: verify: git log --oneline shows scaffold commit
- [x] T0.2  Recon ×12 — verify current config paths/formats for all supported tools :: verify: docs/research/ contains 12 briefs
- [x] T0.3  Draft 3 brand identities (Cortex, Synapse, Cerebra) + HTML dashboard mocks :: verify: brands/<A|B|C>/index.html render without errors
- [x] T0.4  ASCII wireframes for all 8 screens :: verify: DESIGN_PACK.md wireframes section lists all 8 screens
- [x] T0.5  Present GATE 0 questions and await human picks :: verify: DECISIONS.md records choices

## Phase 1 — Design Pack → GATE 1 (human)
- [x] T1.1  Compile DESIGN_PACK.md with tokens, components, voice, accessibility :: verify: every UI task in PLAN.md cites DESIGN_PACK.md section
- [x] T1.2  Write RALPH_PROMPT.md and full task breakdown with verify: checks :: verify: RALPH_PROMPT.md parses as expected
- [x] T1.3  Gate 1: present pack for approval

## Phase 2 — Architecture & Scaffold
- [x] T2.1  Monorepo layout: apps/desktop, packages/core, packages/schema, apps/cli, fixtures :: verify: cargo test and pnpm test green
- [x] T2.2  Empty Tauri app launches on all 3 OSes :: verify: build matrix passes
- [x] T2.3  CLI --help complete :: verify: cli --help outputs verbs

## Phase 3 — Core Engine: adapters + projection → GATE 2 (human)
- [x] T3.1  Adapter-smith swarm: 12 adapters, detect/import/project + round-trip :: verify: 12/12 round-trip green
- [x] T3.2  Projection engine: symlink-vs-generate policy table, mappings.json, drift detector :: verify: policy table present and tested
- [x] T3.3  Red-team pass: symlink escape, path traversal, circular links, malformed configs :: verify: red-team report in docs/security.md
- [x] T3.4  Gate 2: human runs cli scan && cli import --dry-run on real machine and approves

## Phase 4 — Sync Daemon & Time Machine
- [x] T4.1  Filesystem watcher (debounced) + OS scheduler registration :: verify: watcher test passes
- [x] T4.2  Conflict queue API + three-way merge :: verify: conflict fixture test
- [x] T4.3  Git snapshot/rollback commands + crash-safe lock :: verify: rollback byte-identical test

## Phase 5 — GUI → GATE 3 (human)
- [x] T5.1  ui-builder implements 8 screens strictly from DESIGN_PACK.md :: verify: screenshot folder populated
- [x] T5.2  Onboarding wizard wraps dry-run flow :: verify: e2e onboarding test
- [x] T5.3  Gate 3: human reviews screenshots + demo

## Phase 6 — Marketplace & MCP Hub
- [x] T6.1  Marketplace importers for skill sources :: verify: 3 real skills from anthropics/skills import
- [x] T6.2  MCP registry importers + health-check handshake :: verify: 2 registry MCP servers end-to-end
- [x] T6.3  Secrets flow: keychain, placeholders, project-to-all-tools :: verify: secret fixture round-trip

## Phase 7 — Hardening, Docs, Self-Update
- [x] T7.1  Threat-model pass + red-team sign-off :: verify: docs/security.md sign-off
- [x] T7.2  Doctor rules library (≥12 diagnoses) :: verify: doctor fixes seeded faults
- [x] T7.3  Auto-update channel dry-run :: verify: updater test
- [x] T7.4  Doc set: README, user guide, adapter-authoring guide, CHANGELOG :: verify: docs build

## Phase 8 — Package & Release → GATE 4 (human)
- [x] T8.1  Signed installers .dmg/.msi/AppImage/.deb + CLI formulae drafts :: verify: smoke-test install matrix
- [x] T8.2  Reproducible-build notes :: verify: build notes present
- [ ] T8.3  Gate 4: human installs on real machine and runs onboarding
