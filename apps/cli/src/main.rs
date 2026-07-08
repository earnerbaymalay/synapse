use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
        Command::Scan { json } => not_yet_implemented("scan", &format!("json={json}")),
        Command::Import { dry_run } => not_yet_implemented("import", &format!("dry_run={dry_run}")),
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

/// `packages/core` (Phase 3) has not landed yet, so every verb is wired up
/// and argument-complete but not yet backed by the scanner/adapter engine.
fn not_yet_implemented(verb: &str, args: &str) -> ExitCode {
    eprintln!(
        "neurosurgeon {verb}: not yet implemented ({args}) — see PLAN.md Phase 3/4"
    );
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
        for verb in ["scan", "import", "project", "sync", "doctor", "snapshot", "rollback"] {
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
}
