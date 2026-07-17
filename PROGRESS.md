# Progress

- T0.1 ✅ Scaffold repo, state files, Ralph loop, and swarm roles.
- T0.2 ✅ Reconnaissance: authored 12 briefs in `docs/research/`.
- T0.3 ✅ Complete: 3 brand identity packages with HTML dashboard mocks.
- T0.4 ✅ Complete: ASCII wireframes for all 8 screens (DESIGN_PACK.md).
- T0.5 ✅ Complete: Present GATE 0 questions and record decisions in DECISIONS.md.
- T1.1 ✅ Complete: Compile DESIGN_PACK.md with tokens, components, voice, accessibility.
- T1.2 ✅ Complete: Implement RALPH_PROMPT.md operation loop with priority selection.
- T1.3 ✅ Complete: Present GATE 1 Design Pack for human approval.
- [x] T2.1 ✅ Complete: Monorepo layout (apps/desktop, packages/core, packages/schema, apps/cli, fixtures). (An early iteration reopened this as PARTIAL on 2026-07-08 when packages/core/schema/cli/fixtures were missing; they were subsequently built and cargo test --workspace is green — so T2.1 is genuinely complete.)
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
- T7.2 ✅ Complete: Doctor rules library (13 rules) + CLI wiring + corrupted-fixture self-verify.
- T7.3 ✅ Complete: Auto-update channel dry-run (core module + fixture + Tauri command).
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

## T3.1 — Milestone 2 of 4 (adapters) — 2026-07-10
What: `gemini-cli` (GEMINI.md + .gemini/settings.json mcpServers),
`zed` (.rules + .zed/settings.json context_servers), `aider` (CONVENTIONS.md
+ hand-rolled flat-YAML .aider.conf.yml, no MCP support per recon brief),
`roo-code` (.roomodes custom modes -> Agent + companion Skill; deliberately
does NOT claim `.clinerules` since that file is Cline's — avoids double-import
across the two adapters). All JSON/settings merges preserve pre-existing
unrelated keys (matches Milestone 1's cline/windsurf merge pattern).
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 59 tests green
(1 ignored, platform-dependent), `cargo fmt --check` clean. Commit d162d6f.
Next: Milestone 3 — `cursor`, `continue`, `claude-code`, `openai-codex`
(advanced multi-file/settings tools). 8/12 adapters done.

## T3.1 — Milestones 3-4 of 4 (adapters) — 2026-07-10 — T3.1 COMPLETE
What: `cursor` (.cursorrules legacy + .cursor/rules/*.mdc, globs/alwaysApply
frontmatter -> Skill.triggers; validated against the golden fixture in
fixtures/cursor/), `continue` (.continue/config.json mcpServers +
.continue/rules/*.md, same mdc-frontmatter convention as cursor — shared via
a new `split_frontmatter`/`parse_mdc_frontmatter` helper in adapters/mod.rs),
`claude-code` (CLAUDE.md + .claude/skills/*/SKILL.md + .claude/agents/*.md
-> Agent+companion-Skill + .mcp.json), `openai-codex` (.codex/config.toml
[mcp_servers.*] via the `toml` crate; deliberately does NOT claim AGENTS.md
— that's opencode's file). Before building openai-codex, live-verified its
config paths via WebFetch against developers.openai.com/codex since the
recon brief had them marked VERIFY (confirmed: AGENTS.md walk-up + project
`.codex/config.toml`, user `~/.codex/config.toml`, TOML not JSON).
Added a registry sanity test (`adapters::registry_tests`): exactly 12
adapters registered, unique ids, none false-detects an empty root.
T3.1's PLAN.md verify condition (12/12 round-trip green) is now met — ticked
the checkbox. PROJECT.md's Milestone 4 line is marked done at the unit-test
level; the cross-tool `packages/e2e` suite (tier1-4) is separate follow-on
work, not required by T3.1's own verify text.
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 77 tests green
(1 ignored, platform-dependent), `cargo fmt --check` clean.
Next: T3.2 (projection engine: symlink-vs-generate policy table,
mappings.json, drift detector) or T3.3 (red-team pass) — both unblocked now
that T3.1 is done. T3.4 is Gate 2 (human-run `cli scan && cli import
--dry-run`), which halts the RALPH loop for review once reached.

## T3.2 — Projection engine — 2026-07-12
What: Read every adapter's `project()` in `packages/core/src/adapters/*.rs`
to classify each tool's output artifacts as Brain-owned-outright (`Rules`,
`Skill`, `Agent`) vs merged-into-a-file-with-unrelated-user-state
(`MergedConfig` — e.g. mcp settings JSON, `.aider.conf.yml`,
`.codex/config.toml`). Extended `packages/core/src/projector.rs` with an
`Artifact` enum, a `POLICY_TABLE` covering all 12 registered adapter ids,
`is_symlink_candidate()`, and `policy_for(tool_id, artifact,
symlink_privilege_available)`; unknown `(tool_id, artifact)` pairs default
to `Generate` (safe fallback, never a guessed `Symlink`). Added
`packages/core/src/mappings.rs` (`Mapping`/`MappingsFile`, serde JSON
load/save, missing-file-on-first-run returns empty rather than erroring —
matches MASTER_PROMPT.md's `.brain/mappings.json` "source↔canonical↔
projection + hashes" description) and `packages/core/src/drift.rs`
(`DriftStatus`: Clean/Missing/ContentChanged/SymlinkDetached/
SymlinkRetargeted, using `fs::symlink_metadata`+`read_link` to avoid
following symlinks and `compute_sha256` reused from `adapters::mod` for
content checks). Wired both into `lib.rs`. Scope was `packages/core` only —
adapters' `project()` methods do not yet call the policy table or write
mappings.json; that wiring is follow-on work, not required by T3.2's own
verify text ("policy table present and tested").
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 72 lib tests
(was 59) + 16 stress tests (1 ignored, platform-dependent) + 5 CLI tests,
all green; `cargo fmt --check -p neurosurgeon-core` clean. Ticked T3.2's
checkbox in PLAN.md.
Next: T3.3 (red-team pass: symlink escape, path traversal, circular links,
malformed configs — report in docs/security.md) is next and unblocked.
T3.4 is Gate 2 (human-run `cli scan && cli import --dry-run`), which halts
the RALPH loop for review once reached — do not attempt to pass that gate
autonomously.

## T3.3 — Red-team pass — 2026-07-12 — T3.3 COMPLETE
What: Adversarially reviewed every adapter's `import()`/`project()` for
symlink escape, path traversal, circular links, and malformed configs, per
MASTER_PROMPT.md §1 pillar 8 ("import never follows symlinks outside
scanned roots"). Found and fixed 4 real issues, all pre-existing in the
T3.1 code (not introduced this session): (1) `github-copilot`'s scoped-skill
projection had a live path-traversal bug — the repo's own
`adapter_stress_tests.rs` already contained a test that *asserted the
vulnerability existed*, rather than that it was blocked; (2) the identical
slug-into-path pattern in `cursor`, `continue`, and `claude-code`'s
projection (rule/skill/agent slugs built from imported data, joined onto
`root` unvalidated); (3) `github-copilot`'s directory scanner
(`find_instruction_files`) followed symlinks and hung forever on a symlink
cycle — the repo's stress test for this was `#[ignore]`d rather than fixed;
(4) `cursor`/`continue`/`claude-code`'s `import()` directory listings
(`.cursor/rules`, `.continue/rules`, `.claude/skills`, `.claude/agents`)
read any symlinked entry's target content into the canonical Brain
regardless of where it pointed. Fixed all four via a new shared
`adapters::safe_join()` helper (rejects `..`/absolute components,
`packages/core/src/adapters/mod.rs`) and switching directory-listing
filters from `Path::is_dir()`/`is_file()` (follows symlinks) to
`DirEntry::file_type()` (does not). Reviewed all 12 adapters and confirmed
in docs/security.md that only these 4 adapters have path-construction-from-data
or directory-walk surface at all — the other 8 write/read only fixed
literal filenames. `windsurf`'s known write to `$HOME/.codeium/...`
(outside the project root) was reviewed and confirmed intentional, not a
bug — it's a fixed literal path, not attacker-influenced or root-relative.
Malformed-config handling was reviewed and found already adequately
covered by T3.1's existing stress tests; no new findings there. Full
report: docs/security.md.
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 106 tests
green (84 lib + 17 stress + 5 CLI), 0 ignored (was 1 ignored pre-fix — the
symlink-loop hang test now runs and passes); `cargo fmt --check
-p neurosurgeon-core` clean. Ticked T3.3's checkbox in PLAN.md.
Next: T3.4 is Gate 2 — a human runs `cli scan && cli import --dry-run` on
their real machine and approves the printed migration report before
anything is written. This halts the RALPH loop; do not attempt to pass
this gate autonomously. Phase 3 (T3.1–T3.3) is now fully complete and
green; at the time this entry was first written, `apps/cli`'s verbs were
still `not_yet_implemented` stubs (T2.3's scope was only `--help`
completeness), which would have made Gate 2 unreachable. That gap has
since been closed — see the entry below.

## Wired `cli scan` / `cli import --dry-run` to packages/core — 2026-07-12
What: T3.4's Gate 2 asks a human to run `cli scan && cli import --dry-run`
and approve the printed report, but `apps/cli` had no code path to
`packages/core` yet — every verb was a `not_yet_implemented` stub (T2.3's
own scope was only `--help` completeness, not real behavior). This isn't a
PLAN.md line item itself, but it's a hard prerequisite for T3.4 to be
attemptable at all, so closed it now rather than leaving the gate
unreachable. Added `neurosurgeon-core` + `serde_json` as `apps/cli`
dependencies. `cli scan [--json]` now calls every registered adapter's
`detect()` against the current directory and reports which tools were
found. `cli import --dry-run` calls `detect()` + `import()` per adapter and
prints a migration report (skill/agent/mcp-server counts and ids) —
nothing is written to disk. Real (non-dry-run) `cli import` still refuses
and points at PLAN.md T4.x, since no Brain-write path exists in
`packages/core` yet (writing imported data into `~/AIBrain` is Phase 4
scope, not part of T3.2/T3.3). `project`/`sync`/`doctor`/`snapshot`/
`rollback` remain stubs — unaffected, out of scope for reaching Gate 2.
Manually ran the built binary against this repo as a smoke test:
`cargo run -p neurosurgeon -- scan` correctly detected `claude-code` (this
repo's own CLAUDE.md + .claude/agents/*.md), and `import --dry-run`
printed a real report (7 skills, 6 agents, 0 mcp servers) without touching
the filesystem — confirmed by a test that diffs `read_dir` before/after.
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 110 tests
green (9 CLI + 84 core lib + 17 core stress + 0 doctests), 0 ignored;
`cargo fmt --check -p neurosurgeon-core -p neurosurgeon` clean.
Next: T3.4 (Gate 2) is now genuinely reachable — a human can run
`cli scan && cli import --dry-run` against their real machine today and
get a real, honest report. This halts the RALPH loop; a human must run it
and approve before any further Phase 3/4 work touches real filesystem
writes. Do not attempt to simulate or bypass this gate.

## T3.4 — Gate 2 — APPROVED — 2026-07-12
What: Ran `cargo run -p neurosurgeon -- scan` then
`cargo run -p neurosurgeon -- import --dry-run` against this repo
(`/home/vers/Desktop/llm-neurosurgeon`) as the human-run smoke test T3.4
requires. `scan` detected 1 tool (`claude-code`). `import --dry-run`
reported 7 skills, 6 agents, 0 MCP servers and wrote nothing. Before
asking for approval, cross-checked every claim in the printed report
against the actual filesystem rather than trusting the tool's own output:
confirmed no other adapter's config files exist at repo root (no
`.cursorrules`, `.clinerules`, `AGENTS.md`, `GEMINI.md`, `.windsurfrules`,
`.aider.conf.yml`, `.roomodes`, `.codex/config.toml`, `.continue/`,
`.zed/`, `.github/copilot-instructions.md` — `.github/` only holds
`workflows/`, correctly not triggering the copilot adapter); confirmed the
reported `claude-code-memory` skill's sha256 matches `sha256sum CLAUDE.md`
byte-for-byte; confirmed the 6 reported agent skills/agents match the 6
files in `.claude/agents/` 1:1; confirmed 0 MCP servers is correct (no
`.mcp.json` at root). Human reviewed this and responded "approved".
**Gate 2 is now passed.** Ticked T3.4's checkbox in PLAN.md.
Next: Phase 4 — Sync Daemon & Time Machine is now unblocked: T4.1
(filesystem watcher + OS scheduler registration), T4.2 (conflict queue API
+ three-way merge), T4.3 (git snapshot/rollback + crash-safe lock). No
further gate stands between here and Phase 5's GATE 3 (human review of the
GUI), so Phase 4 work can proceed without stopping for approval at each
task — only pause if a design decision requires human judgment the plan
doesn't already resolve.

## T4.1 — Filesystem watcher + OS scheduler registration — 2026-07-12
What: Added `notify = "6"` as a `packages/core` dependency and built two
new modules. `packages/core/src/watcher.rs`: `DebouncedWatcher::watch(root,
debounce)` wraps `notify`'s raw per-syscall event stream (an editor save
can be temp-write + rename + chmod — three raw events for one logical
change) and `next_batch(initial_wait)` blocks up to `initial_wait` for the
first event, then keeps collecting until `debounce` passes with no further
events, returning every distinct path touched. Bounded on both ends by
design, so it can never hang forever even if the host has no working
inotify. `packages/core/src/scheduler.rs`: renders a `ScheduledJob` into
each OS's native recurring-task format — a launchd `.plist` for macOS, a
systemd user `.service`+`.timer` pair for Linux, an `schtasks /create`
command line for Windows (Windows has no declarative unit-file format the
way launchd/systemd do) — parameterized by a `SchedulerOs` enum so all
three render from one test binary regardless of host OS, same pattern as
T3.2's per-tool `Artifact` table. `write_job_files()` writes the rendered
config under a caller-supplied `home` path (tests use tempdir; the real
default is `scheduler::default_home()`, unread by anything yet).
Deliberately stopped short of calling `launchctl load` / `systemctl --user
enable --now` / running the `schtasks` command against the real host —
registering a persistent background service is a system-wide,
hard-to-reverse action that belongs behind an explicit human-approved
install step (same posture as Gate 2: generate/dry-run first), not
something this pass does silently. Wired both modules into `lib.rs`.
While verifying the full suite, found and fixed a **pre-existing, unrelated
test-isolation bug**: three tests in `adapter_stress_tests.rs`
(`test_windsurf_adapter_missing_files_graceful`,
`test_windsurf_adapter_malformed_json`,
`test_windsurf_adapter_writes_outside_root`) each mutate the process-global
`$HOME` env var and run concurrently by default, so one test's `HOME`
could leak into another's assertions — reproduced the flake, then fixed it
with a shared `Mutex<()>` (`HOME_ENV_LOCK`) all three now hold for their
full duration. Confirmed fixed by rerunning the stress suite 5x in a row,
green every time (was intermittently red before).
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 119 tests
green (9 CLI + 93 core lib + 17 core stress + 0 doctests), 0 ignored,
stable across 3 consecutive full runs; `cargo fmt --check
-p neurosurgeon-core -p neurosurgeon` clean; no new build warnings.
Next: T4.2 (conflict queue API + three-way merge) or T4.3 (git
snapshot/rollback + crash-safe lock) — both unblocked. Neither depends on
the other, but T4.3's snapshot-before-destroy machinery is the safer one
to land first per CLAUDE.md's standing rule 4 ("snapshot before destroy"),
since T4.2's conflict resolution will want it as a safety net once it
starts writing merged content.

## T4.3 — Git snapshot/rollback + crash-safe lock — 2026-07-12
What: `packages/core/src/snapshot.rs`. Shells out to the real `git` binary
via `std::process::Command` rather than adding a `git2`/libgit2 binding —
this project's own Tauri build already hit missing-system-library failures
(`libsoup`/`javascriptcoregtk`) in sandboxed environments, and `git` is
guaranteed present. `ensure_repo()` idempotently `git init`s the Brain
directory and sets a local commit identity (so it works with no global
git config, common in fresh sandboxes/CI). `snapshot(brain_root, message)`
stages everything and commits, but returns the existing `HEAD` instead of
an empty commit when nothing changed — a quiescent Brain shouldn't spam
the log. `rollback(brain_root, commit)` restores the working tree to
match `commit` via `git checkout <commit> -- .`, then deletes any files
present now but absent from that commit's tree (checkout alone only
restores paths that exist in the target — it doesn't delete paths added
since), and records the restore as a **new** commit rather than moving
HEAD backwards — per MASTER_PROMPT.md's "Time Machine" framing, rolling
back is a new point in time, not a rewrite of history, so "undo the
rollback" is just another rollback to whatever came right before it.
`SnapshotLock` is the crash-safe lock: an RAII guard that atomically
creates a PID-stamped lock file and removes it on drop; if `acquire()`
finds an existing lock, it checks whether the recorded PID is still alive
via `/proc/<pid>` (Unix) before deciding to block vs. reclaim — a lock
left behind by a process that crashed mid-sync is detected as stale and
reclaimed rather than wedging the daemon forever. Wired into `lib.rs`.
Test evidence for PLAN.md's own verify text ("rollback byte-identical
test"): `rollback_restores_working_tree_byte_identical_and_preserves_history`
snapshots a tree, diverges it (edit one file, delete another, add a third),
snapshots again, rolls back to the first snapshot, and asserts a full
sha256 hash of every file in the tree is identical to a hash taken right
after the first snapshot — plus asserts `git log` shows 3 commits, not 1,
proving history wasn't rewritten.
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 126 tests
green (9 CLI + 100 core lib + 17 core stress + 0 doctests), 0 ignored,
stable across 3 consecutive runs of the new snapshot tests specifically
(git-subprocess-based tests are exactly the kind that can flake, so
verified repeatedly rather than trusting one green run); `cargo fmt
--check -p neurosurgeon-core -p neurosurgeon` clean; no new build
warnings.
Next: T4.2 (conflict queue API + three-way merge) is the last item in
Phase 4, now with T4.3's snapshot/rollback available as its safety net.
Phase 4's own self-verify text in MASTER_PROMPT.md (kill-during-sync
leaves repo clean; simultaneous-edit test produces exactly one queued
conflict; rollback restores byte-identical state; 1h soak test) goes
beyond what any single T4.x task's own `verify:` line asks for — the soak
test in particular is not something to attempt inside a single automated
session. After T4.2, Phase 5 begins with GATE 3 (human review of the GUI),
which halts the RALPH loop; the `apps/desktop` Tauri app cannot even be
built in this sandbox yet (T2.2's unresolved GTK/libsoup system-library
blockers), so reaching Phase 5 will need that fixed first, likely on a
real (non-sandboxed) machine — flag this to the human rather than
attempting to build it into this session's own PLAN.md task graph.

## T4.2 — Conflict queue API + three-way merge — 2026-07-12 — PHASE 4 COMPLETE
What: `packages/core/src/merge.rs` and `packages/core/src/conflict_queue.rs`.
Rather than hand-rolling a diff3 algorithm — a buggy bespoke three-way
merge would silently corrupt a user's skill/rule content, exactly what
this project's safety-by-design pillar exists to prevent — added `diffy`
(pure Rust, no C dependency) and wrapped its `merge()` in
`three_way_merge(base, local, remote) -> MergeOutcome`. Probed diffy's
actual behavior with a throwaway scratch binary before writing any real
code against it (not just trusting the docs): confirmed disjoint edits
merge cleanly, *identical* concurrent edits also merge cleanly (both
sides making the same change isn't a conflict), and only genuinely
overlapping+different edits produce `<<<<<<< / ||||||| / ======= /
>>>>>>>` markers — the same format `git merge` leaves, directly reusable
as the GUI review queue's starting content. `conflict_queue.rs`'s
`ConflictQueue` (load/save via the same missing-file-is-empty convention
as `MappingsFile`) holds `QueuedConflict` entries (id, canonical path, all
three sides, and the pre-merged markers text); `reconcile(queue,
canonical_path, base, local, remote)` is the entry point the sync daemon
will call per changed item — returns `Some(merged)` safe to write back on
a clean merge, enqueues and returns `None` on a real conflict so nothing
gets written until a human resolves it. Hit a genuine edge case while
writing tests: a 3-line fixture with adjacent single-line edits and no
surrounding unchanged context (e.g. editing line 2 and line 3 of a
3-line file) triggered a conflict even though the edits were
logically disjoint — this is an inherent property of line-based diffing
(insufficient context to prove non-overlap, not a bug in this code or in
diffy) and the same ambiguity `git merge` exhibits on short files.
Fixed the test to use a realistic fixture with unchanged lines around each
edit rather than masking it, and documented why in a comment so a future
reader doesn't mistake it for a real bug.
Evidence: `cargo test -p neurosurgeon-core -p neurosurgeon` — 137 tests
green (9 CLI + 111 core lib + 17 core stress + 0 doctests), 0 ignored,
stable across 3 consecutive runs; `cargo fmt --check -p neurosurgeon-core
-p neurosurgeon` clean; no new build warnings. Ticked T4.2's checkbox —
**Phase 4 is now fully complete (T4.1, T4.2, T4.3 all green).**
Next: Phase 5 begins with T5.1 (`ui-builder` implements 8 screens from
DESIGN_PACK.md) and ends at **GATE 3** (human review of the GUI + demo) —
a human gate, so the RALPH loop halts there; do not attempt to pass it
autonomously. Before any GUI work is meaningful, `apps/desktop/src-tauri`
needs to actually build — T2.2 flagged unresolved GTK/libsoup/
javascriptcoregtk system-library blockers that reproduced again in this
session's own sandbox (`cargo build` fails on `apps/desktop/src-tauri`
specifically; `packages/core`/`apps/cli` are unaffected since neither
depends on Tauri/GTK). This blocks T5.1 from being verifiable in this
environment and needs a human on a real desktop machine (or a sandbox
with the GTK/webkit2gtk dev packages installed) before Phase 5 can
proceed — flagging this now rather than attempting GUI work that can't be
built or screenshotted here.

## Unblocked apps/desktop/src-tauri build (human-directed) — 2026-07-12
What: Human asked to install the missing system packages. Installed
`libwebkit2gtk-4.1-dev`, `libjavascriptcoregtk-4.1-dev`, `libsoup2.4-dev`,
`libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`,
`libssl-dev`, `build-essential` via `apt-get`. This resolved the
`libsoup-2.4` half of the original error but exposed the real, deeper
blocker: Ubuntu 24.04/Linux Mint 22 don't ship `webkit2gtk-4.0`/
`javascriptcoregtk-4.0` at all (only the 4.1 generation) — Tauri **v1.5.0**
(what `apps/desktop/src-tauri/Cargo.toml` was pinned to) hardcodes lookups
for the 4.0 names, so no package install could ever fix it. Presented this
to the human as a choice (upgrade to Tauri v2 / hacky pkg-config symlink
aliasing / stop here); human chose **upgrade to Tauri v2** — also the
right call since MASTER_PROMPT.md's own default stack says "Tauri 2" and
the v1.5.0 pin was itself a pre-existing spec deviation.
Migration: bumped `tauri`/`tauri-build` to `2` in Cargo.toml. Pulled
current Tauri v2 API docs via Context7 before writing code (menu API
signatures, `WebviewWindowBuilder`/`WebviewUrl`, `tauri.conf.json`'s v2
schema) rather than guessing from training-data knowledge of v1. Rewrote
`tauri.conf.json` to the v2 schema (top-level `identifier`/`productName`/
`version`, `app.windows`, `build.frontendDist`; dropped the v1 `allowlist`
block entirely since it only gates built-in JS APIs this app doesn't use,
not custom `#[tauri::command]`s). Ported `lib.rs`'s menu construction from
v1's `CustomMenuItem`/`Menu::new().add_item()` to v2's
`tauri::menu::{Menu, MenuItem, Submenu}` builders (constructed inside
`setup()` via `app.handle()`, since v2 menu items need a manager
reference) and the window from `WindowBuilder`/`WindowUrl` to
`WebviewWindowBuilder`/`WebviewUrl` (v2 split "window" and "webview" as
concepts). Switched `commands::open_settings`'s `Window` parameter to
`WebviewWindow` per v2 docs. Along the way, fixed a **latent runtime bug**
predating this session: v1's `tauri.conf.json` declared its own default
window AND `lib.rs`'s `setup()` imperatively built a second window with
the identical `"main"` label — first launch would have panicked on a
duplicate label. Removed the config-declared window (the app clearly
means to build its window in code, not both).
Iterating to a clean build surfaced 4 more previously-unreached errors
(none related to the Tauri version — pre-existing, just never compiled
far enough to hit): missing `icons/icon.png` (`tauri::generate_context!()`
hard-requires one regardless of the bundle icon list being empty —
generated a placeholder RGBA PNG set with ImageMagick; real branding is
T5.1 scope, not this fix), `log::set_boxed_logger` needing the `log`
crate's `std` feature explicitly enabled, a real borrow-checker error in
`commands::import_config` (`app_state.update_adapter_status(app_state.
adapter_count, ...)` — fixed by binding `adapter_count` to a local before
the mutable-borrowing call), and `AdapterCommand` needing `pub` for
`generate_handler!`'s macro-generated code to see it.
Also bumped `apps/desktop/package.json`'s `@tauri-apps/api` to `^2.0.0` to
match (frontend work is still T5.1 scope; this just avoids leaving a
v1-vs-v2 API mismatch sitting there). Added `apps/desktop/src-tauri/gen/`
to `.gitignore` (Tauri v2's build.rs generates capability-schema JSON
there, same category as `target/`). Added a dated addendum to the top of
`docs/CODE_REVIEW.md` rather than rewriting its historical body — noted
which of its old §1 blockers were already fixed before this session
(before I ever touched this file) vs. what this session actually fixed.
Evidence: `cargo build --workspace` — clean, zero warnings, all three
crates (`packages/core`, `apps/cli`, `apps/desktop/src-tauri`) compiling
together for the first time. `cargo test --workspace` — 0 failures across
every test binary (137 core lib+stress tests, 9 CLI tests, desktop-app has
no unit tests yet — untouched by this fix, not a regression). `cargo fmt
--all -- --check` clean across the whole workspace.
Next: `apps/desktop/src-tauri` now genuinely builds, so T5.1 (`ui-builder`
implements 8 screens from DESIGN_PACK.md) is unblocked in this
environment for the first time. Checked docs/CODE_REVIEW.md §3's claim
that the frontend scaffold is missing — also stale: `index.html`,
`vite.config.ts`, `tsconfig.json`, and `src/App.tsx` already exist. Fixed
one thing my own `@tauri-apps/api` v1→v2 bump broke: `App.tsx` imported
`invoke` from `@tauri-apps/api/tauri` (the v1 path); v2 moved it to
`@tauri-apps/api/core`. Updated the import, then verified it for real:
`npm install` in `apps/desktop` (no `pnpm` binary available in this
sandbox despite the root `pnpm-lock.yaml`/`pnpm-workspace.yaml`; used
`npm` against `apps/desktop`'s own `package-lock.json` instead) resolved
`@tauri-apps/api` to `2.11.1`, then `npm run build` (`tsc && vite build`)
succeeded — `tsc` type-checked cleanly (would have failed loudly on a bad
import path) and `vite build` produced `dist/` output (142.98 kB JS
bundle). `npm audit` flagged one high-severity `vite` dev-server CVE
cluster — pre-existing, unrelated to this migration, and fixing it means
a breaking `vite@8` major bump; left alone, not this session's job.
Phase 5 still ends at **GATE 3** (human review of the GUI + demo), a
human gate; the RALPH loop halts there. With both the Rust and frontend
builds now confirmed working end-to-end, T5.1 has no remaining
environment blocker — it's ready to start.

## T5.1 — 8 screens from DESIGN_PACK.md — 2026-07-12
What: Implemented all 8 screens from DESIGN_PACK.md's "ASCII Wireframes -
T0.4" section (the canonical, literal spec — not the separate "UI
Components Overview" bullet count, which sums to 8 for a different
grouping): Main Dashboard, Configuration Manager, Adapter Inspector,
Status Monitor, Debug Console, Onboarding Wizard, Marketplace, MCP Hub.
Each screen (`apps/desktop/src/screens/*.tsx`) translates its wireframe's
sections structurally — same cards, same field labels, same status
values, same bottom action-bar buttons — rather than a loose
reinterpretation. Built 3 shared primitives (`components/ui.tsx`): `Card`
(the boxed panels every wireframe uses), `StatusPill` (tone-colored badge
for OK/Syncing/Error/Inactive states), `Toolbar`/`ToolbarButton` (the
bottom action row). Set the Visual Tokens from DESIGN_PACK.md: primary
`#667eea` (Brand A/Cortex — the first-listed candidate, since Gate 0's
brand pick is still unconfirmed in QUESTIONS.md; documented as such in a
`tailwind.config.js` comment so it's easy to swap once a human confirms),
dark theme default (`color-scheme: dark`, slate-950 background), Inter/SF
Mono per the stated typography. `App.tsx` is now a sidebar-nav shell
switching between the 8 screens via local `useState` — no router library
added, matching the Voice Rules' "No Boarders: no unnecessary
abstractions." Guarded the existing `invoke('get_version')` call behind a
`'__TAURI_INTERNALS__' in window` check so the app degrades gracefully
(shows "—" instead of crashing) when previewed in a plain browser rather
than inside the real Tauri webview — needed for the verification below.
Verification (per the standing rule to actually drive UI changes in a
browser, not just claim they render): `mcp__claude-in-chrome` had no
browser extension connected in this sandbox, so fell back to a scripted
Playwright-over-system-Chrome driver (`playwright-core` pointed at
`/usr/bin/google-chrome` via `executablePath`, avoiding a fresh browser
download) — started `npm run dev`, clicked through all 8 sidebar nav
items, screenshotted each, and read every one back to actually look at
the rendered output rather than trust the click-succeeded log line.
Confirmed: dark theme and primary color render correctly, status pills
are tone-colored correctly (green/amber/red), the debug console's
monospace log block and the onboarding wizard's radio-button state both
render as intended, and all 8 screens structurally match their wireframes.
Copied the 8 screenshots into `apps/desktop/screenshots/` — satisfies
T5.1's own verify text ("screenshot folder populated") with real,
human-reviewable evidence, not a claim. Stopped the dev server and
cleaned up the scratch Playwright driver afterward — nothing left running.
Evidence: `npm run build` (`tsc && vite build`) clean, zero type errors;
`cargo build --workspace` still clean (Rust side untouched by this
screen work). Ticked T5.1's checkbox in PLAN.md.
Next: T5.2 (onboarding wizard wraps the dry-run flow — i.e. wire
`OnboardingWizard.tsx`'s static mock UI to the real `cli import
--dry-run`-equivalent path once a Tauri command exists for it) is
unblocked. T5.3 is **GATE 3** — a human reviews the screenshots + a live
demo and approves; this halts the RALPH loop, do not attempt to pass it
autonomously. Two things worth flagging to the human before T5.3: (1) the
brand primary color defaulted to Cortex (#667eea) since Gate 0's brand
pick was never actually confirmed in QUESTIONS.md despite PLAN.md marking
T0.5 complete — worth resolving properly rather than leaving this
default; (2) these 8 screens are static/mock-data UI only — no commands
beyond the pre-existing `get_version` are wired to the real
`packages/core` engine yet, which is expected for T5.1's own scope but
means the GUI doesn't yet do anything real end-to-end.

## T5.2 — Onboarding wizard wraps dry-run flow — 2026-07-12
What: Added a real Tauri command,
`apps/desktop/src-tauri/src/dry_run.rs::scan_dry_run()`, that mirrors
`apps/cli`'s `report_scan`/`report_import_dry_run` exactly — same
`detect()` + `import()` calls against every `packages/core` adapter, same
nothing-is-written guarantee — just returning structured
`DryRunReport`/`DetectedTool` JSON instead of printing text. Added
`neurosurgeon-core` as an `apps/desktop/src-tauri` dependency to make this
possible (mirrors what T3.4's CLI-wiring work already did for `apps/cli`).
Rewrote `OnboardingWizard.tsx` from its T5.1 single-step mock into a real
3-step flow: Step 1 (environment select, unchanged) → Step 2 (calls
`scan_dry_run()` via `invoke()`, shows a loading state, then the real
detected-tool report with per-tool skill/agent/mcp counts and an explicit
"Dry run — nothing was written" notice) → Step 3 (completion, honest that
writing into the Brain isn't implemented yet). Guarded the invoke call
with the same `'__TAURI_INTERNALS__' in window` check `App.tsx` already
used, so the wizard fails with a visible, non-crashing message rather
than an uncaught exception when there's no Tauri backend (e.g. previewed
in a plain browser).
Set up Vitest + Testing Library in `apps/desktop` for the first time
(`vitest`, `@testing-library/react`, `@testing-library/jest-dom`,
`@testing-library/user-event`, `jsdom` as new dev dependencies;
`vite.config.ts` extended with a `test` block; `npm test` now runs
`vitest run` instead of the old placeholder echo) and wrote the actual
"e2e onboarding test" T5.2's own verify line asks for:
`src/screens/__tests__/OnboardingWizard.e2e.test.tsx` — 4 tests that
drive the wizard by clicking through it with `@testing-library/user-event`
(not calling internals directly), mocking only the one seam that
genuinely can't run outside a real Tauri webview (`@tauri-apps/api/core`'s
`invoke`): the full 3-step walk with a real-shaped detected-tool report,
an empty-detection state, the no-Tauri-backend graceful-degradation path
(this is exactly the situation T5.1's own screenshot verification hit),
and back-navigation from the scan step.
Verification delegated to a Workflow at the user's request (`ultracode` →
explicit ask to run T5.2's remaining checks via multi-agent orchestration):
5 parallel agents each independently ran one check (`cargo build
--workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`,
`npm run build`, `npm test`) against the real repo. All 5 came back
clean; a conditional fix-pass agent (triggered by an overly broad regex
in the workflow's own pass/fail classifier — a real false positive, not
an actual failure) independently re-ran all 5 checks itself, confirmed
they were genuinely green, and made no changes — a good instance of an
agent verifying rather than trusting a possibly-wrong signal.
Evidence: `cargo test --workspace` — 138 tests green (was 137 pre-T5.2;
+1 for `dry_run::tests::scan_dry_run_succeeds_against_the_real_current_directory`),
0 failed. `npm test` (vitest) — 4/4 passed. `npm run build` and `cargo
build --workspace` both clean. `cargo fmt --all -- --check` clean. Ticked
T5.2's checkbox in PLAN.md.
Next: T5.3 is **GATE 3** — a human reviews the screenshots (published as
an Artifact gallery earlier this session) plus a live demo and approves.
This halts the RALPH loop; do not attempt to pass it autonomously. Two
things still worth flagging going into that review: (1) the brand primary
color is still the Cortex placeholder (#667eea) — the human explicitly
chose to leave Gate 0's brand pick open rather than resolve it now; (2)
the onboarding wizard's dry-run step scans the directory the Tauri
process's CWD happens to be — there's no folder picker yet, which a real
user would need (adding one is `tauri-plugin-dialog` + a permissions
capability, not attempted here — kept T5.2 scoped to "wraps dry-run flow"
as literally stated, not scope-creeping into UX polish beyond that).

## T5.3 — Gate 3 — APPROVED — 2026-07-12
What: Republished the T5.1 screenshot gallery (same Artifact URL) with
T5.2's changes folded in — the Onboarding Wizard card now shows all 3
real steps (env select → live `scan_dry_run` call with its loading state
→ completion) instead of the single stale pre-T5.2 screenshot, with an
explicit caption noting step 2's backend response is mocked for
screenshot purposes while the request/response wiring itself is the real
code path (same one the e2e test exercises). Human reviewed the 10-screen
gallery and responded "approved." **Gate 3 is now passed.** Ticked T5.3's
checkbox in PLAN.md. Also logged in DECISIONS.md that the human
explicitly chose to leave the brand primary color as the Cortex
placeholder for now — a real decision (asked directly, not defaulted
silently), just not a Gate 0 resolution; QUESTIONS.md's brand/theme/
stack/brain-location checkboxes remain formally unconfirmed.
Next: **Phase 6 — Marketplace & MCP Hub** is unblocked: T6.1 (marketplace
importers for skill sources, verify: 3 real skills from anthropics/skills
import — this one needs real network access to a real GitHub repo, worth
confirming that's available/intended before assuming it can run
unattended in a sandbox), T6.2 (MCP registry importers + health-check
handshake, verify: 2 registry MCP servers end-to-end — also likely needs
real network access to real MCP registries), T6.3 (secrets flow: keychain,
placeholders, project-to-all-tools, verify: secret fixture round-trip —
this one is probably fully offline-testable, unlike T6.1/T6.2). No human
gate stands until Phase 7 (T7.1's red-team sign-off, not itself a human
gate per PLAN.md's own gate markers) — Phase 8's T8.3 is the next actual
human gate (Gate 4). Given T6.1/T6.2 likely need real external network
access this sandbox may or may not have reliably, flag that to the human
before assuming those can run fully autonomously end-to-end.

## T6.1 — Marketplace importers — 2026-07-12
What: Recon-verified `api.github.com/repos/anthropics/skills` is live and
inspected its real structure before writing code (17 skills under
`skills/<slug>/SKILL.md`, frontmatter with name/description/license).
Added `packages/core/src/marketplace.rs`: `list_anthropic_skills()` +
`fetch_anthropic_skill(slug)` → `MarketplaceSkill` carrying provenance
(exact source URL), license note, sha256 checksum, an
executable-content flag (extension scan of the skill's file tree — not a
sandboxed lint, deliberately marked as such), and **`enabled: false`
always** per MASTER_PROMPT.md pillar 8's untrusted-import rule; nothing
here writes into the Brain or any tool config. New deps: `ureq`
(rustls-based — no system TLS library, deliberately avoiding another
libsoup-style trap) and `base64`. Scope note: only the `anthropics/skills`
source is implemented; MASTER_PROMPT.md's other listed sources
(awesome-lists, Gemini extensions) have no stable API and are
feature-flagged out per its own "verify endpoints live via recon first"
instruction. Evidence: T6.1's verify condition ("3 real skills from
anthropics/skills import") is met by a real network test that imports 3
live skills end-to-end with 3 distinct sha256 checksums (skips loudly if
offline rather than failing spuriously); 4 more hermetic unit tests cover
frontmatter parsing and the enable/disable invariant.

## T6.2 — MCP registry importers + health-check handshake — 2026-07-12
What: Recon-verified `registry.modelcontextprotocol.io/v0/servers` live
and inspected both entry shapes (remote `remotes[]` with
streamable-http URLs; stdio `packages[]` with npm identifier +
`runtimeHint: npx` + `environmentVariables` with `isSecret` flags) before
writing code. Added `packages/core/src/mcp_registry.rs`:
`search_official_registry(query, limit)` → `RegistryServer` (canonical
`McpServer` + description/version/sha256 provenance). Mapping rules:
remote endpoint wins over package when both exist; npm packages become
`npx -y <identifier>` stdio invocations; env vars are captured as **names
only** (`env_placeholders`) — values never fetched/stored, keychain half
is T6.3; entries with no usable endpoint are dropped. Health check per
MASTER_PROMPT.md's "spawn + handshake": `health_check_stdio()` actually
spawns the command, writes a JSON-RPC `initialize` request to stdin, and
waits (bounded, reader-thread + recv_timeout, same never-hang pattern as
watcher.rs) for a valid id-matched response; child killed afterward.
`health_check_remote()` POSTs the same initialize request
(Accept: json + SSE per streamable-http), treats 401/403 as Healthy
(alive but wants auth). Other 4 catalogs (Smithery/PulseMCP/mcp.so/
Docker) feature-flagged out — not recon-verified this pass.
Evidence: T6.2's verify ("2 registry MCP servers end-to-end") met twice
over: (1) live test imports 2 real registry entries with distinct
checksums; (2) `examples/probe_remote.rs` ran a real initialize handshake
against two live registry servers (`api.inference.sh/mcp`,
`tandem.ac/mcp`) — both returned Healthy. Fixture-side, a shell-script
stdio MCP server exercises the full spawn+handshake hermetically
(Phase 6 self-verify's "end-to-end on fixtures"), plus dead-command and
never-responds timeout cases. `cargo test --workspace` — 150 tests green,
0 ignored; `cargo fmt --all --check` clean.
Next: T6.3 (secrets flow: keychain, placeholders, project-to-all-tools,
verify: secret fixture round-trip). Note for T6.3: real OS-keychain
access from this sandbox is unlikely to work headlessly (Secret Service
needs a DBus session + unlocked keyring); the honest scope is the
placeholder/round-trip machinery against a pluggable keystore trait with
a file/memory fixture backend, with the OS-keychain backend
feature-gated for real desktop use.

## T6.3 — Secrets flow — 2026-07-12 — PHASE 6 COMPLETE
What: The prediction in the note above turned out wrong in the good
direction — probed first and found this machine has a **live**
`org.freedesktop.secrets` gnome-keyring daemon on the user DBus, so the
real keychain backend was buildable AND testable here, not just
feature-gated theory. Added `packages/core/src/secrets.rs`: a
`SecretStore` trait with two backends — `MemorySecretStore` (fixture,
default) and `OsKeychainStore` behind a new `os-keychain` cargo feature
(the `keyring` crate with `sync-secret-service`/`apple-native`/
`windows-native` features: Secret Service on Linux, Security.framework on
macOS, Credential Manager on Windows, one dependency line; optional so
headless CI never needs an unlocked keyring). Flow per MASTER_PROMPT.md
pillar 6 ("secrets stored in the OS keychain and written to tool configs
as env placeholders only"): `harvest_env()` moves real values into the
store under namespaced keys (`mcp/<server>/<VAR>`) and returns a map
holding `${VAR}` placeholders only — never stores placeholders or empty
values; `resolve_env()` swaps placeholders back, failing loudly
(`NotFound`) on a placeholder with no stored secret rather than launching
a child with a literal `${...}` in its environment. One stored secret
serves every projected tool ("project-to-all-tools") since all adapters
share the same store key.
Evidence: T6.3's verify ("secret fixture round-trip") is
`secret_fixture_round_trips_through_harvest_and_resolve` — real value →
placeholder → identical value back, plus the missing-secret failure path.
Beyond the required fixture: ran the **real OS-keychain round trip**
against this machine's live gnome-keyring
(`cargo test --features os-keychain`) — set/get/delete all green, probe
entry deleted in-test so nothing lingers in the user's keyring.
`cargo test --workspace` — 155 tests green, 0 ignored; `cargo fmt --all
--check` clean; no build warnings. Ticked T6.3 — **Phase 6 complete
(T6.1, T6.2, T6.3 all green).**
Next: Phase 7 — T7.1 (threat-model pass + red-team sign-off in
docs/security.md; there is real new attack surface since T3.3's pass:
marketplace fetches, registry fetches, health-check process spawning,
secrets handling), T7.2 (Doctor rules library ≥12 diagnoses), T7.3
(auto-update channel dry-run — scope carefully: a real Tauri updater
needs signing keys that don't exist yet; the honest T7.3 is config +
dry-run verification), T7.4 (doc set). No human gate until T8.3
(Gate 4).

## T7.2 — Doctor rules library — 2026-07-14 — COMPLETE
What: The diagnostic engine (`packages/core/src/doctor.rs`) already existed
from a prior session with 13 rules and `apply_fixes`, but two things were
missing to actually close T7.2: (1) the CLI `doctor` command was a
`not_yet_implemented` stub, and (2) there was no single "corrupted fixture
Brain" self-verify — only per-rule unit tests. Both are now done.

CLI (`apps/cli/src/main.rs`): `neurosurgeon doctor [--fix]` is wired to
`diagnose`/`apply_fixes` with a clinical report (severity-tagged lines,
"(fixable — rerun with --fix)" hints). Added `--brain <PATH>` and
`--tool-root <PATH>` overrides; default resolution is
`$NEUROSURGEON_BRAIN` else `~/AIBrain` for the Brain (the documented
default), and `$NEUROSURGEON_TOOL_ROOT` else `$HOME` for the tool root.
Exit code is FAILURE when any Critical diagnosis remains (script/CI
gating), SUCCESS otherwise. Added `dirs = "6"` (already in the lock tree)
for cross-platform home resolution.

Self-verify: added `doctor_fixes_every_seeded_fault_in_a_corrupted_brain`
— one Brain seeded with five simultaneous faults (non-git Brain, missing
generated projection, detached symlink, retargeted symlink, and one
human-only fault: a mapping whose canonical source was deleted). One
`apply_fixes` pass clears every auto-fixable fault (verified by real
filesystem evidence: the generated file has the provenance header + body,
the detached path is now a symlink, the retargeted symlink points at the
right canonical file, `.git` exists) while the human-only Critical fault
stays reported but untouched. Idempotent on a second pass.

Bug found by driving the real binary end-to-end (not just tests): after
`doctor --fix` re-projected a file whose recorded `content_sha256` was
empty/stale, the very next `diagnose` falsely flagged it as
`generated-file-edited` — the Doctor accusing its own handiwork of being a
phantom hand-edit. Fixed `reproject` to heal the record too: it now
updates the mapping's checksum in `mappings.json` to match what it wrote,
so a re-diagnose is a clean bill of health. Regression test:
`fixing_a_missing_projection_heals_the_checksum_so_rediagnose_is_clean`.
Confirmed live: seed empty-checksum Brain → `doctor --fix` → second
`doctor` prints "clean bill of health", exit 0.

Evidence: T7.2's verify ("doctor fixes seeded faults") met by the
corrupted-fixture self-verify above plus the existing per-rule tests and a
real end-to-end CLI run. `cargo test --workspace` — 167 passed, 0 failed;
`cargo fmt --all --check` clean. Ticked T7.2.
Next: T7.3 (auto-update channel dry-run — scope carefully: a real Tauri
updater needs signing keys that don't exist yet, so the honest T7.3 is
config + dry-run verification), then T7.4 (doc set). No human gate until
T8.3 (Gate 4).

## T7.3 — Auto-update channel dry-run — 2026-07-17 — COMPLETE
Honest scope (as flagged at T7.2 close): a shipping Tauri updater verifies a
minisign/ed25519 signature over every downloaded artifact, which needs a
release signing keypair that does not exist yet — generating/safeguarding
those keys is Phase 8 (Package & Release) work behind GATE 4. So T7.3
delivers the half that is real and fully testable now: the update-channel
**decision + dry-run**, with signing represented honestly rather than faked.

What:
- `packages/core/src/updater.rs` — the dry-run engine. `Channel`
  (Stable/Beta; Stable ignores pre-releases, Beta accepts them), a
  `ReleaseManifest`/`Release`/`PlatformAsset` model (the JSON a channel
  endpoint serves), and `check_for_update(current, manifest, channel,
  target)` which parses versions with real semver ordering, picks the
  newest channel-eligible release that ships an asset for the platform, and
  returns an `UpdateDecision` (UpToDate / UpdateAvailable / UnsupportedPlatform
  / NoReleaseForChannel). It **downloads, verifies, and installs nothing** —
  that is the dry-run boundary. A single malformed manifest entry is skipped,
  not fatal; a bad current version is a loud error.
- Signing represented, not faked: `UPDATE_PUBLIC_KEY` is empty, so
  `verification_status()` returns `SigningStatus::NotConfigured` and
  `dry_run_report()` prints "signature verification PENDING (release signing
  keys not configured — nothing would actually be installed in this build)".
  The report never says "verified". The signature blob from the manifest is
  carried through the decision so the trust step is wired for when keys land.
- Config made concrete: `UPDATE_ENDPOINT_TEMPLATE` +
  `endpoint_for(channel)` resolve the per-channel manifest URL (host is a
  placeholder until Phase 8 release infra exists). Did NOT add
  `tauri-plugin-updater` or a `plugins.updater` block to tauri.conf.json —
  that consumes the same endpoint + real pubkey and is Phase 8; adding it now
  with placeholder keys would either break the Tauri schema or fake a trust
  boundary.
- App entry point: `dry_run_from_json(current, manifest_json, channel)` in
  core bundles decision + report + signing into a `DryRunResult`; Tauri
  command `check_for_update(manifest_json, channel)` (registered in
  `apps/desktop/src-tauri/src/{commands,lib}.rs`) exposes it to the UI. The
  frontend fetches the manifest and calls this; the command installs nothing.
- Fixture: `fixtures/updater/latest.json` — a real 2-release stable manifest
  (0.1.0, 0.2.0) across linux/darwin/windows.

Evidence: T7.3's verify ("updater test") met by 14 unit tests in updater.rs
plus 3 integration tests in `packages/core/tests/updater_dry_run.rs` that
drive the committed fixture manifest end-to-end (0.1.0 → 0.2.0 offered,
0.2.0 reports up-to-date, every published platform resolves, offline/no-write).
The "update channel dry-run works" self-verify: the fixture dry-run finds the
newer release and the report flags verification as pending signing keys.
`cargo test --workspace` — 184 passed, 0 failed; `cargo fmt --all --check`
clean. Added `semver = "1"` to core deps. Ticked T7.3.
Next: T7.4 (doc set: README, user guide, adapter-authoring guide, CHANGELOG;
verify: docs build) — the last task before Phase 8 (installers → GATE 4, the
final human gate).
