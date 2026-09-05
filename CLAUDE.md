# LLM Neurosurgeon — Orchestrator Context

Self-executing spec: .ai/MASTER_PROMPT.md.

## Standing rules for this agent
1. File-state over chat-state: every decision and plan lives in the repo.
2. No model-swarm lies: this is a single chat. Simulate swarm roles by writing files and executing commands.
3. Default stack: Tauri 2 (Rust core + React-TS-Tailwind) + clap CLI unless human overrides at Gate 0.
4. Dry-run before write, snapshot before destroy, checksum after copy.
5. Gate-stop: only human at gates. Log non-blocking questions to .ai/QUESTIONS.md.
