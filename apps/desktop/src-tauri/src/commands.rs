//! The desktop's bridge to `neurosurgeon-core`.
//!
//! Per IDENTITY.md: nothing is displayed that isn't measured. Every command
//! here reads real state off disk through core and returns it verbatim — no
//! command fabricates a count, and none reports success for work it did not
//! do. The previous `run_adapter_command` / `import_config` / `export_config`
//! stubs did exactly that (they printed to stdout and returned a formatted
//! "…successfully" string regardless of outcome), so they are gone rather
//! than restyled.

use std::path::PathBuf;

use neurosurgeon_core::adapters::all_adapters;
use neurosurgeon_core::doctor::{diagnose, DoctorContext, Severity};
use serde::Serialize;

// ── path resolution ─────────────────────────────────────────────────────

/// Resolves the Brain directory, mirroring the CLI's precedence so both
/// surfaces examine the same Brain: an explicit path, then
/// `$NEUROSURGEON_BRAIN_PATH` / `$NEUROSURGEON_BRAIN`, then `~/AIBrain`.
fn resolve_brain_root(explicit: Option<String>) -> Result<PathBuf, String> {
    if let Some(p) = explicit.filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    if let Some(env) = std::env::var_os("NEUROSURGEON_BRAIN_PATH")
        .or_else(|| std::env::var_os("NEUROSURGEON_BRAIN"))
    {
        return Ok(PathBuf::from(env));
    }
    directories::UserDirs::new()
        .map(|d| d.home_dir().join("AIBrain"))
        .ok_or_else(|| "no home directory; set a Brain path explicitly".to_string())
}

/// Resolves the tool config root that projections are relative to, mirroring
/// the CLI: an explicit path, then `$NEUROSURGEON_WORKSPACE_PATH` /
/// `$NEUROSURGEON_TOOL_ROOT`, then the home directory.
fn resolve_tool_root(explicit: Option<String>) -> Result<PathBuf, String> {
    if let Some(p) = explicit.filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    if let Some(env) = std::env::var_os("NEUROSURGEON_WORKSPACE_PATH")
        .or_else(|| std::env::var_os("NEUROSURGEON_TOOL_ROOT"))
    {
        return Ok(PathBuf::from(env));
    }
    directories::UserDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .ok_or_else(|| "no home directory; set a tool root explicitly".to_string())
}

// ── intake (scan) ───────────────────────────────────────────────────────

/// One adapter's presence under the scanned site. `present == false` is a
/// real finding and is rendered as such, not filtered out.
#[derive(Debug, Serialize)]
pub struct ToolFinding {
    pub id: &'static str,
    pub present: bool,
    /// Artifact counts, populated only when `present` — `None` means "not
    /// measured", which the UI must distinguish from zero.
    pub skills: Option<usize>,
    pub agents: Option<usize>,
    pub mcp_servers: Option<usize>,
    /// Set when the adapter was detected but could not be read.
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IntakeReport {
    pub site: String,
    pub total: usize,
    pub present: usize,
    pub findings: Vec<ToolFinding>,
}

/// Scans `site` for every registered adapter and, for the ones present, reads
/// what an import would take. Read-only: this runs each adapter's `import()`
/// in memory and writes nothing.
#[tauri::command]
pub fn intake(site: Option<String>) -> Result<IntakeReport, String> {
    let root = resolve_tool_root(site)?;
    let mut findings = Vec::new();

    for adapter in all_adapters() {
        if !adapter.detect(&root) {
            findings.push(ToolFinding {
                id: adapter.id(),
                present: false,
                skills: None,
                agents: None,
                mcp_servers: None,
                error: None,
            });
            continue;
        }

        match adapter.import(&root) {
            Ok(result) => findings.push(ToolFinding {
                id: adapter.id(),
                present: true,
                skills: Some(result.skills.len()),
                agents: Some(result.agents.len()),
                mcp_servers: Some(result.mcp_servers.len()),
                error: None,
            }),
            Err(e) => findings.push(ToolFinding {
                id: adapter.id(),
                present: true,
                skills: None,
                agents: None,
                mcp_servers: None,
                error: Some(e.to_string()),
            }),
        }
    }

    // Present first, then absent, alphabetical within each group — the same
    // ordering the CLI chart uses, so the two surfaces read alike.
    findings.sort_by_key(|f| (!f.present, f.id));

    let present = findings.iter().filter(|f| f.present).count();
    Ok(IntakeReport {
        site: root.display().to_string(),
        total: findings.len(),
        present,
        findings,
    })
}

// ── examination (doctor) ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct Finding {
    pub rule_id: &'static str,
    /// "critical" | "warning" | "info" — lowercased for the frontend's
    /// mark lookup.
    pub severity: &'static str,
    pub message: String,
    pub subject: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Serialize)]
pub struct ExamReport {
    pub brain: String,
    pub tools: String,
    pub criticals: usize,
    pub fixable: usize,
    pub findings: Vec<Finding>,
}

/// Runs the Doctor rule library and returns its findings unmodified.
///
/// Read-only by construction: this calls `diagnose` and never `apply_fixes`,
/// so opening the screen cannot change the Brain. Applying fixes is a
/// separate, explicit action and is not exposed here yet — the desktop must
/// not mutate a Brain on render.
#[tauri::command]
pub fn examine(brain: Option<String>, tools: Option<String>) -> Result<ExamReport, String> {
    let brain_root = resolve_brain_root(brain)?;
    let tool_root = resolve_tool_root(tools)?;

    let ctx = DoctorContext {
        brain_root: brain_root.clone(),
        tool_root: tool_root.clone(),
        mappings_path: brain_root.join(".brain/mappings.json"),
    };

    let diagnoses = diagnose(&ctx);
    let criticals = diagnoses
        .iter()
        .filter(|d| d.severity == Severity::Critical)
        .count();
    let fixable = diagnoses.iter().filter(|d| d.auto_fixable).count();

    let findings = diagnoses
        .into_iter()
        .map(|d| Finding {
            rule_id: d.rule_id,
            severity: match d.severity {
                Severity::Critical => "critical",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
            message: d.message,
            subject: d.subject,
            auto_fixable: d.auto_fixable,
        })
        .collect();

    Ok(ExamReport {
        brain: brain_root.display().to_string(),
        tools: tool_root.display().to_string(),
        criticals,
        fixable,
        findings,
    })
}

// ── build metadata ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Dry-run of the auto-update channel (T7.3). Given the release manifest a
/// channel endpoint served (fetched by the frontend), decides whether a
/// newer build exists for this platform — WITHOUT downloading, verifying, or
/// installing anything. Until release signing keys exist (Phase 8), the
/// result's `signing` field is `not_configured` and the UI must not offer a
/// one-click install: this is a check, not an installer.
#[tauri::command]
pub fn check_for_update(
    manifest_json: String,
    channel: String,
) -> Result<neurosurgeon_core::updater::DryRunResult, String> {
    let current = env!("CARGO_PKG_VERSION");
    neurosurgeon_core::updater::dry_run_from_json(current, &manifest_json, &channel)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    // A handful of adapters (zed, openai-codex, windsurf) deliberately also
    // consult `$HOME` — a user-level tool install counts as present even
    // when the scanned site has nothing. That makes `intake` tests
    // environment-dependent unless HOME is pinned to an empty directory: on
    // a machine that happens to have e.g. `~/.config/zed` (this one does),
    // an "empty site" test would see zed as present and fail for reasons
    // that have nothing to do with the code under test.
    //
    // `neurosurgeon-core` has an identical guard (`test_home`) for its own
    // adapter tests, but it is `pub(crate)` there and this is a different
    // crate, so the ~15 lines are duplicated rather than shared.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    struct HomeGuard {
        _lock: MutexGuard<'static, ()>,
        previous: Option<OsString>,
    }

    impl HomeGuard {
        fn empty(dir: &Path) -> Self {
            let lock = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let previous = std::env::var_os("HOME");
            std::env::set_var("HOME", dir);
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn intake_reports_absent_tools_as_findings() {
        // An empty site detects nothing, but every adapter still gets a row:
        // absence is reported, not hidden (IDENTITY.md).
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _home = HomeGuard::empty(home.path());

        let report = intake(Some(dir.path().display().to_string())).unwrap();

        assert_eq!(report.present, 0);
        assert_eq!(report.findings.len(), report.total);
        assert!(report.total > 0, "no adapters registered");
        assert!(report.findings.iter().all(|f| !f.present));
    }

    #[test]
    fn intake_measures_a_detected_tool() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _home = HomeGuard::empty(home.path());
        std::fs::write(dir.path().join(".clinerules"), "always write tests").unwrap();

        let report = intake(Some(dir.path().display().to_string())).unwrap();
        let cline = report
            .findings
            .iter()
            .find(|f| f.id == "cline")
            .expect("cline adapter missing");

        assert!(cline.present);
        // Counts are measured, never assumed: a present tool reports Some(n).
        assert_eq!(cline.skills, Some(1));
        assert!(cline.error.is_none());
    }

    #[test]
    fn intake_sorts_present_tools_first() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _home = HomeGuard::empty(home.path());
        std::fs::write(dir.path().join(".clinerules"), "rules").unwrap();

        let report = intake(Some(dir.path().display().to_string())).unwrap();
        let first_absent = report.findings.iter().position(|f| !f.present).unwrap();
        assert!(
            report.findings[..first_absent].iter().all(|f| f.present),
            "present and absent rows are interleaved",
        );
    }

    #[test]
    fn examine_never_writes_to_the_brain() {
        // Opening the examination screen must not mutate anything, so a
        // non-existent Brain stays non-existent after a diagnose.
        let brain = tempfile::tempdir().unwrap();
        let tools = tempfile::tempdir().unwrap();
        let target = brain.path().join("nothing-here");

        let report = examine(
            Some(target.display().to_string()),
            Some(tools.path().display().to_string()),
        )
        .unwrap();

        assert!(!target.exists(), "examine created the Brain directory");
        assert!(!report.findings.is_empty(), "a missing Brain is a finding");
    }

    #[test]
    fn examine_counts_match_the_findings_it_returns() {
        let brain = tempfile::tempdir().unwrap();
        let tools = tempfile::tempdir().unwrap();
        let report = examine(
            Some(brain.path().display().to_string()),
            Some(tools.path().display().to_string()),
        )
        .unwrap();

        let criticals = report
            .findings
            .iter()
            .filter(|f| f.severity == "critical")
            .count();
        let fixable = report.findings.iter().filter(|f| f.auto_fixable).count();

        assert_eq!(report.criticals, criticals);
        assert_eq!(report.fixable, fixable);
    }

    #[test]
    fn explicit_paths_win_over_the_environment() {
        let dir = tempfile::tempdir().unwrap();
        let explicit = dir.path().display().to_string();
        assert_eq!(
            resolve_brain_root(Some(explicit.clone())).unwrap(),
            dir.path(),
        );
        assert_eq!(resolve_tool_root(Some(explicit)).unwrap(), dir.path());
    }

    #[test]
    fn an_empty_explicit_path_falls_through_to_the_default() {
        // The frontend sends "" for "unset"; that must not become the
        // filesystem root.
        assert_ne!(
            resolve_brain_root(Some(String::new())).unwrap(),
            PathBuf::from(""),
        );
    }
}
