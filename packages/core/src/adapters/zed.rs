use super::{compute_sha256, strip_provenance};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, HealthStatus, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ZedAdapter;

impl Adapter for ZedAdapter {
    fn id(&self) -> &'static str {
        "zed"
    }

    fn detect(&self, root: &Path) -> bool {
        let in_root = root.join(".rules").exists()
            || root.join(".zed/settings.json").exists()
            || root.join(".config/zed/AGENTS.md").exists()
            || root.join(".config/zed/settings.json").exists();
        let in_home = std::env::var_os("HOME").map(PathBuf::from).map_or(false, |h| {
            h.join(".config/zed/AGENTS.md").exists() || h.join(".config/zed/settings.json").exists()
        });
        in_root || in_home
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        let rules_path = root.join(".rules");
        let zed_agents_path = root.join(".config/zed/AGENTS.md");
        let home_agents_path = std::env::var_os("HOME").map(PathBuf::from).map(|h| h.join(".config/zed/AGENTS.md"));

        let target_rules_path = if rules_path.exists() {
            Some(rules_path)
        } else if zed_agents_path.exists() {
            Some(zed_agents_path)
        } else if let Some(hp) = home_agents_path {
            if hp.exists() {
                Some(hp)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(path) = target_rules_path {
            let raw = fs::read_to_string(&path)
                .map_err(|e| AdapterError::Io(format!("Failed to read {}: {}", path.display(), e)))?;
            let content = strip_provenance(&raw);
            let sha256 = compute_sha256(&content);
            skills.push(Skill {
                id: "zed-rules".to_string(),
                version: "1.0.0".to_string(),
                triggers: vec!["*".to_string()],
                targets: vec!["zed".to_string()],
                source: content,
                sha256,
            });
        }

        let settings_path = root.join(".zed/settings.json");
        if settings_path.exists() {
            let raw_json = fs::read_to_string(&settings_path).map_err(|e| {
                AdapterError::Io(format!("Failed to read .zed/settings.json: {}", e))
            })?;
            let parsed: Value = serde_json::from_str(&raw_json).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .zed/settings.json: {}", e))
            })?;

            if let Some(servers) = parsed.get("context_servers").and_then(|v| v.as_object()) {
                for (id, val) in servers {
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
                                .map(|item| item.as_str().unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default();

                    let command_or_url = if args.is_empty() {
                        command
                    } else {
                        format!("{} {}", command, args.join(" "))
                    };

                    let mut env_placeholders = Vec::new();
                    if let Some(env_obj) = val.get("env").and_then(|v| v.as_object()) {
                        for key in env_obj.keys() {
                            env_placeholders.push(key.clone());
                        }
                    }
                    env_placeholders.sort();

                    mcp_servers.push(McpServer {
                        id: id.clone(),
                        transport: "stdio".to_string(),
                        command_or_url,
                        env_placeholders,
                        targets: vec!["zed".to_string()],
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

        let zed_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"zed".to_string()))
            .collect();

        if !zed_skills.is_empty() {
            let concatenated: String = zed_skills
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<&str>>()
                .join("\n\n---\n\n");
            let output = format!(
                "<!-- GENERATED BY LLM NEUROSURGEON — edit in the Brain -->\n{}",
                concatenated
            );
            fs::write(root.join(".rules"), &output)
                .map_err(|e| AdapterError::Io(format!("Failed to write .rules: {}", e)))?;
            written.push(".rules".to_string());

            let config_zed = root.join(".config/zed");
            let target_config_zed = if config_zed.exists() || root.join(".config").exists() {
                Some(config_zed)
            } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                Some(home.join(".config/zed"))
            } else {
                None
            };

            if let Some(dir) = target_config_zed {
                fs::create_dir_all(&dir).map_err(|e| {
                    AdapterError::Io(format!("Failed to create .config/zed directory: {}", e))
                })?;
                let agents_path = dir.join("AGENTS.md");
                fs::write(&agents_path, &output).map_err(|e| {
                    AdapterError::Io(format!("Failed to write .config/zed/AGENTS.md: {}", e))
                })?;
                written.push(".config/zed/AGENTS.md".to_string());
            }
        }

        let zed_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"zed".to_string()))
            .collect();

        if !zed_servers.is_empty() {
            let settings_dir = root.join(".zed");
            fs::create_dir_all(&settings_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .zed directory: {}", e)))?;
            let settings_path = settings_dir.join("settings.json");

            let mut current_json = if settings_path.exists() {
                let raw_json = fs::read_to_string(&settings_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .zed/settings.json: {}", e))
                })?;
                serde_json::from_str(&raw_json).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .zed/settings.json: {}", e))
                })?
            } else {
                json!({})
            };

            if !current_json.is_object() {
                return Err(AdapterError::Malformed(
                    ".zed/settings.json root is not an object".to_string(),
                ));
            }

            let servers_map = current_json
                .as_object_mut()
                .unwrap()
                .entry("context_servers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("context_servers is not an object".to_string())
                })?;

            for server in zed_servers {
                let parts: Vec<&str> = server.command_or_url.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let command = parts[0].to_string();
                let args: Vec<Value> = parts[1..]
                    .iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect();

                let mut env_obj = serde_json::Map::new();
                if let Some(existing_server) = servers_map.get(&server.id) {
                    if let Some(existing_env) =
                        existing_server.get("env").and_then(|v| v.as_object())
                    {
                        env_obj = existing_env.clone();
                    }
                }

                for key in &server.env_placeholders {
                    if !env_obj.contains_key(key) {
                        env_obj.insert(key.clone(), Value::String("".to_string()));
                    }
                }

                let new_server = json!({
                    "command": command,
                    "args": args,
                    "env": env_obj
                });

                servers_map.insert(server.id.clone(), new_server);
            }

            let pretty = serde_json::to_string_pretty(&current_json)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize JSON: {}", e)))?;
            fs::write(&settings_path, pretty).map_err(|e| {
                AdapterError::Io(format!("Failed to write .zed/settings.json: {}", e))
            })?;
            written.push(".zed/settings.json".to_string());
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
    fn test_zed_detect() {
        let dir = tempdir().unwrap();
        let adapter = ZedAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join(".rules"), "test").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_zed_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = ZedAdapter;

        fs::write(dir.path().join(".rules"), "Always write tests first").unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        fs::write(
            dir.path().join(".zed/settings.json"),
            r#"{"vim_mode": true, "context_servers": {"docs": {"command": "uvx", "args": ["docs-server"]}}}"#,
        ).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "zed-rules");
        assert_eq!(imported.skills[0].source, "Always write tests first");

        assert_eq!(imported.mcp_servers.len(), 1);
        assert_eq!(imported.mcp_servers[0].id, "docs");
        assert_eq!(imported.mcp_servers[0].command_or_url, "uvx docs-server");

        let out_dir = tempdir().unwrap();
        fs::create_dir_all(out_dir.path().join(".zed")).unwrap();
        fs::write(
            out_dir.path().join(".zed/settings.json"),
            r#"{"vim_mode": true}"#,
        )
        .unwrap();

        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &[], &imported.mcp_servers)
            .unwrap();

        assert!(project_res.written.contains(&".rules".to_string()));
        assert!(project_res.written.iter().any(|w| w.contains("settings.json")));
        let projected_rules = fs::read_to_string(out_dir.path().join(".rules")).unwrap();
        assert!(projected_rules.contains("Always write tests first"));

        let projected_settings =
            fs::read_to_string(out_dir.path().join(".zed/settings.json")).unwrap();
        let parsed: Value = serde_json::from_str(&projected_settings).unwrap();
        assert_eq!(parsed["vim_mode"], true);
        assert_eq!(parsed["context_servers"]["docs"]["command"], "uvx");
    }

    #[test]
    fn test_zed_stress_malformed_settings() {
        let dir = tempdir().unwrap();
        let adapter = ZedAdapter;
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        fs::write(dir.path().join(".zed/settings.json"), "{ broken").unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }
}
