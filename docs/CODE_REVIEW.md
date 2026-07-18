> **2026-07-12 addendum:** this document is historical (as of Phase 2/T2.1,
> before `packages/core`/`apps/cli` existed). By the start of this session,
> §1.2–§1.6 and §2.1 were already resolved (build.rs, tauri.conf.json,
> `#[tauri::command]` attributes, single entry point, `.manage()`, and
> `config.rs` using an OS config dir via the `directories` crate rather than
> writing into the repo's `.claude/` — none of that was touched this
> session). What was **still genuinely blocking `cargo build` on
> `apps/desktop/src-tauri`** going into this session: Ubuntu 24.04/Linux
> Mint 22 don't ship `webkit2gtk-4.0`/`javascriptcoregtk-4.0` (only the 4.1
> generation), which Tauri **v1.5.0** (what this crate was pinned to)
> hardcodes lookups for — no `apt install` can fix that, since the 4.0
> packages simply aren't built for this OS version. Fixed by upgrading to
> **Tauri v2** (`tauri`/`tauri-build` → `2`, `tauri.conf.json` rewritten to
> the v2 schema, `lib.rs`'s menu/window code ported to
> `tauri::menu::{Menu, MenuItem, Submenu}` + `WebviewWindowBuilder`), which
> natively supports webkit2gtk-4.1 and also matches MASTER_PROMPT.md's own
> stated default stack (the v1.5.0 pin was itself a pre-existing deviation
> from spec). While migrating, also fixed several previously-unreached
> compile errors that only surfaced once the build got this far: a missing
> `icons/icon.png` (generated a placeholder — real branding is T5.1 scope),
> `log::set_boxed_logger` needing the crate's `std` feature explicitly
> enabled, a borrow-checker error in `commands::import_config`, and
> `AdapterCommand` needing to be `pub` for `generate_handler!` to accept it.
> Also fixed a **latent runtime bug** unrelated to compilation: v1's
> `tauri.conf.json` declared a default window AND `lib.rs`'s `setup()`
> imperatively built a second window with the same `"main"` label — would
> have panicked on first launch. Removed the config-declared window since
> the app clearly intends to build its window in code. Full workspace
> (`packages/core` + `apps/cli` + `apps/desktop/src-tauri`) now builds and
> tests clean together — see PROGRESS.md's dated entry for evidence. §2
> (beyond 2.1)–§5 below were not re-audited this session and may also be
> stale; treat them as of their original date, not current fact.

# Code Review — LLM Neurosurgeon (as of Phase 2 / T2.1)

**Scope:** the code and scaffolding that shipped with the initial upload —
`apps/desktop` (Tauri core), the project state files, and the brand mocks.
**Verdict:** the project is a promising, well-documented **early scaffold**.
The planning/design layer is strong; the executable code is **placeholder
that does not yet compile**, and the "monorepo" claimed by task T2.1 is only
partially present. None of this is a surprise for a project paused at the
start of Phase 2 — this document just makes the gap explicit and lists the
work needed to make Phase 2's own `verify:` (`cargo test && pnpm test` green)
achievable.

The one thing fixed as part of this review is the **`Cargo.toml`, which was
invalid TOML** and blocked every Rust tool (even `cargo fmt`). Everything else
is reported here rather than rewritten, to avoid fabricating a working app the
project has not actually reached.

---

## 1. Blockers — the desktop app cannot compile

### 1.1 `Cargo.toml` was not valid TOML *(fixed in this review)*
The manifest had two fatal parse errors:
- `[target."cfg(not(target_os = "linux"))"]` — unescaped inner quotes; TOML
  cannot parse the table header.
- `cfg-set = ["target.cpu=""native""]` — not a real Cargo profile key, and the
  string literal is malformed.

It also **declared no `chrono` or `log` dependency**, yet `state.rs`, `lib.rs`
and `logger.rs` import them. The manifest has been rewritten to valid TOML with
those dependencies added and the bogus keys removed, so `cargo fmt`/`cargo
metadata` now work. It still will **not build** until the items below are done.

### 1.2 No `build.rs` and no `tauri.conf.json`
`main.rs`/`lib.rs` call `tauri::generate_context!()`, which reads
`tauri.conf.json` at compile time and is wired up by a `build.rs` calling
`tauri_build::build()`. Both files are missing, so the macro fails. Tauri also
needs a `distDir` (a built frontend) to embed — there is none (see §3).

### 1.3 Command handlers are missing `#[tauri::command]`
`commands.rs` exposes `get_version`, `open_settings`, `run_adapter_command`,
`import_config`, `export_config` to `generate_handler!`, but none carries the
`#[tauri::command]` attribute. The macro will not accept them.

### 1.4 `tauri::Result` misuse in `commands.rs`
Line 3 imports `tauri::Result` (a one-type-parameter alias,
`Result<T, tauri::Error>`), then the handlers write `Result<(), String>` and
`Result<String, String>` (two type parameters). These do not match the alias
and will not type-check. Use `std::result::Result<T, String>` (or drop the
import and return `Result<T, String>` from the prelude).

### 1.5 Two entry points / duplicated state
- `lib.rs` **and** `main.rs` both define `pub fn main()` and both build the
  main window, with different Tauri idioms (`main.rs` uses a
  `.build(...).run(...)` split that is Tauri-2 shaped; `lib.rs` uses the
  Tauri-1 `.run(...)` form). Only one entry point should exist. The standard
  Tauri layout is: `lib.rs` exposes a `run()` that owns the builder; `main.rs`
  is a thin `fn main() { desktopapplib::run() }`.
- `AppState` is defined **twice** — once in `lib.rs` and once in `state.rs` —
  with the same fields. Keep the `state.rs` version (it returns a `Mutex`) and
  delete the copy in `lib.rs`.

### 1.6 State is never `manage()`d, but commands request it
`run_adapter_command`/`import_config`/`export_config` take
`state: State<AppState>`, but neither builder calls `.manage(...)`. At runtime
Tauri would panic ("state not managed"). `state::AppState::new()` also returns a
`Mutex<AppState>`, so the managed type and the `State<AppState>` the commands
ask for don't line up — commands should take `State<'_, Mutex<AppState>>`.

### 1.7 Tauri-1 API misuse in `main.rs`
- `Menu::new().add_item(MenuItem::new("File", true))` is not the Tauri-1 menu
  API (`MenuItem` variants are things like `MenuItem::Quit`; custom entries use
  `CustomMenuItem`/`Submenu`).
- `WindowUrl` is imported but never used; the window is built with no URL.
- `config::load_config()` in `setup` **writes** `.claude/settings.json` on
  first run as a side effect (see §2.1).

---

## 2. Correctness / design concerns (compiles-or-not aside)

### 2.1 `config.rs` writes into the repo's `.claude/` at runtime
`load_config()` defaults to `".claude/settings.json"`, and if absent it
**creates** it. That path is relative to the process CWD and collides with
Claude Code's own config directory. User config belongs in an OS config dir
(e.g. via the `directories`/`dirs` crate), not in the repo tree, and reads
should not silently write.

### 2.2 `AppConfig::adapter_paths` bakes in fragile literals
Defaults like `".claude/CLAUDE.md"` and `"~/.config/zed/AGENTS.md"` hard-code
paths and won't expand `~`. Per the project's own MASTER_PROMPT rule ("do not
trust memorized paths — verify from official docs"), these should come from the
per-tool adapters, not a constant.

### 2.3 `logger.rs` is dead code
`init_logger()` is never called, and `log` was not even a declared dependency
until this review. Either wire it into `setup` or use `tauri-plugin-log`.

### 2.4 Idiomatic nits
- `get_version` uses `return env!(...).to_string();` — drop `return` and the
  semicolon (Clippy `needless_return`).
- Several `state`/`window` parameters are unused (`open_settings`,
  `import_config`, `export_config`) and will warn.

---

## 3. Missing pieces vs. the plan

`PLAN.md` marks **T2.1 (monorepo layout) complete**, but the repo only contains
`apps/desktop`. Absent:
- `packages/core` (the Rust engine: scanner, canonical model, adapters,
  projector, sync) — the heart of the product.
- `packages/schema` (JSON Schema + generated TS types).
- `apps/cli` (the `clap` binary: `scan/import/project/sync/doctor/snapshot/
  rollback`). **Note:** this crate needs no system libraries, so it is the
  fastest path to a *real, green, cross-platform* `cargo test` in CI.
- `fixtures/` (golden config trees per tool).
- The **frontend**: `package.json` references Vite/React/TS/Tailwind, but there
  is no `vite.config.ts`, `tsconfig.json`, `index.html`, or `src/`. So
  `pnpm build` cannot run and there is no `distDir` for Tauri to embed.
- No `Cargo.lock`, no root workspace manifest, no `pnpm-workspace.yaml`.

`PROGRESS.md`/`PLAN.md` should be corrected to reflect that T2.1 is **partially
done** rather than complete.

---

## 4. What's good

- **Strong planning spine.** `MASTER_PROMPT.md`, `PLAN.md`, `PROGRESS.md`,
  `DECISIONS.md`, `DESIGN_PACK.md` and the RALPH loop are coherent, and every
  task carries a `verify:` — that discipline is the project's biggest asset.
- **12 recon briefs** in `docs/research/` give each adapter a real spec source.
- **Three complete brand mocks** render and are valid HTML (CI checks this).
- **Safety doctrine** (dry-run default, snapshot-before-destroy, checksum-after-
  copy, untrusted-import flagging) is well thought out and should be enforced in
  code as the engine lands.

---

## 5. Recommended next steps (in order)

1. Make one crate genuinely build + test: scaffold `apps/cli` (`clap`) and
   `packages/core` with a couple of unit tests. Zero system deps ⇒ a real green
   `cargo test` matrix on all 3 OSes (satisfies part of T2.1/T2.3).
2. Fix the desktop app compile blockers (§1.2–§1.7): add `build.rs` +
   `tauri.conf.json`, a minimal frontend (`index.html` + Vite), `#[tauri::
   command]` attributes, correct `Result` types, single entry point, and
   `.manage(Mutex::new(AppState))`.
3. Flip `.github/workflows/build.yml` from `workflow_dispatch` to
   `push`/`pull_request` once the app compiles, making it a required gate.
4. Reconcile `PLAN.md`/`PROGRESS.md` with reality (T2.1 partial).
5. Move runtime config out of the repo tree (§2.1) and wire logging (§2.3).
