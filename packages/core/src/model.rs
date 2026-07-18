//! Canonical Brain model — the in-memory shape of `AIBrain/` described in
//! MASTER_PROMPT.md §1. Every adapter imports into these types and projects
//! out of them; no tool-specific format leaks past the adapter boundary.

/// `AIBrain/skills/<slug>/skill.yaml`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: String,
    pub version: String,
    pub triggers: Vec<String>,
    pub targets: Vec<String>,
    pub source: String,
    pub sha256: String,
}

/// `AIBrain/agents/<slug>.md` frontmatter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub slug: String,
    pub tools: Vec<String>,
    pub model_hints: Vec<String>,
    pub targets: Vec<String>,
}

/// `AIBrain/mcp/servers/<id>.yaml`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub id: String,
    pub transport: String,
    pub command_or_url: String,
    pub env_placeholders: Vec<String>,
    pub targets: Vec<String>,
    pub health: HealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Unreachable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_round_trips_through_plain_construction() {
        let skill = Skill {
            id: "repo-conventions".into(),
            version: "1.0.0".into(),
            triggers: vec!["*.rs".into()],
            targets: vec!["claude-code".into(), "cursor".into()],
            source: "AIBrain/skills/repo-conventions".into(),
            sha256: "deadbeef".into(),
        };
        assert_eq!(skill.targets.len(), 2);
        assert_eq!(skill.id, "repo-conventions");
    }

    #[test]
    fn mcp_server_defaults_to_unknown_health() {
        let server = McpServer {
            id: "filesystem".into(),
            transport: "stdio".into(),
            command_or_url: "npx @modelcontextprotocol/server-filesystem".into(),
            env_placeholders: vec![],
            targets: vec!["claude-code".into()],
            health: HealthStatus::Unknown,
        };
        assert_eq!(server.health, HealthStatus::Unknown);
    }
}
