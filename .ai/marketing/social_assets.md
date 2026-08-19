# SYNAPSE — Launch Copy

Ready-to-post extracts. Fill in `[link]` before posting; nothing else needs editing.

---

## X / Twitter — launch thread

**1/**
One Brain. All Models.

SYNAPSE unifies the configuration of every AI coding tool on your machine — Claude, Cursor, Windsurf, Copilot, 9 others — into one canonical, git-backed Brain.

Edit once. Every model stays equally skilled.

[link]

**2/**
The problem: every AI tool speaks its own config dialect. `CLAUDE.md`, `.cursorrules`, `GEMINI.md`, `.windsurfrules`. You're maintaining the same rules N times and formats drift apart within a week.

**3/**
The loop is three commands:

```
synapse scan
synapse import --dry-run
synapse import
```

Dry-run shows you exactly what moves into the Brain before anything writes. Then `synapse project` pushes it back out to every tool.

**4/**
Every sync is a Git commit. Full history, diffable, revertable — `synapse rollback <hash>` and you're back to any prior state of your entire AI configuration.

**5/**
Zero telemetry. Local-first. API keys live in your OS Keychain, never in a config file on disk.

**6/**
13 verified adapters. MIT licensed. Built on Tauri 2 + Rust.

[link] — try it in the next five minutes.

---

## LinkedIn

**One Brain. All Models. — introducing SYNAPSE**

If you use more than one AI coding tool, you're maintaining the same knowledge in multiple places — different formats, different locations, different drift rates.

SYNAPSE solves this with a single, canonical, git-backed Brain that every tool reads from. Scan your machine, import into the Brain, project back out to every tool, and let the sync daemon keep it all in lockstep.

13 adapters. Four core commands. Zero telemetry. Every sync is a Git commit — a full Time Machine for your AI configuration.

Open source, MIT licensed, local-first from the ground up.

[link]

---

## Hacker News — Show HN

**Title:** Show HN: SYNAPSE – one git-backed Brain for every AI coding tool's config

**Post body:**

I got tired of maintaining the same rules and skills across Claude Code, Cursor, Windsurf and a handful of others — every tool wants its own file, its own format, and they drift apart the moment I update one and forget the rest.

SYNAPSE scans your machine for installed AI tools, imports their configs losslessly into one canonical directory (`~/AIBrain`, backed by Git), and projects that Brain back out to every tool — symlinks where the tool tolerates them, generated files where it doesn't. A debounced daemon keeps everything in sync with a 3-way merge, and every sync is a commit, so the whole thing has a real history you can diff and roll back.

13 adapters are implemented and tested (Claude, Cursor, Windsurf, Gemini CLI, Codex CLI, Zed, Cline, Continue, Aider, Copilot, OpenCode, Roo, Antigravity). Local-first, zero telemetry, MIT licensed. Built with Tauri 2 + Rust for the core engine, with a CLI and a desktop GUI sharing the same library.

Happy to answer questions about the adapter architecture, the 3-way merge, or the sandboxing around imported skills.

**First comment (maker, post immediately after):**

A few specifics people usually ask: dry-run is the default posture for anything destructive (`import --dry-run`, `project --dry-run`), the Doctor command (`synapse doctor --fix`) catches broken symlinks and config drift, and imported third-party skills/agents from the Marketplace carry SHA-256 provenance and keep their original license — nothing gets silently relicensed into the Brain.

---

## Reddit

**r/programming** — title: *"I got tired of copy-pasting AI tool configs, so I built a git-backed 'Brain' that syncs to all of them"* — body: same as HN post, lead with the pain point paragraph.

**r/rust** — title: *"SYNAPSE: a Tauri 2 + Rust tool for unifying AI coding tool configs (13 adapters, 179/179 tests passing)"* — body: lead with the architecture — `packages/core` in Rust (scanner, adapters, projector, 3-way merge, doctor), Clap-based CLI, Tauri 2 desktop shell. Link the adapter authoring guide for anyone who wants to add a 14th.

**r/LocalLLaMA** — title: *"One config Brain for Claude, Cursor, Gemini CLI, and everything else you run locally"* — body: emphasize local-first, zero telemetry, OS Keychain secrets — this audience cares about that first.

---

## Product Hunt

**Tagline:** One Brain. All Models. Zero friction.

**Description:** SYNAPSE unifies the configuration of every AI coding tool on your machine — 13 adapters and counting — into one canonical, git-backed Brain, then keeps every tool in sync automatically. Scan, import, project, done. Every sync is a Git commit, so your entire AI configuration has a real, revertable history. Local-first, zero telemetry, MIT licensed.

**Maker's first comment:** Built this because I was hand-syncing the same rules across four different AI tools and losing track of which one was stale. Happy to walk through the adapter architecture or the 3-way merge logic in the comments.

---

## dev.to — article outline

**Title:** "Your AI tools don't need another feature. They need one Brain."

1. The drift problem, told as a specific 2am story (a rule updated in Cursor, forgotten in Claude Code, causing a bad merge).
2. Why a canonical directory + Git beats a database or a cloud sync service for this (inspectable, diffable, no vendor lock-in).
3. Walk through `scan → import --dry-run → import → project` with real terminal output.
4. The adapter model — what makes an adapter "verified," and how the 3-way merge resolves conflicting local edits.
5. Close on the Doctor + Time Machine as the payoff for sticking with it past week one.

---

## Email / newsletter blurb

**Subject:** One Brain. All Models.

SYNAPSE just shipped: a local-first tool that unifies every AI coding tool's configuration into one canonical, git-backed Brain, and keeps all 13 supported tools in sync automatically. If you've ever updated a rule in one AI tool and forgotten to copy it to the other three, this is for you. MIT licensed, zero telemetry. [link]
