# LLM Neurosurgeon — Reproducible Builds & Deterministic Artifact Specification

This document details the reproducibility guidelines, compiler configuration, dependency locking, binary stripping, verification workflows, and checksum generation processes for **LLM Neurosurgeon**.

---

## 1. Overview & Principles of Reproducibility

Reproducible builds ensure that compiling the exact same source code with the specified toolchain, configuration, and build environment produces **bit-for-bit identical binaries** across different build machines and execution environments.

### Core Principles
1. **Deterministic Toolchains**: Pinned compiler versions (Rust edition 2021, standard `rust-toolchain.toml`) and node runtime.
2. **Strict Workspace Dependency Locking**: Sealed dependency graphs using `Cargo.lock` and `pnpm-lock.yaml` enforced with strict CLI flags.
3. **Path Prefix Remapping**: Eliminating host-specific filesystem paths from compiled debug info and symbol tables.
4. **Environment Normalization**: Fixed build timestamps, locales, timezones, and environment variable states.
5. **Independent Dual-Build Verification**: Automated CI verification running parallel independent builds to compare SHA-256 digests.

---

## 2. Deterministic Toolchain Configuration

### 2.1 Rust Toolchain Specification (`rust-toolchain.toml`)
The compiler version, components, and target triples are strictly pinned in the repository root:

```toml
[toolchain]
channel = "1.84.0"
components = ["rustc", "cargo", "rustfmt", "clippy", "llvm-tools-preview"]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc"
]
profile = "minimal"
```

### 2.2 Path Prefix Remapping & Compiler Flags (`.cargo/config.toml`)
Host-specific directory structures (e.g. `/home/user/...` or `C:\Users\...`) must not be embedded into binary symbols. The workspace `.cargo/config.toml` enforces prefix remapping:

```toml
[build]
rustflags = [
    "-C", "remap-path-prefix=/home/earn/llm-neuro-surgeon=/neurosurgeon-src",
    "-C", "remap-path-prefix=/home/runner/work/llm-neuro-surgeon/llm-neuro-surgeon=/neurosurgeon-src",
    "-C", "remap-path-prefix=C:/a/llm-neuro-surgeon/llm-neuro-surgeon=/neurosurgeon-src",
    "-C", "symbol-mangling-version=v0"
]

[env]
SOURCE_DATE_EPOCH = "1773990000" # Fixed Epoch for deterministic timestamps (2026-03-20T07:00:00Z)
TZ = "UTC"
LC_ALL = "C"
```

---

## 3. Workspace Dependency Locking

To guarantee that no transitive dependency floats or changes unexpectedly during release builds:

### 3.1 Cargo Dependency Locking
- **Lockfile**: Root `Cargo.lock` is tracked under Git version control.
- **CI Build Command**: All release compilation MUST invoke `cargo` with the `--locked` flag:
  ```bash
  cargo build --workspace --release --locked
  ```

### 3.2 Frontend Dependency Locking
- **Lockfile**: Root `pnpm-lock.yaml` locked under pnpm 10.x.
- **CI Build Command**: All frontend builds MUST invoke `pnpm` with `--frozen-lockfile`:
  ```bash
  pnpm install --frozen-lockfile
  pnpm --filter desktop build
  ```

---

## 4. Binary Stripping & Release Optimization Profile

The root `Cargo.toml` specifies the deterministic release profile settings:

```toml
[profile.release]
codegen-units = 1     # Ensures single codegen unit per crate for deterministic optimization passes
lto = true             # Link-Time Optimization enabled across all workspace crates
opt-level = "s"        # Optimized for binary size and consistent instruction layout
strip = true           # Automatically strips debug symbols and symbol tables from release binaries
panic = "abort"        # Eliminates unwinding landing pads for uniform binary layout
```

### 4.1 Post-Build Symbol Verification
Release pipelines verify that debug symbols have been fully stripped:

```bash
# Verify no debug symbols remain in the compiled CLI binary
file target/release/neurosurgeon | grep "stripped"

# Alternatively verify using llvm-strip or nm
nm -a target/release/neurosurgeon 2>&1 | grep "no symbols"
```

---

## 5. Automated SHA-256 Checksum Generation

Every release pipeline generates a canonical, PGP/minisign-verifiable `SHA256SUMS` manifest covering all built binaries, installer packages, and source archives.

### 5.1 Canonical Manifest Format
`SHA256SUMS` uses the standard two-space GNU format:

```
a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0  neurosurgeon-x86_64-unknown-linux-gnu.tar.gz
b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef01  neurosurgeon-x86_64-apple-darwin.tar.gz
c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef012  neurosurgeon-aarch64-apple-darwin.tar.gz
d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0123  LLM_Neurosurgeon_0.7.4_amd64.deb
e5f67890123456789abcdef0123456789abcdef0123456789abcdef01234  LLM_Neurosurgeon_0.7.4_amd64.AppImage
f67890123456789abcdef0123456789abcdef0123456789abcdef012345  LLM_Neurosurgeon_0.7.4_x64.dmg
7890123456789abcdef0123456789abcdef0123456789abcdef01234567  LLM_Neurosurgeon_0.7.4_x64_en-US.msi
```

### 5.2 Verification Commands

```bash
# Verification on Linux / macOS (GNU coreutils)
sha256sum -c SHA256SUMS

# Verification on macOS (BSD / shasum)
shasum -a 256 -c SHA256SUMS

# Verification on Windows PowerShell
Get-FileHash -Algorithm SHA256 neurosurgeon.exe
```

---

## 6. CI Reproducible Build Verification Workflow

The GitHub Actions workflow below runs two completely isolated builds (`runner-a` and `runner-b`) on separate runner environments and asserts bit-for-bit equality of the resulting compiled artifacts:

```yaml
name: Reproducible Build Verification

on:
  push:
    branches: [ main ]
    tags: [ 'v*' ]
  pull_request:
    branches: [ main ]

jobs:
  build-runner-a:
    name: Build Runner A
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Rust Toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.84.0
      - name: Build CLI Release Binary (Runner A)
        run: |
          cargo build --workspace --release --locked
          sha256sum target/release/neurosurgeon > runner_a.sha256
      - name: Upload Runner A Hash
        uses: actions/upload-artifact@v4
        with:
          name: runner-a-hash
          path: runner_a.sha256

  build-runner-b:
    name: Build Runner B
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - name: Setup Rust Toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.84.0
      - name: Build CLI Release Binary (Runner B)
        run: |
          cargo build --workspace --release --locked
          sha256sum target/release/neurosurgeon > runner_b.sha256
      - name: Upload Runner B Hash
        uses: actions/upload-artifact@v4
        with:
          name: runner-b-hash
          path: runner_b.sha256

  compare-hashes:
    name: Verify Bit-for-Bit Determinism
    needs: [build-runner-a, build-runner-b]
    runs-on: ubuntu-latest
    steps:
      - name: Download Runner A Hash
        uses: actions/download-artifact@v4
        with:
          name: runner-a-hash
      - name: Download Runner B Hash
        uses: actions/download-artifact@v4
        with:
          name: runner-b-hash
      - name: Compare Binary Hashes
        run: |
          HASH_A=$(cat runner_a.sha256 | awk '{print $1}')
          HASH_B=$(cat runner_b.sha256 | awk '{print $1}')
          echo "Runner A Hash: $HASH_A"
          echo "Runner B Hash: $HASH_B"
          if [ "$HASH_A" != "$HASH_B" ]; then
            echo "ERROR: Build determinism check failed! Hashes do not match."
            exit 1
          fi
          echo "SUCCESS: Reproducible build check passed. Hashes are bit-for-bit identical."
```

---

## 7. Determinism Audit & Drift Resolution

If build artifacts drift between independent environments:
1. **Inspect embedded strings**: Use `strings target/release/neurosurgeon | grep -E "home|runner|work"` to identify leaked build paths.
2. **Compare ELF/Mach-O/PE Headers**: Use `diffoscope` (`diffoscope binary_a binary_b`) to locate non-deterministic header sections or timestamps.
3. **Audit Transitive Dependencies**: Ensure `Cargo.lock` has no platform-conditional crate features causing non-deterministic code generation across runners.
