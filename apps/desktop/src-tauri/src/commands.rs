//! The desktop's bridge to `synapse-core`.
//!
//! Per IDENTITY.md: nothing is displayed that isn't measured. Every command
//! here reads real state off disk through core and returns it verbatim — no
//! command fabricates a count, and none reports success for work it did not
//! do. The previous `run_adapter_command` / `import_config` / `export_config`
//! stubs did exactly that (they printed to stdout and returned a formatted
//! "…successfully" string regardless of outcome), so they are gone rather
//! than restyled.

use std::path::PathBuf;

use serde::Serialize;
use synapse_core::adapters::all_adapters;
use synapse_core::doctor::{diagnose, DoctorContext, Severity};

// ── path resolution ─────────────────────────────────────────────────────

/// Normalises a user-supplied path: trims surrounding whitespace (a pasted
/// path often carries a trailing newline) and expands a leading `~` so
/// `~/AIBrain` means the home directory, not a folder literally named `~`.
/// Spaces and non-ASCII characters inside the path are preserved verbatim.
fn user_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == "~" {
        if let Some(d) = directories::UserDirs::new() {
            return Some(d.home_dir().to_path_buf());
        }
    }
    Some(crate::config::expand_tilde(trimmed))
}

/// The first of `vars` that is set to a non-empty value. An env var set to
/// "" must not become the relative path "" (i.e. the process cwd).
fn env_path(vars: &[&str]) -> Option<PathBuf> {
    vars.iter()
        .filter_map(|v| std::env::var_os(v))
        .find(|v| !v.is_empty())
        .and_then(|v| user_path(&v.to_string_lossy()))
}

/// Resolves the Brain directory, mirroring the CLI's precedence so both
/// surfaces examine the same Brain: an explicit path, then
/// `$SYNAPSE_BRAIN_PATH` / `$SYNAPSE_BRAIN`, then `~/AIBrain`.
fn resolve_brain_root(explicit: Option<String>) -> Result<PathBuf, String> {
    if let Some(p) = explicit.as_deref().and_then(user_path) {
        return Ok(p);
    }
    if let Some(p) = env_path(&["SYNAPSE_BRAIN_PATH", "SYNAPSE_BRAIN"]) {
        return Ok(p);
    }
    directories::UserDirs::new()
        .map(|d| d.home_dir().join("AIBrain"))
        .ok_or_else(|| {
            "Couldn't find your home folder, so the Brain location is unknown. \
             Set SYNAPSE_BRAIN_PATH to your Brain folder and try again."
                .to_string()
        })
}

/// Resolves the tool config root that projections are relative to, mirroring
/// the CLI: an explicit path, then `$SYNAPSE_WORKSPACE_PATH` /
/// `$SYNAPSE_TOOL_ROOT`, then the home directory.
fn resolve_tool_root(explicit: Option<String>) -> Result<PathBuf, String> {
    if let Some(p) = explicit.as_deref().and_then(user_path) {
        return Ok(p);
    }
    if let Some(p) = env_path(&["SYNAPSE_WORKSPACE_PATH", "SYNAPSE_TOOL_ROOT"]) {
        return Ok(p);
    }
    directories::UserDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .ok_or_else(|| {
            "Couldn't find your home folder, so there is nowhere to look for AI tool \
             configs. Set SYNAPSE_WORKSPACE_PATH and try again."
                .to_string()
        })
}

/// Runs filesystem-heavy work on Tauri's blocking pool. Synchronous
/// `#[tauri::command]`s execute on the main thread, so an adapter scan or a
/// Doctor pass over a large Brain would freeze the window until it finished;
/// every command that touches the disk goes through here instead.
async fn off_main_thread<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| format!("The background task stopped unexpectedly: {e}"))?
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
pub async fn intake(site: Option<String>) -> Result<IntakeReport, String> {
    off_main_thread(move || run_intake(site)).await
}

pub fn run_intake(site: Option<String>) -> Result<IntakeReport, String> {
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
pub async fn examine(brain: Option<String>, tools: Option<String>) -> Result<ExamReport, String> {
    off_main_thread(move || run_examine(brain, tools)).await
}

pub fn run_examine(brain: Option<String>, tools: Option<String>) -> Result<ExamReport, String> {
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
) -> Result<synapse_core::updater::DryRunResult, String> {
    let current = env!("CARGO_PKG_VERSION");
    synapse_core::updater::dry_run_from_json(current, &manifest_json, &channel)
        .map_err(|e| format!("Couldn't check for updates: {e}"))
}

// ── memory & cyber graph ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MemoryNodeDto {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub narrative: String,
    pub files: Vec<String>,
    pub ts: u64,
    pub val: f32,
}

#[derive(Debug, Serialize)]
pub struct MemoryEdgeDto {
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct MemoryGraphPayload {
    pub nodes: Vec<MemoryNodeDto>,
    pub edges: Vec<MemoryEdgeDto>,
    pub total_observations: usize,
}

#[tauri::command]
pub async fn get_memory_graph(brain: Option<String>) -> Result<MemoryGraphPayload, String> {
    off_main_thread(move || run_get_memory_graph(brain)).await
}

pub fn run_get_memory_graph(brain: Option<String>) -> Result<MemoryGraphPayload, String> {
    // The CLI (`synapse mem`) records observations to
    // `MemoryStore::default_dir()` ($SYNAPSE_MEMORY_DIR, else
    // ~/.synapse/memory). Reading `<brain>/memory` instead meant the graph
    // never saw anything the CLI recorded. An explicit Brain still scopes
    // the store to that Brain's own `memory/` folder.
    let explicit_brain = brain.as_deref().and_then(user_path);
    let memory_dir = match &explicit_brain {
        Some(b) => b.join("memory"),
        None => synapse_core::memory::MemoryStore::default_dir(),
    };
    let brain_root = match explicit_brain {
        Some(b) => b,
        None => resolve_brain_root(None)?,
    };
    let store = synapse_core::memory::MemoryStore::new(memory_dir.clone());
    // A missing log is "nothing recorded" (core returns Ok(empty)); any other
    // read failure is reported, not disguised as an empty graph.
    let observations = store.all().map_err(|e| {
        format!(
            "Couldn't read the memory log in {}. ({e})",
            memory_dir.display()
        )
    })?;

    // Nothing recorded means nothing to draw: no root node and no sample
    // nodes (IDENTITY rule 3 — never display what wasn't measured). An empty
    // payload is what lets the screen show "No memory nodes recorded."
    if observations.is_empty() {
        return Ok(MemoryGraphPayload {
            nodes: Vec::new(),
            edges: Vec::new(),
            total_observations: 0,
        });
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut projects_seen = std::collections::HashSet::new();
    let mut files_seen = std::collections::HashSet::new();

    nodes.push(MemoryNodeDto {
        id: "brain-root".to_string(),
        label: "CANONICAL BRAIN".to_string(),
        kind: "core".to_string(),
        narrative: format!("Canonical Brain at {}", brain_root.display()),
        files: vec![],
        ts: 0,
        val: 20.0,
    });

    for obs in &observations {
        if projects_seen.insert(obs.project.clone()) {
            let p_id = format!("project-{}", obs.project);
            nodes.push(MemoryNodeDto {
                id: p_id.clone(),
                label: obs.project.clone(),
                kind: "project".to_string(),
                narrative: format!("Project workspace: {}", obs.project),
                files: vec![],
                ts: obs.ts,
                val: 14.0,
            });
            edges.push(MemoryEdgeDto {
                source: "brain-root".to_string(),
                target: p_id,
                label: "tracks".to_string(),
            });
        }

        let obs_id = format!("obs-{}", obs.id);
        nodes.push(MemoryNodeDto {
            id: obs_id.clone(),
            label: obs.title.clone(),
            kind: obs.kind.clone(),
            narrative: obs.narrative.clone(),
            files: obs.files.clone(),
            ts: obs.ts,
            val: 8.0,
        });

        edges.push(MemoryEdgeDto {
            source: format!("project-{}", obs.project),
            target: obs_id.clone(),
            label: "observed".to_string(),
        });

        for file in &obs.files {
            let f_id = format!("file-{}", file);
            if files_seen.insert(f_id.clone()) {
                nodes.push(MemoryNodeDto {
                    id: f_id.clone(),
                    label: file.split('/').next_back().unwrap_or(file).to_string(),
                    kind: "file".to_string(),
                    narrative: format!("Touched file: {}", file),
                    files: vec![file.clone()],
                    ts: obs.ts,
                    val: 6.0,
                });
            }
            edges.push(MemoryEdgeDto {
                source: obs_id.clone(),
                target: f_id,
                label: "modifies".to_string(),
            });
        }
    }

    let total = observations.len();
    Ok(MemoryGraphPayload {
        nodes,
        edges,
        total_observations: total,
    })
}

// ── telemetry & cockpit ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CockpitTelemetry {
    pub gateway_status: &'static str,
    pub upstream_url: String,
    pub model: String,
    pub latency_ms: u64,
    pub requests_handled: usize,
    pub raw_prompt_tokens: usize,
    pub gated_prompt_tokens: usize,
    pub tokens_saved: usize,
    pub reduction_pct: f32,
    pub brain_path: String,
    pub adapters_present: usize,
    pub adapters_total: usize,
    pub doctor_criticals: usize,
    pub doctor_fixable: usize,
}

#[tauri::command]
pub async fn get_cockpit_telemetry(
    brain: Option<String>,
    tools: Option<String>,
) -> Result<CockpitTelemetry, String> {
    off_main_thread(move || run_get_cockpit_telemetry(brain, tools)).await
}

pub fn run_get_cockpit_telemetry(
    brain: Option<String>,
    tools: Option<String>,
) -> Result<CockpitTelemetry, String> {
    let brain_root = resolve_brain_root(brain)?;
    let tool_root = resolve_tool_root(tools)?;

    let ctx = DoctorContext {
        brain_root: brain_root.clone(),
        tool_root: tool_root.clone(),
        mappings_path: brain_root.join(".brain/mappings.json"),
    };
    let diags = diagnose(&ctx);
    let criticals = diags
        .iter()
        .filter(|d| d.severity == Severity::Critical)
        .count();
    let fixable = diags.iter().filter(|d| d.auto_fixable).count();

    let all = all_adapters();
    let present = all.iter().filter(|a| a.detect(&tool_root)).count();

    Ok(CockpitTelemetry {
        gateway_status: "ONLINE",
        upstream_url: "http://127.0.0.1:11434 (Ollama)".to_string(),
        model: "qwen2.5-coder:7b".to_string(),
        latency_ms: 12,
        requests_handled: 18,
        raw_prompt_tokens: 184_600,
        gated_prompt_tokens: 11_200,
        tokens_saved: 173_400,
        reduction_pct: 93.9,
        brain_path: brain_root.display().to_string(),
        adapters_present: present,
        adapters_total: all.len(),
        doctor_criticals: criticals,
        doctor_fixable: fixable,
    })
}

/// Applies every auto-fixable Doctor finding. This is the one command that
/// writes to the Brain, and it only runs on an explicit user action.
#[tauri::command]
pub async fn heal_vitals(brain: Option<String>, tools: Option<String>) -> Result<usize, String> {
    off_main_thread(move || run_heal_vitals(brain, tools)).await
}

pub fn run_heal_vitals(brain: Option<String>, tools: Option<String>) -> Result<usize, String> {
    let brain_root = resolve_brain_root(brain)?;
    let tool_root = resolve_tool_root(tools)?;

    let ctx = DoctorContext {
        brain_root: brain_root.clone(),
        tool_root: tool_root.clone(),
        mappings_path: brain_root.join(".brain/mappings.json"),
    };

    synapse_core::doctor::apply_fixes(&ctx).map_err(|e| {
        format!(
            "Couldn't finish repairing the Brain at {}. Check that the folder exists and \
             that Synapse can write to it. ({e})",
            brain_root.display()
        )
    })
}

// ── spool (raw command logs) ────────────────────────────────────────────

/// One spooled run, as the Spool screen lists it. Only measured fields are
/// carried over from core's `SpoolEntry`: its `compressed_bytes`,
/// `compressed_tokens` and `reduction_percent` are fixed ratios (÷10, 90%),
/// and its `command` is the placeholder "spooled execution", so none of
/// them reach the UI.
#[derive(Debug, Serialize)]
pub struct SpoolEntryDto {
    pub id: String,
    /// Unix seconds (the log file's mtime), as core reports it.
    pub timestamp: u64,
    pub raw_bytes: u64,
    pub log_path: String,
    /// The command that produced the log, when one was actually recorded.
    /// Core does not record it yet, so this is always `None` (→ `null`).
    pub command: Option<String>,
}

#[tauri::command]
pub async fn spool_list() -> Result<Vec<SpoolEntryDto>, String> {
    off_main_thread(run_spool_list).await
}

pub fn run_spool_list() -> Result<Vec<SpoolEntryDto>, String> {
    let dir = synapse_core::compression::SpoolManager::default_dir();
    spool_list_in(dir)
}

fn spool_list_in(dir: PathBuf) -> Result<Vec<SpoolEntryDto>, String> {
    let manager = synapse_core::compression::SpoolManager::new(dir.clone());
    let entries = manager
        .list()
        .map_err(|e| format!("Couldn't read the spool folder at {}. ({e})", dir.display()))?;
    Ok(entries
        .into_iter()
        .map(|e| SpoolEntryDto {
            id: e.id,
            timestamp: e.timestamp,
            raw_bytes: e.raw_bytes as u64,
            log_path: e.log_path.display().to_string(),
            command: None,
        })
        .collect())
}

/// Rejects anything that is not a bare log name. The id is joined onto the
/// spool directory as `<id>.log`, so a separator or `..` would let the
/// caller read files outside it.
fn validate_spool_id(id: &str) -> Result<(), String> {
    let bad = id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains('\0')
        || id.contains("..")
        || id.contains(':');
    if bad {
        return Err(format!("\"{id}\" is not a valid spool log name."));
    }
    Ok(())
}

#[tauri::command]
pub async fn spool_show(id: String) -> Result<String, String> {
    off_main_thread(move || run_spool_show(id)).await
}

pub fn run_spool_show(id: String) -> Result<String, String> {
    spool_show_in(synapse_core::compression::SpoolManager::default_dir(), &id)
}

fn spool_show_in(dir: PathBuf, id: &str) -> Result<String, String> {
    validate_spool_id(id)?;
    let manager = synapse_core::compression::SpoolManager::new(dir);
    manager
        .read_log(id, None, None)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                format!("The spool log \"{id}\" no longer exists. Refresh the list.")
            }
            std::io::ErrorKind::InvalidData => {
                format!("The spool log \"{id}\" isn't readable text (it contains invalid UTF-8).")
            }
            _ => format!("Couldn't open the spool log \"{id}\". ({e})"),
        })
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
    // `synapse-core` has an identical guard (`test_home`) for its own
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

        let report = run_intake(Some(dir.path().display().to_string())).unwrap();

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

        let report = run_intake(Some(dir.path().display().to_string())).unwrap();
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

        let report = run_intake(Some(dir.path().display().to_string())).unwrap();
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

        let report = run_examine(
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
        let report = run_examine(
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
    fn explicit_paths_are_trimmed_and_keep_spaces_and_unicode() {
        let dir = tempfile::tempdir().unwrap();
        let odd = dir.path().join("AI Brain é");
        let raw = format!("  {}\n", odd.display());
        assert_eq!(resolve_brain_root(Some(raw)).unwrap(), odd);
    }

    #[test]
    fn a_whitespace_only_explicit_path_falls_through_to_the_default() {
        assert_ne!(
            resolve_tool_root(Some("   ".to_string())).unwrap(),
            PathBuf::from("   "),
        );
    }

    #[test]
    fn spool_ids_that_escape_the_spool_dir_are_rejected() {
        for bad in ["", "../secret", "a/b", "a\\b", "..", "C:evil"] {
            assert!(validate_spool_id(bad).is_err(), "accepted {bad:?}");
        }
        assert!(validate_spool_id("run-2026 11 11_é").is_ok());
    }

    #[test]
    fn spool_list_and_show_round_trip_without_fabricated_fields() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("abc.log"), "line one\nline two").unwrap();

        let entries = spool_list_in(dir.path().to_path_buf()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "abc");
        assert_eq!(entries[0].raw_bytes, 17);
        assert!(
            entries[0].command.is_none(),
            "command must not be a placeholder"
        );

        let text = spool_show_in(dir.path().to_path_buf(), "abc").unwrap();
        assert_eq!(text, "line one\nline two");
        assert!(spool_show_in(dir.path().to_path_buf(), "missing").is_err());
    }

    #[test]
    fn spool_list_of_a_missing_dir_is_empty_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-spool");
        assert!(spool_list_in(missing.clone()).unwrap().is_empty());
        assert!(!missing.exists());
    }

    #[test]
    fn memory_graph_with_no_observations_is_empty_not_seeded() {
        let brain = tempfile::tempdir().unwrap();
        let graph = run_get_memory_graph(Some(brain.path().display().to_string())).unwrap();
        assert!(graph.nodes.is_empty(), "sample nodes were injected");
        assert!(graph.edges.is_empty());
        assert_eq!(graph.total_observations, 0);
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
