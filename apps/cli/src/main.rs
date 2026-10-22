mod chart;
mod mem;
mod onboard;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use synapse_core::adapters::all_adapters;
use synapse_core::doctor::{apply_fixes, diagnose, DoctorContext, Severity};

/// Synapse — scan, import, project, and sync AI tool configs
/// through one canonical Brain.
#[derive(Debug, Parser)]
#[command(name = "synapse", version, about, long_about = None)]
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
        /// Brain directory to examine (defaults to $SYNAPSE_BRAIN, else ~/AIBrain)
        #[arg(long, value_name = "PATH")]
        brain: Option<PathBuf>,
        /// Tool config root that projections are relative to (defaults to $SYNAPSE_TOOL_ROOT, else your home directory)
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
    /// Execute a command and compress its terminal output for AI agents
    Exec {
        /// Compression level: balanced (default), aggressive, or strict
        #[arg(short, long)]
        level: Option<String>,
        /// Custom spool directory for raw logs (defaults to ~/.synapse/spool)
        #[arg(long, value_name = "PATH")]
        spool_dir: Option<PathBuf>,
        /// Command and arguments to execute
        #[arg(trailing_var_arg = true, required = true, value_name = "COMMAND")]
        command: Vec<String>,
    },
    /// Stream stdin through the Synaptic compression filter
    Filter {
        /// Compression level: balanced (default), aggressive, or strict
        #[arg(short, long)]
        level: Option<String>,
        /// Custom spool directory for raw logs
        #[arg(long, value_name = "PATH")]
        spool_dir: Option<PathBuf>,
    },
    /// Inspect or list raw spooled execution logs
    Spool {
        #[command(subcommand)]
        action: SpoolAction,
    },
    /// Start the SYNAPSE local runtime gateway proxy on localhost
    Serve {
        /// Port to bind the local proxy to (default: 8088)
        #[arg(short, long, default_value_t = 8088)]
        port: u16,
        /// Host address to bind to (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Upstream LLM endpoint (default: http://127.0.0.1:11434)
        #[arg(short, long, default_value = "http://127.0.0.1:11434")]
        upstream: String,
        /// Default model name when not specified in request
        #[arg(short, long)]
        model: Option<String>,
        /// Sensory Gating compression level: balanced (default), aggressive, strict, or off
        #[arg(short, long, default_value = "balanced")]
        gate_level: String,
        /// Optional upstream API key (prefer the env var: argv is visible to other users)
        #[arg(long, env = "SYNAPSE_UPSTREAM_API_KEY", hide_env_values = true)]
        api_key: Option<String>,
        /// Fold tool results older than the newest N messages into stubs restorable
        /// via `synapse spool show <id>` (off unless set; try 8)
        #[arg(long, value_name = "N")]
        fold_keep: Option<usize>,
    },
    /// Persistent cross-session memory for AI agents (search, timeline, get, hooks)
    Mem {
        #[command(subcommand)]
        action: mem::MemAction,
    },
    /// Open the live Vital Signs HUD & Terminal Cockpit
    Hud {
        /// Gateway address to connect to (default: 127.0.0.1:8088)
        #[arg(short, long, default_value = "127.0.0.1:8088")]
        gateway: String,
        /// Disable ANSI color formatting
        #[arg(long)]
        no_color: bool,
        /// Refresh interval in seconds (default: 2)
        #[arg(short, long, default_value_t = 2)]
        interval: u64,
        /// Print a single snapshot and exit
        #[arg(long)]
        once: bool,
        /// Brain directory to examine
        #[arg(long, value_name = "PATH")]
        brain: Option<PathBuf>,
        /// Tool config root
        #[arg(long, value_name = "PATH")]
        tool_root: Option<PathBuf>,
    },
    /// Interactive onboarding interview to configure your master prompt (SYNAPSE.md)
    Onboard {
        /// Tech stack (e.g. "Rust + React", "Python", "Go")
        #[arg(long)]
        stack: Option<String>,
        /// Architectural pattern (e.g. "Clean Architecture", "Modular", "Monolith")
        #[arg(long)]
        arch: Option<String>,
        /// Testing rigor (e.g. "Strict TDD", "Assertion-heavy", "Pragmatic")
        #[arg(long)]
        testing: Option<String>,
        /// Agent communication style (e.g. "Terse/Clinical", "Pair Programmer", "Auditor")
        #[arg(long)]
        demeanor: Option<String>,
        /// Brain directory (defaults to $SYNAPSE_BRAIN, else ~/AIBrain)
        #[arg(long, value_name = "PATH")]
        brain: Option<PathBuf>,
        /// Tool config root
        #[arg(long, value_name = "PATH")]
        tool_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum SpoolAction {
    /// List all cached execution logs and token reduction statistics
    List {
        #[arg(long, value_name = "PATH")]
        spool_dir: Option<PathBuf>,
    },
    /// Show raw uncompressed log for a specific execution ID
    Show {
        /// Spool log ID (e.g. 8f9b2a)
        id: String,
        /// Limit output to last N lines
        #[arg(short, long)]
        tail: Option<usize>,
        /// Filter lines matching substring
        #[arg(short, long)]
        grep: Option<String>,
        #[arg(long, value_name = "PATH")]
        spool_dir: Option<PathBuf>,
    },
}

use std::io::{Read, Write};
use synapse_core::compression::{execute_with_compression, CompressionLevel, SpoolManager};
use synapse_core::gateway::{GatewayConfig, GatewayServer, UpstreamKind};
use synapse_core::hud::{format_hud, HudSnapshot};
use synapse_core::snapshot::{rollback, snapshot};
use synapse_core::sync::{perform_import, perform_project, perform_sync, SyncLock, SyncOutcome};

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
                match with_brain_lock(&brain_root, || perform_import(&root, &brain_root)) {
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
                            Some("synapse snapshot \"after import\""),
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        chart::fault("intake", &e.to_string(), Some("synapse doctor"));
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
                    Some("synapse project"),
                );
                ExitCode::SUCCESS
            } else {
                match with_brain_lock(&brain_root, || perform_project(&brain_root, &tool_root)) {
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
                            Some("synapse doctor"),
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        chart::fault("graft", &e.to_string(), Some("synapse doctor"));
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
                        Some("check whether another synapse is running"),
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
                    chart::close("Nothing to do.", Some("synapse doctor"));
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
                        Some("synapse snapshot \"after sync\""),
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
                    chart::fault("circulation", &e.to_string(), Some("synapse doctor"));
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
                        "The Brain is protected and can be returned to this state anytime.",
                        Some("synapse hud"),
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
            let tool_root = match resolve_tool_root(None) {
                Ok(t) => t,
                Err(e) => {
                    chart::fault("reversal", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            // Project straight after restoring: otherwise the next sync sees
            // the tools' newer files as edits and imports them over the
            // rollback.
            let restored = with_brain_lock(&brain_root, || {
                let sha = rollback(&brain_root, &snapshot_ref).map_err(|e| e.to_string())?;
                let projected = perform_project(&brain_root, &tool_root)
                    .map_err(|e| format!("restored to {sha} but projection failed: {e}"))?;
                Ok((sha, projected))
            });
            match restored {
                Ok((sha, projected)) => {
                    chart::open("reversal", "brain restored");
                    chart::field("Brain", &brain_root.display().to_string());
                    chart::field("Tools", &tool_root.display().to_string());
                    chart::field("To", &snapshot_ref);
                    println!();
                    chart::row(chart::Mark::Present, "restored", &sha);
                    chart::row(
                        chart::Mark::Present,
                        "projected",
                        &chart::plural(projected.len(), "file"),
                    );
                    chart::close(
                        "Brain and tools are back at that snapshot.",
                        Some("synapse doctor"),
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    chart::fault("reversal", &e.to_string(), None);
                    ExitCode::FAILURE
                }
            }
        }
        Command::Exec {
            level,
            spool_dir,
            command,
        } => {
            if command.is_empty() {
                chart::fault("exec", "No command specified to execute", None);
                return ExitCode::FAILURE;
            }
            let comp_level = level
                .as_deref()
                .map(parse_level)
                .transpose()
                .unwrap_or_else(|e| {
                    chart::fault("exec", &e, None);
                    std::process::exit(2)
                })
                .unwrap_or_default();

            let bin = &command[0];
            let args = &command[1..];

            match execute_with_compression(bin, args, comp_level, spool_dir.as_deref()) {
                Ok((compressed, status)) => {
                    println!("{}", compressed.text);
                    if status.success() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(status.code().unwrap_or(1) as u8)
                    }
                }
                Err(e) => {
                    chart::fault("exec", &e.to_string(), None);
                    ExitCode::FAILURE
                }
            }
        }
        Command::Filter { level, spool_dir } => {
            let comp_level = level
                .as_deref()
                .map(parse_level)
                .transpose()
                .unwrap_or_else(|e| {
                    chart::fault("filter", &e, None);
                    std::process::exit(2)
                })
                .unwrap_or_default();

            let mut input = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut input) {
                chart::fault("filter", &e.to_string(), None);
                return ExitCode::FAILURE;
            }

            let spool_path = spool_dir.unwrap_or_else(SpoolManager::default_dir);
            let spooler = SpoolManager::new(spool_path);
            match spooler.record("stdin stream", &input, comp_level) {
                Ok(compressed) => {
                    println!("{}", compressed.text);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    chart::fault("filter", &e.to_string(), None);
                    ExitCode::FAILURE
                }
            }
        }
        Command::Spool { action } => match action {
            SpoolAction::List { spool_dir } => {
                let spool_path = spool_dir.unwrap_or_else(SpoolManager::default_dir);
                let spooler = SpoolManager::new(spool_path.clone());
                match spooler.list() {
                    Ok(entries) => {
                        chart::open("spool", &chart::plural(entries.len(), "execution log"));
                        chart::field("Spool Dir", &spool_path.display().to_string());
                        println!();
                        if entries.is_empty() {
                            chart::row(chart::Mark::Absent, "empty", "no spooled logs recorded");
                        } else {
                            for e in entries {
                                let desc = format!(
                                    "{} tokens ({}% reduction) • {}",
                                    e.raw_tokens, e.reduction_percent, e.command
                                );
                                chart::row(chart::Mark::Present, &e.id, &desc);
                            }
                        }
                        chart::close("View full logs with: synapse spool show <id>", None);
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        chart::fault("spool", &e.to_string(), None);
                        ExitCode::FAILURE
                    }
                }
            }
            SpoolAction::Show {
                id,
                tail,
                grep,
                spool_dir,
            } => {
                let spool_path = spool_dir.unwrap_or_else(SpoolManager::default_dir);
                let spooler = SpoolManager::new(spool_path);
                match spooler.read_log(&id, tail, grep.as_deref()) {
                    Ok(content) => {
                        println!("{}", content);
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        chart::fault("spool", &e.to_string(), None);
                        ExitCode::FAILURE
                    }
                }
            }
        },
        Command::Serve {
            port,
            host,
            upstream,
            model,
            gate_level,
            api_key,
            fold_keep,
        } => {
            let comp_level = match parse_level(&gate_level) {
                Ok(l) => l,
                Err(e) => {
                    chart::fault("serve", &e, None);
                    return ExitCode::from(2);
                }
            };
            let upstream_kind = UpstreamKind::from_url_or_name(&upstream);
            let bind_addr = format!("{}:{}", host, port);

            let config = GatewayConfig {
                bind_addr: bind_addr.clone(),
                upstream_url: upstream.clone(),
                upstream_kind,
                default_model: model.clone(),
                gate_level: comp_level,
                api_key,
                fold_keep_recent: fold_keep,
            };

            chart::open("gateway", "runtime server starting");
            chart::field("Bind", &format!("http://{}", bind_addr));
            chart::field("Upstream", &upstream);
            chart::field("Sensory Gate", &format!("{:?}", comp_level));
            if let Some(ref m) = model {
                chart::field("Model", m);
            }
            if let Some(n) = fold_keep {
                chart::field(
                    "Context Fold",
                    &format!("tool results older than last {n} messages"),
                );
            }
            println!();
            chart::row(
                chart::Mark::Present,
                "openai",
                &format!("http://{}/v1/chat/completions", bind_addr),
            );
            chart::row(
                chart::Mark::Present,
                "anthropic",
                &format!("http://{}/v1/messages", bind_addr),
            );
            chart::row(
                chart::Mark::Present,
                "health",
                &format!("http://{}/health", bind_addr),
            );
            println!();
            chart::close("Point Claude Code, AGY CLI, or Cursor to this proxy endpoint. Press Ctrl+C to stop.", None);

            let mut server = GatewayServer::new(config);
            if let Err(e) = server.start_background() {
                chart::fault("gateway", &e, None);
                return ExitCode::FAILURE;
            }
            // Clients authenticate /v1/* with this per-launch token
            // (Authorization: Bearer <token> or x-api-key: <token>).
            match server.token() {
                Some(token) => println!("Gateway token: {token}"),
                None => println!("Gateway token: disabled (SYNAPSE_GATEWAY_NO_TOKEN)"),
            }

            while server.is_running() {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            ExitCode::SUCCESS
        }
        Command::Mem { action } => mem::run(action),
        Command::Hud {
            gateway,
            no_color,
            interval,
            once,
            brain,
            tool_root,
        } => {
            let brain_root = match resolve_brain_root(brain) {
                Ok(b) => b,
                Err(e) => {
                    chart::fault("hud", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            let tool_root = match resolve_tool_root(tool_root) {
                Ok(t) => t,
                Err(e) => {
                    chart::fault("hud", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };

            if once {
                let snap = HudSnapshot::collect(&gateway, &brain_root, &tool_root);
                print!("{}", format_hud(&snap, !no_color));
                ExitCode::SUCCESS
            } else {
                run_interactive_hud(&gateway, &brain_root, &tool_root, interval, !no_color)
            }
        }
        Command::Onboard {
            stack,
            arch,
            testing,
            demeanor,
            brain,
            tool_root,
        } => {
            let brain_root = match resolve_brain_root(brain) {
                Ok(b) => b,
                Err(e) => {
                    chart::fault("onboard", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            let tool_root = match resolve_tool_root(tool_root) {
                Ok(t) => t,
                Err(e) => {
                    chart::fault("onboard", &e.to_string(), None);
                    return ExitCode::FAILURE;
                }
            };
            onboard::run_onboard(
                &brain_root,
                &tool_root,
                onboard::OnboardArgs {
                    stack,
                    arch,
                    testing,
                    demeanor,
                },
            )
        }
    }
}

fn run_interactive_hud(
    gateway: &str,
    brain_root: &Path,
    tool_root: &Path,
    interval: u64,
    use_color: bool,
) -> ExitCode {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use std::time::{Duration, Instant};

    struct RawModeGuard;
    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            print!("\r\n");
            let _ = std::io::stdout().flush();
        }
    }

    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}", e);
        return ExitCode::FAILURE;
    }
    let _guard = RawModeGuard;

    let mut local_gateway: Option<GatewayServer> = None;
    let mut status_msg: Option<String> = None;
    let mut input_mode = false;
    let mut input_buffer = String::new();
    let mut last_render = Instant::now() - Duration::from_secs(interval + 1);
    let poll_interval = Duration::from_millis(50);

    loop {
        if last_render.elapsed() >= Duration::from_secs(interval) || input_mode {
            let snap = HudSnapshot::collect(gateway, brain_root, tool_root);
            let mut rendered = format_hud(&snap, use_color);
            rendered = rendered.replace('\n', "\r\n");
            print!("\x1b[2J\x1b[1;1H{}", rendered);

            if let Some(msg) = &status_msg {
                let bullet = if use_color {
                    "\x1b[1;32m●\x1b[0m"
                } else {
                    "●"
                };
                print!("  {} {}\r\n", bullet, msg);
            }

            if input_mode {
                let cyan = if use_color { "\x1b[1;36m" } else { "" };
                let reset = if use_color { "\x1b[0m" } else { "" };
                let blink = if use_color { "\x1b[5m" } else { "" };
                print!(
                    "  {cyan}COMMAND ❯{reset} {}{blink}█{reset}\r\n",
                    input_buffer
                );
            } else {
                let dim = if use_color { "\x1b[2m" } else { "" };
                let reset = if use_color { "\x1b[0m" } else { "" };
                print!("  {dim}Console: Press ':' to enter a command (e.g. :serve, :doctor --fix, :snapshot <msg>){reset}\r\n");
            }

            let _ = std::io::stdout().flush();
            last_render = Instant::now();
        }

        if event::poll(poll_interval).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    if input_mode {
                        match key.code {
                            KeyCode::Esc => {
                                input_mode = false;
                                input_buffer.clear();
                                status_msg = Some("Command canceled.".to_string());
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Backspace => {
                                input_buffer.pop();
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Enter => {
                                input_mode = false;
                                let cmd = input_buffer.trim().to_string();
                                input_buffer.clear();

                                if cmd.is_empty() {
                                    // no-op
                                } else if cmd == "q" || cmd == "quit" || cmd == "exit" {
                                    break;
                                } else if cmd == "g"
                                    || cmd == "serve"
                                    || cmd == "gateway"
                                    || cmd == "start"
                                {
                                    if let Some(mut gw) = local_gateway.take() {
                                        gw.stop();
                                        status_msg = Some("Gateway proxy stopped.".to_string());
                                    } else {
                                        let is_active =
                                            HudSnapshot::collect(gateway, brain_root, tool_root)
                                                .gateway_active;
                                        if is_active {
                                            status_msg = Some(format!(
                                                "Gateway is already running on http://{}",
                                                gateway
                                            ));
                                        } else {
                                            let config = GatewayConfig {
                                                bind_addr: gateway.to_string(),
                                                upstream_url: "https://api.openai.com/v1"
                                                    .to_string(),
                                                upstream_kind: UpstreamKind::OpenAi,
                                                default_model: None,
                                                gate_level: CompressionLevel::Balanced,
                                                api_key: std::env::var("SYNAPSE_UPSTREAM_API_KEY")
                                                    .or_else(|_| std::env::var("OPENAI_API_KEY"))
                                                    .ok(),
                                                fold_keep_recent: None,
                                            };
                                            let mut srv = GatewayServer::new(config);
                                            match srv.start_background() {
                                                Ok(()) => {
                                                    local_gateway = Some(srv);
                                                    status_msg = Some(format!("Runtime Gateway launched in background on http://{}", gateway));
                                                }
                                                Err(e) => {
                                                    status_msg = Some(format!(
                                                        "Failed to start gateway: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                } else if cmd == "stop" {
                                    if let Some(mut gw) = local_gateway.take() {
                                        gw.stop();
                                        status_msg = Some("Gateway proxy stopped.".to_string());
                                    } else {
                                        status_msg = Some(
                                            "No local gateway running in this HUD session."
                                                .to_string(),
                                        );
                                    }
                                } else if cmd == "s" || cmd == "sync" {
                                    match with_brain_lock(brain_root, || {
                                        perform_sync(brain_root, tool_root)
                                    }) {
                                        Ok(SyncOutcome::Applied { changed_paths }) => {
                                            status_msg = Some(format!(
                                                "Sync: {} applied.",
                                                chart::plural(changed_paths.len(), "change")
                                            ));
                                        }
                                        Ok(SyncOutcome::NoChanges) => {
                                            status_msg = Some(
                                                "Sync: Brain is already in sync with every tool."
                                                    .to_string(),
                                            );
                                        }
                                        Ok(SyncOutcome::ConflictQueued { conflict_ids }) => {
                                            status_msg = Some(format!(
                                                "Sync: {} need human resolution.",
                                                chart::plural(conflict_ids.len(), "conflict")
                                            ));
                                        }
                                        Err(e) => {
                                            status_msg = Some(format!("Sync failed: {}", e));
                                        }
                                    }
                                } else if cmd == "d" || cmd == "doctor" {
                                    let mappings_path =
                                        if brain_root.join(".brain/mappings.json").exists() {
                                            brain_root.join(".brain/mappings.json")
                                        } else {
                                            brain_root.join("mappings.json")
                                        };
                                    let ctx = synapse_core::doctor::DoctorContext {
                                        brain_root: brain_root.to_path_buf(),
                                        tool_root: tool_root.to_path_buf(),
                                        mappings_path,
                                    };
                                    let diags = synapse_core::doctor::diagnose(&ctx);
                                    let criticals = diags
                                        .iter()
                                        .filter(|d| {
                                            d.severity == synapse_core::doctor::Severity::Critical
                                        })
                                        .count();
                                    let warnings = diags
                                        .iter()
                                        .filter(|d| {
                                            d.severity == synapse_core::doctor::Severity::Warning
                                        })
                                        .count();
                                    if diags.is_empty() {
                                        status_msg = Some(
                                            "Doctor: Clean bill of health. No drift or faults."
                                                .to_string(),
                                        );
                                    } else {
                                        status_msg = Some(format!(
                                            "Doctor: {} criticals, {} warnings.",
                                            criticals, warnings
                                        ));
                                    }
                                } else if cmd == "fix" || cmd == "doctor --fix" {
                                    let mappings_path =
                                        if brain_root.join(".brain/mappings.json").exists() {
                                            brain_root.join(".brain/mappings.json")
                                        } else {
                                            brain_root.join("mappings.json")
                                        };
                                    let ctx = synapse_core::doctor::DoctorContext {
                                        brain_root: brain_root.to_path_buf(),
                                        tool_root: tool_root.to_path_buf(),
                                        mappings_path,
                                    };
                                    match synapse_core::doctor::apply_fixes(&ctx) {
                                        Ok(n) => {
                                            status_msg =
                                                Some(format!("Doctor: Fixed {} issues.", n));
                                        }
                                        Err(e) => {
                                            status_msg = Some(format!("Doctor fix error: {}", e));
                                        }
                                    }
                                } else if cmd == "p" || cmd == "project" {
                                    match with_brain_lock(brain_root, || {
                                        perform_project(brain_root, tool_root)
                                    }) {
                                        Ok(paths) => {
                                            status_msg = Some(format!(
                                                "Project: {} projected out of the Brain.",
                                                chart::plural(paths.len(), "file")
                                            ));
                                        }
                                        Err(e) => {
                                            status_msg = Some(format!("Project failed: {}", e));
                                        }
                                    }
                                } else if let Some(rest) = cmd.strip_prefix("snapshot ") {
                                    match snapshot(brain_root, rest.trim()) {
                                        Ok(sha) => {
                                            status_msg =
                                                Some(format!("Snapshot recorded: {}", sha));
                                        }
                                        Err(e) => {
                                            status_msg = Some(format!("Snapshot failed: {}", e));
                                        }
                                    }
                                } else if cmd == "snapshot" {
                                    match snapshot(brain_root, "HUD checkpoint") {
                                        Ok(sha) => {
                                            status_msg =
                                                Some(format!("Snapshot recorded: {}", sha));
                                        }
                                        Err(e) => {
                                            status_msg = Some(format!("Snapshot failed: {}", e));
                                        }
                                    }
                                } else if cmd == "help" {
                                    status_msg = Some("Commands: :sync, :doctor, :fix, :project, :serve, :snapshot <msg>, :quit".to_string());
                                } else {
                                    status_msg = Some(format!(
                                        "Unknown command '{}'. Type :help for commands.",
                                        cmd
                                    ));
                                }

                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break
                            }
                            KeyCode::Char(':') | KeyCode::Char('/') => {
                                input_mode = true;
                                input_buffer.clear();
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Char('g') => {
                                if let Some(mut gw) = local_gateway.take() {
                                    gw.stop();
                                    status_msg = Some("Gateway proxy stopped.".to_string());
                                } else {
                                    let is_active =
                                        HudSnapshot::collect(gateway, brain_root, tool_root)
                                            .gateway_active;
                                    if is_active {
                                        status_msg = Some(format!(
                                            "Gateway is already running on http://{}",
                                            gateway
                                        ));
                                    } else {
                                        let config = GatewayConfig {
                                            bind_addr: gateway.to_string(),
                                            upstream_url: "https://api.openai.com/v1".to_string(),
                                            upstream_kind: UpstreamKind::OpenAi,
                                            default_model: None,
                                            gate_level: CompressionLevel::Balanced,
                                            api_key: std::env::var("SYNAPSE_UPSTREAM_API_KEY")
                                                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                                                .ok(),
                                            fold_keep_recent: None,
                                        };
                                        let mut srv = GatewayServer::new(config);
                                        match srv.start_background() {
                                            Ok(()) => {
                                                local_gateway = Some(srv);
                                                status_msg = Some(format!(
                                                    "Runtime Gateway launched on http://{}",
                                                    gateway
                                                ));
                                            }
                                            Err(e) => {
                                                status_msg =
                                                    Some(format!("Failed to start gateway: {}", e));
                                            }
                                        }
                                    }
                                }
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Char('s') => {
                                match with_brain_lock(brain_root, || {
                                    perform_sync(brain_root, tool_root)
                                }) {
                                    Ok(SyncOutcome::Applied { changed_paths }) => {
                                        status_msg = Some(format!(
                                            "Sync: {} applied.",
                                            chart::plural(changed_paths.len(), "change")
                                        ));
                                    }
                                    Ok(SyncOutcome::NoChanges) => {
                                        status_msg = Some(
                                            "Sync: Brain is already in sync with every tool."
                                                .to_string(),
                                        );
                                    }
                                    Ok(SyncOutcome::ConflictQueued { conflict_ids }) => {
                                        status_msg = Some(format!(
                                            "Sync: {} need human resolution.",
                                            chart::plural(conflict_ids.len(), "conflict")
                                        ));
                                    }
                                    Err(e) => {
                                        status_msg = Some(format!("Sync failed: {}", e));
                                    }
                                }
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Char('d') => {
                                let mappings_path =
                                    if brain_root.join(".brain/mappings.json").exists() {
                                        brain_root.join(".brain/mappings.json")
                                    } else {
                                        brain_root.join("mappings.json")
                                    };
                                let ctx = synapse_core::doctor::DoctorContext {
                                    brain_root: brain_root.to_path_buf(),
                                    tool_root: tool_root.to_path_buf(),
                                    mappings_path,
                                };
                                let diags = synapse_core::doctor::diagnose(&ctx);
                                let criticals = diags
                                    .iter()
                                    .filter(|d| {
                                        d.severity == synapse_core::doctor::Severity::Critical
                                    })
                                    .count();
                                let warnings = diags
                                    .iter()
                                    .filter(|d| {
                                        d.severity == synapse_core::doctor::Severity::Warning
                                    })
                                    .count();
                                if diags.is_empty() {
                                    status_msg = Some(
                                        "Doctor: Clean bill of health. No drift or faults."
                                            .to_string(),
                                    );
                                } else {
                                    status_msg = Some(format!(
                                        "Doctor: {} criticals, {} warnings.",
                                        criticals, warnings
                                    ));
                                }
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            KeyCode::Char('p') => {
                                match with_brain_lock(brain_root, || {
                                    perform_project(brain_root, tool_root)
                                }) {
                                    Ok(paths) => {
                                        status_msg = Some(format!(
                                            "Project: {} projected out of the Brain.",
                                            chart::plural(paths.len(), "file")
                                        ));
                                    }
                                    Err(e) => {
                                        status_msg = Some(format!("Project failed: {}", e));
                                    }
                                }
                                last_render = Instant::now() - Duration::from_secs(interval + 1);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if let Some(mut gw) = local_gateway {
        gw.stop();
    }

    ExitCode::SUCCESS
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
        Some("synapse import --dry-run")
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
        chart::close("Nothing to import.", Some("synapse scan"));
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
        chart::close(&finding, Some("synapse doctor"));
        ExitCode::FAILURE
    } else {
        chart::close(&finding, Some("synapse import"));
        ExitCode::SUCCESS
    }
}

/// Resolves the Brain directory for `doctor`. Precedence: an explicit
/// `--brain` flag, then `$SYNAPSE_BRAIN_PATH` / `$SYNAPSE_BRAIN`, then
/// `~/Desktop/AI BRAIN` if present on disk, else the default `~/AIBrain`.
fn resolve_brain_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Some(env) =
        std::env::var_os("SYNAPSE_BRAIN_PATH").or_else(|| std::env::var_os("SYNAPSE_BRAIN"))
    {
        return Ok(PathBuf::from(env));
    }
    if let Some(home) = dirs::home_dir() {
        let desktop_brain = home.join("Desktop/AI BRAIN");
        if desktop_brain.is_dir() && !home.join("AIBrain").exists() {
            return Ok(desktop_brain);
        }
        return Ok(home.join("AIBrain"));
    }
    Err("cannot locate a home directory; pass --brain <PATH>".to_string())
}

/// Runs a Brain-mutating operation under the same lock `synapse sync`
/// takes, so import/project/rollback and the HUD can't race a sync.
fn with_brain_lock<T>(
    brain_root: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let _lock = SyncLock::acquire(brain_root)
        .map_err(|e| format!("could not acquire the Brain lock: {e}"))?;
    f()
}

/// Parses a gating level, rejecting unknown names instead of silently
/// falling back to Balanced.
fn parse_level(s: &str) -> Result<CompressionLevel, String> {
    CompressionLevel::parse(s).ok_or_else(|| {
        format!("unknown level '{s}' (expected balanced, aggressive, strict, or off)")
    })
}

/// Resolves the tool config root that projection paths are relative to.
/// Precedence: `--tool-root`, then `$SYNAPSE_WORKSPACE_PATH` / `$SYNAPSE_TOOL_ROOT`, then home.
fn resolve_tool_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Some(env) =
        std::env::var_os("SYNAPSE_WORKSPACE_PATH").or_else(|| std::env::var_os("SYNAPSE_TOOL_ROOT"))
    {
        return Ok(PathBuf::from(env));
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
        chart::close("Clean bill of health.", Some("synapse sync --once"));
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
        Some("synapse doctor --fix")
    } else if criticals > 0 {
        None
    } else {
        Some("synapse sync --once")
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
    eprintln!("synapse {verb}: not yet implemented ({args}) — see PLAN.md Phase 3/4");
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
            "scan", "import", "project", "sync", "doctor", "snapshot", "rollback", "exec",
            "filter", "spool", "serve", "hud",
        ] {
            assert!(help.contains(verb), "--help is missing verb: {verb}");
        }
    }

    #[test]
    fn parses_each_verb() {
        assert!(Cli::try_parse_from(["synapse", "scan"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "import", "--dry-run"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "project"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "sync", "--once"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "doctor", "--fix"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "snapshot", "before upgrade"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "rollback", "abc123"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "exec", "--", "cargo", "test"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "filter", "--level", "aggressive"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "spool", "list"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "spool", "show", "test_id"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "serve", "--port", "8088"]).is_ok());
        assert!(Cli::try_parse_from(["synapse", "hud", "--once"]).is_ok());
    }

    #[test]
    fn rejects_unknown_verb() {
        assert!(Cli::try_parse_from(["synapse", "frobnicate"]).is_err());
    }

    #[test]
    fn rollback_requires_a_snapshot_argument() {
        assert!(Cli::try_parse_from(["synapse", "rollback"]).is_err());
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
        // With no override, the default is <home>/Desktop/AI BRAIN (if present) or <home>/AIBrain.
        if let Some(home) = dirs::home_dir() {
            if std::env::var_os("SYNAPSE_BRAIN").is_none()
                && std::env::var_os("SYNAPSE_BRAIN_PATH").is_none()
            {
                let expected =
                    if home.join("Desktop/AI BRAIN").is_dir() && !home.join("AIBrain").exists() {
                        home.join("Desktop/AI BRAIN")
                    } else {
                        home.join("AIBrain")
                    };
                assert_eq!(resolve_brain_root(None).unwrap(), expected);
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
        use synapse_core::mappings::{Mapping, MappingsFile};
        use synapse_core::projector::ProjectionPolicy;

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
