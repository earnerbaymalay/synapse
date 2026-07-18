use super::compute_sha256;
use crate::adapter::{Adapter, AdapterError, ImportResult, ProjectResult};
use crate::model::{Agent, McpServer, Skill};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct RooCodeAdapter;

/// Roo Code's own artifact is `.roomodes` (custom mode definitions). Its
/// `.clinerules` compatibility file is deliberately left to `ClineAdapter` —
/// both tools read the same filename, so owning it here too would double-import
/// the same content under two adapters.
impl Adapter for RooCodeAdapter {
    fn id(&self) -> &'static str {
        "roo-code"
    }

    fn detect(&self, root: &Path) -> bool {
        root.join(".roomodes").exists()
    }

    fn import(&self, root: &Path) -> Result<ImportResult, AdapterError> {
        let mut agents = Vec::new();
        let mut skills = Vec::new();

        let modes_path = root.join(".roomodes");
        if modes_path.exists() {
            let raw = fs::read_to_string(&modes_path)
                .map_err(|e| AdapterError::Io(format!("Failed to read .roomodes: {}", e)))?;
            let parsed: Value = serde_json::from_str(&raw).map_err(|e| {
                AdapterError::Malformed(format!("Invalid JSON in .roomodes: {}", e))
            })?;

            let modes = parsed
                .get("customModes")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    AdapterError::Malformed("customModes is missing or not an array".to_string())
                })?;

            for mode in modes {
                let slug = mode
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AdapterError::Malformed("custom mode missing `slug`".to_string())
                    })?
                    .to_string();

                let groups: Vec<String> = mode
                    .get("groups")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| item.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let role_definition = mode
                    .get("roleDefinition")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let custom_instructions = mode
                    .get("customInstructions")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                agents.push(Agent {
                    slug: slug.clone(),
                    tools: groups,
                    model_hints: Vec::new(),
                    targets: vec!["roo-code".to_string()],
                });

                let body = if custom_instructions.is_empty() {
                    role_definition.to_string()
                } else if role_definition.is_empty() {
                    custom_instructions.to_string()
                } else {
                    format!("{}\n\n{}", role_definition, custom_instructions)
                };
                let sha256 = compute_sha256(&body);

                skills.push(Skill {
                    id: format!("roo-mode-{}", slug),
                    version: "1.0.0".to_string(),
                    triggers: vec!["*".to_string()],
                    targets: vec!["roo-code".to_string()],
                    source: body,
                    sha256,
                });
            }
        }

        Ok(ImportResult {
            skills,
            agents,
            mcp_servers: Vec::new(),
        })
    }

    fn project(
        &self,
        root: &Path,
        skills: &[Skill],
        agents: &[Agent],
        _mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError> {
        let mut written = Vec::new();

        let roo_agents: Vec<&Agent> = agents
            .iter()
            .filter(|a| a.targets.contains(&"roo-code".to_string()))
            .collect();

        if !roo_agents.is_empty() {
            let mut custom_modes = Vec::new();
            for agent in &roo_agents {
                let companion = skills
                    .iter()
                    .find(|s| s.id == format!("roo-mode-{}", agent.slug));
                let (role_definition, custom_instructions) = match companion {
                    Some(s) => match s.source.split_once("\n\n") {
                        Some((role, rest)) => (role.to_string(), rest.to_string()),
                        None => (s.source.clone(), String::new()),
                    },
                    None => (String::new(), String::new()),
                };

                custom_modes.push(json!({
                    "slug": agent.slug,
                    "roleDefinition": role_definition,
                    "customInstructions": custom_instructions,
                    "groups": agent.tools,
                }));
            }

            let output = json!({ "customModes": custom_modes });
            let pretty = serde_json::to_string_pretty(&output).map_err(|e| {
                AdapterError::Malformed(format!("Failed to serialize .roomodes: {}", e))
            })?;
            fs::write(root.join(".roomodes"), pretty)
                .map_err(|e| AdapterError::Io(format!("Failed to write .roomodes: {}", e)))?;
            written.push(".roomodes".to_string());
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
    fn test_roo_code_detect() {
        let dir = tempdir().unwrap();
        let adapter = RooCodeAdapter;
        assert!(!adapter.detect(dir.path()));

        fs::write(dir.path().join(".roomodes"), "{}").unwrap();
        assert!(adapter.detect(dir.path()));
    }

    #[test]
    fn test_roo_code_does_not_claim_clinerules() {
        let dir = tempdir().unwrap();
        let adapter = RooCodeAdapter;
        fs::write(dir.path().join(".clinerules"), "shared with cline").unwrap();
        assert!(!adapter.detect(dir.path()));
    }

    #[test]
    fn test_roo_code_import_export_roundtrip() {
        let dir = tempdir().unwrap();
        let adapter = RooCodeAdapter;

        let modes_json = r#"{
            "customModes": [
                {
                    "slug": "designer",
                    "name": "Designer",
                    "roleDefinition": "You design UI components.",
                    "customInstructions": "Prefer Tailwind utility classes.",
                    "groups": ["read", "edit"]
                }
            ]
        }"#;
        fs::write(dir.path().join(".roomodes"), modes_json).unwrap();

        let imported = adapter.import(dir.path()).unwrap();
        assert_eq!(imported.agents.len(), 1);
        assert_eq!(imported.agents[0].slug, "designer");
        assert_eq!(imported.agents[0].tools, vec!["read", "edit"]);

        assert_eq!(imported.skills.len(), 1);
        assert_eq!(imported.skills[0].id, "roo-mode-designer");
        assert!(imported.skills[0]
            .source
            .contains("You design UI components."));
        assert!(imported.skills[0]
            .source
            .contains("Prefer Tailwind utility classes."));

        let out_dir = tempdir().unwrap();
        let project_res = adapter
            .project(out_dir.path(), &imported.skills, &imported.agents, &[])
            .unwrap();

        assert_eq!(project_res.written, vec![".roomodes".to_string()]);

        let projected = fs::read_to_string(out_dir.path().join(".roomodes")).unwrap();
        let parsed: Value = serde_json::from_str(&projected).unwrap();
        assert_eq!(parsed["customModes"][0]["slug"], "designer");
        assert_eq!(
            parsed["customModes"][0]["roleDefinition"],
            "You design UI components."
        );
        assert_eq!(
            parsed["customModes"][0]["customInstructions"],
            "Prefer Tailwind utility classes."
        );
        assert_eq!(parsed["customModes"][0]["groups"][0], "read");
    }

    #[test]
    fn test_roo_code_stress_malformed_json() {
        let dir = tempdir().unwrap();
        let adapter = RooCodeAdapter;
        fs::write(dir.path().join(".roomodes"), "{ not json").unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_roo_code_stress_missing_custom_modes_key() {
        let dir = tempdir().unwrap();
        let adapter = RooCodeAdapter;
        fs::write(dir.path().join(".roomodes"), "{}").unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }

    #[test]
    fn test_roo_code_stress_mode_missing_slug() {
        let dir = tempdir().unwrap();
        let adapter = RooCodeAdapter;
        fs::write(
            dir.path().join(".roomodes"),
            r#"{"customModes": [{"name": "no slug"}]}"#,
        )
        .unwrap();
        let res = adapter.import(dir.path());
        assert!(matches!(res, Err(AdapterError::Malformed(_))));
    }
}
