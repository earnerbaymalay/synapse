<div align="center">

# 📚 Synapse // LLM-NeuroSurgeon Documentation Hub

[🌐 Live Landing Page](index.html) • [⬅️ Return to Repository Root](../README.md)

---
</div>

## 🗺️ Documentation Map

### 🚀 1. Onboarding & Guides
* **[Quickstart Guide](QUICKSTART.md)** — Install dependencies, configure OS permissions, and perform your first sync in under 60 seconds.
* **[User Guide](USER_GUIDE.md)** — Day-to-day commands, managing Model Context Protocol (MCP) servers, and using the Synapse Doctor.
* **[Onboarding Journey](ONBOARDING.md)** — Four-phase walkthrough from fragmented configs to permanent lockstep.

### 🏛️ 2. Architecture & Design
* **[Architecture Overview](ARCHITECTURE.md)** — Monorepo layout, file system watchers, AST dialect parsers, and 3-way merge resolution.
* **[Architecture Decisions (ADRs)](DECISIONS.md)** — Record of architectural RFCs and choices.
* **[Design Pack](DESIGN_PACK.md)** — Design tokens, component specs, and ASCII wireframes.
* **[Security Architecture](security.md)** — Threat model, path traversal defense (`safe_join`), symlink loop guards, and OS Keychain integration.

### 🔌 3. Verified Tool Adapters (13 Ecosystems)
* **[Adapters Overview](adapters/README.md)** — Full schema translation table across all 13 AI coding tools.
* **[Adapter Authoring Guide](ADAPTER_AUTHORING_GUIDE.md)** — Step-by-step instructions for implementing custom Rust adapters.
* Individual Adapters:
  * [Claude Code / Desktop](adapters/claude.md)
  * [Cursor](adapters/cursor.md)
  * [Gemini CLI](adapters/gemini.md)
  * [Windsurf](adapters/windsurf.md)
  * [Cline](adapters/cline.md)
  * [Roo Code](adapters/roo-code.md)
  * [GitHub Copilot](adapters/github-copilot.md)
  * [Zed](adapters/zed.md)
  * [Antigravity CLI](adapters/antigravity.md)
  * [OpenAI Codex CLI](adapters/openai-codex.md)
  * [Aider](adapters/aider.md)
  * [Continue](adapters/continue.md)
  * [OpenCode](adapters/opencode.md)

### 🛠️ 4. Contributing, Packaging & Testing
* **[Contributing Guide](development/CONTRIBUTING.md)** — Pull request lifecycle, Rust conventions, and testing guidelines.
* **[Test Infrastructure](development/TEST_INFRA.md)** — 4-tier Vitest E2E test suites and mock dialect fixtures.
* **[Release Packaging](packaging/RELEASE_PACKAGING.md)** — Tauri v2 desktop bundles, `.deb`, `.dmg`, `.msi`, and shell installers.
* **[Reproducible Builds](REPRODUCIBLE_BUILDS.md)** — Deterministic compiler configuration, dependency locking, and SHA-256 verification.
* **[Code Review](CODE_REVIEW.md)** — Historical codebase audit notes and resolved items.

---

## ⌨️ Command Reference

```bash
synapse scan              # Scan filesystem for active AI coding tools
synapse import --dry-run  # Preview config ingestion into ~/AIBrain
synapse import            # Ingest native configs into ~/AIBrain
synapse project           # Project canonical Brain configs out to all tools
synapse sync --daemon     # Launch real-time background sync daemon
synapse doctor            # Run diagnostic self-healing health check
synapse doctor --fix      # Remediate config drift and broken symlinks
synapse snapshot "msg"    # Commit a named snapshot in the Brain Time Machine
synapse rollback <hash>   # Revert Brain working tree to a prior snapshot
```

---
[⬅️ Back to Main Repository](../README.md)

