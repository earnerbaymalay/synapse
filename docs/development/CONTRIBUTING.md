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
pnpm --filter desktop tauri dev
```

---

## 🧪 Testing Guidelines

Before opening a pull request, ensure all test suites pass:

* `cargo test --workspace` (179/179 Rust tests)
* `pnpm --filter e2e test` (142/142 E2E tests)
* `cargo clippy --workspace --all-targets` (0 warnings)

---

[⬅️ Back to Docs Hub](../README.md)
