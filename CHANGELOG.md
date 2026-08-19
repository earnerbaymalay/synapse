# Changelog

All notable changes to **LLM Neurosurgeon** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **SYNAPSE rebrand**: adopted the SYNAPSE brand system (`brands/synapse/`) as
  canonical, superseding the prior Cortex/Synapse/Cerebra three-way
  exploration and the ad-hoc "Operating Theatre" placeholder identity.
  - CLI binary renamed `neurosurgeon` → `synapse` (the internal
    `neurosurgeon-core` library crate and `NEUROSURGEON_*` env vars are
    unchanged).
  - Desktop app repainted in the SYNAPSE palette (ink/accent-blue, semantic
    green for status, gold reserved for marketing only), three-font system
    (Rubik Mono One, Inter, JetBrains Mono) vendored locally rather than
    fetched from a CDN, and a blueprint corner-mark frame around each chart.
  - README, `docs/ONBOARDING.md`, and the full `brands/synapse/marketing-pack/`
    content set replaced/added.
  - See `IDENTITY.md` for what's brand-specific versus what's a permanent
    rule (no emoji, no placeholder data, absence-is-a-finding).

## [1.0.0] - 2026-08-14

### Added
- **Antigravity CLI Adapter (`agy-cli`)**:
  - Implemented 13th tool adapter supporting Antigravity CLI rule files (`AGENTS.md`), custom skills (`.agy/skills/`), and settings configurations (`.gemini/settings.json`).
  - Added adapter unit tests, policy table mapping, and registry verification.
- **Full CLI Non-Dry-Run Write Paths**:
  - Implemented write execution for `neurosurgeon import`, `neurosurgeon project`, `neurosurgeon sync` (`--once`, `--daemon`, `--poll-interval`), `neurosurgeon snapshot`, and `neurosurgeon rollback`.
- **E2E Quality & Test Hardening (100% Pass Rate)**:
  - Achieved 142/142 (100%) test pass rate across the full E2E Vitest suite:
    - Sanity suite: 4/4 passing (100%)
    - Tier 1 Feature Coverage: 60/60 passing (100%) across all 13 tool adapters
    - Tier 2 Boundary & Corner Cases: 60/60 passing (100%) across path traversal, malformed configs, missing fields, and write protection
    - Tier 3 Combinations & State Transitions: 12/12 passing (100%) across 3-way merge, concurrent lock safety, rapid event debounce, and multi-target projects
    - Tier 4 Real-world Workloads: 6/6 passing (100%) across large monorepo synchronization scenarios
  - Achieved 179/179 (100%) test pass rate across Rust Core, CLI, stress, and updater test suites with zero Clippy warnings.
- **Centralized Security Enforcement**:
  - Added `parse_and_validate_mcp_server` helper enforcing path traversal defense and required field validation across all adapter MCP parsing paths.
  - Hardened write protection error handling and home directory configuration resolution across all adapters.
- **Obsidian Antigravity Integration**:
  - Integrated automated session worklog and repository checklist tracking with Obsidian vaults.

### Changed
- Unified all workspace crate versions, package manifests, and desktop configurations to `v1.0.0`.
- Normalized line endings (CRLF to LF) and path separators across adapter import pipelines.

---

## [0.7.4] - 2026-07-20

### Added
- **Documentation Set (T7.4)**:
  - Created comprehensive **User Guide** (`docs/USER_GUIDE.md`) detailing setup, canonical Brain structure, supported AI tools, CLI & Desktop GUI usage, Doctor diagnostics, and Time Machine capabilities.
  - Created developer-facing **Adapter Authoring Guide** (`docs/ADAPTER_AUTHORING_GUIDE.md`) covering the `Adapter` trait interface, canonical data models, security guidelines (`safe_join`, non-following directory walks), and projection policy rules.
  - Authored standard **Changelog** (`CHANGELOG.md`) documenting all development milestones from Phase 0 to Phase 7.
  - Updated root `README.md` to reflect completed Phase 7 status, architecture overview, Quickstart instructions, and documentation links.
- **Doctor Engine & CLI Wiring (T7.2)**:
  - Implemented `neurosurgeon doctor [--fix]` CLI subcommand wired to `packages/core/src/doctor.rs`.
  - Added 13 diagnostic rules covering un-initialized Git repositories, missing projections, detached symlinks, retargeted symlinks, and content checksum drift.
  - Implemented self-verifying corrupted-brain fixture test (`doctor_fixes_every_seeded_fault_in_a_corrupted_brain`).
- **Update Channel Dry-Run Engine (T7.3)**:
  - Implemented dry-run updater module (`packages/core/src/updater.rs`) handling version comparison, platform asset matching, and release manifest parsing.
  - Exposed Tauri command `check_for_update` to desktop GUI.

### Fixed
- Fixed a bug where `doctor --fix` re-projecting a file with an empty recorded checksum would falsely diagnose the file as hand-edited on subsequent runs; `reproject` now updates `mappings.json` checksums atomically.
- Updated `vite` to version 6.4.3 clearing open Dependabot alerts across JS/TS workspace members.

---

## [0.6.3] - 2026-07-12

### Added
- **Marketplace Skill Importers (T6.1)**:
  - Implemented `packages/core/src/marketplace.rs` fetching community skills from `anthropics/skills` GitHub repository.
  - Enforced security invariant: imported skills default to `enabled: false` with recorded SHA-256 provenance and executable script warnings.
- **MCP Registry Importers & Health Check (T6.2)**:
  - Implemented `packages/core/src/mcp_registry.rs` searching official Model Context Protocol (MCP) registries (`registry.modelcontextprotocol.io`).
  - Added stdio JSON-RPC `initialize` handshake spawning child process and remote SSE/POST health check probes.
- **OS Keychain & Secrets Management (T6.3)**:
  - Implemented `packages/core/src/secrets.rs` supporting OS Keychain storage (Gnome Keyring on Linux, Security.framework on macOS, Credential Manager on Windows) via `keyring` crate.
  - Added env variable harvesting to strip plain-text secrets and insert `${VAR}` placeholders into tool configuration files.

---

## [0.5.3] - 2026-07-12

### Added
- **Desktop GUI Interface (T5.1)**:
  - Implemented all 8 screens strictly following `DESIGN_PACK.md`: Main Dashboard, Configuration Manager, Adapter Inspector, Status Monitor, Debug Console, Onboarding Wizard, Marketplace, and MCP Hub.
  - Added visual primitives (`Card`, `StatusPill`, `Toolbar`, `ToolbarButton`) with Tailwind styling and dark mode default.
  - Populated screenshots directory (`apps/desktop/screenshots/`).
- **Onboarding Dry-Run Flow (T5.2)**:
  - Added Tauri IPC command `scan_dry_run()` returning structured detected tool reports.
  - Built 3-step interactive Onboarding Wizard in React/TS.
  - Created Vitest + React Testing Library E2E suite (`OnboardingWizard.e2e.test.tsx`).

---

## [0.4.3] - 2026-07-12

### Added
- **Filesystem Watcher & Schedulers (T4.1)**:
  - Implemented `packages/core/src/watcher.rs` wrapping `notify` crate with debounced event batching.
  - Implemented `packages/core/src/scheduler.rs` generating native OS recurring job files (`launchd` plist, `systemd` user service/timer, Windows `schtasks`).
- **Conflict Queue & 3-Way Merge (T4.2)**:
  - Integrated `diffy` crate in `packages/core/src/merge.rs` for 3-way text merging.
  - Built `packages/core/src/conflict_queue.rs` managing queued conflict state for overlapping concurrent edits.
- **Time Machine (Git Snapshot & Rollback) (T4.3)**:
  - Implemented `packages/core/src/snapshot.rs` backing up Brain history into `.git`.
  - Added crash-safe lock (`SnapshotLock`) detecting stale PIDs.
  - Built byte-identical `rollback` command preserving full git commit lineage.

---

## [0.3.4] - 2026-07-12

### Added
- **Complete Adapter Suite (T3.1)**:
  - Built and verified all 12 tool adapters: `claude-code`, `gemini-cli`, `openai-codex`, `cursor`, `windsurf`, `cline`, `roo-code`, `aider`, `continue`, `github-copilot`, `zed`, `opencode`.
  - Implemented semantic round-trip tests (`import -> canonical -> project == semantic identity`).
- **Projection Engine & Drift Detector (T3.2)**:
  - Built `packages/core/src/projector.rs` with `POLICY_TABLE` mapping artifact types to `Symlink` or `Generate` policies.
  - Implemented `packages/core/src/mappings.rs` managing `mappings.json` records.
  - Built `packages/core/src/drift.rs` checking file content SHA-256 hashes and symlink targets.
- **Red-Team Security Pass (T3.3)**:
  - Implemented `safe_join` helper blocking symlink escapes and path traversal attacks.
  - Converted directory traversal functions to use non-symlink-following `DirEntry::file_type()`.

---

## [0.2.3] - 2026-07-08

### Added
- **Monorepo Architecture (T2.1 - T2.3)**:
  - Configured monorepo workspace containing `apps/desktop` (Tauri v2 + React), `apps/cli` (Clap CLI), `packages/core` (Rust core engine), `packages/schema` (JSON Schemas), and `fixtures/` (golden tool configs).
  - Implemented CLI `--help` with subcommand definitions for `scan`, `import`, `project`, `sync`, `doctor`, `snapshot`, and `rollback`.

---

## [0.1.3] - 2026-07-05

### Added
- **Design System & Execution Loop (T1.1 - T1.3)**:
  - Created `DESIGN_PACK.md` establishing color tokens, typography scales, component specs, microcopy rules, and accessibility standards.
  - Authored `RALPH_PROMPT.md` defining the file-state autonomous agent loop.

---

## [0.0.5] - 2026-07-01

### Added
- **Discovery & Wireframes (T0.1 - T0.5)**:
  - Initialized repository state files (`PLAN.md`, `PROGRESS.md`, `DECISIONS.md`, `QUESTIONS.md`).
  - Authored 12 tool reconnaissance briefs in `docs/research/`.
  - Created 3 brand identity package mocks (Cortex, Synapse, Cerebra).
  - Drafted ASCII wireframes for all 8 application screens.
