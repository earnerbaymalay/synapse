<div align="center">

# 📚 Synapse // LLM-NeuroSurgeon Documentation Hub

[🌐 Live Landing Page](index.html) • [⬅️ Return to Repository Root](../README.md)

---
</div>

## 🗺️ Documentation Map

### 🚀 1. Onboarding & Guides
* **[Quickstart Guide](QUICKSTART.md)** — Install dependencies, configure OS permissions, and perform your first sync in under 60 seconds.
* **[User Guide](USER_GUIDE.md)** — Day-to-day commands, managing Model Context Protocol (MCP) servers, and using the Synapse Doctor.

### 🏛️ 2. Architecture & Design
* **[Architecture Overview](ARCHITECTURE.md)** — Monorepo layout, file system watchers, AST dialect parsers, and 3-way merge resolution.
* **[Architecture Decisions (ADRs)](DECISIONS.md)** — Record of architectural RFCs and choices.

### 🔌 3. Verified Tool Adapters (13 Ecosystems)
* **[Adapters Overview](adapters/README.md)** — Full schema translation table across all 13 AI coding tools.
* Individual Adapters:
  * [Claude Code / Desktop](adapters/claude.md)
  * [Cursor MDC](adapters/cursor.md)
  * [Gemini CLI](adapters/gemini.md)
  * [Windsurf](adapters/windsurf.md)
  * [Cline & Roo Code](adapters/cline.md)
  * [GitHub Copilot & Zed](adapters/copilot-zed.md)

### 🛠️ 4. Contributing & Testing
* **[Contributing Guide](development/CONTRIBUTING.md)** — Pull request lifecycle, Rust conventions, and UI testing.
* **[Test Infrastructure](development/TEST_INFRA.md)** — Workspace integration tests and mock dialect fixtures.

---

## ⌨️ Command Reference

```bash
synapse scan              # Scan filesystem for active AI coding tools
synapse import --dry-run  # Preview config ingestion into ~/AIBrain
synapse import            # Ingest native configs into ~/AIBrain
synapse sync --daemon     # Launch real-time background sync daemon
synapse doctor            # Run diagnostic self-healing health check
synapse doctor --fix      # Remediate config drift and broken symlinks
```

---
[⬅️ Back to Main Repository](../README.md)
