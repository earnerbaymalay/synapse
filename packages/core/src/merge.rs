//! Three-way text merge — T4.2. Per MASTER_PROMPT.md pillar 4: when both
//! sides of a sync changed since the last snapshot, "three-way merge
//! against last git snapshot; disjoint markdown merges auto-resolve, real
//! conflicts enter a review queue in the GUI." This module is the merge
//! half; `crate::conflict_queue` is the review-queue half.
//!
//! Wraps the `diffy` crate's diff3-style merge rather than hand-rolling
//! one — a buggy bespoke three-way merge would silently corrupt a user's
//! skill/rule content, which is exactly the kind of mistake this
//! project's "safety by design" pillar exists to prevent. `diffy` is pure
//! Rust (no C dependency) and its `merge()` matches the semantics this
//! module documents: disjoint edits and identical concurrent edits both
//! auto-resolve; only genuinely overlapping-and-different edits conflict.

/// The result of merging `local` and `remote` against their common
/// ancestor `base`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Every changed region was disjoint, or both sides made the same
    /// edit. Contains the merged content — safe to write back to both
    /// sides without any human review.
    Clean(String),
    /// At least one region was edited differently by both sides.
    /// `merged_with_markers` embeds standard `<<<<<<< / ||||||| / =======
    /// / >>>>>>>` conflict markers at each conflicting hunk (the same
    /// format `git merge` leaves in a conflicted file), so it's directly
    /// usable as the content a human reviews/edits in the GUI's queue.
    Conflict { merged_with_markers: String },
}

/// Merges `local` and `remote`, both descended from `base`.
pub fn three_way_merge(base: &str, local: &str, remote: &str) -> MergeOutcome {
    match diffy::merge(base, local, remote) {
        Ok(merged) => MergeOutcome::Clean(merged),
        Err(merged_with_markers) => MergeOutcome::Conflict {
            merged_with_markers,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "line1\nline2\nline3\nline4\nline5\n";

    #[test]
    fn disjoint_edits_merge_cleanly() {
        let local = "line1\nLOCAL2\nline3\nline4\nline5\n";
        let remote = "line1\nline2\nline3\nREMOTE4\nline5\n";

        let outcome = three_way_merge(BASE, local, remote);

        match outcome {
            MergeOutcome::Clean(merged) => {
                assert!(merged.contains("LOCAL2"));
                assert!(merged.contains("REMOTE4"));
            }
            other => panic!("expected Clean, got {other:?}"),
        }
    }

    #[test]
    fn identical_concurrent_edits_merge_cleanly() {
        let local = "line1\nSAME\nline3\nline4\nline5\n";
        let remote = "line1\nSAME\nline3\nline4\nline5\n";

        let outcome = three_way_merge(BASE, local, remote);

        assert!(matches!(outcome, MergeOutcome::Clean(_)));
    }

    #[test]
    fn overlapping_different_edits_conflict() {
        let local = "line1\nLOCAL_VERSION\nline3\nline4\nline5\n";
        let remote = "line1\nREMOTE_VERSION\nline3\nline4\nline5\n";

        let outcome = three_way_merge(BASE, local, remote);

        match outcome {
            MergeOutcome::Conflict {
                merged_with_markers,
            } => {
                assert!(merged_with_markers.contains("<<<<<<<"));
                assert!(merged_with_markers.contains("======="));
                assert!(merged_with_markers.contains(">>>>>>>"));
                assert!(merged_with_markers.contains("LOCAL_VERSION"));
                assert!(merged_with_markers.contains("REMOTE_VERSION"));
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[test]
    fn unchanged_content_merges_cleanly_to_the_original() {
        let outcome = three_way_merge(BASE, BASE, BASE);
        assert_eq!(outcome, MergeOutcome::Clean(BASE.to_string()));
    }
}
