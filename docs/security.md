# Security — T3.3 Red-Team Pass

**Date:** 2026-07-12
**Scope:** `packages/core/src/adapters/*.rs` — all 12 registered adapters'
`import()`/`project()` implementations, per PLAN.md T3.3: "Red-team pass:
symlink escape, path traversal, circular links, malformed configs."
**Method:** manual adversarial read of every adapter's filesystem-touching
code (not a fuzzer run), checked against MASTER_PROMPT.md §1 pillar 8's
"Safety by design" contract — specifically "import never follows symlinks
outside scanned roots." Findings were fixed in the same pass, each backed
by a new regression test that fails on the pre-fix code.

## Findings

### 1. Path traversal in `github-copilot`'s scoped-skill projection — CONFIRMED, FIXED
**Where:** `github_copilot.rs::project()`, scoped-skill write path.
**What:** `relative_target` was built from a `Skill`'s `triggers` (data
that arrived via `import()` from another tool's config — e.g. a
`cursor`/`continue` `.mdc` file's `globs:` frontmatter) and joined onto
`root` with no validation. A trigger of `"../traversal_output/**/*"`
produced a `relative_target` containing a `..` component, and
`root.join(relative_target)` walked out of the target directory. A repo
importing a crafted rules file, later projected to `github-copilot`, could
write a file anywhere the process has write access — outside the tool
root entirely.
**Evidence pre-fix:** `packages/core/tests/adapter_stress_tests.rs` already
contained `test_github_copilot_adapter_path_traversal`, which *asserted the
vulnerability existed* (its own comment: "If it wrote the file outside the
root, this is a path traversal vulnerability!"). That test passed before
this fix — i.e. the vulnerability was already proven, just not treated as
a bug.
**Fix:** added `adapters::safe_join(root, relative) -> Result<PathBuf,
AdapterError>` (`packages/core/src/adapters/mod.rs`), which walks
`relative`'s path components and rejects `ParentDir`/`RootDir`/`Prefix`
components with `AdapterError::Malformed`. Applied at the scoped-skill
write site.
**Test:** `test_github_copilot_adapter_path_traversal_is_blocked` (renamed
from the vulnerability-proving test; now asserts `project()` returns
`Err(Malformed)` and that no file lands outside the root).

### 2. The same path-traversal shape in `cursor`, `continue`, `claude-code` — CONFIRMED, FIXED
**Where:**
- `cursor.rs::project()` — rule slug (from `Skill.id`, stripped of
  `RULE_ID_PREFIX`) interpolated into `.cursor/rules/{slug}.mdc`.
- `continue_adapter.rs::project()` — same pattern,
  `.continue/rules/{slug}.md`.
- `claude_code.rs::project()` — skill slug into
  `.claude/skills/{slug}/SKILL.md` (both the directory `.join(slug)` and
  the `rel_path` string), and `Agent.slug` directly into
  `.claude/agents/{slug}.md`.
**What:** `Skill.id`/`Agent.slug` are populated during `import()` from
another tool's config content (a file stem, a JSON `"slug"` field, a
frontmatter value) — the same class of attacker-influenced data as
finding #1, just one adapter earlier in the import→project chain. None of
these four write sites validated the slug before building a path.
**Fix:** all four now build the target path via `safe_join(root,
&rel_path)` instead of `root.join(&rel_path)`.
**Tests:** `test_cursor_project_rejects_path_traversal_in_rule_slug`,
`test_continue_project_rejects_path_traversal_in_rule_slug`,
`test_claude_code_project_rejects_path_traversal_in_skill_slug`,
`test_claude_code_project_rejects_path_traversal_in_agent_slug`.

### 3. Symlink loop hangs `github-copilot`'s scoped-instruction scan — CONFIRMED, FIXED
**Where:** `github_copilot.rs::find_instruction_files()`.
**What:** the BFS directory walk used `path.is_dir()`, which *follows*
symlinks. A directory symlinked back to an ancestor (or to itself) is
re-queued forever — an unbounded loop, i.e. a denial-of-service on
`import()` triggered by nothing more than a symlink present in a scanned
project (e.g. a cloned repo).
**Evidence pre-fix:** `packages/core/tests/adapter_stress_tests.rs` already
had `test_github_copilot_adapter_symlink_loop`, marked `#[ignore] // Run
manually because it hangs due to infinite loop` — the hang was known and
worked around by not running the test, rather than fixed.
**Fix:** switched to `DirEntry::file_type()`, which reports the entry
itself without following it; symlinked entries are skipped outright before
the dir/file branch.
**Test:** un-ignored as
`test_github_copilot_adapter_symlink_loop_does_not_hang`, which now
asserts `import()` returns `Ok` (i.e. completes) instead of hanging.

### 4. Symlink-escape reads in `cursor`, `continue`, `claude-code` import scans — CONFIRMED, FIXED
**Where:** `cursor.rs::import()` (`.cursor/rules/*.mdc`),
`continue_adapter.rs::import()` (`.continue/rules/*.md`),
`claude_code.rs::import()` (`.claude/skills/*/SKILL.md` and
`.claude/agents/*.md`).
**What:** each of these lists a directory and reads every matching entry
with `fs::read_to_string`, filtering only by extension (or, for the skills
directory, `path.is_dir()`) — never checking whether the entry is itself a
symlink. A project containing e.g.
`.cursor/rules/planted.mdc -> ~/.ssh/id_rsa` (trivially plantable by
cloning a malicious repo into a directory you later scan) would have that
file's content read straight into the canonical Brain as a "skill" —
which the projection engine would then happily fan out to every other
connected tool. This directly violates MASTER_PROMPT.md §1 pillar 8:
"import never follows symlinks outside scanned roots."
**Fix:** each directory listing's filter now additionally requires
`DirEntry::file_type()` to report a real file (or, for the `.claude/skills`
directory, a real directory) — `file_type()` does not follow symlinks, so
a symlinked entry is excluded regardless of what it points at.
**Tests:**
`test_cursor_import_does_not_follow_symlinked_rule_files`,
`test_continue_import_does_not_follow_symlinked_rule_files`,
`test_claude_code_import_does_not_follow_symlinked_skill_dir`,
`test_claude_code_import_does_not_follow_symlinked_agent_files` — each
plants a symlink pointing at a file outside the scanned root and asserts
`import()` does not surface its content.

### 5. `windsurf` writes outside the scanned project root — REVIEWED, NOT A BUG
**Where:** `windsurf.rs::project()`, MCP server write path;
`get_windsurf_mcp_path()` in `adapters/mod.rs`.
**What:** `test_windsurf_adapter_writes_outside_root` confirms the adapter
writes Windsurf's MCP config to `$HOME/.codeium/windsurf/mcp.json`, outside
the project root passed to `project()`.
**Verdict:** this is not the same bug class as findings #1–#2. It is not
attacker-influenced (the path is a fixed, hardcoded literal — not built
from imported skill/trigger/slug data) and not a traversal from `root` (it
never derives from `root` at all). It reflects a real constraint,
confirmed by this project's own recon brief
(`docs/research/windsurf.md`): Windsurf's MCP settings genuinely live in
the user's home directory, not the project. No change made; the existing
test stands as documentation of intended behavior, not a vulnerability.

### Malformed configs — already covered, no new findings
Every adapter already carries stress tests for malformed JSON/JSONC/TOML
input (`adapter_stress_tests.rs` plus each adapter's own `#[cfg(test)]`
module — e.g. `test_cline_adapter_malformed_json`,
`test_openai_codex_stress_malformed_toml`,
`test_gemini_cli_stress_project_non_object_settings`). Reviewed all 12
adapters' parsing paths in this pass; found no new malformed-input
crash/panic surface beyond what those existing tests already exercise. No
adapter uses `.unwrap()`/`panic!` on untrusted input — parse failures
consistently return `AdapterError::Malformed`.

## Coverage note

All 12 adapters' `project()` methods were read to classify their output
artifacts for T3.2's policy table (see PROGRESS.md's T3.2 entry) — that
same pass is what surfaced findings #1–#2 here. Of the 12, only
`github-copilot`, `cursor`, `continue`, and `claude-code` build a
filesystem path from imported data (skill id / agent slug / trigger); the
other 8 (`aider`, `cline`, `gemini-cli`, `opencode`, `openai-codex`,
`roo-code`, `windsurf`, `zed`) write only to fixed, literal filenames and
so have no path-traversal surface. Only `github-copilot`, `cursor`,
`continue`, and `claude-code` walk a directory during `import()`; the
other 8 read single named files and so have no symlink-escape-via-directory-listing
surface. This was verified by grepping every adapter's `project()`/
`import()` for path-construction and `fs::read_dir` call sites, not
assumed.

## Test evidence

`cargo test -p neurosurgeon-core -p neurosurgeon` — 106 tests green
(84 lib + 17 stress + 5 CLI), 0 ignored (down from 1 ignored pre-fix —
the symlink-loop test now runs and passes instead of being skipped).
`cargo fmt --check -p neurosurgeon-core` clean.

## Out of scope for this pass

- `apps/desktop`/Tauri IPC surface — no filesystem-adjacent adapter code
  lives there yet; revisit once T2.2's Tauri compile blockers are cleared.
- The sync daemon / conflict queue (T4.1/T4.2) — does not exist yet.
- Secrets handling (keychain, placeholders) — scoped to Phase 6 (T6.3),
  not yet implemented.
- Fuzzing / property-based testing beyond the hand-crafted adversarial
  inputs above.

---

# §T7.1 — Threat-Model Pass & Red-Team Sign-Off (Phase 7)

**Date:** 2026-07-12
**Scope:** the attack surface added since the T3.3 pass above — network
fetches (marketplace, MCP registry), local **process spawning**
(health-check handshake), and **secrets handling** — plus a re-confirmation
that T3.3's adapter fixes still hold. Per PLAN.md T7.1 ("Threat-model pass +
red-team sign-off") and MASTER_PROMPT.md §4 Phase 7 ("Threat-model pass (fs
writes, downloaded content, secrets) with red-team sign-off").
**Method:** hands-on adversarial read of every new network/spawn/secret code
path in `packages/core`, each claim treated as disprovable (checked the
actual code, not the doc-comments). New surface, one finding by finding:

## 1. Process spawning in health checks — HIGHEST RISK, mitigated + documented
**Where:** `mcp_registry.rs::health_check_stdio()`.
**What:** it runs `command_or_url` as a real child process. For a registry
entry, `map_record()` builds that string as `npx -y <identifier>` from
registry-supplied data — so health-checking a registry entry downloads and
executes an arbitrary npm package. This is inherent to MCP (these servers
are *meant* to run locally), not a bug, but it is the single most dangerous
operation in the codebase.
**Mitigations verified:**
- **No shell.** It uses `Command::new(program).args(args)` with a
  `split_whitespace()` tokenization — never `sh -c`. Confirmed by reading
  the spawn site: a crafted identifier like `evil; rm -rf ~` cannot inject
  a shell command; the worst it does is pass extra *arguments* to the named
  program (`npx` in the registry case). Blast radius is bounded to npx's own
  arg surface, not arbitrary shell.
- **Bounded, never hangs.** Reader-thread + `recv_timeout` + guaranteed
  `child.kill()`/`wait()` in all paths (same never-hang pattern as
  `watcher.rs`). A server that spawns and then hangs cannot wedge the caller.
- **stderr suppressed, stdin/stdout piped** — the child cannot scribble on
  the parent's terminal.
**Residual gap (documented, not code-enforced):** nothing at the type level
*forces* a caller to only health-check a user-enabled server. The required
invariant — MASTER_PROMPT.md pillar 6's "user-enable toggle default OFF",
health-check only after explicit enable — is now spelled out in a `# Security`
doc-comment on `health_check_stdio` itself. When the GUI install pipeline
lands (Phase 6's fetch→card→diff→lint→enable flow is only partly built), the
enable gate must sit in front of this call. **This is the one thing a
reviewer of the eventual install-pipeline code must check.**

## 2. Registry field injection into the spawn command — CONFIRMED bounded
**Where:** `mcp_registry.rs::map_record()`.
**What:** `command_or_url` is `format!("{} -y {}", runner, pkg.identifier)`
where both `runner` (`runtimeHint`) and `identifier` come from the registry.
**Verdict:** bounded, not fixed, because it isn't exploitable beyond #1's
already-accepted "npx runs a package" risk. Since the consumer
(`health_check_stdio`) tokenizes on whitespace and never invokes a shell,
the only thing a malicious identifier controls is additional argv entries
to the runner — it cannot escape into a second command. An identifier is
also not attacker-*chosen* per victim: it's whatever the public registry
serves. Documented as bounded; no code change.

## 3. Marketplace fetch of untrusted content — CONFIRMED safe
**Where:** `marketplace.rs`.
**What:** fetches `SKILL.md` + file listings over HTTPS from
`anthropics/skills`.
**Mitigations verified:**
- **Never executes anything.** The module only reads text and computes a
  sha256. No spawn, no filesystem write.
- **`enabled: false` always.** Every `MarketplaceSkill` is constructed
  disabled; a unit test (`fetched_skill_always_starts_disabled_regardless_of_content`)
  pins the invariant even for skills flagged as containing executable
  content. Nothing downstream may treat a fetched skill as trusted.
- **Provenance + checksum recorded** (`source_url`, `sha256`) per
  MASTER_PROMPT.md's "show source + checksum".
- **base64/UTF-8 decode is fallible-guarded** — malformed API responses
  return `Parse` errors, not panics.
**Residual note:** `fetch_anthropic_skill(slug)` interpolates `slug` into a
GitHub API URL. A slug with `../` would manipulate the *URL path* (a request
to GitHub's API), not a local filesystem path — worst case an unexpected
404/JSON, no local effect. In practice slugs come from
`list_anthropic_skills()` (GitHub-served directory names). Low severity; left
as-is with this note rather than adding URL-encoding that would break on the
real slugs.

## 4. MCP registry fetch — CONFIRMED safe
**Where:** `mcp_registry.rs::search_official_registry()`.
**What:** HTTPS GET + JSON parse. No execution, no writes. Env var *names*
are captured; **values are never fetched or stored** (verified: `EnvVarRecord`
deserializes only `name`). Malformed JSON → `Parse` error, not panic.

## 5. Secrets handling — CONFIRMED sound
**Where:** `secrets.rs`.
**Mitigations verified:**
- **Values never land in a projected config.** `harvest_env()` returns a map
  of `${VAR}` placeholders only; a test asserts no real value string survives
  in the redacted output.
- **Placeholders/empties are never stored** as if they were secrets
  (`harvest_never_stores_placeholders_or_empties`).
- **Resolve fails loudly.** A placeholder with no stored secret returns
  `NotFound` rather than leaking a literal `${...}` into a spawned child's
  environment.
- **Real keychain, not a hand-rolled store.** `OsKeychainStore` (behind the
  `os-keychain` feature) delegates to the `keyring` crate — OS Secret
  Service / Security.framework / Credential Manager. Round-trip verified
  against this machine's live gnome-keyring, probe entry deleted in-test.
- **Transport:** `ureq` with rustls (no system OpenSSL); all fetches are
  HTTPS. No secret is ever placed in a URL query string.

## 6. Re-confirmation of the T3.3 adapter fixes — still holding
All T3.3 findings (path traversal via `safe_join`, symlink-escape via
`DirEntry::file_type()`) remain in place and green; the new modules add no
new filesystem-write path that bypasses them (marketplace/registry write
nothing; secrets write only to the keychain backend).

## Sign-off

**Signed off with one tracked condition.** The network, registry, and secret
paths are safe as written — no code execution from fetched content, secrets
never leak into configs, no shell injection. The **one** thing that is not
code-enforced is the enable-gate in front of `health_check_stdio` (§1): it is
a raw spawn primitive, correctly documented as such, but the UI/pipeline that
calls it must enforce "user-enabled only." That gate belongs to the
not-yet-complete Phase 6 install pipeline and **must be verified when that
code lands** — it is the open item this sign-off is conditional on.

## Test evidence (T7.1)

`cargo test --workspace` — 155 tests green, 0 ignored (incl. real
network tests importing live skills + registry servers, and a real
gnome-keyring round trip under `--features os-keychain`). `cargo fmt --all
-- --check` clean. No build warnings.
