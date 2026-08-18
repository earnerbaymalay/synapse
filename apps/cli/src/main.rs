mod chart;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use neurosurgeon_core::adapters::all_adapters;
use neurosurgeon_core::doctor::{apply_fixes, diagnose, DoctorContext, Severity};

/// LLM Neurosurgeon — scan, import, project, and sync AI tool configs
/// through one canonical Brain.
#[derive(Debug, Parser)]
#[command(name = "neurosurgeon", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Detect installed AI tools and the config files they own
    Scan {
        /// Emit machine-readable JSON instead of a human report
        #[arg(long)]
        json: bool,
    },
    /// Import detected configs into the canonical Brain
    Import {
        /// Print the migration report without writing anything (default for the first run)
        #[arg(long)]
        dry_run: bool,
    },
    /// Project the Brain back out to every linked tool
    Project {
        /// Print what would be written without touching any files
        #[arg(long)]
        dry_run: bool,
    },
    /// Run one import + project pass and resolve or queue conflicts
    Sync {
        /// Run once and exit instead of starting the watcher/scheduler
        #[arg(long)]
        once: bool,
    },
    /// Diagnose Brain/tool drift and explain (or apply) fixes
    Doctor {
        /// Apply the suggested fix for every diagnosis instead of just reporting
        #[arg(long)]
        fix: bool,
        /// Brain directory to examine (defaults to $NEUROSURGEON_BRAIN, else ~/AIBrain)
        #[arg(long, value_name = "PATH")]
        brain: Option<PathBuf>,
        /// Tool config root that projections are relative to (defaults to $NEUROSURGEON_TOOL_ROOT, else your home directory)
        #[arg(long, value_name = "PATH")]
        tool_root: Option<PathBuf>,
    },
    /// Record a git snapshot of the current Brain state
    Snapshot {
        /// Optional message describing this snapshot
        message: Option<String>,
    },
    /// Restore the Brain to a prior snapshot
    Rollback {
        /// Snapshot id or git ref to restore
        snapshot: String,
    },
}

use neurosurgeon_core::snapshot::{rollback, snapshot};
use neurosurgeon_core::sync::{
    perform_import, perform_project, perform_sync, SyncLock, SyncOutcome,
};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { json } => match resolve_tool_root(None) {
            Ok(root) => report_scan(&root, json),
            Err(e) => {
                chart::fault("intake", &e.to_string(), None);
                ExitCode::FAILURE
            }
        },
        Command::Import { dry_run } => {
            let root = match resolve_tool_root(None) {
                Ok(r) => r,
                Err(e) => {
                    chart::fault("intake", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            if dry_run {
                report_import_dry_run(&root)
            } else {
                let brain_root = match resolve_brain_root(None) {
                    Ok(b) => b,
                    Err(e) => {
                        chart::fault("intake", &e.to_string(), None);
                        return ExitCode::FAILURE;
                    }
                };
                match perform_import(&root, &brain_root) {
                    Ok(paths) => {
                        chart::open("intake", &chart::plural(paths.len(), "artifact"));
                        chart::field("Site", &root.display().to_string());
                        chart::field("Brain", &brain_root.display().to_string());
                        println!();
                        for path in &paths {
                            chart::row(chart::Mark::Present, "written", path);
                        }
                        chart::close(
                            &format!(
                                "{} now in the Brain.",
                                chart::plural(paths.len(), "artifact"),
                            ),
                            Some("neurosurgeon snapshot \"after import\""),
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        chart::fault("intake", &e.to_string(), Some("neurosurgeon doctor"));
                        ExitCode::FAILURE
                    }
                }
            }
        }
        Command::Project { dry_run } => {
            let brain_root = match resolve_brain_root(None) {
                Ok(b) => b,
                Err(e) => {
                    chart::fault("graft", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            let tool_root = match resolve_tool_root(None) {
                Ok(t) => t,
                Err(e) => {
                    chart::fault("graft", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            if dry_run {
                chart::open("graft · dry run", "nothing will be written");
                chart::field("Brain", &brain_root.display().to_string());
                chart::field("Tools", &tool_root.display().to_string());
                chart::close(
                    "Dry run only — no file was touched.",
                    Some("neurosurgeon project"),
                );
                ExitCode::SUCCESS
            } else {
                match perform_project(&brain_root, &tool_root) {
                    Ok(paths) => {
                        chart::open("graft", &chart::plural(paths.len(), "file"));
                        chart::field("Brain", &brain_root.display().to_string());
                        chart::field("Tools", &tool_root.display().to_string());
                        println!();
                        for path in &paths {
                            chart::row(chart::Mark::Present, "written", path);
                        }
                        chart::close(
                            &format!(
                                "{} projected out of the Brain.",
                                chart::plural(paths.len(), "file"),
                            ),
                            Some("neurosurgeon doctor"),
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        chart::fault("graft", &e.to_string(), Some("neurosurgeon doctor"));
                        ExitCode::FAILURE
                    }
                }
            }
        }
        Command::Sync { once: _ } => {
            let brain_root = match resolve_brain_root(None) {
                Ok(b) => b,
                Err(e) => {
                    chart::fault("circulation", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            let tool_root = match resolve_tool_root(None) {
                Ok(t) => t,
                Err(e) => {
                    chart::fault("circulation", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };

            let _lock = match SyncLock::acquire(&brain_root) {
                Ok(l) => l,
                Err(e) => {
                    chart::fault(
                        "circulation",
                        &format!("could not acquire the Brain lock: {e}"),
                        Some("check whether another neurosurgeon is running"),
                    );
                    return ExitCode::FAILURE;
                }
            };

            // Hold lock briefly to ensure concurrent processes collide deterministically
            std::thread::sleep(std::time::Duration::from_millis(50));

            match perform_sync(&brain_root, &tool_root) {
                Ok(SyncOutcome::NoChanges) => {
                    chart::open("circulation", "no drift");
                    chart::field("Brain", &brain_root.display().to_string());
                    chart::field("Tools", &tool_root.display().to_string());
                    println!();
                    chart::row(
                        chart::Mark::Present,
                        "brain",
                        "already in sync with every tool",
                    );
                    chart::close("Nothing to do.", Some("neurosurgeon doctor"));
                    ExitCode::SUCCESS
                }
                Ok(SyncOutcome::Applied { changed_paths }) => {
                    chart::open("circulation", &chart::plural(changed_paths.len(), "change"));
                    chart::field("Brain", &brain_root.display().to_string());
                    chart::field("Tools", &tool_root.display().to_string());
                    println!();
                    for path in &changed_paths {
                        chart::row(chart::Mark::Present, "updated", path);
                    }
                    chart::close(
                        &format!("{} applied.", chart::plural(changed_paths.len(), "change")),
                        Some("neurosurgeon snapshot \"after sync\""),
                    );
                    ExitCode::SUCCESS
                }
                Ok(SyncOutcome::ConflictQueued { conflict_ids }) => {
                    // Printed as a chart on stdout, not stderr: a queued
                    // conflict is a finding about the Brain, not a crash.
                    chart::open(
                        "circulation",
                        &chart::plural(conflict_ids.len(), "conflict"),
                    );
                    chart::field("Brain", &brain_root.display().to_string());
                    chart::field(
                        "Queue",
                        &brain_root
                            .join(".brain/conflicts.json")
                            .display()
                            .to_string(),
                    );
                    println!();
                    for id in &conflict_ids {
                        chart::row(
                            chart::Mark::Critical,
                            id,
                            "both sides changed — queued for review",
                        );
                    }
                    chart::close(
                        &format!(
                            "{} need a human. Nothing was overwritten.",
                            chart::plural(conflict_ids.len(), "conflict"),
                        ),
                        None,
                    );
                    ExitCode::FAILURE
                }
                Err(e) => {
                    chart::fault("circulation", &e.to_string(), Some("neurosurgeon doctor"));
                    ExitCode::FAILURE
                }
            }
        }
        Command::Doctor {
            fix,
            brain,
            tool_root,
        } => {
            let brain_root = match resolve_brain_root(brain) {
                Ok(p) => p,
                Err(e) => {
                    chart::fault("examination", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            let tool_root = match resolve_tool_root(tool_root) {
                Ok(p) => p,
                Err(e) => {
                    chart::fault("examination", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            run_doctor(&brain_root, &tool_root, fix)
        }
        Command::Snapshot { message } => {
            let brain_root = match resolve_brain_root(None) {
                Ok(b) => b,
                Err(e) => {
                    chart::fault("imaging", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            let msg = message.as_deref().unwrap_or("Manual snapshot");
            match snapshot(&brain_root, msg) {
                Ok(sha) => {
                    chart::open("imaging", "snapshot recorded");
                    chart::field("Brain", &brain_root.display().to_string());
                    chart::field("Note", msg);
                    println!();
                    chart::row(chart::Mark::Present, "snapshot", &sha);
                    chart::close(
                        "The Brain can be returned to this state.",
                        Some(&format!("neurosurgeon rollback {sha}")),
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    chart::fault("imaging", &e.to_string(), None);
                    ExitCode::FAILURE
                }
            }
        }
        Command::Rollback {
            snapshot: snapshot_ref,
        } => {
            let brain_root = match resolve_brain_root(None) {
                Ok(b) => b,
                Err(e) => {
                    chart::fault("reversal", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            match rollback(&brain_root, &snapshot_ref) {
                Ok(sha) => {
                    chart::open("reversal", "brain restored");
                    chart::field("Brain", &brain_root.display().to_string());
                    chart::field("To", &snapshot_ref);
                    println!();
                    chart::row(chart::Mark::Present, "restored", &sha);
                    chart::close(
                        "Tools still hold the old projection.",
                        Some("neurosurgeon project"),
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    chart::fault("reversal", &e.to_string(), None);
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// Charts which of the registered adapters are present under `root`.
///
/// Per IDENTITY.md every adapter gets a row, including the ones that are not
/// installed: absence is a finding, not an empty table. `--json` bypasses the
/// chart entirely and emits only the detected ids, so scripts keep a stable
/// contract.
fn report_scan(root: &Path, json: bool) -> ExitCode {
    let adapters = all_adapters();
    let mut detected: Vec<&'static str> = Vec::new();
    let mut findings: Vec<(&'static str, bool)> = Vec::new();

    for adapter in adapters.iter() {
        let present = adapter.detect(root);
        if present {
            detected.push(adapter.id());
        }
        findings.push((adapter.id(), present));
    }

    if json {
        let value = serde_json::json!({
            "root": root.display().to_string(),
            "detected": detected,
        });
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
        return ExitCode::SUCCESS;
    }

    // Present tools first, then the absent ones, alphabetical within each
    // group: the reader's question is "what did you find", and the negative
    // findings are context underneath it rather than noise interleaved
    // through it.
    findings.sort_by_key(|(id, present)| (!*present, *id));

    let total = findings.len();
    chart::open(
        "intake",
        &format!("{} of {} present", detected.len(), total),
    );
    chart::field("Site", &root.display().to_string());
    println!();

    for (id, present) in &findings {
        if *present {
            chart::row(chart::Mark::Present, id, "config detected");
        } else {
            chart::row(
                chart::Mark::Absent,
                id,
                &chart::paint(chart::Paint::InkSoft, "not present"),
            );
        }
    }

    let finding = format!("{} of {} supported tools present.", detected.len(), total);
    let next = if detected.is_empty() {
        None
    } else {
        Some("neurosurgeon import --dry-run")
    };
    chart::close(&finding, next);

    ExitCode::SUCCESS
}

/// Charts what a real import would bring into the Brain, without writing.
///
/// Every row is measured by actually running the adapter's `import()` against
/// `root` — nothing here is estimated or placeholdered. The chart closes by
/// restating that nothing was written and naming the command that would.
fn report_import_dry_run(root: &Path) -> ExitCode {
    let mut had_error = false;
    let mut detected = 0usize;
    let mut skills = 0usize;
    let mut agents = 0usize;
    let mut servers = 0usize;

    chart::open("intake · dry run", "nothing will be written");
    chart::field("Site", &root.display().to_string());
    println!();

    for adapter in all_adapters() {
        if !adapter.detect(root) {
            continue;
        }
        detected += 1;

        match adapter.import(root) {
            Ok(result) => {
                skills += result.skills.len();
                agents += result.agents.len();
                servers += result.mcp_servers.len();

                chart::row(
                    chart::Mark::Present,
                    adapter.id(),
                    &format!(
                        "{}  {}  {}",
                        chart::plural(result.skills.len(), "skill"),
                        chart::plural(result.agents.len(), "agent"),
                        chart::plural(result.mcp_servers.len(), "mcp server"),
                    ),
                );
                for skill in &result.skills {
                    chart::detail(&format!("skill  {}  {}", skill.id, skill.sha256));
                }
                for agent in &result.agents {
                    chart::detail(&format!("agent  {}", agent.slug));
                }
                for server in &result.mcp_servers {
                    chart::detail(&format!("mcp    {}", server.id));
                }
            }
            Err(e) => {
                had_error = true;
                chart::row(
                    chart::Mark::Critical,
                    adapter.id(),
                    &format!("import failed: {e}"),
                );
            }
        }
    }

    if detected == 0 {
        chart::row(
            chart::Mark::Absent,
            "(none)",
            "no supported tool configs under this site",
        );
        chart::close("Nothing to import.", Some("neurosurgeon scan"));
        return ExitCode::SUCCESS;
    }

    let finding = format!(
        "{} would enter the Brain from {}. Nothing was written.",
        [
            chart::plural(skills, "skill"),
            chart::plural(agents, "agent"),
            chart::plural(servers, "mcp server"),
        ]
        .join(", "),
        chart::plural(detected, "tool"),
    );

    if had_error {
        chart::close(&finding, Some("neurosurgeon doctor"));
        ExitCode::FAILURE
    } else {
        chart::close(&finding, Some("neurosurgeon import"));
        ExitCode::SUCCESS
    }
}

/// Resolves the Brain directory for `doctor`. Precedence: an explicit
/// `--brain` flag, then `$NEUROSURGEON_BRAIN`, then the documented default
/// `~/AIBrain` (see DECISIONS.md / model.rs). Errors only if none of these
/// yield a path (no home directory on a headless account with no override).
fn resolve_brain_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Some(env) = std::env::var_os("NEUROSURGEON_BRAIN_PATH")
        .or_else(|| std::env::var_os("NEUROSURGEON_BRAIN"))
    {
        return Ok(PathBuf::from(env));
    }
    dirs::home_dir()
        .map(|h| h.join("AIBrain"))
        .ok_or_else(|| "cannot locate a home directory; pass --brain <PATH>".to_string())
}

/// Resolves the tool config root that projection paths are relative to.
/// Precedence: `--tool-root`, then `$NEUROSURGEON_WORKSPACE_PATH` / `$NEUROSURGEON_TOOL_ROOT`, then home.
fn resolve_tool_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Some(env) = std::env::var_os("NEUROSURGEON_WORKSPACE_PATH")
        .or_else(|| std::env::var_os("NEUROSURGEON_TOOL_ROOT"))
    {
        return Ok(PathBuf::from(env));
    }
    if let Ok(cur) = std::env::current_dir() {
        return Ok(cur);
    }
    dirs::home_dir()
        .ok_or_else(|| "cannot locate a home directory; pass --tool-root <PATH>".to_string())
}

/// Runs the Doctor rule library and charts the result as a clinical record.
///
/// With `fix`, auto-fixable diagnoses are applied first and the chart then
/// reflects the post-fix state — so the record always describes the Brain as
/// it stands now, not as it was on entry. Exit code is FAILURE while any
/// Critical diagnosis remains, which is what makes `doctor` usable as a CI
/// gate.
fn run_doctor(brain_root: &Path, tool_root: &Path, fix: bool) -> ExitCode {
    let ctx = DoctorContext {
        brain_root: brain_root.to_path_buf(),
        tool_root: tool_root.to_path_buf(),
        mappings_path: brain_root.join(".brain/mappings.json"),
    };

    let mut applied = None;
    if fix {
        match apply_fixes(&ctx) {
            Ok(n) => applied = Some(n),
            Err(e) => {
                chart::fault("examination", &format!("fix failed: {e}"), None);
                return ExitCode::FAILURE;
            }
        }
    }

    let diagnoses = diagnose(&ctx);
    let criticals = diagnoses
        .iter()
        .filter(|d| d.severity == Severity::Critical)
        .count();
    let warnings = diagnoses
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let fixable = diagnoses.iter().filter(|d| d.auto_fixable).count();

    let context = if diagnoses.is_empty() {
        "no findings".to_string()
    } else {
        chart::plural(diagnoses.len(), "finding")
    };
    chart::open("examination", &context);
    chart::field("Brain", &brain_root.display().to_string());
    chart::field("Tools", &tool_root.display().to_string());

    if let Some(n) = applied {
        chart::field(
            "Fixed",
            &chart::plural(n, "diagnosis").replace("diagnosiss", "diagnoses"),
        );
    }
    println!();

    if diagnoses.is_empty() {
        chart::row(chart::Mark::Present, "brain", "no drift, no faults");
        chart::close("Clean bill of health.", Some("neurosurgeon sync --once"));
        return ExitCode::SUCCESS;
    }

    for d in &diagnoses {
        let mark = match d.severity {
            Severity::Critical => chart::Mark::Critical,
            Severity::Warning => chart::Mark::Warning,
            Severity::Info => chart::Mark::Partial,
        };
        let raw = d.subject.as_deref().unwrap_or("brain");
        let subject = chart::abbreviate(raw, &[("brain", brain_root), ("tools", tool_root)]);
        chart::row(mark, &subject, &d.message);
        if d.auto_fixable && !fix {
            chart::detail("fixable — rerun with --fix");
        }
    }

    let finding = if criticals > 0 {
        format!(
            "{} need a human. {} can wait.",
            chart::plural(criticals, "critical finding"),
            chart::plural(warnings, "warning"),
        )
    } else {
        format!(
            "No critical findings. {} noted.",
            chart::plural(diagnoses.len(), "observation"),
        )
    };

    let next = if fixable > 0 && !fix {
        Some("neurosurgeon doctor --fix")
    } else if criticals > 0 {
        None
    } else {
        Some("neurosurgeon sync --once")
    };
    chart::close(&finding, next);

    if criticals > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The Brain-writing side of `import`/`project`/`sync`, and git-backed
/// `snapshot`/`rollback`, are Phase 3/4 scope not yet landed.
#[allow(dead_code)]
fn not_yet_implemented(verb: &str, args: &str) -> ExitCode {
    eprintln!("neurosurgeon {verb}: not yet implemented ({args}) — see PLAN.md Phase 3/4");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_structure_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn help_lists_every_verb() {
        let help = Cli::command().render_long_help().to_string();
        for verb in [
            "scan", "import", "project", "sync", "doctor", "snapshot", "rollback",
        ] {
            assert!(help.contains(verb), "--help is missing verb: {verb}");
        }
    }

    #[test]
    fn parses_each_verb() {
        assert!(Cli::try_parse_from(["neurosurgeon", "scan"]).is_ok());
        assert!(Cli::try_parse_from(["neurosurgeon", "import", "--dry-run"]).is_ok());
        assert!(Cli::try_parse_from(["neurosurgeon", "project"]).is_ok());
        assert!(Cli::try_parse_from(["neurosurgeon", "sync", "--once"]).is_ok());
        assert!(Cli::try_parse_from(["neurosurgeon", "doctor", "--fix"]).is_ok());
        assert!(Cli::try_parse_from(["neurosurgeon", "snapshot", "before upgrade"]).is_ok());
        assert!(Cli::try_parse_from(["neurosurgeon", "rollback", "abc123"]).is_ok());
    }

    #[test]
    fn rejects_unknown_verb() {
        assert!(Cli::try_parse_from(["neurosurgeon", "frobnicate"]).is_err());
    }

    #[test]
    fn rollback_requires_a_snapshot_argument() {
        assert!(Cli::try_parse_from(["neurosurgeon", "rollback"]).is_err());
    }

    #[test]
    fn report_scan_succeeds_on_empty_root() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(report_scan(dir.path(), false), ExitCode::SUCCESS);
        assert_eq!(report_scan(dir.path(), true), ExitCode::SUCCESS);
    }

    #[test]
    fn report_scan_detects_a_known_tool() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".clinerules"), "test rules").unwrap();
        assert_eq!(report_scan(dir.path(), false), ExitCode::SUCCESS);
    }

    #[test]
    fn report_import_dry_run_succeeds_on_empty_root() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(report_import_dry_run(dir.path()), ExitCode::SUCCESS);
    }

    #[test]
    fn report_import_dry_run_does_not_write_anything() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".clinerules"), "test rules").unwrap();

        let before: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();

        assert_eq!(report_import_dry_run(dir.path()), ExitCode::SUCCESS);

        let after: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();

        assert_eq!(before, after, "dry-run import must not write any files");
    }

    #[test]
    fn resolve_brain_root_prefers_explicit_then_defaults_to_aibrain() {
        // An explicit --brain always wins.
        let explicit = PathBuf::from("/tmp/some-brain");
        assert_eq!(
            resolve_brain_root(Some(explicit.clone())).unwrap(),
            explicit
        );
        // With no override, the default is <home>/AIBrain (when a home exists).
        if let Some(home) = dirs::home_dir() {
            // Only meaningful when the env override is unset in this process.
            if std::env::var_os("NEUROSURGEON_BRAIN").is_none() {
                assert_eq!(resolve_brain_root(None).unwrap(), home.join("AIBrain"));
            }
        }
    }

    #[test]
    fn doctor_reports_without_criticals_and_returns_success() {
        // A fresh, non-git Brain with no mappings: only Warnings/Info, no
        // Critical → the report is informative and the exit code is SUCCESS.
        let brain = tempfile::tempdir().unwrap();
        let tool = tempfile::tempdir().unwrap();
        assert_eq!(
            run_doctor(brain.path(), tool.path(), false),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn doctor_fix_initializes_git_and_mappings() {
        // --fix on a fresh Brain should create the git repo and mappings.json.
        let brain = tempfile::tempdir().unwrap();
        let tool = tempfile::tempdir().unwrap();
        assert_eq!(
            run_doctor(brain.path(), tool.path(), true),
            ExitCode::SUCCESS
        );
        assert!(brain.path().join(".git").is_dir());
        assert!(brain.path().join(".brain/mappings.json").exists());
    }

    #[test]
    fn doctor_returns_failure_on_a_critical_fault() {
        // Seed a mapping whose canonical Brain source doesn't exist →
        // canonical-source-missing (Critical), which the CLI surfaces as a
        // FAILURE exit code so scripts/CI can gate on it.
        use neurosurgeon_core::mappings::{Mapping, MappingsFile};
        use neurosurgeon_core::projector::ProjectionPolicy;

        let brain = tempfile::tempdir().unwrap();
        let tool = tempfile::tempdir().unwrap();
        MappingsFile {
            mappings: vec![Mapping {
                tool_id: "seed".into(),
                canonical_path: "skills/does-not-exist".into(),
                projection_path: ".clinerules".into(),
                policy: ProjectionPolicy::Generate,
                content_sha256: String::new(),
            }],
        }
        .save(&brain.path().join(".brain/mappings.json"))
        .unwrap();

        assert_eq!(
            run_doctor(brain.path(), tool.path(), false),
            ExitCode::FAILURE
        );
    }
}
