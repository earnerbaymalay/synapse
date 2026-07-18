# DECISIONS.md

2026-07-07 — Project root set to `/home/vers/Desktop/llm-neurosurgeon` because previous session's `/home/claude/llm-neurosurgeon` was not present on this machine and Desktop is user-visible and persistent.
2026-07-07 — Default stack candidate: Tauri 2 (Rust core + React/TS/Tailwind) + clap CLI, pending Gate 0 confirmation.
2026-07-07 — Default Brain candidate: `~/AIBrain`, pending Gate 0 confirmation.
2026-07-12 — Gate 2 (T3.4) APPROVED: human ran `cli scan && cli import --dry-run` against `/home/vers/Desktop/llm-neurosurgeon`, reviewed the printed migration report (1 tool detected, 7 skills/6 agents/0 mcp servers, every claim cross-checked against the actual filesystem), and approved it. Phase 4 is unblocked.
2026-07-12 — Brand primary color left as the Cortex (#667eea) placeholder pending a full brand-identity decision; human explicitly chose to leave it open rather than confirm now (asked directly during T5.1 review). Not a Gate 0 resolution — QUESTIONS.md's brand/theme/stack/brain-location checkboxes remain unconfirmed.
2026-07-12 — Gate 3 (T5.3) APPROVED: human reviewed the 10-screen gallery (8 DESIGN_PACK.md screens + the onboarding wizard's 3-step dry-run flow, published as a Claude Artifact) and approved. Phase 6 is unblocked.
