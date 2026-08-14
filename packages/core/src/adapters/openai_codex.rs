use super::{compute_sha256, has_path_traversal, parse_and_validate_mcp_server, strip_provenance};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

pub struct OpenAiCodexAdapter;

impl Adapter for OpenAiCodexAdapter {
    fn id(&self) -> &'static str {
        "openai-codex"
    }

    fn detect(&self, root: &Path) -> bool {
        let in_root = root.join(".codex/config.toml").exists()
            || root.join(".codex/config.json").exists()
            || root.join(".codex/instructions.md").exists()
            || root.join("AGENTS.md").exists();
        let in_home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .is_some_and(|h| {
                h.join(".codex/config.toml").exists()
                    || h.join(".codex/config.json").exists()
                    || h.join(".codex/instructions.md").exists()
            });
        in_root || in_home
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        // 1. Check rules in root (AGENTS.md or .codex/instructions.md)
        let rules_path = if root.join("AGENTS.md").exists() {
            Some(root.join("AGENTS.md"))
        } else if root.join(".codex/instructions.md").exists() {
            Some(root.join(".codex/instructions.md"))
        } else {
            None
        };

        if let Some(path) = rules_path {
            let raw = fs::read_to_string(&path).map_err(|e| {
                AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
            })?;
            let content = strip_provenance(&raw);
            if has_path_traversal(&content) {
                return Err(AdapterError::Malformed(format!(
                    "Path traversal detected in {}",
                    path.display()
                )));
            }
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "codex-rules".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["openai-codex".to_string()],
                source: content,
                sha256,
            });
        }

        // 1. Check .codex/config.json in root or home
        let json_path = if root.join(".codex/config.json").exists() {
            Some(root.join(".codex/config.json"))
        } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            if home.join(".codex/config.json").exists() {
                Some(home.join(".codex/config.json"))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(path) = json_path {
            let raw = fs::read_to_string(&path).map_err(|e| {
                AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
            })?;
            let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in {}: {}", path.display(), e))
            })?;

            if let Some(cmd) = parsed.get("command").and_then(|v| v.as_str()) {
                if has_path_traversal(cmd) {
                    return Err(AdapterError::Malformed(format!(
                        "Path traversal in .codex/config.json command: {cmd}"
                    )));
                }
            }

            let servers_opt = parsed
                .get("mcp_servers")
                .or_else(|| parsed.get("mcpServers"))
                .and_then(|v| v.as_object());

            if let Some(servers) = servers_opt {
                for (id, val) in servers {
                    parse_and_validate_mcp_server(id, val, "openai-codex", &mut mcp_servers)?;
                }
            }
        }

        // 2. Check .codex/config.toml in root or home
        let toml_path = if root.join(".codex/config.toml").exists() {
            Some(root.join(".codex/config.toml"))
        } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            if home.join(".codex/config.toml").exists() {
                Some(home.join(".codex/config.toml"))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(config_path) = toml_path {
            let raw = fs::read_to_string(&config_path).map_err(|e| {
                AdapterError::Io(format!("Failed to read {}: {}", config_path.display(), e))
            })?;
            let parsed: Value = toml::from_str(&raw).map_err(|e| {
                AdapterError::Malformed(format!("Invalid TOML in {}: {}", config_path.display(), e))
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

                    if url.is_none() && command.trim().is_empty() {
                        return Err(AdapterError::Malformed(format!(
                            "MCP server '{id}' is missing required `command` or `url`"
                        )));
                    }

                    if let Some(u) = url {
                        if has_path_traversal(u) {
                            return Err(AdapterError::Malformed(format!(
                                "Path traversal in MCP server '{id}' url: {u}"
                            )));
                        }
                    }
                    if !command.is_empty() && has_path_traversal(&command) {
                        return Err(AdapterError::Malformed(format!(
                            "Path traversal in MCP server '{id}' command: {command}"
                        )));
                    }
                    for arg in &args {
                        if has_path_traversal(arg) {
                            return Err(AdapterError::Malformed(format!(
                                "Path traversal in MCP server '{id}' arg: {arg}"
                            )));
                        }
                    }

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
            skills,
            agents: Vec::new(),
            mcp_servers,
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        _agents: &[Agent],
        mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();

        let codex_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"openai-codex".to_string()))
            .collect();

        if !codex_skills.is_empty() {
            let concatenated: String = codex_skills
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<&str>>()
                .join("\n\n---\n\n");
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                concatenated
            );
            let rules_path = root.join("AGENTS.md");
            fs::write(&rules_path, &output)
                .map_err(|e| AdapterError::Io(format!("Failed to write AGENTS.md: {}", e)))?;
            written.push("AGENTS.md".to_string());
        }

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
    fn test_openai_codex_claims_agents_md() {
        let dir = tempdir().unwrap();
        let adapter = OpenAiCodexAdapter;
        fs::write(dir.path().join("AGENTS.md"), "shared with opencode").unwrap();
        assert!(adapter.detect(dir.path()));
        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "codex-rules");
    }
}
