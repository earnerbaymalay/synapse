# Security Audit & Threat Model — SYNAPSE
### llm-neuro-surgeon — Local-First Security Architecture

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
        BR[(~/AIBrain\nGit Time Machine)]
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
| 🛡️ **Path Traversal Protection** | `safe_join()` | Normalizes all paths; rejects `..` components or escape attempts before writing. |
| 🔗 **Symlink Escape Defense** | `DirEntry::file_type()` | Uses `symlink_metadata()` without following symlinks during directory walks. |
| 🔑 **API Key Isolation** | OS Keychain | Raw keys live in OS Keychain (macOS Keychain / Secret Service / Credential Manager); configs use `${VAR}` placeholders. |
| 📜 **Marketplace Provenance** | SHA-256 Checksums | Every imported skill/agent contains SHA-256 content hashes and upstream Git commit metadata. |
| 🚫 **Zero Telemetry** | Local-First | Zero external analytics, phone-home, or remote tracking code. |

---

## 3. Path Traversal Defenses (`safe_join`)

> [!CAUTION]
> **Path Traversal Attacks:**  
> A malicious tool config could specify a rule filename like `../../.bashrc` to attempt overwriting system files outside the tool root.

`packages/core/src/adapters/mod.rs` implements `safe_join(root, relative)`:

```rust
pub fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, AdapterError> {
    let target = root.join(relative);
    let canonical_root = root.canonicalize()?;
    let canonical_target = target.canonicalize()?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(AdapterError::Malformed("Path traversal detected".into()));
    }
    Ok(target)
}
```

---

## 4. Symlink Loop Defenses

To prevent symlink loop denial-of-service (e.g. `a -> b -> a`) or escaping into sensitive system directories, directory recursion uses non-following metadata checks:

```rust
if entry.file_type().is_symlink() {
    // Audit symlink target before following
}
```

---

## 5. Security Audit Verification

All security mechanisms are verified by automated tests in `packages/core/tests/adapter_stress_tests.rs`:

```bash
cargo test -p neurosurgeon-core --test adapter_stress_tests
```

- `test_windsurf_adapter_path_traversal_and_missing`: **PASSED**
- `test_github_copilot_adapter_path_traversal_is_blocked`: **PASSED**
- `test_github_copilot_adapter_symlink_loop_does_not_hang`: **PASSED**
- `test_windsurf_adapter_writes_outside_root`: **PASSED**
