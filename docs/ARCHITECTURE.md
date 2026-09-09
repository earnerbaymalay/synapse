# 🏛️ Architecture & Engine Internals
### Synapse // LLM-NeuroSurgeon

[Docs Hub](README.md) > **Architecture**

This document describes the architectural layout, core Rust engine components, 3-way merge algorithm, and IPC IPC communications of **Synapse (LLM-NeuroSurgeon)**.

---

## 🏗️ Monorepo Component Layout

```text
llm-neuro-surgeon/
├── apps/
│   ├── cli/             # Rust CLI entry point (`synapse` / `neurosurgeon`)
│   └── desktop/         # Tauri 2 + React TS Dark Precision GUI
├── packages/
│   ├── core/            # Core engine: scanner, 13 adapters, projector, sync, doctor
│   ├── e2e/             # Vitest E2E integration test suites (Tiers 1-4)
│   └── schema/          # JSON Schema specifications for skills & agents
├── docs/                # Public documentation & GitHub Pages landing site
└── .ai/                 # Internal prompt scratchpads & development history
```

---

## 🔄 3-Way Merge & Sync Engine

`synapse sync` runs this merge in a single pass and exits — there is no
daemon or watch mode. `packages/core` ships `watcher.rs`/`scheduler.rs` for
a future continuous-sync mode, but no CLI command wires them up yet.

```text
                  ┌───────────────────────┐
                  │ Base: last git-        │
                  │ committed content      │
                  └──────────┬────────────┘
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
┌───────────────────────┐         ┌───────────────────────┐
│ Local Brain Edit      │         │ Foreign Tool Edit     │
│ (~/AIBrain)           │         │ (e.g. .cursorrules)   │
└───────────┬───────────┘         └───────────┬───────────┘
            │                                 │
            └────────────────┬────────────────┘
                             ▼
                ┌─────────────────────────┐
                │   3-Way Merge Engine    │
                └────────────┬────────────┘
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
┌───────────────────────┐         ┌───────────────────────┐
│ Clean Merge: Commit   │         │ Conflict: Queued,     │
│ to Git Time Machine   │         │ reported by CLI sync  │
└───────────────────────┘         └───────────────────────┘
```

The queue's only consumer today is `synapse sync`'s own CLI output
(`SyncOutcome::ConflictQueued`) — there is no GUI review screen and
`synapse doctor` does not read the conflict queue (see
`packages/core/src/conflict_queue.rs`'s module doc for the full gap).

---

[⬅️ Back to Docs Hub](README.md)
