use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use neurosurgeon_core::adapters::all_adapters;

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
        Command::Doctor { fix } => not_yet_implemented("doctor", &format!("fix={fix}")),
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

/// The Brain-writing side of `import`/`project`/`sync`/`doctor`, and
/// git-backed `snapshot`/`rollback`, are Phase 3/4 scope not yet landed.
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
}
