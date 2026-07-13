//! Compares recorded mappings against real filesystem state to detect
//! drift — per MASTER_PROMPT.md pillar 7 ("Vitals & Doctor... dashboard
//! showing... drift/broken-link detection"). `cli doctor` (T4.x) will call
//! this to explain and fix issues like "Cursor lost 3 rules — reattach?".

use crate::adapters::compute_sha256;
use crate::mappings::Mapping;
use crate::projector::ProjectionPolicy;
use std::fs;
use std::path::Path;

/// What changed, per mapping, between the last sync and now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftStatus {
    /// The projected file matches what the last sync recorded.
    Clean,
    /// Nothing exists at `projection_path` anymore.
    Missing,
    /// A `Generate`-policy file's content hash no longer matches
    /// `content_sha256` — the user (or another tool) hand-edited a
    /// generated file.
    ContentChanged,
    /// A `Symlink`-policy path is no longer a symlink at all — something
    /// replaced it with a plain file.
    SymlinkDetached,
    /// A `Symlink`-policy path is still a symlink, but points somewhere
    /// other than the expected canonical target.
    SymlinkRetargeted { actual_target: String },
}

/// One mapping's drift result, carrying enough of the mapping to report
/// without the caller re-joining against the mappings list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    pub tool_id: String,
    pub projection_path: String,
    pub status: DriftStatus,
}

/// Checks one mapping against the filesystem. `tool_root` is the directory
/// the adapter wrote `projection_path` into (relative); `brain_root` is the
/// Brain directory `canonical_path` is relative to — a `Symlink` mapping's
/// expected target is `brain_root.join(canonical_path)`.
pub fn detect_drift(mapping: &Mapping, tool_root: &Path, brain_root: &Path) -> DriftReport {
    let full_path = tool_root.join(&mapping.projection_path);

    let status = match fs::symlink_metadata(&full_path) {
        Err(_) => DriftStatus::Missing,
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                check_symlink(&full_path, mapping, brain_root)
            } else {
                match mapping.policy {
                    ProjectionPolicy::Symlink => DriftStatus::SymlinkDetached,
                    ProjectionPolicy::Generate => check_content(&full_path, mapping),
                }
            }
        }
    };

    DriftReport {
        tool_id: mapping.tool_id.clone(),
        projection_path: mapping.projection_path.clone(),
        status,
    }
}

fn check_symlink(full_path: &Path, mapping: &Mapping, brain_root: &Path) -> DriftStatus {
    if mapping.policy != ProjectionPolicy::Symlink {
        // A Generate mapping whose path somehow became a symlink: still
        // judge it by content, since that's what the tool actually reads.
        return check_content(full_path, mapping);
    }

    match fs::read_link(full_path) {
        Err(_) => DriftStatus::SymlinkDetached,
        Ok(actual_target) => {
            let expected_target = brain_root.join(&mapping.canonical_path);
            if actual_target == expected_target {
                DriftStatus::Clean
            } else {
                DriftStatus::SymlinkRetargeted {
                    actual_target: actual_target.display().to_string(),
                }
            }
        }
    }
}

fn check_content(full_path: &Path, mapping: &Mapping) -> DriftStatus {
    match fs::read_to_string(full_path) {
        Err(_) => DriftStatus::Missing,
        Ok(content) => {
            if compute_sha256(&content) == mapping.content_sha256 {
                DriftStatus::Clean
            } else {
                DriftStatus::ContentChanged
            }
        }
    }
}

/// Runs `detect_drift` over every mapping.
pub fn detect_all(mappings: &[Mapping], tool_root: &Path, brain_root: &Path) -> Vec<DriftReport> {
    mappings
        .iter()
        .map(|m| detect_drift(m, tool_root, brain_root))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_mapping(content: &str) -> Mapping {
        Mapping {
            tool_id: "cline".into(),
            canonical_path: "skills/repo-conventions".into(),
            projection_path: ".clinerules".into(),
            policy: ProjectionPolicy::Generate,
            content_sha256: compute_sha256(content),
        }
    }

    fn symlink_mapping() -> Mapping {
        Mapping {
            tool_id: "cursor".into(),
            canonical_path: "skills/repo-conventions".into(),
            projection_path: ".cursorrules".into(),
            policy: ProjectionPolicy::Symlink,
            content_sha256: compute_sha256("irrelevant for symlink checks"),
        }
    }

    #[test]
    fn clean_generate_mapping_when_hash_matches() {
        let root = tempfile::tempdir().unwrap();
        let content = "always write tests first";
        fs::write(root.path().join(".clinerules"), content).unwrap();

        let report = detect_drift(&generate_mapping(content), root.path(), root.path());

        assert_eq!(report.status, DriftStatus::Clean);
    }

    #[test]
    fn content_changed_when_hash_mismatches() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".clinerules"), "hand-edited by user").unwrap();

        let mapping = generate_mapping("original brain content");
        let report = detect_drift(&mapping, root.path(), root.path());

        assert_eq!(report.status, DriftStatus::ContentChanged);
    }

    #[test]
    fn missing_when_generate_file_absent() {
        let root = tempfile::tempdir().unwrap();
        let mapping = generate_mapping("original brain content");

        let report = detect_drift(&mapping, root.path(), root.path());

        assert_eq!(report.status, DriftStatus::Missing);
    }

    #[test]
    fn missing_when_symlink_mapping_absent() {
        let root = tempfile::tempdir().unwrap();
        let report = detect_drift(&symlink_mapping(), root.path(), root.path());

        assert_eq!(report.status, DriftStatus::Missing);
    }

    #[cfg(unix)]
    #[test]
    fn clean_symlink_mapping_when_target_matches() {
        let tool_root = tempfile::tempdir().unwrap();
        let brain_root = tempfile::tempdir().unwrap();
        let canonical = brain_root.path().join("skills/repo-conventions");
        fs::create_dir_all(canonical.parent().unwrap()).unwrap();
        fs::write(&canonical, "brain content").unwrap();

        std::os::unix::fs::symlink(&canonical, tool_root.path().join(".cursorrules")).unwrap();

        let report = detect_drift(&symlink_mapping(), tool_root.path(), brain_root.path());

        assert_eq!(report.status, DriftStatus::Clean);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_detached_when_replaced_by_plain_file() {
        let tool_root = tempfile::tempdir().unwrap();
        let brain_root = tempfile::tempdir().unwrap();
        fs::write(tool_root.path().join(".cursorrules"), "no longer a symlink").unwrap();

        let report = detect_drift(&symlink_mapping(), tool_root.path(), brain_root.path());

        assert_eq!(report.status, DriftStatus::SymlinkDetached);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_retargeted_when_pointing_elsewhere() {
        let tool_root = tempfile::tempdir().unwrap();
        let brain_root = tempfile::tempdir().unwrap();
        let wrong_target = tool_root.path().join("somewhere-else.md");
        fs::write(&wrong_target, "not the brain").unwrap();

        std::os::unix::fs::symlink(&wrong_target, tool_root.path().join(".cursorrules")).unwrap();

        let report = detect_drift(&symlink_mapping(), tool_root.path(), brain_root.path());

        assert!(matches!(
            report.status,
            DriftStatus::SymlinkRetargeted { .. }
        ));
    }

    #[test]
    fn detect_all_reports_one_entry_per_mapping() {
        let root = tempfile::tempdir().unwrap();
        let content = "always write tests first";
        fs::write(root.path().join(".clinerules"), content).unwrap();

        let mappings = vec![generate_mapping(content), symlink_mapping()];
        let reports = detect_all(&mappings, root.path(), root.path());

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].status, DriftStatus::Clean);
        assert_eq!(reports[1].status, DriftStatus::Missing);
    }
}
