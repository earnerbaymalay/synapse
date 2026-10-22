use super::{
    clean_jsonc, compute_sha256, parse_and_validate_mcp_server, safe_join, split_frontmatter,
    strip_provenance, write_if_changed,
};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Next-Gen Adapter for AWS Kiro IDE (`.kiro/steering/*.md` and `.kiro/settings/mcp.json`).
pub struct KiroAdapter;

const STEERING_ID_PREFIX: &str = "kiro-steering-";

fn parse_inclusion_triggers(fm: &str) -> Result<Vec<String>, AdapterError> {
    let mut file_match_pattern: Option<String> = None;
    let mut inclusion_mode = String::from("always");

    for line in fm.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            match key {
                "inclusion" => inclusion_mode = val.to_string(),
                "fileMatchPattern" => {
                    if super::has_path_traversal(val) {
                        return Err(AdapterError::Malformed(format!(
                            "Path traversal in Kiro steering fileMatchPattern: {val}"
                        )));
                    }
                    file_match_pattern = Some(val.to_string());
                }
                _ => {}
            }
        }
    }

    if inclusion_mode == "fileMatch" {
        if let Some(pat) = file_match_pattern.filter(|s| !s.is_empty()) {
            return Ok(vec![pat]);
        }
    }
    Ok(vec!["*".to_string()])
}

impl Adapter for KiroAdapter {
    fn id(&self) -> &'static str {
        "kiro"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join(".kiro/steering").is_dir() || root.join(".kiro/settings/mcp.json").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        let steering_dir = root.join(".kiro/steering");
        if steering_dir.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&steering_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read .kiro/steering: {e}")))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                        && e.path().extension().and_then(|ext| ext.to_str()) == Some("md")
                })
                .collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let path = entry.path();
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("steering")
                    .to_string();
                let raw = fs::read_to_string(&path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
                })?;
                let content = strip_provenance(&raw);
                let (fm_opt, body) = split_frontmatter(&content);
                let triggers = match fm_opt {
                    Some(fm) => parse_inclusion_triggers(&fm)?,
                    None => vec!["*".to_string()],
                };
                let body = body.trim().to_string();
                let sha256 = compute_sha256(&body);
                skills.push(Skill {
                    id: format!("{STEERING_ID_PREFIX}{stem}"),
                    version: "1.0.0".to_string(),
                    triggers,
                    targets: vec!["kiro".to_string()],
                    source: body,
                    sha256,
                });
            }
        }

        let mcp_path = root.join(".kiro/settings/mcp.json");
        if mcp_path.exists() {
            let meta = fs::symlink_metadata(&mcp_path).map_err(|e| {
                AdapterError::Io(format!("Failed to stat .kiro/settings/mcp.json: {e}"))
            })?;
            if meta.file_type().is_symlink() {
                return Err(AdapterError::Malformed(
                    "Refusing to follow symlinked .kiro/settings/mcp.json".to_string(),
                ));
            }
            let raw_json = fs::read_to_string(&mcp_path).map_err(|e| {
                AdapterError::Io(format!("Failed to read .kiro/settings/mcp.json: {e}"))
            })?;
            let cleaned = clean_jsonc(&raw_json);
            let parsed: Value = serde_json::from_str(&cleaned).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .kiro/settings/mcp.json: {e}"))
            })?;
            if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (id, val) in servers {
                    parse_and_validate_mcp_server(id, val, "kiro", &mut mcp_servers)?;
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

        let kiro_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"kiro".to_string()))
            .collect();

        if !kiro_skills.is_empty() {
            let steering_dir = root.join(".kiro/steering");
            fs::create_dir_all(&steering_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .kiro/steering: {e}")))?;

            for skill in kiro_skills {
                let slug = skill
                    .id
                    .strip_prefix(STEERING_ID_PREFIX)
                    .unwrap_or(&skill.id);
                let rel_path = format!(".kiro/steering/{slug}.md");
                let target_path = safe_join(root, &rel_path)?;

                let fm = if skill.triggers.is_empty() || skill.triggers == vec!["*".to_string()] {
                    "---\ninclusion: always\n---".to_string()
                } else {
                    format!(
                        "---\ninclusion: fileMatch\nfileMatchPattern: \"{}\"\n---",
                        skill.triggers[0]
                    )
                };

                let output = format!(
                    "<!-- GENERATED BY Synapse — edit in the Brain -->\n{}\n\n{}",
                    fm, skill.source
                );
                write_if_changed(&target_path, output)
                    .map_err(|e| AdapterError::Io(format!("Failed to write {rel_path}: {e}")))?;
                written.push(rel_path);
            }
        }

        let kiro_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"kiro".to_string()))
            .collect();

        if !kiro_servers.is_empty() {
            let rel_mcp = ".kiro/settings/mcp.json";
            let mcp_path = safe_join(root, rel_mcp)?;
            if let Some(parent) = mcp_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AdapterError::Io(format!("Failed to create .kiro/settings: {e}"))
                })?;
            }

            let mut current_json = if mcp_path.exists() {
                let raw_json = fs::read_to_string(&mcp_path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read .kiro/settings/mcp.json: {e}"))
                })?;
                let cleaned = clean_jsonc(&raw_json);
                serde_json::from_str(&cleaned).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .kiro/settings/mcp.json: {e}"))
                })?
            } else {
                json!({})
            };

            let servers_map = current_json
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed(
                        ".kiro/settings/mcp.json root is not an object".to_string(),
                    )
                })?
                .entry("mcpServers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("mcpServers is not an object".to_string())
                })?;

            for server in kiro_servers {
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
                for key in &server.env_placeholders {
                    env_obj.insert(key.clone(), Value::String(String::new()));
                }

                servers_map.insert(
                    server.id.clone(),
                    json!({
                        "command": command,
                        "args": args,
                        "env": env_obj
                    }),
                );
            }

            let pretty = serde_json::to_string_pretty(&current_json)
                .map_err(|e| AdapterError::Malformed(format!("Failed to serialize JSON: {e}")))?;
            write_if_changed(&mcp_path, pretty).map_err(|e| {
                AdapterError::Io(format!("Failed to write .kiro/settings/mcp.json: {e}"))
            })?;
            written.push(rel_mcp.to_string());
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
    use tempfile::tempdir;

    #[test]
    fn test_kiro_detect_and_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = KiroAdapter;
        assert!(!adapter.detect(dir.path()));

        let steering_dir = dir.path().join(".kiro/steering");
        fs::create_dir_all(&steering_dir).unwrap();
        fs::write(
            steering_dir.join("product.md"),
            "---\ninclusion: fileMatch\nfileMatchPattern: \"src/**/*.rs\"\n---\n\nPrefer zero-copy parsing.",
        )
        .unwrap();

        let settings_dir = dir.path().join(".kiro/settings");
        fs::create_dir_all(&settings_dir).unwrap();
        fs::write(
            settings_dir.join("mcp.json"),
            r#"{"mcpServers": {"aws-docs": {"command": "uvx", "args": ["awslabs.aws-documentation-mcp-server"]}}}"#,
        )
        .unwrap();

        assert!(adapter.detect(dir.path()));

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "kiro-steering-product");
        assert_eq!(imported.skills[0].triggers, vec!["src/**/*.rs".to_string()]);
        assert_eq!(imported.skills[0].source, "Prefer zero-copy parsing.");
        assert_eq!(imported.mcp_servers.len(), 1);
        assert_eq!(imported.mcp_servers[0].id, "aws-docs");

        let out_dir = tempdir().unwrap();
        let projected = adapter
            .project(out_dir.path(), &imported.skills, &[], &imported.mcp_servers)
            .unwrap();
        assert_eq!(projected.written.len(), 2);
        assert!(out_dir.path().join(".kiro/steering/product.md").exists());
        assert!(out_dir.path().join(".kiro/settings/mcp.json").exists());
    }

    #[test]
    fn test_kiro_rejects_traversal_and_symlinks() {
        let dir = tempdir().unwrap();
        let adapter = KiroAdapter;

        let skill = Skill {
            id: "kiro-steering-../../evil".to_string(),
            version: "1.0.0".to_string(),
            triggers: vec!["*".to_string()],
            targets: vec!["kiro".to_string()],
            source: "bad".to_string(),
            sha256: "hash".to_string(),
        };
        assert!(matches!(
            adapter.project(dir.path(), &[skill], &[], &[]),
            Err(AdapterError::Malformed(_))
        ));
    }
}
