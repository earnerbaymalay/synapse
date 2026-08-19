# SYNAPSE / llm-neuro-surgeon — Adapter Authoring Guide

This guide describes how to author new tool adapters for **SYNAPSE / llm-neuro-surgeon**. Adapters are responsible for detecting, importing, and projecting configuration files for specific AI coding tools (such as Claude Code, Cursor, Gemini CLI, Aider, Antigravity CLI, etc.).

---

## Table of Contents

- [1. Adapter Architecture Overview](#1-adapter-architecture-overview)
- [2. The `Adapter` Trait Interface](#2-the-adapter-trait-interface)
- [3. Canonical Model Structs](#3-canonical-model-structs)
- [4. Step-by-Step Walkthrough](#4-step-by-step-walkthrough)
- [5. Projection Policies & Policy Engine](#5-projection-policies--policy-engine)
- [6. Security & Red-Team Requirements](#6-security--red-team-requirements)
- [7. Helpful Adapter Utilities](#7-helpful-adapter-utilities)
- [8. Testing & Verification Checklist](#8-testing--verification-checklist)

---

## 1. Adapter Architecture Overview

All tool adapters reside in `packages/core/src/adapters/`. Each adapter is an isolated Rust module implementing the `Adapter` trait defined in `packages/core/src/adapter.rs`.

Adapters operate on two primary directions:
1. **Import (`Tool Config -> Canonical Model`)**: Ingesting tool-specific rule files, agent definitions, skills, and MCP servers into canonical Rust structs (`Skill`, `Agent`, `McpServer`).
2. **Project (`Canonical Model -> Tool Config`)**: Writing canonical rules, skills, agents, or MCP server configurations out to tool-specific paths (via symlinks or generated/merged files).

---

## 2. The `Adapter` Trait Interface

The core interface in `packages/core/src/adapter.rs` is defined as follows:

```rust
use std::path::Path;
use crate::model::{Agent, McpServer, Skill};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportResult {
    pub skills: Vec<Skill>,
    pub agents: Vec<Agent>,
    pub mcp_servers: Vec<McpServer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResult {
    pub written: Vec<String>,
    pub symlinked: Vec<String>,
}

pub trait Adapter {
    /// Unique stable identifier (e.g., "claude-code", "cursor", "agy-cli", "my-custom-tool").
    fn id(&self) -> &'static str;

    /// Returns true if this tool's config files exist under `root`.
    fn detect(&self, root: &Path) -> bool;

    /// Ingests the tool's config files into canonical data structures.
    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError>;

    /// Projects canonical data structures back into tool configuration files.
    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        agents: &[Agent],
        mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError>;
}
```

---

## 3. Canonical Model Structs

Found in `packages/core/src/model.rs`:

```rust
/// Canonical representation of a skill or instruction set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub version: String,
    pub triggers: Vec<String>,
    pub targets: Vec<String>,
    pub source: String,
    pub sha256: String,
}

/// Canonical representation of a custom AI agent persona.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    pub slug: String,
    pub tools: Vec<String>,
    pub model_hints: Vec<String>,
    pub targets: Vec<String>,
}

/// Canonical representation of an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub transport: String,
    pub command_or_url: String,
    pub env_placeholders: Vec<String>,
    pub targets: Vec<String>,
    pub health: HealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Unreachable,
}
```

---

## 4. Step-by-Step Walkthrough

Follow these steps to create a new adapter (e.g. for `my-tool`):

### Step 1: Create Module File

Create `packages/core/src/adapters/my_tool.rs`:

```rust
use std::fs;
use std::path::Path;
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::adapters::{compute_sha256, safe_join, strip_provenance, write_if_changed};
use crate::model::{Agent, McpServer, Skill};

pub struct MyToolAdapter;

impl Adapter for MyToolAdapter {
    fn id(&self) -> &'static str {
        "my-tool"
    }

    fn detect(&self, root: &Path) -> bool {
        // Detect presence of configuration file
        root.join(".mytoolrules").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let config_path = root.join(".mytoolrules");
        if !config_path.exists() {
            return Ok(ImportResult {
                skills: vec![],
                agents: vec![],
                mcp_servers: vec![],
            });
        }

        let raw_content = fs::read_to_string(&config_path)
            .map_err(|e| AdapterError::Io(e.to_string()))?;
        let clean_content = strip_provenance(&raw_content);

        let skill = Skill {
            id: "my-tool-rules".to_string(),
            version: "1.0.0".to_string(),
            triggers: vec!["*".to_string()],
            targets: vec!["my-tool".to_string()],
            source: clean_content.clone(),
            sha256: compute_sha256(&clean_content),
        };

        Ok(ImportResult {
            skills: vec![skill],
            agents: vec![],
            mcp_servers: vec![],
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        _agents: &[Agent],
        _mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let target_path = safe_join(root, ".mytoolrules")?;
        let mut combined_content = String::from("<!-- GENERATED BY SYNAPSE / LLM NEUROSURGEON — edit in the Brain -->\n\n");

        for skill in skills {
            if skill.targets.is_empty() || skill.targets.contains(&"my-tool".to_string()) {
                combined_content.push_str(&skill.source);
                combined_content.push_str("\n\n");
            }
        }

        let written = write_if_changed(&target_path, combined_content.as_bytes())
            .map_err(|e| AdapterError::Io(e.to_string()))?;

        Ok(ProjectResult {
            written: if written { vec![".mytoolrules".to_string()] } else { vec![] },
            symlinked: vec![],
        })
    }
}
```

### Step 2: Register in `adapters/mod.rs`

Add the module declaration and include it in `all_adapters()` inside `packages/core/src/adapters/mod.rs`:

```rust
pub mod my_tool;

pub fn all_adapters() -> Vec<Box<dyn Adapter>> {
    vec![
        // ... existing 13 adapters ...
        Box::new(my_tool::MyToolAdapter),
    ]
}
```

### Step 3: Register in Policy Table

In `packages/core/src/projector.rs`, add an entry to `POLICY_TABLE` for your tool ID:

```rust
("my-tool", Artifact::Rules) => Policy::Symlink,
("my-tool", Artifact::MergedConfig) => Policy::Generate,
```

---

## 5. Projection Policies & Policy Engine

SYNAPSE / llm-neuro-surgeon enforces strict rules on how projections are created:

1. **Standalone Rule Files (`Artifact::Rules`)**:
   Tools that consume whole Markdown files (e.g. `.cursorrules`, `.rules`, `CLAUDE.md`) are candidates for symlinking (`Policy::Symlink`).
2. **Merged Configurations (`Artifact::MergedConfig`)**:
   Tools that store settings or MCP configs in shared JSON/TOML/YAML files (e.g. `.zed/settings.json`, `.aider.conf.yml`) MUST use generation/merging (`Policy::Generate`). The adapter must read existing file content, parse the structure, modify only the relevant keys, and serialize back without disturbing unrelated keys.
3. **Provenance Header Requirement**:
   Every generated file MUST start with the standard header:
   ```markdown
   <!-- GENERATED BY SYNAPSE / LLM NEUROSURGEON — edit in the Brain -->
   ```


---

## 6. Security & Red-Team Requirements

All adapters undergo strict security review. You MUST adhere to these rules:

### Rule 1: Path Traversal Protection with `safe_join` & `has_path_traversal`

NEVER construct output file paths by raw `root.join(user_supplied_slug)`. An attacker could name a skill `../../.bashrc` or `/etc/passwd`.

Always use `adapters::safe_join`:

```rust
// GOOD
let safe_path = safe_join(root, &relative_filename)?;

// BAD (VULNERABLE TO PATH TRAVERSAL)
let bad_path = root.join(user_slug);
```

`safe_join` verifies that all path components are normal relative filenames and that the resulting path remains inside `root`.
For command strings, URLs, or argument lists, use `adapters::has_path_traversal(arg)` to reject path navigation attacks (`../`, `..\`).

### Rule 2: Symlink Loop Protection in Directory Walks

When listing files inside directories (e.g. `.cursor/rules/` or `.claude/skills/`), DO NOT use `path.is_dir()` or `path.is_file()` because they follow symlinks. If a user has a symlink loop (e.g. `dir/link -> dir`), `is_dir()` will cause an infinite recursion and crash the daemon.

Instead, check `DirEntry::file_type()` directly:

```rust
// GOOD
for entry in fs::read_dir(rules_dir)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    if file_type.is_file() {
        // process file...
    }
}
```

### Rule 3: Safe Parsing of Untrusted JSON/JSONC

Use `adapters::clean_jsonc()` to sanitize JSON inputs containing single/multi-line comments or trailing commas before passing them to `serde_json::from_str`.

### Rule 4: Centralized MCP Server Validation

Use `adapters::parse_and_validate_mcp_server(id, val, target_adapter, &mut out_servers)?` to validate MCP server configurations across standard JSON/TOML/YAML formats. It enforces required `command`/`url` fields, detects path traversal in commands and arguments, and extracts environment variable placeholders.

---

## 7. Helpful Adapter Utilities

`packages/core/src/adapters/mod.rs` provides useful helpers:

- **`compute_sha256(content: &str) -> String`**: Computes hex-encoded SHA-256 digest of text.
- **`strip_provenance(content: &str) -> String`**: Strips the LLM Neurosurgeon generated header if present.
- **`write_if_changed(path: &Path, content: impl AsRef<[u8]>) -> io::Result<bool>`**: Writes only when content differs, preserving mtimes.
- **`clean_jsonc(input: &str) -> String`**: Strips comments and trailing commas from JSONC files.
- **`safe_join(root: &Path, target: &str) -> Result<PathBuf, AdapterError>`**: Path traversal defense for filesystem paths.
- **`has_path_traversal(input: &str) -> bool`**: Returns true if string contains `..` path components.
- **`parse_and_validate_mcp_server(...)`**: Validates JSON objects for MCP commands, URLs, args, and env keys.
- **`split_frontmatter(content: &str) -> (Option<String>, String)`**: Extracts YAML frontmatter blocks (`--- ... ---`).
- **`parse_mdc_frontmatter(fm: &str) -> Result<MdcFrontmatter, AdapterError>`**: Parses `.mdc` style `globs` and `alwaysApply` fields.
- **`get_home_dir() -> Option<PathBuf>`**: Resolves current user's `$HOME` / `$USERPROFILE` path.

---

## 8. Testing & Verification Checklist

Before submitting a new adapter, make sure all of the following pass:

1. **Unit Tests**: Add tests inside your adapter file for `detect()`, `import()`, and `project()`.
2. **Registry Sanity Test**:
   ```bash
   cargo test -p neurosurgeon-core adapters::registry_tests
   ```
   Verifies all 13 adapters have unique IDs and do not false-detect an empty root directory.
3. **Semantic Round-Trip Test**:
   Ensure `import() -> Canonical -> project()` produces semantically identical configurations.
4. **Stress & Red-Team Tests**:
   Ensure your adapter handles missing files, malformed JSON/TOML, empty files, and malicious path components gracefully without panicking.
5. **E2E Integration Test**:
   Verify integration coverage in `packages/e2e/src/suites/tier1_feature.test.ts` and `tier2_boundary.test.ts`.
6. **Code Formatting & Linting**:
   ```bash
   cargo fmt --all --check
   cargo clippy --package neurosurgeon-core --package neurosurgeon
   ```
