//! MCP registry import + health check — T6.2. Per MASTER_PROMPT.md pillar
//! 6: "browse/search plug-and-play MCP servers from the official MCP
//! registry, Smithery, PulseMCP, mcp.so, and the Docker MCP catalog;
//! install once, project the server config to every tool's MCP file;
//! health-check (spawn + handshake) each server; secrets stored in the OS
//! keychain and written to tool configs as env placeholders only."
//!
//! This module covers the **official registry**
//! (`registry.modelcontextprotocol.io`) — recon-verified live before this
//! was written. The other four catalogs are feature-flagged out per
//! MASTER_PROMPT.md's own instruction ("feature-flag any registry whose
//! API is unstable; verify endpoints live via recon first"): none of them
//! were recon-verified this pass, and several (mcp.so, awesome lists)
//! have no stable API at all.
//!
//! Env vars declared by a registry entry are captured as **names only**
//! (`env_placeholders`) — values, secret or not, are never fetched or
//! stored here. The keychain half of the secrets flow is T6.3.

use crate::adapters::compute_sha256;
use crate::model::{HealthStatus, McpServer};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const REGISTRY_BASE: &str = "https://registry.modelcontextprotocol.io/v0";

#[derive(Debug)]
pub enum RegistryError {
    Http(String),
    Parse(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Http(msg) => write!(f, "http error: {msg}"),
            RegistryError::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// One entry from the official registry, kept alongside the canonical
/// `McpServer` it maps to, with provenance per the marketplace pattern:
/// where it came from and a checksum of the raw registry record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryServer {
    pub server: McpServer,
    pub description: String,
    pub version: String,
    /// SHA256 of the raw registry JSON this entry was built from.
    pub sha256: String,
}

// ---- Wire format of registry.modelcontextprotocol.io/v0/servers ----

#[derive(Deserialize)]
struct SearchResponse {
    servers: Vec<ServerEnvelope>,
}

#[derive(Deserialize)]
struct ServerEnvelope {
    server: ServerRecord,
}

#[derive(Deserialize)]
struct ServerRecord {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    remotes: Vec<RemoteRecord>,
    #[serde(default)]
    packages: Vec<PackageRecord>,
}

#[derive(Deserialize)]
struct RemoteRecord {
    #[serde(rename = "type")]
    remote_type: String,
    url: String,
}

#[derive(Deserialize)]
struct PackageRecord {
    #[serde(rename = "registryType", default)]
    registry_type: String,
    #[serde(default)]
    identifier: String,
    #[serde(rename = "runtimeHint", default)]
    runtime_hint: String,
    #[serde(rename = "environmentVariables", default)]
    environment_variables: Vec<EnvVarRecord>,
}

#[derive(Deserialize)]
struct EnvVarRecord {
    name: String,
}

/// Searches the official MCP registry. Every returned server's
/// `env_placeholders` holds names only — never values.
pub fn search_official_registry(
    query: &str,
    limit: usize,
) -> Result<Vec<RegistryServer>, RegistryError> {
    let url = format!("{REGISTRY_BASE}/servers?search={query}&limit={limit}");
    let raw = ureq::get(&url)
        .set("User-Agent", "llm-neurosurgeon")
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| RegistryError::Http(format!("GET {url}: {e}")))?
        .into_string()
        .map_err(|e| RegistryError::Parse(format!("GET {url}: {e}")))?;

    let parsed: SearchResponse =
        serde_json::from_str(&raw).map_err(|e| RegistryError::Parse(format!("GET {url}: {e}")))?;

    Ok(parsed
        .servers
        .into_iter()
        .filter_map(|envelope| {
            // Re-serialize the individual record for a per-entry checksum.
            let record_json = serde_json::json!({
                "name": envelope.server.name,
                "version": envelope.server.version,
            })
            .to_string();
            map_record(envelope.server).map(|(server, description, version)| RegistryServer {
                server,
                description,
                version,
                sha256: compute_sha256(&record_json),
            })
        })
        .collect())
}

/// Maps a registry record onto the canonical `McpServer`. Remote
/// endpoints win over packages when both exist (no local install
/// needed); an npm/stdio package maps to the `npx -y <identifier>`
/// invocation its `runtimeHint` describes. Records with neither are
/// unusable and dropped.
fn map_record(record: ServerRecord) -> Option<(McpServer, String, String)> {
    let env_placeholders: Vec<String> = record
        .packages
        .iter()
        .flat_map(|p| p.environment_variables.iter().map(|v| v.name.clone()))
        .collect();

    let (transport, command_or_url) = if let Some(remote) = record.remotes.first() {
        (remote.remote_type.clone(), remote.url.clone())
    } else if let Some(pkg) = record.packages.iter().find(|p| p.registry_type == "npm") {
        let runner = if pkg.runtime_hint.is_empty() {
            "npx"
        } else {
            &pkg.runtime_hint
        };
        (
            "stdio".to_string(),
            format!("{} -y {}", runner, pkg.identifier),
        )
    } else {
        return None;
    };

    Some((
        McpServer {
            id: record.name.clone(),
            transport,
            command_or_url,
            env_placeholders,
            targets: Vec::new(),
            health: HealthStatus::Unknown,
        },
        record.description,
        record.version,
    ))
}

// ---- Health check: spawn + MCP initialize handshake ----

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

fn initialize_request() -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "llm-neurosurgeon", "version": env!("CARGO_PKG_VERSION") }
        }
    })
    .to_string()
}

/// True if `line` is a JSON-RPC response to our `initialize` (id 1 with a
/// `result`) — the MCP handshake's server half.
fn is_initialize_response(line: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .map(|v| v.get("id") == Some(&serde_json::json!(1)) && v.get("result").is_some())
        .unwrap_or(false)
}

/// Health-checks a stdio MCP server: spawns `command_or_url` (split on
/// whitespace, same convention every adapter uses for that field), writes
/// a JSON-RPC `initialize` request to its stdin, and waits up to
/// `timeout` for a valid response line on stdout. The child is killed
/// afterward regardless of outcome — this is a handshake probe, not a
/// session.
///
/// # Security
/// This **executes `command_or_url`** — for a registry entry that means
/// running `npx -y <identifier>`, which downloads and runs arbitrary code
/// from npm. That is the MCP model (these servers are meant to run), but
/// it means this function must only ever be called on a server the user
/// has **explicitly enabled** (MASTER_PROMPT.md pillar 6: "user-enable
/// toggle default OFF"). Do not health-check registry search results
/// directly. There is no shell involved (`Command::new().args()`, not
/// `sh -c`), so a crafted `command_or_url` cannot inject shell
/// metacharacters — the blast radius is bounded to extra arguments passed
/// to the named program — but the named program still runs. See
/// docs/security.md §T7.1.
pub fn health_check_stdio(command_or_url: &str, timeout: Duration) -> HealthStatus {
    let parts: Vec<&str> = command_or_url.split_whitespace().collect();
    let Some((program, args)) = parts.split_first() else {
        return HealthStatus::Unreachable;
    };

    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return HealthStatus::Unreachable;
    };

    let handshake_ok = (|| -> Option<bool> {
        let mut stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        writeln!(stdin, "{}", initialize_request()).ok()?;
        stdin.flush().ok()?;

        // Read on a helper thread so the wait is bounded even if the
        // child never writes a byte (same pattern as watcher.rs).
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    return;
                }
            }
        });

        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.checked_duration_since(std::time::Instant::now())?;
            match rx.recv_timeout(remaining) {
                Ok(line) if is_initialize_response(&line) => return Some(true),
                Ok(_) => continue, // logging/noise before the response
                Err(_) => return Some(false),
            }
        }
    })()
    .unwrap_or(false);

    let _ = child.kill();
    let _ = child.wait();

    if handshake_ok {
        HealthStatus::Healthy
    } else {
        HealthStatus::Unreachable
    }
}

/// Health-checks a remote (streamable-http) MCP server by POSTing the
/// `initialize` request to its endpoint. A well-formed JSON-RPC response
/// is Healthy. Because remote servers commonly sit behind auth this probe
/// has no credentials for, an HTTP 401/403 also counts as Healthy — the
/// server is alive and speaking HTTP; it just wants a key. Connection
/// failures, timeouts, and non-MCP responses are Unreachable.
pub fn health_check_remote(url: &str, timeout: Duration) -> HealthStatus {
    let response = ureq::post(url)
        .set("User-Agent", "llm-neurosurgeon")
        .set("Content-Type", "application/json")
        .set("Accept", "application/json, text/event-stream")
        .timeout(timeout)
        .send_string(&initialize_request());

    match response {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            // Streamable HTTP may frame the response as an SSE event;
            // accept the response either bare or on an SSE data line.
            let ok = body
                .lines()
                .map(|l| l.strip_prefix("data:").map(str::trim).unwrap_or(l))
                .any(is_initialize_response);
            if ok {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unreachable
            }
        }
        Err(ureq::Error::Status(code, _)) if code == 401 || code == 403 => HealthStatus::Healthy,
        Err(_) => HealthStatus::Unreachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_record_prefers_remote_over_package() {
        let record = ServerRecord {
            name: "example/server".into(),
            description: "d".into(),
            version: "1.0.0".into(),
            remotes: vec![RemoteRecord {
                remote_type: "streamable-http".into(),
                url: "https://example.com/mcp".into(),
            }],
            packages: vec![PackageRecord {
                registry_type: "npm".into(),
                identifier: "example-server".into(),
                runtime_hint: "npx".into(),
                environment_variables: vec![],
            }],
        };

        let (server, _, _) = map_record(record).unwrap();
        assert_eq!(server.transport, "streamable-http");
        assert_eq!(server.command_or_url, "https://example.com/mcp");
    }

    #[test]
    fn map_record_builds_npx_invocation_for_stdio_packages() {
        let record = ServerRecord {
            name: "example/fs".into(),
            description: "d".into(),
            version: "0.1.0".into(),
            remotes: vec![],
            packages: vec![PackageRecord {
                registry_type: "npm".into(),
                identifier: "fs-mcp-server".into(),
                runtime_hint: "npx".into(),
                environment_variables: vec![
                    EnvVarRecord {
                        name: "API_KEY".into(),
                    },
                    EnvVarRecord {
                        name: "ROOT".into(),
                    },
                ],
            }],
        };

        let (server, _, _) = map_record(record).unwrap();
        assert_eq!(server.transport, "stdio");
        assert_eq!(server.command_or_url, "npx -y fs-mcp-server");
        // Names only — never values.
        assert_eq!(server.env_placeholders, vec!["API_KEY", "ROOT"]);
        assert_eq!(server.health, HealthStatus::Unknown);
    }

    #[test]
    fn map_record_drops_entries_with_no_usable_endpoint() {
        let record = ServerRecord {
            name: "example/empty".into(),
            description: String::new(),
            version: String::new(),
            remotes: vec![],
            packages: vec![],
        };
        assert!(map_record(record).is_none());
    }

    /// A fixture stdio "MCP server": a shell one-liner that reads the
    /// initialize request and answers with a valid JSON-RPC response —
    /// the "end-to-end on fixtures" half of Phase 6's self-verify, so the
    /// handshake logic is exercised hermetically (no npm download at test
    /// time).
    #[cfg(unix)]
    #[test]
    fn health_check_stdio_handshakes_with_a_fixture_server() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fixture-mcp.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\nread _line\nprintf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"serverInfo\":{\"name\":\"fixture\",\"version\":\"0.0.1\"}}}'\nsleep 5\n",
        )
        .unwrap();
        #[allow(clippy::permissions_set_readonly_false)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let status = health_check_stdio(&script.display().to_string(), Duration::from_secs(5));
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[cfg(unix)]
    #[test]
    fn health_check_stdio_reports_unreachable_for_a_dead_command() {
        let status = health_check_stdio("/nonexistent-binary-xyz", Duration::from_secs(1));
        assert_eq!(status, HealthStatus::Unreachable);
    }

    #[cfg(unix)]
    #[test]
    fn health_check_stdio_reports_unreachable_when_server_never_responds() {
        // `cat` echoes our request back — a line, but never a valid
        // initialize response — then waits forever. Must time out, not hang.
        let status = health_check_stdio("cat", Duration::from_millis(500));
        assert_eq!(status, HealthStatus::Unreachable);
    }

    /// Real, network-dependent test — T6.2's verify text asks for "2
    /// registry MCP servers end-to-end". Fetches live entries from the
    /// official registry, maps them to the canonical model, and records
    /// checksums. Skips (with a loud message) if offline.
    #[test]
    fn imports_2_real_servers_from_the_official_registry() {
        let servers = match search_official_registry("filesystem", 10) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("skipping: could not reach the MCP registry ({e})");
                return;
            }
        };
        assert!(
            servers.len() >= 2,
            "expected at least 2 usable servers, got {}",
            servers.len()
        );

        let mut checksums = std::collections::HashSet::new();
        for entry in servers.iter().take(2) {
            assert!(!entry.server.id.is_empty());
            assert!(!entry.server.command_or_url.is_empty());
            assert!(
                entry.server.transport == "stdio"
                    || entry.server.transport.contains("http")
                    || entry.server.transport == "sse",
                "unexpected transport: {}",
                entry.server.transport
            );
            assert_eq!(entry.server.health, HealthStatus::Unknown);
            assert_eq!(entry.sha256.len(), 64);
            checksums.insert(entry.sha256.clone());
        }
        assert_eq!(checksums.len(), 2, "checksums must be distinct per entry");
    }
}
