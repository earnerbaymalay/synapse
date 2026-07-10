use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use std::fs;
use std::path::Path;
use toml::{Table, Value};

pub struct OpenAiCodexAdapter;

/// Codex CLI's own project-scoped artifact is `.codex/config.toml` (its
/// `[mcp_servers.*]` tables). `AGENTS.md` is deliberately left to
/// `OpenCodeAdapter` — both tools converge on that filename as the emerging
/// cross-tool instructions standard (see MASTER_PROMPT.md §1), so claiming
/// it here too would double-import the same content under two adapters.
/// Paths confirmed live against developers.openai.com/codex (2026-07-10),
/// since the original recon brief (docs/research/openai-codex.md) had them
/// marked VERIFY.
impl Adapter for OpenAiCodexAdapter {
    fn id(&self) -> &'static str {
        "openai-codex"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join(".codex/config.toml").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut mcp_servers = Vec::new();

        let config_path = root.join(".codex/config.toml");
        if config_path.exists() {
            let raw = fs::read_to_string(&config_path).map_err(|e| {
                AdapterError::Io(format!("Failed to read .codex/config.toml: {}", e))
            })?;
            let parsed: Value = toml::from_str(&raw).map_err(|e| {
                AdapterError::Malformed(format!("Invalid TOML in .codex/config.toml: {}", e))
            })?;

            if let Some(servers) = parsed.get("mcp_servers").and_then(|v| v.as_table()) {
                for (id, val) in servers {
                    let url = val.get("url").and_then(|v| v.as_str());
                    let command = val
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args: Vec<String> = val
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let (transport, command_or_url) = if let Some(url) = url {
                        ("http".to_string(), url.to_string())
                    } else if args.is_empty() {
                        ("stdio".to_string(), command)
                    } else {
                        (
                            "stdio".to_string(),
                            format!("{} {}", command, args.join(" ")),
                        )
                    };

                    let mut env_placeholders = Vec::new();
                    if let Some(env_table) = val.get("env").and_then(|v| v.as_table()) {
                        for key in env_table.keys() {
                            env_placeholders.push(key.clone());
                        }
                    }
                    env_placeholders.sort();

                    mcp_servers.push(McpServer {
                        id: id.clone(),
                        transport,
                        command_or_url,
                        env_placeholders,
                        targets: vec!["openai-codex".to_string()],
                        health: HealthStatus::Unknown,
                    });
                }
            }
        }

        Ok(ImportResult {
            skills: Vec::new(),
            agents: Vec::new(),
            mcp_servers,
        })
    }

    fn project(
        &self,
        root: &Path,
        _skills: &[Skill],
        _agents: &[Agent],
        mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();

        let codex_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"openai-codex".to_string()))
            .collect();

        if !codex_servers.is_empty() {
            let config_dir = root.join(".codex");
            fs::create_dir_all(&config_dir).map_err(|e| {
                AdapterError::Io(format!("Failed to create .codex directory: {}", e))
            })?;
            let config_path = config_dir.join("config.toml");

            let mut current: Value = if config_path.exists() {
                let raw = fs::read_to_string(&config_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .codex/config.toml: {}", e))
                })?;
                toml::from_str(&raw).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid TOML in .codex/config.toml: {}", e))
                })?
            } else {
                Value::Table(Table::new())
            };

            if !current.is_table() {
                return Err(AdapterError::Malformed(
                    ".codex/config.toml root is not a table".to_string(),
                ));
            }

            let root_table = current.as_table_mut().unwrap();
            if !root_table.contains_key("mcp_servers") {
                root_table.insert("mcp_servers".to_string(), Value::Table(Table::new()));
            }
            let servers_table = root_table
                .get_mut("mcp_servers")
                .unwrap()
                .as_table_mut()
                .ok_or_else(|| AdapterError::Malformed("mcp_servers is not a table".to_string()))?;

            for server in codex_servers {
                let mut entry = Table::new();
                if server.transport == "http" {
                    entry.insert(
                        "url".to_string(),
                        Value::String(server.command_or_url.clone()),
                    );
                } else {
                    let parts: Vec<&str> = server.command_or_url.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    entry.insert("command".to_string(), Value::String(parts[0].to_string()));
                    if parts.len() > 1 {
                        entry.insert(
                            "args".to_string(),
                            Value::Array(
                                parts[1..]
                                    .iter()
                                    .map(|s| Value::String(s.to_string()))
                                    .collect(),
                            ),
                        );
                    }
                }

                let mut env_table = Table::new();
                if let Some(existing) = servers_table.get(&server.id) {
                    if let Some(existing_env) = existing.get("env").and_then(|v| v.as_table()) {
                        env_table = existing_env.clone();
                    }
                }
                for key in &server.env_placeholders {
                    if !env_table.contains_key(key) {
                        env_table.insert(key.clone(), Value::String(String::new()));
                    }
                }
                if !env_table.is_empty() {
                    entry.insert("env".to_string(), Value::Table(env_table));
                }

                servers_table.insert(server.id.clone(), Value::Table(entry));
            }

            let serialized = toml::to_string_pretty(&current)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize TOML: {}", e)))?;
            fs::write(&config_path, serialized).map_err(|e| {
                AdapterError::Io(format!("Failed to write .codex/config.toml: {}", e))
            })?;
            written.push(".codex/config.toml".to_string());
        }

        Ok(ProjectResult {
            written,
            symlinked: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_openai_codex_detect() {
        let dir = tempdir().unwrap();
        let adapter = OpenAiCodexAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::create_dir_all(dir.path().join(".codex")).unwrap();
        fs::write(dir.path().join(".codex/config.toml"), "").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_openai_codex_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = OpenAiCodexAdapter;

        fs::create_dir_all(dir.path().join(".codex")).unwrap();
        fs::write(
            dir.path().join(".codex/config.toml"),
            r#"
model = "gpt-5-codex"

[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@mcp/fs"]
env = { ROOT = "/tmp" }

[mcp_servers.remote]
url = "https://api.example.com/mcp"
"#,
        )
        .unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.mcp_servers.len(), 2);

        let fs_server = imported
            .mcp_servers
            .iter()
            .find(|s| s.id == "filesystem")
            .unwrap();
        assert_eq!(fs_server.transport, "stdio");
        assert_eq!(fs_server.command_or_url, "npx -y @mcp/fs");
        assert_eq!(fs_server.env_placeholders, vec!["ROOT".to_string()]);

        let remote_server = imported
            .mcp_servers
            .iter()
            .find(|s| s.id == "remote")
            .unwrap();
        assert_eq!(remote_server.transport, "http");
        assert_eq!(remote_server.command_or_url, "https://api.example.com/mcp");

        let out_dir = tempdir().unwrap();
        // Pre-existing unrelated top-level key must survive the merge.
        fs::create_dir_all(out_dir.path().join(".codex")).unwrap();
        fs::write(
            out_dir.path().join(".codex/config.toml"),
            "model = \"gpt-5-codex\"\n",
        )
        .unwrap();

        let project_res = adapter
            .project(out_dir.path(), &[], &[], &imported.mcp_servers)
            .unwrap();
        assert_eq!(project_res.written, vec![".codex/config.toml".to_string()]);

        let projected = fs::read_to_string(out_dir.path().join(".codex/config.toml")).unwrap();
        let parsed: Value = toml::from_str(&projected).unwrap();
        assert_eq!(parsed["model"].as_str().unwrap(), "gpt-5-codex");
        assert_eq!(
            parsed["mcp_servers"]["filesystem"]["command"]
                .as_str()
                .unwrap(),
            "npx"
        );
        assert_eq!(
            parsed["mcp_servers"]["remote"]["url"].as_str().unwrap(),
            "https://api.example.com/mcp"
        );
    }

    #[test]
    fn test_openai_codex_stress_malformed_toml() {
        let dir = tempdir().unwrap();
        let adapter = OpenAiCodexAdapter;
        fs::create_dir_all(dir.path().join(".codex")).unwrap();
        fs::write(
            dir.path().join(".codex/config.toml"),
            "this is not [valid toml",
        )
        .unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_openai_codex_does_not_claim_agents_md() {
        let dir = tempdir().unwrap();
        let adapter = OpenAiCodexAdapter;
        fs::write(dir.path().join("AGENTS.md"), "shared with opencode").unwrap();
        assert!(!adapter.detect(dir.path()));
    }
}
