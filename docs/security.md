# Security Audit & Threat Model — SYNAPSE
### Synapse — Local-First Security Architecture

This document details the security posture, threat model, and defense mechanisms implemented across SYNAPSE (`packages/core`).

---

## 1. Threat Model Overview

SYNAPSE operates local-first on developer workstations. It ingests configurations from local tools and Git repositories, storing them in `~/AIBrain`.

```mermaid
flowchart TD
    subgraph Untrusted Inputs
        M[Upstream Marketplace Repos]
        T[Malicious Tool Configs / Path Traversal]
    end

    subgraph Security Guards [packages/core]
        SJ[safe_join Path Traversal Defender]
        SL[Symlink Loop & Escape Guard]
        KC[OS Keychain API Key Isolation]
        FM[Frontmatter Schema Validator]
    end

    subgraph Safe Execution
        BR[(Brain\nGit Time Machine)]
        FS[Projected Tool Configs]
    end

    M -->|Marketplace Importer| FM
    T -->|Adapter Import| SJ
    FM --> BR
    SJ --> SL
    SL --> FS
    KC -.->|${VAR} Placeholders| FS
```

---

## 2. Core Security Controls

| Control | Mechanism | Implementation |
|---|---|---|
| 🛡️ **Path Traversal Protection** | `safe_join()` | Rejects `..`, root, and prefix path components, NUL bytes, and symlinks along the path, before writing. |
| 🔗 **Symlink Escape Defense** | `DirEntry::file_type()` | Uses `symlink_metadata()` without following symlinks during directory walks. |
| 🔑 **API Key Isolation** | OS Keychain (opt-in) | With a build that enables `--features synapse-core/os-keychain`, raw keys live in the OS Keychain (macOS Keychain / Secret Service / Credential Manager); configs use `${VAR}` placeholders. |
| 📜 **Marketplace Provenance** | SHA-256 Checksums | Every imported skill/agent contains SHA-256 content hashes and upstream Git commit metadata. |
| 🚫 **Zero Telemetry** | Local-First | Zero external analytics, phone-home, or remote tracking code. |

---

## 3. Path Traversal Defenses (`safe_join`)

> [!CAUTION]
> **Path Traversal Attacks:**  
> A malicious tool config could specify a rule filename like `../../.bashrc` to attempt overwriting system files outside the tool root.

`packages/core/src/adapters/mod.rs` implements `safe_join(root, relative)`. The check is lexical first, then symlink-aware. Abbreviated from the source:

```rust
pub fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, AdapterError> {
    if has_path_traversal(relative) { /* AdapterError::Malformed */ }
    for component in Path::new(relative).components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => { /* AdapterError::Malformed */ }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    crate::security::ensure_no_symlink_escape(root, candidate)
}
```

`has_path_traversal` also rejects NUL bytes. `ensure_no_symlink_escape` (in `packages/core/src/security.rs`) refuses a symlinked root and any symlinked component under it. It does not canonicalize the target, so it works for files that do not exist yet.

---

## 4. Symlink Loop Defenses

Adapters that walk directories do not follow symlinked entries, which prevents loops such as `a -> b -> a` and escapes into other directories. The Antigravity adapter's import, for example, skips symlinked skill directories and `SKILL.md` files.

---

## 5. Security Audit Verification

All security mechanisms are verified by automated tests in `packages/core/tests/adapter_stress_tests.rs`:

```bash
cargo test -p synapse-core --test adapter_stress_tests
```

The suite includes these cases:

- `test_windsurf_adapter_path_traversal_and_missing`
- `test_github_copilot_adapter_path_traversal_is_blocked`
- `test_github_copilot_adapter_symlink_loop_does_not_hang`
- `test_windsurf_adapter_writes_outside_root`

Pass or fail status depends on the build, so run the suite to check it.
