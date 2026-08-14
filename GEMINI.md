# AGENTS.md — LLM Neuro Surgeon workspace

## Session initialization (mandatory)
Before starting any work in this repository:
1. Read `~/Documents/My-Vault/Projects/LLM Neuro Surgeon/Repository To-Do.md` for the current verified state and backlog.
2. Check `~/Documents/My-Vault/Worklog/` for the latest session worklog for your agent (e.g. `Worklog/AGY Sessions/` for agy, `Worklog/Gemini Sessions/` for gemini) to gain context on previous work.
3. Load/apply the `obsidian-antigravity` skill: take notes in the vault throughout the session as work completes.

## Session close-out (mandatory)
At the end of the session (or after each major milestone):
1. Write or update your session worklog note under `~/Documents/My-Vault/Worklog/<Agent> Sessions/` (one note per session; factual bullets, verification evidence, no secrets).
2. Reconcile `~/Documents/My-Vault/Projects/LLM Neuro Surgeon/Repository To-Do.md` with the actual git/GitHub state.

## Project quick facts
- Repo: earnerbaymalay/llm-neuro-surgeon, local clone at `~/workspace/llm-neuro-surgeon`, default branch `main`.
- Stack: Rust workspace (packages/core, apps/cli, apps/desktop/src-tauri) + pnpm workspace (packages/schema, packages/e2e, desktop frontend).
- Verify before any PR: `cargo test --workspace`, `pnpm --filter @llm-neurosurgeon/e2e test`, `cargo clippy`, desktop tests.
- PRs are squash-merged into `main`; keep local `main` synchronized after merges.
- Human gates: Gate 4 (real-machine install) is reserved for the human; log questions to QUESTIONS.md instead of blocking.
