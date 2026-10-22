use super::{
    clean_jsonc, compute_sha256, parse_and_validate_mcp_server, parse_mdc_frontmatter, safe_join,
    serialize_mdc_frontmatter, split_frontmatter, strip_provenance, write_if_changed,
    MdcFrontmatter,
};
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Next-Gen Adapter for Trae IDE (`.trae/rules/*.md` and `.trae/mcp.json`).
pub struct TraeAdapter;

const RULE_ID_PREFIX: &str = "trae-rule-";

impl Adapter for TraeAdapter {
    fn id(&self) -> &'static str {
        "trae"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join(".trae/rules").is_dir() || root.join(".trae/mcp.json").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut skills = Vec::new();
        let mut mcp_servers = Vec::new();

        let rules_dir = root.join(".trae/rules");
        if rules_dir.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&rules_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to read .trae/rules: {e}")))?
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
                    .unwrap_or("project_rules")
                    .to_string();
                let raw = fs::read_to_string(&path).map_err(|e| {
                    AdapterError::Io(format!("Failed to read {}: {}", path.display(), e))
                })?;
                let content = strip_provenance(&raw);
                let (fm_opt, body) = split_frontmatter(&content);
                let fm = if let Some(fm_str) = fm_opt {
                    parse_mdc_frontmatter(&fm_str)?
                } else {
                    MdcFrontmatter {
                        globs: Vec::new(),
                        always_apply: true,
                    }
                };

                let triggers = if !fm.globs.is_empty() {
                    fm.globs
                } else {
                    vec!["*".to_string()]
                };

                let body = body.trim().to_string();
                let sha256 = compute_sha256(&body);
                skills.push(Skill {
                    id: format!("{RULE_ID_PREFIX}{stem}"),
                    version: "1.0.0".to_string(),
                    triggers,
                    targets: vec!["trae".to_string()],
                    source: body,
                    sha256,
                });
            }
        }

        let mcp_path = root.join(".trae/mcp.json");
        if mcp_path.exists() {
            let meta = fs::symlink_metadata(&mcp_path)
                .map_err(|e| AdapterError::Io(format!("Failed to stat .trae/mcp.json: {e}")))?;
            if meta.file_type().is_symlink() {
                return Err(AdapterError::Malformed(
                    "Refusing to follow symlinked .trae/mcp.json".to_string(),
                ));
            }
            let raw_json = fs::read_to_string(&mcp_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read .trae/mcp.json: {e}")))?;
            let cleaned = clean_jsonc(&raw_json);
            let parsed: Value = serde_json::from_str(&cleaned).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .trae/mcp.json: {e}"))
            })?;
            if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (id, val) in servers {
                    parse_and_validate_mcp_server(id, val, "trae", &mut mcp_servers)?;
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

        let trae_skills: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.targets.contains(&"trae".to_string()))
            .collect();

        if !trae_skills.is_empty() {
            let rules_dir = root.join(".trae/rules");
            fs::create_dir_all(&rules_dir)
                .map_err(|e| AdapterError::Io(format!("Failed to create .trae/rules: {e}")))?;

            for skill in trae_skills {
                let slug = skill.id.strip_prefix(RULE_ID_PREFIX).unwrap_or(&skill.id);
                let fm = MdcFrontmatter {
                    globs: skill.triggers.clone(),
                    always_apply: skill.triggers == vec!["*".to_string()],
                };
                let output = format!(
                    "<!-- GENERATED BY Synapse — edit in the Brain -->\n{}\n\n{}",
                    serialize_mdc_frontmatter(&fm),
                    skill.source
                );
                let rel_path = format!(".trae/rules/{slug}.md");
                let target_path = safe_join(root, &rel_path)?;
                write_if_changed(&target_path, output)
                    .map_err(|e| AdapterError::Io(format!("Failed to write {rel_path}: {e}")))?;
                written.push(rel_path);
            }
        }

        let trae_servers: Vec<&McpServer> = mcp_servers
            .iter()
            .filter(|s| s.targets.contains(&"trae".to_string()))
            .collect();

        if !trae_servers.is_empty() {
            let rel_mcp = ".trae/mcp.json";
            let mcp_path = safe_join(root, rel_mcp)?;
            if let Some(parent) = mcp_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| AdapterError::Io(format!("Failed to create .trae: {e}")))?;
            }

            let mut current_json = if mcp_path.exists() {
                let raw_json = fs::read_to_string(&mcp_path)
                    .map_err(|e| AdapterError::Io(format!("Failed to read .trae/mcp.json: {e}")))?;
                let cleaned = clean_jsonc(&raw_json);
                serde_json::from_str(&cleaned).map_err(|e| {
                    AdapterError::Malformed(format!("Invalid JSON in .trae/mcp.json: {e}"))
                })?
            } else {
                json!({})
            };

            let servers_map = current_json
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed(".trae/mcp.json root is not an object".to_string())
                })?
                .entry("mcpServers")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    AdapterError::Malformed("mcpServers is not an object".to_string())
                })?;

            for server in trae_servers {
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
            write_if_changed(&mcp_path, pretty)
                .map_err(|e| AdapterError::Io(format!("Failed to write .trae/mcp.json: {e}")))?;
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
    fn test_trae_detect_and_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = TraeAdapter;
        assert!(!adapter.detect(dir.path()));

        let rules_dir = dir.path().join(".trae/rules");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::write(
            rules_dir.join("project_rules.md"),
            "---\ndescription: style\nglobs: [\"*.ts\"]\nalwaysApply: false\n---\n\nUse strict TypeScript.",
        )
        .unwrap();

        assert!(adapter.detect(dir.path()));

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "trae-rule-project_rules");
        assert_eq!(imported.skills[0].triggers, vec!["*.ts".to_string()]);
        assert_eq!(imported.skills[0].source, "Use strict TypeScript.");

        let out_dir = tempdir().unwrap();
        let projected = adapter
            .project(out_dir.path(), &imported.skills, &[], &[])
            .unwrap();
        assert_eq!(projected.written, vec![".trae/rules/project_rules.md"]);
        assert!(out_dir.path().join(".trae/rules/project_rules.md").exists());
    }
}
