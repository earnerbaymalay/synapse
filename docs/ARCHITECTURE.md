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

## 🔄 3-Way Merge & Sync Daemon Engine

```text
                  ┌───────────────────────┐
                  │ Base Hash (State.json)│
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
│ Clean Merge: Commit   │         │ Conflict: Queue Item  │
│ to Git Time Machine   │         │ for Doctor Resolution │
└───────────────────────┘         └───────────────────────┘
```

---

[⬅️ Back to Docs Hub](README.md)
