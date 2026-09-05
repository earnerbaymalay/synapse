# 🤝 Contributing to Synapse
### LLM-NeuroSurgeon

[Docs Hub](../README.md) > **Contributing**

Thank you for contributing to **Synapse (LLM-NeuroSurgeon)**!

---

## 🛠️ Development Setup

```bash
# 1. Clone repository
git clone https://github.com/earnerbaymalay/llm-neuro-surgeon.git
cd llm-neuro-surgeon

# 2. Run Rust unit and integration tests
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace

# 3. Run E2E Vitest test suites
pnpm install
pnpm --filter e2e test

# 4. Launch Desktop GUI in dev mode
pnpm --filter desktop-app tauri dev
```

---

## 🧪 Testing Guidelines

Before opening a pull request, ensure all test suites pass:

* `cargo test --workspace` (all Rust tests passing)
* `pnpm --filter @llm-neurosurgeon/e2e test` (all E2E tests passing)
* `cargo clippy --workspace --all-targets --all-features -- -D warnings` (0 warnings — enforced in CI's "Rust lints (clippy)" job)

---

[⬅️ Back to Docs Hub](../README.md)
