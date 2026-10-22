# Changelog

All notable changes to **SYNAPSE / Synapse** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Synaptic Memory (`synapse mem`)** — persistent cross-session memory for AI agents, adapted from claude-mem but local-first and zero-telemetry:
  - `mem hook session-start|prompt|post-tool|stop` captures lifecycle events from agent hooks (`mem hooks` prints the Claude Code config; nothing is written automatically).
  - Progressive disclosure: `mem search` (token-costed index) → `mem timeline <id>` → `mem get <ids>`; `mem context` is the SessionStart injection block.
  - No LLM in the loop: tool output is distilled with the Sensory Gating engine; `<private>…</private>` spans are dropped and credential-looking tokens redacted before anything touches disk.
  - Storage is an append-only JSONL at `~/.synapse/memory/` (override: `SYNAPSE_MEMORY_DIR`); `mem forget`, `mem stats`, `mem observe` for management.
  - Tier-5 E2E suite covering `mem` lifecycle hooks and gateway context folding.
- **Reversible context folding (`synapse serve --fold-keep N`)** — inspired by billion-context, but deterministic and model-free: tool results older than the newest N messages (OpenAI `tool` and Anthropic `tool_result` shapes) collapse into a short digest stub; the original is spooled under a content-addressed id and restored with `synapse spool show <id>`. Stubs are byte-stable across requests so prefix caching is preserved. Off by default.

### Changed
- **Tool root defaults to your home directory, not the current directory.** This is a behaviour change. Commands that take `--tool-root` (`doctor`, `onboard`) accept it. Every command reads `$SYNAPSE_WORKSPACE_PATH` / `$SYNAPSE_TOOL_ROOT`. For a per-project run, use `SYNAPSE_TOOL_ROOT=. synapse scan` or pass `--tool-root .`.
- **The gateway now requires a per-launch token.** `synapse serve` prints `Gateway token: …`; clients send it as `Authorization: Bearer <token>` or `x-api-key: <token>`. Pin it with `SYNAPSE_GATEWAY_TOKEN`, or disable it with `SYNAPSE_GATEWAY_NO_TOKEN=1`. `--api-key` can now come from `SYNAPSE_UPSTREAM_API_KEY`. Requests with `"stream": true` are refused with a 400 until streaming is supported.
- `SKILL.md` is the Brain's editable source of truth; `skill.yaml` only carries metadata.
- `synapse rollback` now projects the restored Brain to your tools straight away.
- Unknown `--level` / `--gate-level` values are rejected instead of falling back to `balanced`.
- Subtitle: Synapse — The Canonical AI Brain.

### Fixed
- Gateway gating dropped `tool_call_id`, `tool_calls`, and `name` from compressed messages. Compression now edits message content in place and keeps those fields.
- `synapse serve --gate-level off` silently fell back to `balanced`. `off` is now a real pass-through.
- CI `pnpm --filter` selectors matched no package, so the E2E and schema suites never ran in CI. The selectors now resolve to the intended packages.
- `ralph.sh` used incorrect paths.
- Antigravity adapter documentation listed `.agy/` and `.gemini/settings.json`. The adapter reads `.agents/` and `.gemini/config/`, and the docs now match.
- **Sync data loss.** A Brain-only edit was overwritten by the old tool content, and a clean three-way merge was computed and then discarded. Both now survive; conflicting items are queued and left untouched on both sides.
- **Sync never settled.** Projected files were re-imported by other adapters under new ids, so every sync created more skills. Sync now reaches a fixed point and reports "no drift" when nothing changed.
- Agent `tools` were widened to `*` when projected; agent tools and model hints now round-trip.
- The sync `.lock` was committed into snapshots (blocking every sync after a rollback), and a lock left by a crashed sync blocked forever. The Brain now ignores `.lock`, and stale locks from dead processes are reclaimed.
- `rollback` discarded uncommitted Brain edits; it snapshots them first. The next sync no longer undoes a rollback.
- `import`, `project`, `rollback` and the HUD could race a running sync; every Brain mutation now takes the sync lock.
- `doctor` flagged every fresh projection as hand-edited, and `doctor --fix` followed `../` or absolute paths from `mappings.json`; both fixed.

### Security
- Gateway: removed `Access-Control-Allow-Origin: *` (any website could spend your upstream API key through `127.0.0.1`), added a `Host` allowlist against DNS rebinding, token auth, an 8 MiB body cap, a bounded worker pool, and no upstream URL in `/health`. Client credentials are never forwarded.
- Memory and spool: secret redaction now catches `KEY=…`, JSON `"api_key":"…"`, `Authorization: Bearer`, URL passwords, GitHub/GitLab/Slack/Stripe/AWS/Google tokens and PEM private keys; files are owner-only (`0600`, dirs `0700`) and symlink-safe. The predictable `/tmp` spool fallback is gone.
- MCP server commands/URLs containing credentials are replaced with `${SYNAPSE_REDACTED_N}` placeholders before they reach the Brain's git history.
- The Zed adapter no longer writes to `$HOME` when projecting into another folder.
- MCP registry: queries are encoded, runtimes are limited to `npx`/`uvx`/`docker`/`dnx`, and checksums cover the full record. Marketplace executable-content checks are recursive.
- Desktop: a Content Security Policy is set, and `heal_vitals` ignores paths sent from the webview.
- Docs hub: closed a DOM XSS via `?doc=`; CDN scripts are pinned with subresource integrity.

### Performance
- `mem search` is linear instead of quadratic (43 s → well under a second at 10k observations); memory hooks no longer re-read the whole log, and the stop hook only appends.
- The gateway reuses one HTTP client with connect/read timeouts.

## [1.0.0] - 2026-08-19

### Added
- **SYNAPSE Rebrand Package & Visual Identity**:
  - Integrated complete SYNAPSE brand identity (`SYNAPSE / Synapse: One Brain. All Models.`).
  - Applied design tokens: Synapse Blue accent (`#1d9bf0`), dark ink interface (`#06080b`), Inter, JetBrains Mono, and Rubik Mono One typography, and "Surgical precision. Zero friction." tone anchor.
- **Clinical Record React Desktop UI (`chart.tsx`, `Intake.tsx`, `Examination.tsx`)**:
  - Implemented the clinical operating theatre record identity per `IDENTITY.md` backed by live `synapse-core` calls with zero fabricated sample data.
- **4-Phase Onboarding Journey Guide (`docs/ONBOARDING.md`)**:
  - Authored structured 4-phase journey covering Phase 1 (The Hook), Phase 2 (The Solution - The Brain), Phase 3 (Immediate Value - `synapse scan` / `import` / `project`), and Phase 4 (Long-Term Power - Time Machine, Auto-Sync Daemon, Doctor, MCP Hub, Marketplace).
- **Interactive Cyber-Dark GitHub Pages Landing Site (`docs/index.html`)**:
  - Standalone interactive terminal simulator with tabbed verbs (`scan`, `import`, `project`, `sync`, `doctor`), live ECG vitals pulse, and 13 tool adapter specifications.
- **CLI Binary Alias Integration**:
  - Single `synapse` binary exposing the CLI commands (`scan`, `import`, `project`, `sync`, `doctor`, `snapshot`, `rollback`).
- **Antigravity CLI Adapter (`agy-cli`)**:
  - Implemented 13th tool adapter supporting Antigravity CLI rule files (`AGENTS.md`), custom skills (`.agents/skills/`), and MCP configuration (`.agents/mcp_config.json`). (Corrected: the earlier text named `.agy/skills/` and `.gemini/settings.json`.)
- **Full CLI Non-Dry-Run Write Paths**:
  - Implemented write execution for `synapse import`, `synapse project`, `synapse sync` (`--once`), `synapse snapshot`, and `synapse rollback`. (Corrected: `sync` has no `--daemon` or `--poll-interval` flags. Each run performs one pass.)
- **E2E Quality & Test Hardening (100% Pass Rate)**:
  - Achieved 142/142 (100%) test pass rate across the full E2E Vitest suite:
    - Sanity suite: 4/4 passing (100%)
    - Tier 1 Feature Coverage: 60/60 passing (100%) across all 13 tool adapters
    - Tier 2 Boundary & Corner Cases: 60/60 passing (100%) across path traversal, malformed configs, missing fields, and write protection
    - Tier 3 Combinations & State Transitions: 12/12 passing (100%) across 3-way merge, concurrent lock safety, rapid event debounce, and multi-target projects
    - Tier 4 Real-world Workloads: 6/6 passing (100%) across large monorepo synchronization scenarios
  - Achieved 203/203 (100%) test pass rate across Rust Core, CLI, stress, and updater test suites with zero Clippy warnings.
- **Centralized Security Enforcement & Dependabot Resolution**:
  - Resolved `nanoid` CVE advisory (`GHSA-2v37-7h3g-55p8`) by updating overrides to `^3.3.18`.
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
  - Implemented `synapse doctor [--fix]` CLI subcommand wired to `packages/core/src/doctor.rs`.
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
