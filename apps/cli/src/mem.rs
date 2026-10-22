//! `synapse mem` — Synaptic Memory commands (see `synapse_core::memory`).
//!
//! Output intended for agents (search/timeline/get/context/hook) is plain text;
//! human-facing confirmations use the chart helpers.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use serde_json::Value;
use synapse_core::memory::{observation_from_tool, summarize_session, MemoryStore, NewObservation};

use crate::chart;

#[derive(Debug, Subcommand)]
pub enum MemAction {
    /// Layer 1 — search memory; returns a compact, token-costed index
    Search {
        /// Keywords (empty = most recent)
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
        /// Limit to a project path (defaults to the current directory)
        #[arg(long)]
        project: Option<String>,
        /// Search across every project
        #[arg(long)]
        all: bool,
        #[arg(short, long, default_value_t = 15)]
        limit: usize,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Layer 2 — chronological context around an observation id
    Timeline {
        id: u64,
        #[arg(long, default_value_t = 3)]
        before: usize,
        #[arg(long, default_value_t = 3)]
        after: usize,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Layer 3 — full detail for specific observation ids
    Get {
        #[arg(required = true)]
        ids: Vec<u64>,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Manually record a note
    Observe {
        title: String,
        /// Longer body text
        #[arg(short, long, default_value = "")]
        note: String,
        #[arg(short, long, default_value = "note")]
        kind: String,
        /// Related file (repeatable)
        #[arg(short, long)]
        file: Vec<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Print the SessionStart context block for a project
    Context {
        #[arg(long)]
        project: Option<String>,
        #[arg(short, long, default_value_t = 25)]
        limit: usize,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Lifecycle-hook entrypoint: reads the agent's hook JSON on stdin.
    /// Events: session-start | prompt | post-tool | stop. Never fails the agent.
    Hook {
        event: String,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Delete observations by id
    Forget {
        #[arg(required = true)]
        ids: Vec<u64>,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Memory size and coverage
    Stats {
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Print the Claude Code hooks config that wires memory in (writes nothing)
    Hooks,
}

fn store(dir: Option<PathBuf>) -> MemoryStore {
    MemoryStore::new(dir.unwrap_or_else(MemoryStore::default_dir))
}

fn cwd_project(explicit: Option<String>) -> String {
    explicit.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    })
}

fn fail(msg: &str) -> ExitCode {
    chart::fault("memory", msg, None);
    ExitCode::FAILURE
}

pub fn run(action: MemAction) -> ExitCode {
    match action {
        MemAction::Search {
            query,
            project,
            all,
            limit,
            dir,
        } => {
            let proj = cwd_project(project);
            let scope = if all { None } else { Some(proj.as_str()) };
            match store(dir).search(&query.join(" "), scope, limit) {
                Ok(hits) if hits.is_empty() => {
                    println!("No matching memories.");
                    ExitCode::SUCCESS
                }
                Ok(hits) => {
                    for o in hits {
                        println!("{}", o.index_line());
                    }
                    println!("\nNext: `synapse mem timeline <id>` or `synapse mem get <id>…`");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e.to_string()),
            }
        }
        MemAction::Timeline {
            id,
            before,
            after,
            dir,
        } => match store(dir).timeline(id, before, after) {
            Ok(rows) if rows.is_empty() => fail(&format!("no observation #{id}")),
            Ok(rows) => {
                for o in rows {
                    let mark = if o.id == id { ">" } else { " " };
                    println!("{mark} {}", o.index_line());
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e.to_string()),
        },
        MemAction::Get { ids, dir } => match store(dir).get(&ids) {
            Ok(rows) if rows.is_empty() => fail("no matching observations"),
            Ok(rows) => {
                for o in rows {
                    println!(
                        "## #{} · {} · {} · {}",
                        o.id,
                        o.kind,
                        synapse_core::memory::format_ts(o.ts),
                        o.project
                    );
                    println!("{}\n", o.title);
                    if !o.narrative.is_empty() {
                        println!("{}\n", o.narrative);
                    }
                    if !o.files.is_empty() {
                        println!("files: {}\n", o.files.join(", "));
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e.to_string()),
        },
        MemAction::Observe {
            title,
            note,
            kind,
            file,
            project,
            dir,
        } => {
            let new = NewObservation {
                session: "manual".into(),
                project: cwd_project(project),
                kind,
                title,
                narrative: note,
                files: file,
            };
            match store(dir).add(new) {
                Ok(Some(o)) => {
                    chart::open("memory", "observation recorded");
                    chart::field("Id", &format!("#{}", o.id));
                    chart::close(&o.title, None);
                    ExitCode::SUCCESS
                }
                Ok(None) => fail("nothing to store (empty, duplicate, or fully <private>)"),
                Err(e) => fail(&e.to_string()),
            }
        }
        MemAction::Context {
            project,
            limit,
            dir,
        } => match store(dir).context(&cwd_project(project), limit) {
            Ok(ctx) => {
                print!("{ctx}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e.to_string()),
        },
        MemAction::Hook { event, dir } => {
            // A memory fault must never block or break the host agent.
            let _ = run_hook(&event, &store(dir));
            ExitCode::SUCCESS
        }
        MemAction::Forget { ids, dir } => match store(dir).forget(&ids) {
            Ok(n) => {
                chart::open("memory", "forget");
                chart::close(
                    &format!("{} removed", chart::plural(n, "observation")),
                    None,
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e.to_string()),
        },
        MemAction::Stats { dir } => {
            let s = store(dir);
            match s.stats() {
                Ok(st) => {
                    chart::open("memory", "store");
                    chart::field(
                        "Location",
                        &MemoryStore::default_dir().display().to_string(),
                    );
                    chart::field("Observations", &st.observations.to_string());
                    chart::field("Sessions", &st.sessions.to_string());
                    chart::field("Projects", &st.projects.to_string());
                    chart::field("Stored tokens", &format!("~{}", st.stored_tokens));
                    chart::field("On disk", &format!("{} bytes", st.bytes));
                    chart::close("local only — zero telemetry", None);
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e.to_string()),
            }
        }
        MemAction::Hooks => {
            println!("{}", HOOKS_SNIPPET);
            ExitCode::SUCCESS
        }
    }
}

fn run_hook(event: &str, store: &MemoryStore) -> std::io::Result<()> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let str_of = |k: &str| {
        payload
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let session = match str_of("session_id") {
        s if s.is_empty() => "unknown".to_string(),
        s => s,
    };
    let project = match str_of("cwd") {
        s if s.is_empty() => cwd_project(None),
        s => s,
    };

    match event {
        "session-start" => {
            print!("{}", store.context(&project, 25)?);
        }
        "prompt" => {
            let prompt = str_of("prompt");
            store.add(NewObservation {
                session,
                project,
                kind: "prompt".into(),
                title: prompt.lines().next().unwrap_or("").to_string(),
                narrative: prompt,
                files: vec![],
            })?;
        }
        "post-tool" => {
            let null = Value::Null;
            if let Some(new) = observation_from_tool(
                &session,
                &project,
                &str_of("tool_name"),
                payload.get("tool_input").unwrap_or(&null),
                payload.get("tool_response").unwrap_or(&null),
            ) {
                store.add(new)?;
            }
        }
        "stop" => {
            if let Some(sum) = summarize_session(store, &session, &project)? {
                // One rolling summary per session: replace the previous one.
                let stale: Vec<u64> = store
                    .all()?
                    .iter()
                    .filter(|o| o.session == session && o.kind == "summary")
                    .map(|o| o.id)
                    .collect();
                store.forget(&stale)?;
                store.add(sum)?;
            }
        }
        _ => {}
    }
    Ok(())
}

const HOOKS_SNIPPET: &str = r#"Merge into ~/.claude/settings.json (or a project's .claude/settings.json):

{
  "hooks": {
    "SessionStart":     [{ "hooks": [{ "type": "command", "command": "synapse mem hook session-start" }] }],
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "synapse mem hook prompt" }] }],
    "PostToolUse":      [{ "matcher": "*", "hooks": [{ "type": "command", "command": "synapse mem hook post-tool" }] }],
    "Stop":             [{ "hooks": [{ "type": "command", "command": "synapse mem hook stop" }] }]
  }
}

SessionStart stdout is injected into the new session as a compact memory index.
Agents then drill down with `synapse mem search | timeline | get`."#;
