//! Dual-Zone Master Prompt (`SYNAPSE.md`) engine.
//!
//! Zone 1: Immutable Clinical Operating Core.
//! Enforces agent identity, Synaptic Memory hooks (`synapse mem hook`), workspace safety bounds,
//! and Sensory Gating token compression rules. Stamped with `<!-- SYNAPSE-CORE-HASH: <sha256> -->`.
//! If tampered with, `synapse doctor` raises an unbypassable critical fault.
//!
//! Zone 2: User Engineering Persona & Context.
//! Fully customizable by the user. Pre-populated with high-leverage suggestion blocks
//! (TDD, Clean Architecture, Strict Typing, Kaizen refactoring).

use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const MASTER_PROMPT_FILENAME: &str = "SYNAPSE.md";
pub const ZONE1_HEADER: &str = "<!-- === SYNAPSE CLINICAL CORE: ZONE 1 (IMMUTABLE) === -->";
pub const ZONE1_FOOTER: &str = "<!-- === END SYNAPSE CLINICAL CORE: ZONE 1 === -->";
pub const ZONE2_HEADER: &str = "<!-- === SYNAPSE USER CONTEXT: ZONE 2 (EDITABLE) === -->";
pub const HASH_PREFIX: &str = "<!-- SYNAPSE-CORE-HASH: ";
pub const HASH_SUFFIX: &str = " -->";

#[derive(Debug)]
pub enum MasterPromptError {
    Io(std::io::Error),
    NotFound(PathBuf),
    MalformedBoundaries,
    MissingChecksum,
    ChecksumMismatch {
        expected: String,
        calculated: String,
    },
}

impl fmt::Display for MasterPromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MasterPromptError::Io(e) => write!(f, "I/O error: {}", e),
            MasterPromptError::NotFound(p) => {
                write!(f, "Master prompt not found at {}", p.display())
            }
            MasterPromptError::MalformedBoundaries => {
                write!(f, "Zone 1 clinical core boundaries missing or corrupted")
            }
            MasterPromptError::MissingChecksum => {
                write!(f, "Zone 1 cryptographic checksum missing or invalid")
            }
            MasterPromptError::ChecksumMismatch {
                expected,
                calculated,
            } => write!(
                f,
                "Zone 1 checksum mismatch: expected {}, calculated {}",
                expected, calculated
            ),
        }
    }
}

impl std::error::Error for MasterPromptError {}

impl From<std::io::Error> for MasterPromptError {
    fn from(err: std::io::Error) -> Self {
        MasterPromptError::Io(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasterPromptStatus {
    Valid,
    Missing,
    Corrupted(String),
}

/// The canonical Zone 1 text (without hash stamp).
pub const CANONICAL_ZONE1_CONTENT: &str = r#"# SYNAPSE CLINICAL OPERATING CORE
<!-- Critical agent invariant: All AI models operating in this repository must adhere to these rules. -->

## 1. AGENT IDENTITY & RESPONSIBILITY
You are an accountable surgical collaborator operating inside a SYNAPSE-governed repository.
- Every modification must be deliberate, verifiable, and minimal.
- Never invent placeholder code, unmeasured statistics, or mock implementations.
- Always check that your changes compile and pass existing test suites before concluding.

## 2. SYNAPTIC MEMORY LIFECYCLE HOOKS
- At session start, load architectural context using `synapse mem context`.
- Query past decisions and historical fixes using `synapse mem search "<query>"`.
- Record non-trivial decisions or resolved anomalies with `synapse mem observe "<summary>" --file <path>`.

## 3. SENSORY GATING & TOKEN CONSERVATION
- Run verbose build and test outputs through Synapse Sensory Gating (`synapse exec -- <command>`).
- Filter repetitive test lines into concise milestone summaries while preserving 100% of errors and stack traces.

## 4. WORKSPACE CONTAINMENT & PRIVACY
- All file reads and writes must remain strictly bounded inside the designated workspace.
- Never write API keys or sensitive secrets in plaintext files. Use `${ENV_VAR}` placeholders backed by the OS Keychain.
"#;

/// Default templates for Zone 2 user customization.
pub const DEFAULT_ZONE2_CONTENT: &str = r#"# USER ENGINEERING CONTEXT & PERSONA
<!-- Edit this section freely to define your project guidelines, tech stack conventions, and persona preferences. -->

## Project Conventions
- Language & Toolchain: Rust 1.75+ / TypeScript React
- Architecture: Modular, decoupled, strictly typed
- Code Style: Terse, idiomatic, thoroughly tested

<!-- ======================================================================= -->
<!-- SYNAPSE PROMPT SUGGESTIONS (Uncomment and customize as needed):         -->
<!--                                                                         -->
<!-- [SUGGESTION: STRICT TEST-DRIVEN DEVELOPMENT]                            -->
<!-- - Write unit tests verifying failures before implementing new features. -->
<!-- - Keep tests isolated, deterministic, and free of network I/O.          -->
<!--                                                                         -->
<!-- [SUGGESTION: CLEAN ARCHITECTURE]                                        -->
<!-- - Enforce strict unidirectional dependencies from domain to adapters.  -->
<!-- - Keep data models independent of framework serialization schemas.     -->
<!--                                                                         -->
<!-- [SUGGESTION: HIGH-DENSITY PAIR PROGRAMMING]                             -->
<!-- - Keep responses concise and focused on code diffs and verification.    -->
<!-- - Proactively identify edge cases, boundary conditions, and race states.-->
<!-- ======================================================================= -->
"#;

/// Calculates the SHA-256 hash of a string, formatted as lowercase hex.
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Builds the complete `SYNAPSE.md` document with Zone 1 stamped with its SHA-256 hash.
pub fn render_master_prompt(zone1_core: &str, zone2_custom: &str) -> String {
    let core_clean = zone1_core.trim();
    let hash = hash_content(core_clean);

    format!(
        "{}\n{}{}{}\n\n{}\n{}\n\n{}\n{}",
        ZONE1_HEADER,
        HASH_PREFIX,
        hash,
        HASH_SUFFIX,
        core_clean,
        ZONE1_FOOTER,
        ZONE2_HEADER,
        zone2_custom.trim()
    )
}

/// Generates the canonical default `SYNAPSE.md`.
pub fn generate_default_master_prompt() -> String {
    render_master_prompt(CANONICAL_ZONE1_CONTENT, DEFAULT_ZONE2_CONTENT)
}

/// Inspects the `SYNAPSE.md` file in `brain_dir` and verifies Zone 1 integrity.
pub fn verify_master_prompt_integrity(brain_dir: &Path) -> MasterPromptStatus {
    let prompt_path = brain_dir.join(MASTER_PROMPT_FILENAME);
    if !prompt_path.exists() {
        return MasterPromptStatus::Missing;
    }

    let content = match fs::read_to_string(&prompt_path) {
        Ok(c) => c,
        Err(e) => return MasterPromptStatus::Corrupted(format!("Failed to read file: {}", e)),
    };

    match extract_zones(&content) {
        Ok((zone1, expected_hash, _zone2)) => {
            let actual_hash = hash_content(&zone1);
            if actual_hash != expected_hash {
                MasterPromptStatus::Corrupted(format!(
                    "Zone 1 hash mismatch: expected {}, calculated {}",
                    expected_hash, actual_hash
                ))
            } else if hash_content(CANONICAL_ZONE1_CONTENT.trim()) != actual_hash {
                MasterPromptStatus::Corrupted(
                    "Zone 1 content differs from canonical clinical core standard".to_string(),
                )
            } else {
                MasterPromptStatus::Valid
            }
        }
        Err(err) => MasterPromptStatus::Corrupted(err.to_string()),
    }
}

/// Extracts Zone 1, its declared hash, and Zone 2 from a `SYNAPSE.md` string.
pub fn extract_zones(content: &str) -> Result<(String, String, String), MasterPromptError> {
    let z1_start = content
        .find(ZONE1_HEADER)
        .ok_or(MasterPromptError::MalformedBoundaries)?;
    let z1_end = content
        .find(ZONE1_FOOTER)
        .ok_or(MasterPromptError::MalformedBoundaries)?;

    if z1_start >= z1_end {
        return Err(MasterPromptError::MalformedBoundaries);
    }

    let z1_slice = &content[z1_start + ZONE1_HEADER.len()..z1_end];

    // Find hash stamp
    let hash_start = z1_slice
        .find(HASH_PREFIX)
        .ok_or(MasterPromptError::MissingChecksum)?;
    let hash_end = z1_slice[hash_start..]
        .find(HASH_SUFFIX)
        .ok_or(MasterPromptError::MissingChecksum)?;

    let declared_hash = z1_slice[hash_start + HASH_PREFIX.len()..hash_start + hash_end]
        .trim()
        .to_string();

    // Zone 1 content is everything after the hash stamp line up to footer
    let content_after_hash = &z1_slice[hash_start + hash_end + HASH_SUFFIX.len()..];
    let zone1_content = content_after_hash.trim().to_string();

    // Zone 2 is everything after ZONE2_HEADER, or empty if absent
    let zone2_content = if let Some(z2_start) = content.find(ZONE2_HEADER) {
        content[z2_start + ZONE2_HEADER.len()..].trim().to_string()
    } else {
        String::new()
    };

    Ok((zone1_content, declared_hash, zone2_content))
}

/// Heals `SYNAPSE.md` in `brain_dir` by restoring canonical Zone 1 while strictly preserving Zone 2.
pub fn heal_master_prompt(brain_dir: &Path) -> Result<(), MasterPromptError> {
    let prompt_path = brain_dir.join(MASTER_PROMPT_FILENAME);
    let zone2_existing = if prompt_path.exists() {
        if let Ok(content) = fs::read_to_string(&prompt_path) {
            extract_zones(&content)
                .map(|(_, _, z2)| z2)
                .unwrap_or_else(|_| DEFAULT_ZONE2_CONTENT.to_string())
        } else {
            DEFAULT_ZONE2_CONTENT.to_string()
        }
    } else {
        DEFAULT_ZONE2_CONTENT.to_string()
    };

    let healed = render_master_prompt(CANONICAL_ZONE1_CONTENT, &zone2_existing);
    fs::write(&prompt_path, healed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_render_and_extract_roundtrip() {
        let rendered = generate_default_master_prompt();
        let (z1, hash, z2) = extract_zones(&rendered).expect("extract should succeed");
        assert_eq!(hash, hash_content(&z1));
        assert!(z1.contains("SYNAPSE CLINICAL OPERATING CORE"));
        assert!(z2.contains("USER ENGINEERING CONTEXT"));
    }

    #[test]
    fn test_integrity_check_valid() {
        let dir = tempdir().unwrap();
        let prompt_path = dir.path().join(MASTER_PROMPT_FILENAME);
        fs::write(&prompt_path, generate_default_master_prompt()).unwrap();

        assert_eq!(
            verify_master_prompt_integrity(dir.path()),
            MasterPromptStatus::Valid
        );
    }

    #[test]
    fn test_integrity_check_tampered() {
        let dir = tempdir().unwrap();
        let prompt_path = dir.path().join(MASTER_PROMPT_FILENAME);
        let valid = generate_default_master_prompt();
        // Tamper with Zone 1 text without updating hash
        let tampered = valid.replace("deliberate, verifiable", "casual and unverified");
        fs::write(&prompt_path, tampered).unwrap();

        match verify_master_prompt_integrity(dir.path()) {
            MasterPromptStatus::Corrupted(_) => {}
            other => panic!("expected Corrupted, got {:?}", other),
        }
    }

    #[test]
    fn test_heal_preserves_zone2() {
        let dir = tempdir().unwrap();
        let prompt_path = dir.path().join(MASTER_PROMPT_FILENAME);
        let custom_zone2 = "## Custom Rule\nAlways write in Haskell.";
        let tampered = render_master_prompt("tampered zone 1", custom_zone2);
        fs::write(&prompt_path, tampered).unwrap();

        heal_master_prompt(dir.path()).unwrap();

        let healed = fs::read_to_string(&prompt_path).unwrap();
        let (z1, _, z2) = extract_zones(&healed).unwrap();
        assert!(z1.contains("SYNAPSE CLINICAL OPERATING CORE"));
        assert!(z2.contains("Always write in Haskell."));
        assert_eq!(
            verify_master_prompt_integrity(dir.path()),
            MasterPromptStatus::Valid
        );
    }
}
