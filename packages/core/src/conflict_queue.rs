//! Conflict queue — T4.2's other half. Per MASTER_PROMPT.md pillar 4,
//! "real conflicts enter a review queue in the GUI" — this module is that
//! queue's data model and the `reconcile()` entry point the sync daemon
//! calls once per canonical item whose Brain-side and tool-projected-side
//! content both changed since the last snapshot.

use crate::adapter::AdapterError;
use crate::merge::{three_way_merge, MergeOutcome};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// One conflict awaiting human review. Carries all three sides so the GUI
/// can show a diff view, plus the pre-computed `merged_with_markers` text
/// as a ready-to-edit starting point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedConflict {
    /// Stable id derived from `canonical_path` — re-running `reconcile()`
    /// on the same item overwrites its existing queue entry rather than
    /// appending a duplicate.
    pub id: String,
    /// Path within the Brain, e.g. `"skills/repo-conventions"`.
    pub canonical_path: String,
    pub base: String,
    pub local: String,
    pub remote: String,
    pub merged_with_markers: String,
}

/// The full conflict queue — one entry per canonical item currently
/// blocked on human review. Sync must not write anything for an item
/// while it has a queued conflict.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictQueue {
    pub conflicts: Vec<QueuedConflict>,
}

impl ConflictQueue {
    /// Adds `conflict`, replacing any existing entry for the same
    /// `canonical_path` (a re-sync of an already-queued item must update
    /// its queue entry, not pile up duplicates).
    pub fn enqueue(&mut self, conflict: QueuedConflict) {
        self.conflicts.retain(|c| c.id != conflict.id);
        self.conflicts.push(conflict);
    }

    /// Removes and returns the conflict for `id`, if queued. The caller
    /// is responsible for writing the human's chosen resolution as the
    /// item's new content and re-snapshotting.
    pub fn resolve(&mut self, id: &str) -> Option<QueuedConflict> {
        let idx = self.conflicts.iter().position(|c| c.id == id)?;
        Some(self.conflicts.remove(idx))
    }

    pub fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Loads `path`. A Brain with no queued conflicts has no file yet, so
    /// a missing file is not an error — it loads as an empty queue, same
    /// convention as `MappingsFile::load`.
    pub fn load(path: &Path) -> Result<ConflictQueue, AdapterError> {
        if !path.exists() {
            return Ok(ConflictQueue::default());
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| AdapterError::Io(format!("Failed to read {}: {}", path.display(), e)))?;
        serde_json::from_str(&raw).map_err(|e| {
            AdapterError::Malformed(format!("Failed to parse {}: {}", path.display(), e))
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), AdapterError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AdapterError::Io(format!("Failed to create {}: {}", parent.display(), e))
            })?;
        }
        let pretty = serde_json::to_string_pretty(self).map_err(|e| {
            AdapterError::Malformed(format!("Failed to serialize conflict queue: {}", e))
        })?;
        fs::write(path, pretty)
            .map_err(|e| AdapterError::Io(format!("Failed to write {}: {}", path.display(), e)))
    }
}

/// Reconciles one canonical item whose Brain-side (`local`) and
/// tool-projected-side (`remote`) content have both changed since the
/// last snapshot (`base`). Disjoint or identical edits auto-resolve —
/// returns `Some(merged)`, safe to write back to both sides. Overlapping,
/// differently-edited regions enqueue a [`QueuedConflict`] in `queue` and
/// return `None` — nothing should be written for `canonical_path` until a
/// human resolves it via [`ConflictQueue::resolve`].
pub fn reconcile(
    queue: &mut ConflictQueue,
    canonical_path: &str,
    base: &str,
    local: &str,
    remote: &str,
) -> Option<String> {
    match three_way_merge(base, local, remote) {
        MergeOutcome::Clean(merged) => Some(merged),
        MergeOutcome::Conflict {
            merged_with_markers,
        } => {
            queue.enqueue(QueuedConflict {
                id: canonical_path.to_string(),
                canonical_path: canonical_path.to_string(),
                base: base.to_string(),
                local: local.to_string(),
                remote: remote.to_string(),
                merged_with_markers,
            });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_conflict(id: &str) -> QueuedConflict {
        QueuedConflict {
            id: id.to_string(),
            canonical_path: id.to_string(),
            base: "base".to_string(),
            local: "local".to_string(),
            remote: "remote".to_string(),
            merged_with_markers: "<<<<<<< ours\nlocal\n=======\nremote\n>>>>>>> theirs".to_string(),
        }
    }

    #[test]
    fn enqueue_then_resolve_round_trips() {
        let mut queue = ConflictQueue::default();
        queue.enqueue(sample_conflict("skills/repo-conventions"));
        assert!(!queue.is_empty());

        let resolved = queue.resolve("skills/repo-conventions").unwrap();
        assert_eq!(resolved.id, "skills/repo-conventions");
        assert!(queue.is_empty());
    }

    #[test]
    fn resolve_missing_id_returns_none() {
        let mut queue = ConflictQueue::default();
        assert!(queue.resolve("nothing-here").is_none());
    }

    #[test]
    fn enqueue_replaces_existing_entry_for_the_same_path() {
        let mut queue = ConflictQueue::default();
        queue.enqueue(sample_conflict("skills/repo-conventions"));
        let mut updated = sample_conflict("skills/repo-conventions");
        updated.local = "changed again".to_string();
        queue.enqueue(updated);

        assert_eq!(queue.conflicts.len(), 1);
        assert_eq!(queue.conflicts[0].local, "changed again");
    }

    #[test]
    fn load_on_missing_file_returns_empty_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".brain/conflicts.json");
        let loaded = ConflictQueue::load(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn round_trips_through_save_then_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".brain/conflicts.json");
        let mut queue = ConflictQueue::default();
        queue.enqueue(sample_conflict("skills/repo-conventions"));

        queue.save(&path).unwrap();
        let loaded = ConflictQueue::load(&path).unwrap();

        assert_eq!(loaded, queue);
    }

    #[test]
    fn reconcile_auto_resolves_disjoint_edits_without_queuing() {
        let mut queue = ConflictQueue::default();
        // Edits need unchanged context between them for a line-based diff
        // to prove they're disjoint — two edited lines with nothing
        // between them is inherently ambiguous to any line-based 3-way
        // merge (this is a property of the algorithm, not this code; see
        // `reconcile_queues_a_conflict_and_returns_none_on_overlap` below
        // for that adjacent-edit case, which correctly conflicts).
        let base = "line1\nline2\nline3\nline4\nline5\n";
        let local = "line1\nLOCAL\nline3\nline4\nline5\n";
        let remote = "line1\nline2\nline3\nline4\nREMOTE\n";

        let merged = reconcile(&mut queue, "skills/x", base, local, remote);

        assert!(merged.is_some());
        let merged = merged.unwrap();
        assert!(merged.contains("LOCAL"));
        assert!(merged.contains("REMOTE"));
        assert!(queue.is_empty(), "disjoint edits must not queue a conflict");
    }

    #[test]
    fn reconcile_queues_a_conflict_and_returns_none_on_overlap() {
        let mut queue = ConflictQueue::default();
        let base = "line1\nline2\nline3\n";
        let local = "line1\nLOCAL_VERSION\nline3\n";
        let remote = "line1\nREMOTE_VERSION\nline3\n";

        let merged = reconcile(&mut queue, "skills/x", base, local, remote);

        assert!(merged.is_none());
        assert_eq!(queue.conflicts.len(), 1);
        assert_eq!(queue.conflicts[0].canonical_path, "skills/x");
        assert!(queue.conflicts[0].merged_with_markers.contains("<<<<<<<"));
    }
}
