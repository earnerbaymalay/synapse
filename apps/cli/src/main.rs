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

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { json } => match std::env::current_dir() {
            Ok(root) => report_scan(&root, json),
            Err(e) => {
                eprintln!("neurosurgeon scan: failed to read current directory: {e}");
                ExitCode::FAILURE
            }
        },
        Command::Import { dry_run } => {
            if !dry_run {
                eprintln!(
                    "neurosurgeon import: only --dry-run is implemented so far — \
                     writing into the Brain is Phase 4 scope, see PLAN.md T4.x"
                );
                return ExitCode::FAILURE;
            }
            match std::env::current_dir() {
                Ok(root) => report_import_dry_run(&root),
                Err(e) => {
                    eprintln!("neurosurgeon import: failed to read current directory: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Project { dry_run } => {
            not_yet_implemented("project", &format!("dry_run={dry_run}"))
        }
        Command::Sync { once } => not_yet_implemented("sync", &format!("once={once}")),
        Command::Doctor {
            fix,
            brain,
            tool_root,
        } => {
            let brain_root = match resolve_brain_root(brain) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("neurosurgeon doctor: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let tool_root = match resolve_tool_root(tool_root) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("neurosurgeon doctor: {e}");
                    return ExitCode::FAILURE;
                }
            };
            run_doctor(&brain_root, &tool_root, fix)
        }
        Command::Snapshot { message } => {
            not_yet_implemented("snapshot", &format!("message={message:?}"))
        }
        Command::Rollback { snapshot } => {
            not_yet_implemented("rollback", &format!("snapshot={snapshot}"))
        }
    }
}

/// Detects which of the 12 registered adapters' config files are present
/// under `root` — the `cli scan` half of T3.4 Gate 2.
fn report_scan(root: &Path, json: bool) -> ExitCode {
    let detected: Vec<&'static str> = all_adapters()
        .iter()
        .filter(|a| a.detect(root))
        .map(|a| a.id())
        .collect();

    if json {
        let value = serde_json::json!({
            "root": root.display().to_string(),
            "detected": detected,
        });
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    } else if detected.is_empty() {
        println!(
            "No supported AI tool configs detected under {}",
            root.display()
        );
    } else {
        println!(
            "Detected {} tool(s) under {}:",
            detected.len(),
            root.display()
        );
        for id in &detected {
            println!("  - {id}");
        }
    }

    ExitCode::SUCCESS
}

/// Runs every detected adapter's `import()` against `root` and prints what
/// it would bring into the Brain, without writing anything. Full (non-dry
/// -run) import — actually persisting into the Brain directory — is Phase 4
/// scope; no such write path exists in `packages/core` yet. Per
/// MASTER_PROMPT.md's safety rule ("dry-run is the default for the first
/// merge"), this is the only mode `cli import` supports today, and it never
/// touches the filesystem.
fn report_import_dry_run(root: &Path) -> ExitCode {
    println!(
        "Dry run — nothing will be written. Migration report for {}:",
        root.display()
    );

    let mut had_error = false;
    let mut any_detected = false;

    for adapter in all_adapters() {
        if !adapter.detect(root) {
            continue;
        }
        any_detected = true;

        match adapter.import(root) {
            Ok(result) => {
                println!(
                    "  {}: {} skill(s), {} agent(s), {} mcp server(s)",
                    adapter.id(),
                    result.skills.len(),
                    result.agents.len(),
                    result.mcp_servers.len()
                );
                for skill in &result.skills {
                    println!("    skill  {} (sha256 {})", skill.id, skill.sha256);
                }
                for agent in &result.agents {
                    println!("    agent  {}", agent.slug);
                }
                for server in &result.mcp_servers {
                    println!("    mcp    {}", server.id);
                }
            }
            Err(e) => {
                eprintln!("  {}: import failed: {}", adapter.id(), e);
                had_error = true;
            }
        }
    }

    if !any_detected {
        println!("  (no supported AI tool configs detected — nothing to import)");
    }

    if had_error {
        ExitCode::FAILURE
    } else {
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
    if let Some(env) = std::env::var_os("NEUROSURGEON_BRAIN") {
        return Ok(PathBuf::from(env));
    }
    dirs::home_dir()
        .map(|h| h.join("AIBrain"))
        .ok_or_else(|| "cannot locate a home directory; pass --brain <PATH>".to_string())
}

/// Resolves the tool config root that projection paths are relative to.
/// Precedence: `--tool-root`, then `$NEUROSURGEON_TOOL_ROOT`, then the home
/// directory (tool configs like `.cursor/…` live under `$HOME`).
fn resolve_tool_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Some(env) = std::env::var_os("NEUROSURGEON_TOOL_ROOT") {
        return Ok(PathBuf::from(env));
    }
    dirs::home_dir()
        .ok_or_else(|| "cannot locate a home directory; pass --tool-root <PATH>".to_string())
}

/// Runs the Doctor rule library against `brain_root`/`tool_root` and prints
/// a clinical report. With `fix`, applies every auto-fixable diagnosis and
/// re-diagnoses so the report reflects the post-fix state. Exit code is
/// FAILURE if any Critical diagnosis remains unresolved (usable in scripts),
/// SUCCESS otherwise.
fn run_doctor(brain_root: &Path, tool_root: &Path, fix: bool) -> ExitCode {
    let ctx = DoctorContext {
        brain_root: brain_root.to_path_buf(),
        tool_root: tool_root.to_path_buf(),
        mappings_path: brain_root.join(".brain/mappings.json"),
    };

    if fix {
        match apply_fixes(&ctx) {
            Ok(0) => println!("Doctor: nothing to fix."),
            Ok(n) => println!("Doctor: applied {n} fix(es)."),
            Err(e) => {
                eprintln!("neurosurgeon doctor: fix failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let diagnoses = diagnose(&ctx);
    if diagnoses.is_empty() {
        println!(
            "Doctor: clean bill of health — no issues found in {}.",
            brain_root.display()
        );
        return ExitCode::SUCCESS;
    }

    println!("Doctor examined {}:", brain_root.display());
    let mut criticals = 0;
    for d in &diagnoses {
        let tag = match d.severity {
            Severity::Critical => {
                criticals += 1;
                "CRITICAL"
            }
            Severity::Warning => "WARNING ",
            Severity::Info => "INFO    ",
        };
        let hint = if d.auto_fixable && !fix {
            "  (fixable — rerun with --fix)"
        } else {
            ""
        };
        match &d.subject {
            Some(s) => println!("  [{tag}] {} — {}{}", d.message, s, hint),
            None => println!("  [{tag}] {}{}", d.message, hint),
        }
    }

    if criticals > 0 {
        eprintln!("\n{criticals} critical issue(s) need a human — see the report above.");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The Brain-writing side of `import`/`project`/`sync`, and git-backed
/// `snapshot`/`rollback`, are Phase 3/4 scope not yet landed.
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
