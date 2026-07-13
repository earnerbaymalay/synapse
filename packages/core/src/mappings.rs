//! `.brain/mappings.json` — the persisted record linking each projected
//! file back to its canonical Brain source, per MASTER_PROMPT.md's Brain
//! layout: "mappings.json (source↔canonical↔projection + hashes)". This
//! module only defines the shape and load/save I/O; nothing in this crate
//! yet writes real mappings during a sync pass (that lands when the
//! adapters are wired to the projector, a later phase) — this is the data
//! contract the drift detector (`crate::drift`) reads.

use crate::adapter::AdapterError;
use crate::projector::ProjectionPolicy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// One projected file's provenance: which tool it belongs to, where it
/// lives in the Brain, where it was written in the tool's config, which
/// policy produced it, and the content hash recorded at that sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    /// The adapter id that owns this mapping, e.g. `"cursor"`.
    pub tool_id: String,
    /// Path within the Brain, e.g. `"skills/repo-conventions"`.
    pub canonical_path: String,
    /// Path the adapter wrote into the tool's root, e.g.
    /// `".cursor/rules/repo-conventions.mdc"`.
    pub projection_path: String,
    /// The policy that produced `projection_path` at this sync. Determines
    /// how the drift detector checks this mapping: a `Symlink` mapping is
    /// checked via `symlink_metadata`/`read_link`; a `Generate` mapping is
    /// checked by re-hashing its content.
    pub policy: ProjectionPolicy,
    /// SHA256 of the canonical content as of this sync (see
    /// `crate::adapters::compute_sha256`). For `Symlink` mappings this is
    /// informational; the drift check itself is target-equality, not a
    /// hash comparison, since the file's content is the canonical file's
    /// content by construction.
    pub content_sha256: String,
}

/// The full `.brain/mappings.json` document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MappingsFile {
    pub mappings: Vec<Mapping>,
}

impl MappingsFile {
    /// Loads `path`. A first-run Brain has no mappings.json yet, so a
    /// missing file is not an error — it loads as an empty `MappingsFile`.
    pub fn load(path: &Path) -> Result<MappingsFile, AdapterError> {
        if !path.exists() {
            return Ok(MappingsFile::default());
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| AdapterError::Io(format!("Failed to read {}: {}", path.display(), e)))?;
        serde_json::from_str(&raw).map_err(|e| {
            AdapterError::Malformed(format!("Failed to parse {}: {}", path.display(), e))
        })
    }

    /// Writes `path`, creating parent directories (e.g. `.brain/`) as
    /// needed.
    pub fn save(&self, path: &Path) -> Result<(), AdapterError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AdapterError::Io(format!("Failed to create {}: {}", parent.display(), e))
            })?;
        }
        let pretty = serde_json::to_string_pretty(self)
            .map_err(|e| AdapterError::Malformed(format!("Failed to serialize mappings: {}", e)))?;
        fs::write(path, pretty)
            .map_err(|e| AdapterError::Io(format!("Failed to write {}: {}", path.display(), e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mapping() -> Mapping {
        Mapping {
            tool_id: "cursor".into(),
            canonical_path: "skills/repo-conventions".into(),
            projection_path: ".cursor/rules/repo-conventions.mdc".into(),
            policy: ProjectionPolicy::Symlink,
            content_sha256: "deadbeef".into(),
        }
    }

    #[test]
    fn load_on_missing_file_returns_empty_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".brain/mappings.json");
        let loaded = MappingsFile::load(&path).unwrap();
        assert!(loaded.mappings.is_empty());
    }

    #[test]
    fn round_trips_through_save_then_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".brain/mappings.json");
        let file = MappingsFile {
            mappings: vec![sample_mapping()],
        };

        file.save(&path).unwrap();
        let loaded = MappingsFile::load(&path).unwrap();

        assert_eq!(loaded, file);
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".brain/nested/mappings.json");
        let file = MappingsFile {
            mappings: vec![sample_mapping()],
        };

        file.save(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn load_rejects_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mappings.json");
        fs::write(&path, "{ not json").unwrap();

        assert!(matches!(
            MappingsFile::load(&path),
            Err(AdapterError::Malformed(_))
        ));
    }
}
