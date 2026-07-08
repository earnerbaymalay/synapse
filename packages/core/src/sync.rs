//! Outcomes of one sync pass (import + project). The watcher/scheduler and
//! the three-way merge logic are Phase 4 (T4.1/T4.2); this module defines
//! the shape both the daemon and `cli sync` will report against.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    /// Nothing changed on either side of the Brain.
    NoChanges,
    /// Import and/or projection applied cleanly.
    Applied { changed_paths: Vec<String> },
    /// Both sides changed the same region since the last snapshot; queued
    /// for human review rather than auto-resolved.
    ConflictQueued { conflict_ids: Vec<String> },
}

impl SyncOutcome {
    pub fn is_clean(&self) -> bool {
        matches!(self, SyncOutcome::NoChanges | SyncOutcome::Applied { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_is_clean() {
        assert!(SyncOutcome::NoChanges.is_clean());
    }

    #[test]
    fn conflict_queued_is_not_clean() {
        let outcome = SyncOutcome::ConflictQueued {
            conflict_ids: vec!["skills/repo-conventions".into()],
        };
        assert!(!outcome.is_clean());
    }
}
