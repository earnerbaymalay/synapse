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
    /// Layer 1 — search memory; returns a compact, token-costed index.
    /// Blends keyword relevance with local nomic-embed-text semantic similarity.
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
        /// Only observations with this topic
        #[arg(long)]
        topic: Option<String>,
        /// Only observations recorded by this model/agent
        #[arg(long)]
        model: Option<String>,
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
        /// Subject area for organisation (e.g. networking, memory-system)
        #[arg(long, default_value = "")]
        topic: String,
        /// Keyword tag (repeatable)
        #[arg(short, long)]
        tag: Vec<String>,
        /// Recording model/agent (defaults to $SYNAPSE_MODEL)
        #[arg(long)]
        model: Option<String>,
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
    /// Compute local embeddings for any observations missing one (uses Ollama)
    Reindex {
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
    },
    /// Distil memory: collapse semantically near-duplicate observations to the
    /// single most-informative one. Reports by default; --apply to remove.
    Distill {
        /// Limit to a project path (defaults to the current directory)
        #[arg(long)]
        project: Option<String>,
        /// Distil across every project
        #[arg(long)]
        all: bool,
        /// Cosine similarity at/above which two observations are duplicates
        #[arg(long, default_value_t = 0.90)]
        threshold: f32,
        /// Actually remove the redundant observations (otherwise dry-run)
        #[arg(long)]
        apply: bool,
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
            topic,
            model,
            limit,
            dir,
        } => {
            let proj = cwd_project(project);
            let scope = if all { None } else { Some(proj.as_str()) };
            match store(dir).search_filtered(
                &query.join(" "),
                scope,
                topic.as_deref(),
                model.as_deref(),
                limit,
            ) {
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
                    let mut meta = Vec::new();
                    if !o.topic.is_empty() {
                        meta.push(format!("topic: {}", o.topic));
                    }
                    if !o.tags.is_empty() {
                        meta.push(format!("tags: {}", o.tags.join(", ")));
                    }
                    if !o.model.is_empty() {
                        meta.push(format!("model: {}", o.model));
                    }
                    if !o.files.is_empty() {
                        meta.push(format!("files: {}", o.files.join(", ")));
                    }
                    if !meta.is_empty() {
                        println!("{}\n", meta.join("  ·  "));
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
            topic,
            tag,
            model,
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
                topic,
                tags: tag,
                model: model.or_else(|| std::env::var("SYNAPSE_MODEL").ok()).unwrap_or_default(),
            };
            match store(dir).add(new) {
                Ok(Some(o)) => {
                    chart::open("memory", "observation recorded");
                    chart::field("Id", &format!("#{}", o.id));
                    if !o.topic.is_empty() {
                        chart::field("Topic", &o.topic);
                    }
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
        MemAction::Reindex { dir } => {
            if !synapse_core::embedding::enabled() {
                return fail(
                    "embeddings are disabled (SYNAPSE_EMBED_MODEL=off); nothing to reindex",
                );
            }
            match store(dir).reembed_missing() {
                Ok(n) => {
                    chart::open("memory", "reindex");
                    chart::field(
                        "Model",
                        &synapse_core::embedding::model().unwrap_or_default(),
                    );
                    chart::close(&format!("{} embedded", chart::plural(n, "observation")), None);
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e.to_string()),
            }
        }
        MemAction::Distill {
            project,
            all,
            threshold,
            apply,
            dir,
        } => {
            let proj = cwd_project(project);
            let scope = if all { None } else { Some(proj.as_str()) };
            match store(dir).distill(scope, threshold, apply) {
                Ok(r) if r.clusters.is_empty() => {
                    if synapse_core::embedding::enabled() {
                        println!(
                            "Nothing to distil — no near-duplicate clusters at cosine ≥ {:.2}.",
                            threshold
                        );
                    } else {
                        println!(
                            "Embeddings are disabled (SYNAPSE_EMBED_MODEL=off); distillation needs them."
                        );
                    }
                    ExitCode::SUCCESS
                }
                Ok(r) => {
                    chart::open(
                        "memory",
                        if r.applied { "distil · applied" } else { "distil · dry-run" },
                    );
                    chart::field("Clusters", &r.clusters.len().to_string());
                    chart::field(
                        if r.applied { "Removed" } else { "Removable" },
                        &chart::plural(r.removed, "observation"),
                    );
                    chart::field(
                        "Tokens",
                        &format!("{} → {}", r.tokens_before, r.tokens_after),
                    );
                    chart::close(
                        if r.applied {
                            "redundant fragments collapsed"
                        } else {
                            "re-run with --apply to collapse"
                        },
                        None,
                    );
                    for c in &r.clusters {
                        println!(
                            "  keep #{:<4} {}  ⟵ drop {:?}  (~{}t)",
                            c.keep, c.keep_title, c.drop, c.tokens_saved
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e.to_string()),
            }
        }
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
                    let embedded = s.load_embeddings().len();
                    chart::field(
                        "Embedded",
                        &format!(
                            "{} / {} ({})",
                            embedded,
                            st.observations,
                            synapse_core::embedding::model().unwrap_or_else(|| "off".into())
                        ),
                    );
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
    // Which agent/model is recording — from the hook payload, else $SYNAPSE_MODEL.
    let model = match str_of("model") {
        s if s.is_empty() => std::env::var("SYNAPSE_MODEL").unwrap_or_default(),
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
                model,
                ..Default::default()
            })?;
        }
        "post-tool" => {
            let null = Value::Null;
            if let Some(mut new) = observation_from_tool(
                &session,
                &project,
                &str_of("tool_name"),
                payload.get("tool_input").unwrap_or(&null),
                payload.get("tool_response").unwrap_or(&null),
            ) {
                new.model = model;
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
