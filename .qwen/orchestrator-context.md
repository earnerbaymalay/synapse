# Orchestrator Context — Synapse

## Current Status
Phase 0 completed. Gate 0 decisions made:
- Brand A: Cortex (HTML/HTML+CSS/JS), Brand B: Synapse (HTML/CSS/JS), Brand C: Cerebra (HTML/CSS/JS)
- Dark theme default
- Stack: Tauri 2 + clap CLI
- Brain: ~/AIBrain

## Tack Next: Phase 1 — Design Pack (GATE 1)
T1.1 is done: DESIGN_PACK.md compiled with tokens, components, voice, and full 8-screen ASCII wireframes.

RALPH is now active: on each turn it reads PLAN.md + PROGRESS.md + DESIGN_PACK.md (if UI task), selects highest-priority unchecked task, implements it, verifies, commits, and ticks off the checkbox.

## RALPH Priority Queue
- [x] T0.1 Initialize repo + state files + swarm roles
- [x] T0.2 Recon ×12 — verify config paths/formats
- [x] T0.3 Draft 3 brand identities + HTML dashboard mocks
- [x] T0.4 ASCII wireframes for all 8 screens
- [x] T0.5 Present GATE 0 questions
- [x] T1.1 Compile DESIGN_PACK.md
- [ ] T1.2 Write RALPH_PROMPT.md and task breakdown
- [ ] T1.3 Gate 1: present pack for approval
- [ ] T2.1 Monorepo layout: apps/desktop, packages/core, apps/cli
- [ ] T2.2 Empty Tauri v2+React app launches
- [ ] T2.3 CLI --help complete
- [ ] T3.1 Adapter-smith swarm: 12 adapters, detect/import/project + round-trip
- ... (continue to T8.3)

## Active Workflow
The agent is now in Phase 1 (Design Pack). T1.2 currently sits as the highest-priority unchecked task after T1.1.

Next action: implement T1.2 (RALPH_PROMPT.md + task breakdown).

## Notes for Future Work
- PHASE 0 complete; moving to Phase 1.
- RALPH ensures a single, verified task at a time.
- Final goal: human approval via Gates 4.

## Memory
This context is the source of truth for the ongoing Synapse project. Keep it in sync.